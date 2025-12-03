use std::path::PathBuf;
use std::time::Duration as StdDuration;

use anyhow::{Context, Result};
use directories::ProjectDirs;
use reqwest::Client;
use tokio::sync::Mutex;
use tracing::{debug, instrument};

use super::types::{
    TelegramApiSpec, TelegramCategory, TelegramCategoryItem, TelegramItem, TelegramTechnology,
};
use docs_mcp_client::cache::{DiskCache, MemoryCache};

const SPEC_URL: &str =
    "https://raw.githubusercontent.com/PaulSonOfLars/telegram-bot-api-spec/main/api.json";
const CACHE_KEY: &str = "telegram_api_spec";

#[derive(Debug)]
pub struct TelegramClient {
    http: Client,
    disk_cache: DiskCache,
    memory_cache: MemoryCache<Vec<u8>>,
    spec_lock: Mutex<()>,
    cache_dir: PathBuf,
}

impl Default for TelegramClient {
    fn default() -> Self {
        Self::new()
    }
}

impl TelegramClient {
    #[must_use]
    pub fn new() -> Self {
        let project_dirs = ProjectDirs::from("com", "RecordAndLearn", "multi-docs-mcp")
            .expect("unable to resolve project directories");

        let cache_dir = project_dirs.cache_dir().join("telegram");
        if let Err(e) = std::fs::create_dir_all(&cache_dir) {
            tracing::warn!(error = %e, "Failed to create Telegram cache directory");
        }

        let http = Client::builder()
            .user_agent("MultiDocsMCP/1.0")
            .timeout(StdDuration::from_secs(30))
            .gzip(true)
            .build()
            .expect("failed to build reqwest client");

        Self {
            http,
            disk_cache: DiskCache::new(&cache_dir),
            memory_cache: MemoryCache::new(time::Duration::minutes(30)),
            spec_lock: Mutex::new(()),
            cache_dir,
        }
    }

    /// Fetch the Telegram Bot API specification
    #[instrument(name = "telegram_client.get_spec", skip(self))]
    async fn get_spec(&self) -> Result<TelegramApiSpec> {
        let cache_key = format!("{CACHE_KEY}.json");

        // Check disk cache
        if let Ok(Some(entry)) = self.disk_cache.load::<TelegramApiSpec>(&cache_key).await {
            debug!("Telegram API spec served from disk cache");
            return Ok(entry.value);
        }

        // Lock to prevent concurrent fetches
        let _lock = self.spec_lock.lock().await;

        // Double-check after acquiring lock
        if let Ok(Some(entry)) = self.disk_cache.load::<TelegramApiSpec>(&cache_key).await {
            debug!("Telegram API spec served from disk cache (after lock)");
            return Ok(entry.value);
        }

        // Fetch from remote
        debug!(url = SPEC_URL, "Fetching Telegram API spec");
        let response = self
            .http
            .get(SPEC_URL)
            .send()
            .await
            .context("Failed to fetch Telegram API spec")?;

        if !response.status().is_success() {
            anyhow::bail!("Telegram API spec fetch failed: {}", response.status());
        }

        let spec: TelegramApiSpec = response
            .json()
            .await
            .context("Failed to parse Telegram API spec")?;

        // Store in cache
        self.disk_cache.store(&cache_key, spec.clone()).await?;

        Ok(spec)
    }

    /// Get available technologies (categories)
    #[instrument(name = "telegram_client.get_technologies", skip(self))]
    pub async fn get_technologies(&self) -> Result<Vec<TelegramTechnology>> {
        let spec = self.get_spec().await?;

        let methods_tech = TelegramTechnology {
            identifier: "telegram:methods".to_string(),
            title: "Telegram Bot API Methods".to_string(),
            description: format!(
                "Bot API {} - {} methods for interacting with Telegram",
                spec.version,
                spec.methods.len()
            ),
            url: "https://core.telegram.org/bots/api#available-methods".to_string(),
            item_count: spec.methods.len(),
        };

        let types_tech = TelegramTechnology {
            identifier: "telegram:types".to_string(),
            title: "Telegram Bot API Types".to_string(),
            description: format!("Bot API {} - {} type definitions", spec.version, spec.types.len()),
            url: "https://core.telegram.org/bots/api#available-types".to_string(),
            item_count: spec.types.len(),
        };

        Ok(vec![methods_tech, types_tech])
    }

    /// Get a category (methods or types)
    #[instrument(name = "telegram_client.get_category", skip(self))]
    pub async fn get_category(&self, identifier: &str) -> Result<TelegramCategory> {
        let spec = self.get_spec().await?;

        match identifier {
            "telegram:methods" | "methods" => {
                let items = spec
                    .methods
                    .iter()
                    .map(|(name, m)| TelegramCategoryItem {
                        name: name.clone(),
                        description: m.description.first().cloned().unwrap_or_default(),
                        kind: "method".to_string(),
                        href: m.href.clone(),
                    })
                    .collect();

                Ok(TelegramCategory {
                    identifier: "telegram:methods".to_string(),
                    title: "Telegram Bot API Methods".to_string(),
                    description: format!("Bot API {} - Available Methods", spec.version),
                    items,
                })
            }
            "telegram:types" | "types" => {
                let items = spec
                    .types
                    .iter()
                    .map(|(name, t)| TelegramCategoryItem {
                        name: name.clone(),
                        description: t.description.first().cloned().unwrap_or_default(),
                        kind: "type".to_string(),
                        href: t.href.clone(),
                    })
                    .collect();

                Ok(TelegramCategory {
                    identifier: "telegram:types".to_string(),
                    title: "Telegram Bot API Types".to_string(),
                    description: format!("Bot API {} - Available Types", spec.version),
                    items,
                })
            }
            _ => anyhow::bail!("Unknown Telegram category: {identifier}"),
        }
    }

    /// Get a specific method or type by name
    #[instrument(name = "telegram_client.get_item", skip(self))]
    pub async fn get_item(&self, name: &str) -> Result<TelegramItem> {
        let spec = self.get_spec().await?;

        // Try to find as method first
        if let Some(method) = spec.methods.get(name) {
            return Ok(TelegramItem::from_method(name, method));
        }

        // Try to find as type
        if let Some(t) = spec.types.get(name) {
            return Ok(TelegramItem::from_type(name, t));
        }

        anyhow::bail!("Telegram item not found: {name}")
    }

    /// Search for methods and types matching a query
    #[instrument(name = "telegram_client.search", skip(self))]
    pub async fn search(&self, query: &str) -> Result<Vec<TelegramItem>> {
        let spec = self.get_spec().await?;
        let query_lower = query.to_lowercase();

        // Split query into individual keywords for better matching
        let keywords: Vec<&str> = query_lower
            .split(|c: char| c.is_whitespace() || c == '-' || c == '_')
            .filter(|s| !s.is_empty() && s.len() > 1)
            .collect();

        let mut scored_results: Vec<(i32, TelegramItem)> = Vec::new();

        // Search methods
        for (name, method) in &spec.methods {
            let name_lower = name.to_lowercase();
            let description_text = method.description.join(" ").to_lowercase();

            let mut score = 0i32;
            for keyword in &keywords {
                // Exact name match gets highest score
                if name_lower == *keyword {
                    score += 50;
                }
                // Name contains keyword
                else if name_lower.contains(keyword) {
                    score += 20;
                }
                // Description contains keyword
                if description_text.contains(keyword) {
                    score += 5;
                }
                // Parameter names contain keyword
                for field in &method.fields {
                    if field.name.to_lowercase().contains(keyword) {
                        score += 3;
                    }
                }
            }

            if score > 0 {
                scored_results.push((score, TelegramItem::from_method(name, method)));
            }
        }

        // Search types
        for (name, t) in &spec.types {
            let name_lower = name.to_lowercase();
            let description_text = t.description.join(" ").to_lowercase();

            let mut score = 0i32;
            for keyword in &keywords {
                // Exact name match gets highest score
                if name_lower == *keyword {
                    score += 50;
                }
                // Name contains keyword
                else if name_lower.contains(keyword) {
                    score += 20;
                }
                // Description contains keyword
                if description_text.contains(keyword) {
                    score += 5;
                }
                // Field names contain keyword
                for field in &t.fields {
                    if field.name.to_lowercase().contains(keyword) {
                        score += 3;
                    }
                }
            }

            if score > 0 {
                scored_results.push((score, TelegramItem::from_type(name, t)));
            }
        }

        // Sort by score (highest first) and return items
        scored_results.sort_by(|a, b| b.0.cmp(&a.0));
        let results = scored_results.into_iter().map(|(_, item)| item).collect();

        Ok(results)
    }

    /// Get the API version
    pub async fn get_version(&self) -> Result<String> {
        let spec = self.get_spec().await?;
        Ok(spec.version)
    }

    pub fn cache_dir(&self) -> &PathBuf {
        &self.cache_dir
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_creation() {
        let _client = TelegramClient::new();
    }
}
