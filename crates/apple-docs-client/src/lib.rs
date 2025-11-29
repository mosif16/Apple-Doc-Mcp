pub mod cache;
pub mod types;

// Re-export commonly used cache types
pub use cache::CombinedCacheStats;

use std::{path::PathBuf, time::Duration as StdDuration};

use anyhow::{anyhow, Context, Result};
use cache::{DiskCache, MemoryCache};
use directories::ProjectDirs;
use reqwest::{Client, StatusCode};
use serde_json::Value;
use std::collections::HashMap;
use thiserror::Error;
use time::Duration;
use tokio::sync::Mutex;
use tracing::{debug, instrument, warn};

use crate::types::{FrameworkData, SymbolData, Technology};

const BASE_URL: &str = "https://developer.apple.com/tutorials/data";
const TECHNOLOGIES_KEY: &str = "technologies";

#[derive(Debug, Clone, Error)]
pub enum ClientError {
    #[error("HTTP request failed: {0}")]
    Http(String),
    #[error("unexpected status code: {0}")]
    Status(StatusCode),
    #[error("cache miss")]
    CacheMiss,
}

#[derive(Debug, Clone)]
pub struct ClientConfig {
    pub cache_dir: PathBuf,
    pub memory_cache_ttl: Duration,
}

impl Default for ClientConfig {
    fn default() -> Self {
        let project_dirs = ProjectDirs::from("com", "RecordAndLearn", "apple-docs-mcp")
            .expect("unable to resolve project directories");

        Self {
            cache_dir: project_dirs.cache_dir().to_path_buf(),
            memory_cache_ttl: Duration::minutes(10),
        }
    }
}

#[derive(Debug)]
pub struct AppleDocsClient {
    http: Client,
    disk_cache: DiskCache,
    technologies_lock: Mutex<()>,
    frameworks_lock: Mutex<()>,
    memory_cache: MemoryCache<Vec<u8>>,
    config: ClientConfig,
}

impl Default for AppleDocsClient {
    fn default() -> Self {
        Self::new()
    }
}

impl AppleDocsClient {
    pub fn with_config(config: ClientConfig) -> Self {
        let http = Client::builder()
            .user_agent("AppleDocsMCP/1.0")
            .timeout(StdDuration::from_secs(15))
            .gzip(true)
            .build()
            .expect("failed to build reqwest client");

        if let Err(error) = std::fs::create_dir_all(&config.cache_dir) {
            warn!(
                error = %error,
                cache_dir = %config.cache_dir.display(),
                "failed to create cache directory; proceeding but disk cache writes may fail"
            );
        }

        let disk_cache = DiskCache::new(&config.cache_dir);
        Self {
            http,
            disk_cache,
            technologies_lock: Mutex::new(()),
            frameworks_lock: Mutex::new(()),
            memory_cache: MemoryCache::new(config.memory_cache_ttl),
            config,
        }
    }

    #[must_use]
    pub fn new() -> Self {
        Self::with_config(ClientConfig::default())
    }

    pub fn cache_dir(&self) -> &PathBuf {
        &self.config.cache_dir
    }

    #[instrument(name = "apple_docs_client.get_framework", skip(self))]
    pub async fn get_framework(&self, framework: &str) -> Result<FrameworkData> {
        let file_name = format!("{}.json", framework);
        if let Some(entry) = self.disk_cache.load::<FrameworkData>(&file_name).await? {
            debug!(framework, "framework served from disk cache");
            return Ok(entry.value);
        }

        let _lock = self.frameworks_lock.lock().await;
        if let Some(entry) = self.disk_cache.load::<FrameworkData>(&file_name).await? {
            debug!(framework, "framework served from disk cache after lock");
            return Ok(entry.value);
        }

        let data: FrameworkData = self
            .fetch_json(&format!("documentation/{framework}.json"))
            .await?;
        self.disk_cache.store(&file_name, data.clone()).await?;
        Ok(data)
    }

    #[instrument(name = "apple_docs_client.refresh_framework", skip(self))]
    pub async fn refresh_framework(&self, framework: &str) -> Result<FrameworkData> {
        let data: FrameworkData = self
            .fetch_json(&format!("documentation/{framework}.json"))
            .await?;
        let file_name = format!("{}.json", framework);
        self.disk_cache.store(&file_name, data.clone()).await?;
        Ok(data)
    }

    #[instrument(name = "apple_docs_client.get_symbol", skip(self))]
    pub async fn get_symbol(&self, path: &str) -> Result<SymbolData> {
        let value = self.load_document(path).await?;
        let symbol = serde_json::from_value::<SymbolData>(value.clone())
            .with_context(|| format!("failed to deserialize symbol at {path}"))?;
        Ok(symbol)
    }

    #[instrument(name = "apple_docs_client.get_technologies", skip(self))]
    pub async fn get_technologies(&self) -> Result<HashMap<String, Technology>> {
        let file_name = format!("{TECHNOLOGIES_KEY}.json");
        if let Some(entry) = self.disk_cache.load::<Value>(&file_name).await? {
            if let Ok((parsed, needs_rewrite)) = Self::extract_technologies(entry.value.clone()) {
                if needs_rewrite {
                    self.disk_cache.store(&file_name, parsed.clone()).await?;
                }
                return Ok(parsed);
            }
        }

        let _lock = self.technologies_lock.lock().await;
        if let Some(entry) = self.disk_cache.load::<Value>(&file_name).await? {
            if let Ok((parsed, needs_rewrite)) = Self::extract_technologies(entry.value.clone()) {
                if needs_rewrite {
                    self.disk_cache.store(&file_name, parsed.clone()).await?;
                }
                return Ok(parsed);
            }
        }

        let value: Value = self
            .fetch_json("documentation/technologies.json")
            .await
            .context("failed to fetch technologies payload")?;
        let (parsed, _) = Self::extract_technologies(value)?;
        self.disk_cache.store(&file_name, parsed.clone()).await?;
        Ok(parsed)
    }

    pub async fn refresh_technologies(&self) -> Result<HashMap<String, Technology>> {
        let value: Value = self
            .fetch_json("documentation/technologies.json")
            .await
            .context("failed to download technologies payload")?;
        let (data, _) = Self::extract_technologies(value)?;
        self.disk_cache
            .store(&format!("{TECHNOLOGIES_KEY}.json"), data.clone())
            .await?;
        Ok(data)
    }

    pub fn clear_memory_cache(&self) {
        self.memory_cache.clear();
    }

    /// Get combined cache statistics from both memory and disk caches
    pub fn cache_stats(&self) -> cache::CombinedCacheStats {
        cache::CombinedCacheStats {
            memory: self.memory_cache.stats().snapshot(),
            disk: self.disk_cache.stats().snapshot(),
        }
    }

    pub async fn load_document(&self, path: &str) -> Result<Value> {
        let clean = path.trim_start_matches('/');
        let safe = clean.replace('/', "__");
        let file_name = format!("{safe}.json");

        if let Some(entry) = self.disk_cache.load::<Value>(&file_name).await? {
            debug!(document = clean, "documentation served from disk cache");
            return Ok(entry.value);
        }

        let data: Value = self.fetch_json(&format!("{clean}.json")).await?;
        self.disk_cache.store(&file_name, data.clone()).await?;
        Ok(data)
    }

    async fn fetch_json<T>(&self, path: &str) -> Result<T>
    where
        T: serde::de::DeserializeOwned,
    {
        let url = format!("{BASE_URL}/{path}");

        if let Some(bytes) = self.memory_cache.get_with_size(&url, |v| v.len()) {
            let value = serde_json::from_slice(&bytes)
                .with_context(|| format!("failed to parse cached json for {url}"))?;
            return Ok(value);
        }

        let response = self
            .http
            .get(&url)
            .send()
            .await
            .map_err(|err| ClientError::Http(err.to_string()))?;
        if !response.status().is_success() {
            warn!(status = %response.status(), url, "Apple docs request failed");
            return Err(ClientError::Status(response.status()).into());
        }

        let bytes = response
            .bytes()
            .await
            .map_err(|err| ClientError::Http(err.to_string()))?;
        self.memory_cache.insert(url.clone(), bytes.to_vec());

        let value = serde_json::from_slice::<T>(&bytes)
            .with_context(|| format!("failed to parse json from {url}"))?;
        Ok(value)
    }

    fn extract_technologies(value: Value) -> Result<(HashMap<String, Technology>, bool)> {
        if let Some(object) = value.as_object() {
            if let Some(references) = object.get("references") {
                let map = references
                    .as_object()
                    .ok_or_else(|| anyhow!("technologies references not an object"))?;
                let mut parsed = HashMap::new();
                for (key, value) in map {
                    if let Ok(tech) = serde_json::from_value::<Technology>(value.clone()) {
                        if tech.role == "collection" {
                            parsed.insert(key.clone(), tech);
                        }
                    }
                }
                return Ok((parsed, true));
            }

            let mut parsed = HashMap::new();
            for (key, value) in object {
                if let Ok(tech) = serde_json::from_value::<Technology>(value.clone()) {
                    parsed.insert(key.clone(), tech);
                }
            }
            return Ok((parsed, false));
        }

        Err(anyhow!("unexpected technologies payload structure"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn defaults_provide_cache_dir() {
        let client = AppleDocsClient::new();
        assert!(client.cache_dir().exists());
    }
}
