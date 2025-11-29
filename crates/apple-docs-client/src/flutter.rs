use std::{path::PathBuf, time::Duration as StdDuration};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use directories::ProjectDirs;
use reqwest::Client;
use std::collections::HashMap;
use thiserror::Error;
use time::Duration;
use tokio::sync::Mutex;
use tracing::{debug, instrument, warn};

use crate::cache::{DiskCache, MemoryCache};

const FLUTTER_BASE_URL: &str = "https://api.flutter.dev/flutter";
#[allow(dead_code)]
const DART_BASE_URL: &str = "https://api.dart.dev/stable";
const FLUTTER_INDEX_KEY: &str = "flutter_index";

#[derive(Debug, Clone, Error)]
pub enum FlutterClientError {
    #[error("HTTP request failed: {0}")]
    Http(String),
    #[error("unexpected status code: {0}")]
    Status(u16),
    #[error("cache miss")]
    CacheMiss,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlutterIndexItem {
    pub name: String,
    #[serde(rename = "qualifiedName")]
    pub qualified_name: String,
    pub href: String,
    #[serde(rename = "type")]
    pub kind: Option<String>,
    #[serde(rename = "overriddenDepth")]
    pub overridden_depth: Option<i32>,
    #[serde(rename = "packageRank")]
    pub package_rank: Option<i32>,
    #[serde(rename = "desc")]
    pub description: Option<String>,
    #[serde(rename = "enclosedBy")]
    pub enclosed_by: Option<EnclosedBy>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnclosedBy {
    pub name: String,
    #[serde(rename = "type")]
    pub kind: Option<String>,
    pub href: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlutterLibrary {
    pub name: String,
    pub href: String,
    pub description: Option<String>,
    pub symbols: Vec<FlutterIndexItem>,
}

#[derive(Debug, Clone)]
pub struct FlutterClientConfig {
    pub cache_dir: PathBuf,
    pub memory_cache_ttl: Duration,
}

impl Default for FlutterClientConfig {
    fn default() -> Self {
        let project_dirs = ProjectDirs::from("com", "RecordAndLearn", "dev-docs-mcp")
            .expect("unable to resolve project directories");

        Self {
            cache_dir: project_dirs.cache_dir().join("flutter"),
            memory_cache_ttl: Duration::minutes(10),
        }
    }
}

#[derive(Debug)]
pub struct FlutterDocsClient {
    http: Client,
    disk_cache: DiskCache,
    index_lock: Mutex<()>,
    memory_cache: MemoryCache<Vec<u8>>,
    config: FlutterClientConfig,
}

impl FlutterDocsClient {
    pub fn with_config(config: FlutterClientConfig) -> Self {
        let http = Client::builder()
            .user_agent("DevDocsMCP/1.0")
            .timeout(StdDuration::from_secs(30))
            .gzip(true)
            .build()
            .expect("failed to build reqwest client");

        if let Err(error) = std::fs::create_dir_all(&config.cache_dir) {
            warn!(
                error = %error,
                cache_dir = %config.cache_dir.display(),
                "failed to create Flutter cache directory"
            );
        }

        let disk_cache = DiskCache::new(&config.cache_dir);
        Self {
            http,
            disk_cache,
            index_lock: Mutex::new(()),
            memory_cache: MemoryCache::new(config.memory_cache_ttl),
            config,
        }
    }

    #[must_use]
    pub fn new() -> Self {
        Self::with_config(FlutterClientConfig::default())
    }

    pub fn cache_dir(&self) -> &PathBuf {
        &self.config.cache_dir
    }

    #[instrument(name = "flutter_client.get_index", skip(self))]
    pub async fn get_index(&self) -> Result<Vec<FlutterIndexItem>> {
        let file_name = format!("{FLUTTER_INDEX_KEY}.json");

        if let Some(entry) = self.disk_cache.load::<Vec<FlutterIndexItem>>(&file_name).await? {
            debug!("Flutter index served from disk cache");
            return Ok(entry.value);
        }

        let _lock = self.index_lock.lock().await;
        if let Some(entry) = self.disk_cache.load::<Vec<FlutterIndexItem>>(&file_name).await? {
            debug!("Flutter index served from disk cache after lock");
            return Ok(entry.value);
        }

        let index = self.fetch_index().await?;
        self.disk_cache.store(&file_name, index.clone()).await?;
        Ok(index)
    }

    #[instrument(name = "flutter_client.refresh_index", skip(self))]
    pub async fn refresh_index(&self) -> Result<Vec<FlutterIndexItem>> {
        let index = self.fetch_index().await?;
        let file_name = format!("{FLUTTER_INDEX_KEY}.json");
        self.disk_cache.store(&file_name, index.clone()).await?;
        Ok(index)
    }

    async fn fetch_index(&self) -> Result<Vec<FlutterIndexItem>> {
        let url = format!("{FLUTTER_BASE_URL}/index.json");

        if let Some(bytes) = self.memory_cache.get(&url) {
            let items = serde_json::from_slice(&bytes)
                .with_context(|| format!("failed to parse cached Flutter index"))?;
            return Ok(items);
        }

        let response = self
            .http
            .get(&url)
            .send()
            .await
            .map_err(|err| FlutterClientError::Http(err.to_string()))?;

        if !response.status().is_success() {
            warn!(status = %response.status(), url, "Flutter docs request failed");
            return Err(FlutterClientError::Status(response.status().as_u16()).into());
        }

        let bytes = response
            .bytes()
            .await
            .map_err(|err| FlutterClientError::Http(err.to_string()))?;
        self.memory_cache.insert(url.clone(), bytes.to_vec());

        let items: Vec<FlutterIndexItem> = serde_json::from_slice(&bytes)
            .with_context(|| format!("failed to parse Flutter index from {url}"))?;
        Ok(items)
    }

    #[instrument(name = "flutter_client.get_libraries", skip(self))]
    pub async fn get_libraries(&self) -> Result<Vec<FlutterLibrary>> {
        let index = self.get_index().await?;

        let mut libraries: HashMap<String, FlutterLibrary> = HashMap::new();

        for item in &index {
            if item.kind.as_deref() == Some("library") {
                libraries.insert(item.qualified_name.clone(), FlutterLibrary {
                    name: item.name.clone(),
                    href: item.href.clone(),
                    description: item.description.clone(),
                    symbols: Vec::new(),
                });
            }
        }

        for item in &index {
            if item.kind.as_deref() != Some("library") {
                if let Some(_enclosed) = &item.enclosed_by {
                    let lib_name = item.qualified_name
                        .split('.')
                        .next()
                        .unwrap_or(&item.qualified_name);

                    if let Some(library) = libraries.get_mut(lib_name) {
                        library.symbols.push(item.clone());
                    }
                }
            }
        }

        Ok(libraries.into_values().collect())
    }

    #[instrument(name = "flutter_client.search", skip(self))]
    pub async fn search(&self, query: &str, max_results: usize) -> Result<Vec<FlutterIndexItem>> {
        let index = self.get_index().await?;
        let query_lower = query.to_lowercase();

        let mut results: Vec<(i32, FlutterIndexItem)> = index
            .into_iter()
            .filter_map(|item| {
                let name_lower = item.name.to_lowercase();
                let qualified_lower = item.qualified_name.to_lowercase();

                let score = if name_lower == query_lower {
                    100
                } else if name_lower.starts_with(&query_lower) {
                    80
                } else if qualified_lower.contains(&query_lower) {
                    60
                } else if name_lower.contains(&query_lower) {
                    40
                } else {
                    return None;
                };

                let package_boost = item.package_rank.unwrap_or(0);
                Some((score + package_boost, item))
            })
            .collect();

        results.sort_by(|a, b| b.0.cmp(&a.0));

        Ok(results.into_iter().take(max_results).map(|(_, item)| item).collect())
    }

    pub fn clear_memory_cache(&self) {
        self.memory_cache.clear();
    }
}

impl Default for FlutterDocsClient {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn defaults_provide_cache_dir() {
        let client = FlutterDocsClient::new();
        assert!(client.cache_dir().to_string_lossy().contains("flutter"));
    }
}
