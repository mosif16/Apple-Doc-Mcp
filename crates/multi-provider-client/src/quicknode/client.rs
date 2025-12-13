use std::path::PathBuf;
use std::time::Duration as StdDuration;

use anyhow::{Context, Result};
use directories::ProjectDirs;
use reqwest::Client;
use scraper::{Html, Selector};
use tokio::sync::Mutex;
use tracing::{debug, instrument, warn};

use super::types::{
    QuickNodeCategory, QuickNodeCategoryItem, QuickNodeExample, QuickNodeMethod,
    QuickNodeMethodKind, QuickNodeParameter, QuickNodeReturnType,
    QuickNodeTechnology, SolanaMethodIndex, SOLANA_HTTP_METHODS, SOLANA_MARKETPLACE_ADDONS,
    SOLANA_WEBSOCKET_METHODS,
};
use docs_mcp_client::cache::{DiskCache, MemoryCache};

const BASE_URL: &str = "https://www.quicknode.com/docs/solana";

#[derive(Debug)]
pub struct QuickNodeClient {
    http: Client,
    disk_cache: DiskCache,
    memory_cache: MemoryCache<String>,
    fetch_lock: Mutex<()>,
    cache_dir: PathBuf,
}

impl Default for QuickNodeClient {
    fn default() -> Self {
        Self::new()
    }
}

impl QuickNodeClient {
    #[must_use]
    pub fn new() -> Self {
        let project_dirs = ProjectDirs::from("com", "RecordAndLearn", "multi-docs-mcp")
            .expect("unable to resolve project directories");

        let cache_dir = project_dirs.cache_dir().join("quicknode");
        if let Err(e) = std::fs::create_dir_all(&cache_dir) {
            warn!(error = %e, "Failed to create QuickNode cache directory");
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
            fetch_lock: Mutex::new(()),
            cache_dir,
        }
    }

    /// Get available technologies (Solana categories)
    #[instrument(name = "quicknode_client.get_technologies", skip(self))]
    pub async fn get_technologies(&self) -> Result<Vec<QuickNodeTechnology>> {
        let http_tech = QuickNodeTechnology {
            identifier: "quicknode:solana:http".to_string(),
            title: "Solana HTTP Methods".to_string(),
            description: format!(
                "Solana JSON-RPC HTTP API - {} methods for interacting with Solana blockchain",
                SOLANA_HTTP_METHODS.len()
            ),
            url: format!("{BASE_URL}/getAccountInfo"),
            item_count: SOLANA_HTTP_METHODS.len(),
        };

        let ws_tech = QuickNodeTechnology {
            identifier: "quicknode:solana:websocket".to_string(),
            title: "Solana WebSocket Methods".to_string(),
            description: format!(
                "Solana WebSocket Subscriptions - {} methods for real-time blockchain data",
                SOLANA_WEBSOCKET_METHODS.len()
            ),
            url: format!("{BASE_URL}/accountSubscribe"),
            item_count: SOLANA_WEBSOCKET_METHODS.len(),
        };

        let marketplace_tech = QuickNodeTechnology {
            identifier: "quicknode:solana:marketplace".to_string(),
            title: "Solana Marketplace Add-ons".to_string(),
            description: format!(
                "QuickNode Marketplace Add-ons - {} specialized APIs for Solana",
                SOLANA_MARKETPLACE_ADDONS.len()
            ),
            url: format!("{BASE_URL}/jito-bundles"),
            item_count: SOLANA_MARKETPLACE_ADDONS.len(),
        };

        Ok(vec![http_tech, ws_tech, marketplace_tech])
    }

    /// Get a category of methods
    #[instrument(name = "quicknode_client.get_category", skip(self))]
    pub async fn get_category(&self, identifier: &str) -> Result<QuickNodeCategory> {
        let (methods, kind, title, description): (
            &[SolanaMethodIndex],
            QuickNodeMethodKind,
            &str,
            &str,
        ) = match identifier {
            "quicknode:solana:http" | "http" | "solana:http" => (
                SOLANA_HTTP_METHODS,
                QuickNodeMethodKind::HttpMethod,
                "Solana HTTP Methods",
                "JSON-RPC HTTP methods for Solana blockchain interaction",
            ),
            "quicknode:solana:websocket" | "websocket" | "solana:websocket" | "ws" => (
                SOLANA_WEBSOCKET_METHODS,
                QuickNodeMethodKind::WebSocketMethod,
                "Solana WebSocket Methods",
                "WebSocket subscription methods for real-time Solana data",
            ),
            "quicknode:solana:marketplace" | "marketplace" | "solana:marketplace" | "addons" => (
                SOLANA_MARKETPLACE_ADDONS,
                QuickNodeMethodKind::MarketplaceAddon,
                "Solana Marketplace Add-ons",
                "QuickNode specialized APIs for Solana",
            ),
            _ => anyhow::bail!("Unknown QuickNode category: {identifier}"),
        };

        let items = methods
            .iter()
            .map(|m| QuickNodeCategoryItem {
                name: m.name.to_string(),
                description: m.description.to_string(),
                kind,
                url: format!("{BASE_URL}/{}", m.name),
            })
            .collect();

        Ok(QuickNodeCategory {
            identifier: identifier.to_string(),
            title: title.to_string(),
            description: description.to_string(),
            items,
        })
    }

    /// Fetch HTML content for a method page
    async fn fetch_method_html(&self, method_name: &str) -> Result<String> {
        let cache_key = format!("method_{method_name}.html");

        // Check memory cache first
        if let Some(html) = self.memory_cache.get(&cache_key) {
            debug!(method = method_name, "QuickNode method served from memory cache");
            return Ok(html);
        }

        // Check disk cache
        if let Ok(Some(entry)) = self.disk_cache.load::<String>(&cache_key).await {
            debug!(method = method_name, "QuickNode method served from disk cache");
            self.memory_cache.insert(cache_key.clone(), entry.value.clone());
            return Ok(entry.value);
        }

        // Lock to prevent concurrent fetches for same method
        let _lock = self.fetch_lock.lock().await;

        // Double-check after acquiring lock
        if let Some(html) = self.memory_cache.get(&cache_key) {
            return Ok(html);
        }

        // Fetch from QuickNode
        let url = format!("{BASE_URL}/{method_name}");
        debug!(url = %url, "Fetching QuickNode method documentation");

        let response = self
            .http
            .get(&url)
            .send()
            .await
            .context("Failed to fetch QuickNode documentation")?;

        if !response.status().is_success() {
            anyhow::bail!(
                "QuickNode documentation fetch failed for {}: {}",
                method_name,
                response.status()
            );
        }

        let html = response
            .text()
            .await
            .context("Failed to read QuickNode response")?;

        // Store in caches
        self.memory_cache.insert(cache_key.clone(), html.clone());
        if let Err(e) = self.disk_cache.store(&cache_key, html.clone()).await {
            warn!(error = %e, "Failed to cache QuickNode method to disk");
        }

        Ok(html)
    }

    /// Parse method documentation from HTML
    fn parse_method_html(
        &self,
        method_name: &str,
        html: &str,
        index_entry: &SolanaMethodIndex,
    ) -> QuickNodeMethod {
        let document = Html::parse_document(html);

        // Parse parameters
        let parameters = self.parse_parameters(&document);

        // Parse return type
        let returns = self.parse_returns(&document);

        // Parse code examples
        let examples = self.parse_examples(&document);

        // Extract description from page if available, otherwise use index description
        let description = self
            .parse_description(&document)
            .unwrap_or_else(|| index_entry.description.to_string());

        QuickNodeMethod {
            name: method_name.to_string(),
            description,
            kind: index_entry.kind,
            url: format!("{BASE_URL}/{method_name}"),
            parameters,
            returns,
            examples,
        }
    }

    fn parse_description(&self, document: &Html) -> Option<String> {
        // Try to find the main description paragraph
        let selector = Selector::parse("article p, .description, main p").ok()?;
        for element in document.select(&selector) {
            let text = element.text().collect::<String>();
            let trimmed = text.trim();
            if !trimmed.is_empty() && trimmed.len() > 20 {
                return Some(trimmed.to_string());
            }
        }
        None
    }

    fn parse_parameters(&self, document: &Html) -> Vec<QuickNodeParameter> {
        let mut parameters = Vec::new();

        // Look for parameter sections in various possible formats
        let section_selectors = [
            "h2:contains('Parameters') + *, h3:contains('Parameters') + *",
            "[class*='parameter'], [class*='param']",
            "table tbody tr",
        ];

        for selector_str in &section_selectors {
            if let Ok(selector) = Selector::parse(selector_str) {
                for element in document.select(&selector) {
                    let text = element.text().collect::<String>();
                    // Try to parse parameter info from text
                    if let Some(param) = self.try_parse_param_from_text(&text) {
                        parameters.push(param);
                    }
                }
            }
        }

        // If no parameters found, try to extract from common patterns
        if parameters.is_empty() {
            if let Ok(selector) = Selector::parse("code, pre") {
                for element in document.select(&selector) {
                    let text = element.text().collect::<String>();
                    if text.contains("params") || text.contains("\"params\"") {
                        // This might be a JSON-RPC example with params
                        break;
                    }
                }
            }
        }

        parameters
    }

    fn try_parse_param_from_text(&self, text: &str) -> Option<QuickNodeParameter> {
        let text = text.trim();
        if text.is_empty() || text.len() < 3 {
            return None;
        }

        // Common patterns: "name (type) - description" or "name: type - description"
        let parts: Vec<&str> = text.splitn(2, |c| ['-', ':'].contains(&c)).collect();
        if parts.len() >= 2 {
            let name_type = parts[0].trim();
            let description = parts.get(1).map_or("", |s| s.trim());

            // Try to extract name and type
            if let Some((name, param_type)) = self.extract_name_type(name_type) {
                let required = text.to_lowercase().contains("required")
                    || !text.to_lowercase().contains("optional");

                return Some(QuickNodeParameter {
                    name,
                    param_type,
                    required,
                    description: description.to_string(),
                    default_value: None,
                });
            }
        }

        None
    }

    fn extract_name_type(&self, text: &str) -> Option<(String, String)> {
        // Patterns: "name (type)", "name: type", just "name"
        let text = text.trim();

        if let Some(paren_start) = text.find('(') {
            if let Some(paren_end) = text.find(')') {
                let name = text[..paren_start].trim().to_string();
                let param_type = text[paren_start + 1..paren_end].trim().to_string();
                if !name.is_empty() {
                    return Some((name, param_type));
                }
            }
        }

        if let Some(colon) = text.find(':') {
            let name = text[..colon].trim().to_string();
            let param_type = text[colon + 1..].trim().to_string();
            if !name.is_empty() {
                return Some((name, param_type));
            }
        }

        // Just a name
        if !text.is_empty() && text.chars().all(|c| c.is_alphanumeric() || c == '_') {
            return Some((text.to_string(), "any".to_string()));
        }

        None
    }

    fn parse_returns(&self, document: &Html) -> Option<QuickNodeReturnType> {
        // Look for return/response sections
        let section_selectors = [
            "h2:contains('Returns') + *, h3:contains('Returns') + *",
            "h2:contains('Response') + *, h3:contains('Response') + *",
            "[class*='return'], [class*='response']",
        ];

        for selector_str in &section_selectors {
            if let Ok(selector) = Selector::parse(selector_str) {
                for element in document.select(&selector) {
                    let text = element.text().collect::<String>();
                    let trimmed = text.trim();
                    if !trimmed.is_empty() && trimmed.len() > 10 {
                        return Some(QuickNodeReturnType {
                            type_name: "object".to_string(),
                            description: trimmed.to_string(),
                            fields: Vec::new(),
                        });
                    }
                }
            }
        }

        None
    }

    fn parse_examples(&self, document: &Html) -> Vec<QuickNodeExample> {
        let mut examples = Vec::new();

        // Look for code blocks
        if let Ok(selector) = Selector::parse("pre code, .code-block, [class*='highlight']") {
            for element in document.select(&selector) {
                let code = element.text().collect::<String>();
                let trimmed = code.trim();

                if trimmed.is_empty() || trimmed.len() < 10 {
                    continue;
                }

                // Try to detect language from class or content
                let class = element
                    .value()
                    .attr("class")
                    .unwrap_or("")
                    .to_lowercase();

                let language = if class.contains("javascript") || class.contains("js") {
                    "javascript"
                } else if class.contains("python") || class.contains("py") {
                    "python"
                } else if class.contains("rust") || class.contains("rs") {
                    "rust"
                } else if class.contains("ruby") || class.contains("rb") {
                    "ruby"
                } else if class.contains("curl")
                    || class.contains("bash")
                    || class.contains("sh")
                    || trimmed.starts_with("curl")
                {
                    "bash"
                } else if trimmed.contains("fetch(") || trimmed.contains("const ") {
                    "javascript"
                } else if trimmed.contains("import ") && trimmed.contains("def ") {
                    "python"
                } else {
                    "json"
                };

                examples.push(QuickNodeExample {
                    language: language.to_string(),
                    code: trimmed.to_string(),
                    description: None,
                });
            }
        }

        examples
    }

    /// Get a specific method by name
    #[instrument(name = "quicknode_client.get_method", skip(self))]
    pub async fn get_method(&self, name: &str) -> Result<QuickNodeMethod> {
        // Find method in index
        let index_entry = SOLANA_HTTP_METHODS
            .iter()
            .chain(SOLANA_WEBSOCKET_METHODS.iter())
            .chain(SOLANA_MARKETPLACE_ADDONS.iter())
            .find(|m| m.name.eq_ignore_ascii_case(name))
            .ok_or_else(|| anyhow::anyhow!("QuickNode method not found: {name}"))?;

        // Fetch and parse HTML
        let html = self.fetch_method_html(index_entry.name).await?;
        Ok(self.parse_method_html(index_entry.name, &html, index_entry))
    }

    /// Search for methods matching a query
    #[instrument(name = "quicknode_client.search", skip(self))]
    pub async fn search(&self, query: &str) -> Result<Vec<QuickNodeMethod>> {
        let query_lower = query.to_lowercase();

        // Split query into keywords
        let keywords: Vec<&str> = query_lower
            .split(|c: char| c.is_whitespace() || c == '-' || c == '_')
            .filter(|s| !s.is_empty() && s.len() > 1)
            .collect();

        let mut scored_results: Vec<(i32, &SolanaMethodIndex)> = Vec::new();

        // Search all methods
        let all_methods = SOLANA_HTTP_METHODS
            .iter()
            .chain(SOLANA_WEBSOCKET_METHODS.iter())
            .chain(SOLANA_MARKETPLACE_ADDONS.iter());

        for method in all_methods {
            let name_lower = method.name.to_lowercase();
            let desc_lower = method.description.to_lowercase();

            let mut score = 0i32;

            for keyword in &keywords {
                // Exact name match
                if name_lower == *keyword {
                    score += 50;
                }
                // Name contains keyword
                else if name_lower.contains(keyword) {
                    score += 20;
                }
                // Description contains keyword
                if desc_lower.contains(keyword) {
                    score += 5;
                }
            }

            if score > 0 {
                scored_results.push((score, method));
            }
        }

        // Sort by score (highest first)
        scored_results.sort_by(|a, b| b.0.cmp(&a.0));

        // Convert to QuickNodeMethod (basic info only, without fetching HTML)
        let results: Vec<QuickNodeMethod> = scored_results
            .into_iter()
            .take(20)
            .map(|(_, m)| QuickNodeMethod {
                name: m.name.to_string(),
                description: m.description.to_string(),
                kind: m.kind,
                url: format!("{BASE_URL}/{}", m.name),
                parameters: Vec::new(),
                returns: None,
                examples: Vec::new(),
            })
            .collect();

        Ok(results)
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
        let _client = QuickNodeClient::new();
    }
}
