use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration as StdDuration;

use anyhow::{Context, Result};
use directories::ProjectDirs;
use reqwest::Client;
use tokio::sync::Mutex;
use tracing::{debug, instrument};

use super::types::{OpenApiSpec, TonCategory, TonEndpoint, TonEndpointSummary, TonTechnology};
use apple_docs_client::cache::{DiskCache, MemoryCache};

const OPENAPI_URL: &str = "https://raw.githubusercontent.com/tonkeeper/opentonapi/master/api/openapi.yml";
const CACHE_KEY: &str = "ton_openapi_spec";

#[derive(Debug)]
pub struct TonClient {
    http: Client,
    disk_cache: DiskCache,
    memory_cache: MemoryCache<Vec<u8>>,
    spec_lock: Mutex<()>,
    cache_dir: PathBuf,
}

impl Default for TonClient {
    fn default() -> Self {
        Self::new()
    }
}

impl TonClient {
    #[must_use]
    pub fn new() -> Self {
        let project_dirs = ProjectDirs::from("com", "RecordAndLearn", "multi-docs-mcp")
            .expect("unable to resolve project directories");

        let cache_dir = project_dirs.cache_dir().join("ton");
        if let Err(e) = std::fs::create_dir_all(&cache_dir) {
            tracing::warn!(error = %e, "Failed to create TON cache directory");
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

    /// Fetch the TON API OpenAPI specification
    #[instrument(name = "ton_client.get_spec", skip(self))]
    async fn get_spec(&self) -> Result<OpenApiSpec> {
        let cache_key = format!("{CACHE_KEY}.json");

        // Check disk cache (we store as JSON after parsing YAML)
        if let Ok(Some(entry)) = self.disk_cache.load::<OpenApiSpec>(&cache_key).await {
            debug!("TON OpenAPI spec served from disk cache");
            return Ok(entry.value);
        }

        // Lock to prevent concurrent fetches
        let _lock = self.spec_lock.lock().await;

        // Double-check after acquiring lock
        if let Ok(Some(entry)) = self.disk_cache.load::<OpenApiSpec>(&cache_key).await {
            debug!("TON OpenAPI spec served from disk cache (after lock)");
            return Ok(entry.value);
        }

        // Fetch from remote (YAML format)
        debug!(url = OPENAPI_URL, "Fetching TON OpenAPI spec (YAML)");
        let response = self
            .http
            .get(OPENAPI_URL)
            .send()
            .await
            .context("Failed to fetch TON OpenAPI spec")?;

        if !response.status().is_success() {
            anyhow::bail!("TON OpenAPI spec fetch failed: {}", response.status());
        }

        let yaml_text = response
            .text()
            .await
            .context("Failed to read TON OpenAPI response")?;

        // Parse YAML
        let spec: OpenApiSpec = serde_yaml::from_str(&yaml_text)
            .map_err(|e| {
                tracing::error!(error = %e, "YAML parsing error details");
                anyhow::anyhow!("Failed to parse TON OpenAPI YAML spec: {}", e)
            })?;

        // Store in cache (as JSON for faster subsequent loads)
        self.disk_cache.store(&cache_key, spec.clone()).await?;

        Ok(spec)
    }

    /// Get available technologies (API categories by tag)
    #[instrument(name = "ton_client.get_technologies", skip(self))]
    pub async fn get_technologies(&self) -> Result<Vec<TonTechnology>> {
        let spec = self.get_spec().await?;

        // Group endpoints by tag
        let mut tag_counts: HashMap<String, usize> = HashMap::new();
        for (_path, path_item) in &spec.paths {
            for (_method, operation) in path_item.operations() {
                for tag in &operation.tags {
                    *tag_counts.entry(tag.clone()).or_insert(0) += 1;
                }
            }
        }

        // Build tag descriptions map
        let tag_descriptions: HashMap<String, String> = spec
            .tags
            .iter()
            .map(|t| {
                (
                    t.name.clone(),
                    t.description.clone().unwrap_or_default(),
                )
            })
            .collect();

        let mut technologies: Vec<TonTechnology> = tag_counts
            .into_iter()
            .map(|(tag, count)| TonTechnology {
                identifier: format!("ton:{}", tag.to_lowercase().replace(' ', "-")),
                title: format!("TON {}", tag),
                description: tag_descriptions
                    .get(&tag)
                    .cloned()
                    .unwrap_or_else(|| format!("TON API endpoints for {}", tag)),
                url: format!("https://tonapi.io/api-doc#/{}", tag),
                endpoint_count: count,
            })
            .collect();

        technologies.sort_by(|a, b| a.title.cmp(&b.title));
        Ok(technologies)
    }

    /// Get endpoints for a specific tag/category
    #[instrument(name = "ton_client.get_category", skip(self))]
    pub async fn get_category(&self, identifier: &str) -> Result<TonCategory> {
        let spec = self.get_spec().await?;

        // Extract tag from identifier (e.g., "ton:accounts" -> "Accounts")
        let tag_search = identifier
            .strip_prefix("ton:")
            .unwrap_or(identifier)
            .to_lowercase();

        // Find matching tag
        let tag = spec
            .tags
            .iter()
            .find(|t| t.name.to_lowercase().replace(' ', "-") == tag_search)
            .map(|t| t.name.clone())
            .ok_or_else(|| anyhow::anyhow!("TON tag not found: {identifier}"))?;

        let description = spec
            .tags
            .iter()
            .find(|t| t.name == tag)
            .and_then(|t| t.description.clone())
            .unwrap_or_default();

        // Collect endpoints for this tag
        let mut endpoints: Vec<TonEndpointSummary> = Vec::new();
        for (path, path_item) in &spec.paths {
            for (method, operation) in path_item.operations() {
                if operation.tags.contains(&tag) {
                    endpoints.push(TonEndpointSummary::from_openapi(path, method, operation));
                }
            }
        }

        endpoints.sort_by(|a, b| a.path.cmp(&b.path));

        Ok(TonCategory {
            tag,
            description,
            endpoints,
        })
    }

    /// Get a specific endpoint by operation ID
    #[instrument(name = "ton_client.get_endpoint", skip(self))]
    pub async fn get_endpoint(&self, operation_id: &str) -> Result<TonEndpoint> {
        let spec = self.get_spec().await?;

        for (path, path_item) in &spec.paths {
            for (method, operation) in path_item.operations() {
                let op_id = operation
                    .operation_id
                    .as_ref()
                    .map(|s| s.as_str())
                    .unwrap_or("");

                if op_id == operation_id {
                    return Ok(TonEndpoint::from_openapi(path, method, operation));
                }
            }
        }

        anyhow::bail!("TON endpoint not found: {operation_id}")
    }

    /// Search for endpoints matching a query
    #[instrument(name = "ton_client.search", skip(self))]
    pub async fn search(&self, query: &str) -> Result<Vec<TonEndpoint>> {
        let spec = self.get_spec().await?;
        let query_lower = query.to_lowercase();

        let mut results: Vec<TonEndpoint> = Vec::new();

        for (path, path_item) in &spec.paths {
            for (method, operation) in path_item.operations() {
                let matches = path.to_lowercase().contains(&query_lower)
                    || operation
                        .operation_id
                        .as_ref()
                        .map(|s| s.to_lowercase().contains(&query_lower))
                        .unwrap_or(false)
                    || operation
                        .summary
                        .as_ref()
                        .map(|s| s.to_lowercase().contains(&query_lower))
                        .unwrap_or(false)
                    || operation
                        .description
                        .as_ref()
                        .map(|s| s.to_lowercase().contains(&query_lower))
                        .unwrap_or(false);

                if matches {
                    results.push(TonEndpoint::from_openapi(path, method, operation));
                }
            }
        }

        Ok(results)
    }

    /// Get the API version
    pub async fn get_version(&self) -> Result<String> {
        let spec = self.get_spec().await?;
        Ok(spec.info.version)
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
        let _client = TonClient::new();
    }
}
