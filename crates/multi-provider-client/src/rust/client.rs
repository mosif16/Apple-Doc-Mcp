use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration as StdDuration;

use anyhow::{Context, Result};
use directories::ProjectDirs;
use reqwest::Client;
use serde_json::Value;
use tokio::sync::{Mutex, RwLock};
use tracing::{debug, instrument, warn};

use super::html_parser::{extract_title_from_html, parse_rustdoc_html};
use super::types::{
    DocsRsCrateData, DocsRsRelease, DocsRsReleasesResponse, RustCategory, RustCategoryItem,
    RustCrate, RustItem, RustItemKind, RustSearchIndex, RustSearchIndexEntry, RustTechnology,
    STD_CRATES,
};
use docs_mcp_client::cache::{DiskCache, MemoryCache};

const STD_SEARCH_INDEX_URL: &str = "https://doc.rust-lang.org/search-index.js";
const DOCS_RS_RELEASES_SEARCH: &str = "https://docs.rs/releases/search";
const DOCS_RS_CRATE_DATA: &str = "https://docs.rs/crate";

#[derive(Debug)]
pub struct RustClient {
    http: Client,
    disk_cache: DiskCache,
    memory_cache: MemoryCache<Vec<u8>>,
    /// Lock to prevent concurrent fetches of std index
    std_lock: Mutex<()>,
    /// Cached std library search indexes
    std_indexes: RwLock<HashMap<String, RustSearchIndex>>,
    /// Cached crate search indexes (for docs.rs crates)
    crate_indexes: RwLock<HashMap<String, RustSearchIndex>>,
    cache_dir: PathBuf,
}

impl Default for RustClient {
    fn default() -> Self {
        Self::new()
    }
}

impl RustClient {
    #[must_use]
    pub fn new() -> Self {
        let project_dirs = ProjectDirs::from("com", "RecordAndLearn", "multi-docs-mcp")
            .expect("unable to resolve project directories");

        let cache_dir = project_dirs.cache_dir().join("rust");
        if let Err(e) = std::fs::create_dir_all(&cache_dir) {
            warn!(error = %e, "Failed to create Rust cache directory");
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
            memory_cache: MemoryCache::new(time::Duration::hours(24)),
            std_lock: Mutex::new(()),
            std_indexes: RwLock::new(HashMap::new()),
            crate_indexes: RwLock::new(HashMap::new()),
            cache_dir,
        }
    }

    /// Get available technologies (std library + popular crates)
    #[instrument(name = "rust_client.get_technologies", skip(self))]
    pub async fn get_technologies(&self) -> Result<Vec<RustTechnology>> {
        let mut technologies = Vec::new();

        // Add standard library crates
        for (name, description) in STD_CRATES {
            let crate_info = RustCrate {
                name: (*name).to_string(),
                version: "latest".to_string(),
                description: (*description).to_string(),
                documentation_url: format!("https://doc.rust-lang.org/{}/", name),
                repository_url: Some("https://github.com/rust-lang/rust".to_string()),
                is_std: true,
            };

            // Get item count from search index if available
            let item_count = match self.get_search_index(name).await {
                Ok(index) => index.items.len(),
                Err(_) => 0,
            };

            technologies.push(RustTechnology::from_crate(crate_info, item_count));
        }

        Ok(technologies)
    }

    /// Get crate information from docs.rs
    #[instrument(name = "rust_client.get_crate", skip(self))]
    pub async fn get_crate(&self, name: &str) -> Result<RustCrate> {
        // Check if it's a standard library crate
        if let Some((_, desc)) = STD_CRATES.iter().find(|(n, _)| *n == name) {
            return Ok(RustCrate {
                name: name.to_string(),
                version: "latest".to_string(),
                description: (*desc).to_string(),
                documentation_url: format!("https://doc.rust-lang.org/{}/", name),
                repository_url: Some("https://github.com/rust-lang/rust".to_string()),
                is_std: true,
            });
        }

        // Fetch from docs.rs
        let cache_key = format!("crate_{}.json", name);

        // Check disk cache first
        if let Ok(Some(entry)) = self.disk_cache.load::<DocsRsCrateData>(&cache_key).await {
            let data = entry.value;
            return Ok(RustCrate {
                name: data.name,
                version: data.version,
                description: data.description.unwrap_or_default(),
                documentation_url: format!("https://docs.rs/{}/latest/", name),
                repository_url: data.repository,
                is_std: false,
            });
        }

        // Fetch from crates.io API (docs.rs doesn't have a JSON API)
        let url = format!("https://crates.io/api/v1/crates/{}", name);
        debug!(url = %url, "Fetching crate data from crates.io");

        let response = self
            .http
            .get(&url)
            .send()
            .await
            .context("Failed to fetch crate data from crates.io")?;

        if !response.status().is_success() {
            anyhow::bail!("Crate '{}' not found on crates.io: {}", name, response.status());
        }

        let json: Value = response
            .json()
            .await
            .context("Failed to parse crates.io response")?;

        // Extract data from crates.io API response format
        let crate_obj = json.get("crate")
            .context("Missing 'crate' field in crates.io response")?;

        let data = DocsRsCrateData {
            name: crate_obj.get("name")
                .and_then(|v| v.as_str())
                .unwrap_or(name)
                .to_string(),
            version: crate_obj.get("newest_version")
                .and_then(|v| v.as_str())
                .unwrap_or("latest")
                .to_string(),
            description: crate_obj.get("description")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            repository: crate_obj.get("repository")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            documentation: crate_obj.get("documentation")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            keywords: crate_obj.get("keywords")
                .and_then(|v| v.as_array())
                .map(|arr| arr.iter()
                    .filter_map(|v| v.as_str())
                    .map(|s| s.to_string())
                    .collect())
                .unwrap_or_default(),
            releases: Vec::new(), // We don't need releases for search functionality
        };

        // Cache the result
        let _ = self.disk_cache.store(&cache_key, data.clone()).await;

        Ok(RustCrate {
            name: data.name,
            version: data.version,
            description: data.description.unwrap_or_default(),
            documentation_url: format!("https://docs.rs/{}/latest/", name),
            repository_url: data.repository,
            is_std: false,
        })
    }

    /// Get category/module listing for a crate
    #[instrument(name = "rust_client.get_category", skip(self))]
    pub async fn get_category(&self, identifier: &str) -> Result<RustCategory> {
        let crate_name = identifier.strip_prefix("rust:").unwrap_or(identifier);

        let index = self.get_search_index(crate_name).await?;
        let crate_info = self.get_crate(crate_name).await?;

        // Group items by module
        let items: Vec<RustCategoryItem> = index
            .items
            .iter()
            .filter(|item| item.kind == RustItemKind::Module || item.parent.is_none())
            .take(100) // Limit for initial display
            .map(|item| {
                let full_path = if item.path.is_empty() {
                    format!("{}::{}", crate_name, item.name)
                } else {
                    format!("{}::{}::{}", crate_name, item.path, item.name)
                };

                RustCategoryItem {
                    name: item.name.clone(),
                    description: item.desc.clone(),
                    kind: item.kind,
                    path: full_path.clone(),
                    url: self.build_item_url(crate_name, &crate_info.version, &full_path),
                }
            })
            .collect();

        Ok(RustCategory {
            identifier: identifier.to_string(),
            title: format!("{} Crate", crate_name),
            description: crate_info.description,
            items,
        })
    }

    /// Get a specific item by path (with detailed documentation)
    #[instrument(name = "rust_client.get_item", skip(self))]
    pub async fn get_item(&self, path: &str) -> Result<RustItem> {
        // Parse the path (e.g., "std::collections::HashMap" or "serde::Deserialize")
        let parts: Vec<&str> = path.split("::").collect();
        if parts.is_empty() {
            anyhow::bail!("Invalid path: {}", path);
        }

        let crate_name = parts[0];
        let crate_info = self.get_crate(crate_name).await?;

        // Try to find in search index first
        let entry = if let Ok(index) = self.get_search_index(crate_name).await {
            let item_name = parts.last().unwrap_or(&"");
            let expected_path = if parts.len() > 2 {
                parts[1..parts.len() - 1].join("::")
            } else {
                String::new()
            };

            index.items.iter().find(|item| {
                item.name == *item_name
                    && (item.path == expected_path
                        || (expected_path.is_empty() && item.path.is_empty()))
            }).cloned()
        } else {
            None
        };

        // If found in index, use that info
        if let Some(entry) = entry {
            let full_path = if entry.path.is_empty() {
                format!("{}::{}", crate_name, entry.name)
            } else {
                format!("{}::{}::{}", crate_name, entry.path, entry.name)
            };

            let url = self.build_item_url(crate_name, &crate_info.version, &full_path);
            return self.fetch_item_with_details(
                &entry.name,
                &full_path,
                entry.kind,
                &entry.desc,
                crate_name,
                &crate_info.version,
                &url,
            ).await;
        }

        // Fallback: try to construct URL directly and fetch HTML
        debug!(path = %path, "Item not in search index, trying direct HTML fetch");
        self.fetch_item_by_path_direct(path, crate_name, &crate_info.version).await
    }

    /// Fetch item by constructing URL directly from path
    async fn fetch_item_by_path_direct(
        &self,
        path: &str,
        crate_name: &str,
        crate_version: &str,
    ) -> Result<RustItem> {
        let parts: Vec<&str> = path.split("::").collect();
        let item_name = parts.last().unwrap_or(&"unknown").to_string();

        // Try different URL patterns for std vs docs.rs
        let urls_to_try = self.build_possible_urls(path, crate_name, crate_version);

        for (url, guessed_kind) in urls_to_try {
            debug!(url = %url, "Trying URL");

            match self.http.get(&url).send().await {
                Ok(response) if response.status().is_success() => {
                    let html = response.text().await?;
                    let parsed = parse_rustdoc_html(&html, guessed_kind);

                    // Extract title from HTML if possible
                    let title = extract_title_from_html(&html).unwrap_or_else(|| item_name.clone());

                    return Ok(RustItem {
                        name: title,
                        path: path.to_string(),
                        kind: guessed_kind,
                        summary: parsed.documentation.clone().unwrap_or_default(),
                        crate_name: crate_name.to_string(),
                        crate_version: crate_version.to_string(),
                        url: url.clone(),
                        declaration: parsed.declaration,
                        documentation: parsed.documentation,
                        examples: parsed.examples,
                        methods: parsed.methods,
                        impl_traits: parsed.impl_traits,
                        associated_types: parsed.associated_types,
                        source_url: parsed.source_url,
                        is_detailed: true,
                    });
                }
                _ => {}
            }
        }

        anyhow::bail!("Item not found: {}", path)
    }

    /// Build possible URLs for an item path
    fn build_possible_urls(&self, path: &str, crate_name: &str, version: &str) -> Vec<(String, RustItemKind)> {
        let parts: Vec<&str> = path.split("::").collect();
        let item_name = parts.last().unwrap_or(&"");

        // Build the module path (everything between crate and item name)
        let module_path = if parts.len() > 2 {
            parts[1..parts.len() - 1].join("/")
        } else {
            String::new()
        };

        let is_std = STD_CRATES.iter().any(|(n, _)| *n == crate_name);
        let base = if is_std {
            format!("https://doc.rust-lang.org/{}", crate_name)
        } else {
            format!("https://docs.rs/{}/{}/{}", crate_name, version, crate_name)
        };

        let module_prefix = if module_path.is_empty() {
            String::new()
        } else {
            format!("{}/", module_path)
        };

        // Try different item type prefixes
        vec![
            (format!("{}/{}struct.{}.html", base, module_prefix, item_name), RustItemKind::Struct),
            (format!("{}/{}enum.{}.html", base, module_prefix, item_name), RustItemKind::Enum),
            (format!("{}/{}trait.{}.html", base, module_prefix, item_name), RustItemKind::Trait),
            (format!("{}/{}fn.{}.html", base, module_prefix, item_name), RustItemKind::Function),
            (format!("{}/{}type.{}.html", base, module_prefix, item_name), RustItemKind::Type),
            (format!("{}/{}macro.{}.html", base, module_prefix, item_name), RustItemKind::Macro),
            (format!("{}/{}constant.{}.html", base, module_prefix, item_name), RustItemKind::Constant),
            (format!("{}/{}static.{}.html", base, module_prefix, item_name), RustItemKind::Static),
            (format!("{}/{}{}/index.html", base, module_prefix, item_name), RustItemKind::Module),
        ]
    }

    /// Fetch item with detailed documentation
    #[allow(clippy::too_many_arguments)]
    async fn fetch_item_with_details(
        &self,
        name: &str,
        full_path: &str,
        kind: RustItemKind,
        summary: &str,
        crate_name: &str,
        crate_version: &str,
        url: &str,
    ) -> Result<RustItem> {
        let mut item = RustItem {
            name: name.to_string(),
            path: full_path.to_string(),
            kind,
            summary: summary.to_string(),
            crate_name: crate_name.to_string(),
            crate_version: crate_version.to_string(),
            url: url.to_string(),
            declaration: None,
            documentation: None,
            examples: Vec::new(),
            methods: Vec::new(),
            impl_traits: Vec::new(),
            associated_types: Vec::new(),
            source_url: None,
            is_detailed: false,
        };

        // Fetch detailed documentation via HTML parsing
        if let Ok(detailed) = self.fetch_detailed_documentation(url, kind).await {
            item.declaration = detailed.declaration;
            item.documentation = detailed.documentation;
            item.examples = detailed.examples;
            item.methods = detailed.methods;
            item.impl_traits = detailed.impl_traits;
            item.associated_types = detailed.associated_types;
            item.source_url = detailed.source_url;
            item.is_detailed = true;
        }

        Ok(item)
    }

    /// Get a specific item by path without fetching detailed docs (for batch operations)
    #[instrument(name = "rust_client.get_item_minimal", skip(self))]
    pub async fn get_item_minimal(&self, path: &str) -> Result<RustItem> {
        // Parse the path (e.g., "std::collections::HashMap" or "serde::Deserialize")
        let parts: Vec<&str> = path.split("::").collect();
        if parts.is_empty() {
            anyhow::bail!("Invalid path: {}", path);
        }

        let crate_name = parts[0];
        let crate_info = self.get_crate(crate_name).await?;
        let index = self.get_search_index(crate_name).await?;

        // Find the item in the index
        let item_name = parts.last().unwrap_or(&"");
        let expected_path = if parts.len() > 2 {
            parts[1..parts.len() - 1].join("::")
        } else {
            String::new()
        };

        let entry = index
            .items
            .iter()
            .find(|item| {
                item.name == *item_name
                    && (item.path == expected_path
                        || (expected_path.is_empty() && item.path.is_empty()))
            })
            .ok_or_else(|| anyhow::anyhow!("Item not found: {}", path))?;

        let full_path = if entry.path.is_empty() {
            format!("{}::{}", crate_name, entry.name)
        } else {
            format!("{}::{}::{}", crate_name, entry.path, entry.name)
        };

        Ok(RustItem {
            name: entry.name.clone(),
            path: full_path.clone(),
            kind: entry.kind,
            summary: entry.desc.clone(),
            crate_name: crate_name.to_string(),
            crate_version: crate_info.version.clone(),
            url: self.build_item_url(crate_name, &crate_info.version, &full_path),
            declaration: None,
            documentation: None,
            examples: Vec::new(),
            methods: Vec::new(),
            impl_traits: Vec::new(),
            associated_types: Vec::new(),
            source_url: None,
            is_detailed: false,
        })
    }

    /// Fetch detailed documentation by parsing HTML
    #[instrument(name = "rust_client.fetch_detailed_documentation", skip(self))]
    async fn fetch_detailed_documentation(
        &self,
        url: &str,
        kind: RustItemKind,
    ) -> Result<super::html_parser::ParsedDocumentation> {
        // Check disk cache first
        let cache_key = format!("html_{}.json", url.replace(['/', ':', '.'], "_"));

        if let Ok(Some(entry)) = self
            .disk_cache
            .load::<super::html_parser::ParsedDocumentation>(&cache_key)
            .await
        {
            debug!(url = %url, "Using cached HTML documentation");
            return Ok(entry.value);
        }

        // Fetch the HTML page
        debug!(url = %url, "Fetching HTML documentation");
        let response = self
            .http
            .get(url)
            .send()
            .await
            .with_context(|| format!("Failed to fetch documentation from {}", url))?;

        if !response.status().is_success() {
            anyhow::bail!(
                "Failed to fetch documentation from {}: {}",
                url,
                response.status()
            );
        }

        let html = response.text().await?;

        // Parse the HTML
        let parsed = parse_rustdoc_html(&html, kind);

        // Cache the result
        let _ = self.disk_cache.store(&cache_key, parsed.clone()).await;

        Ok(parsed)
    }

    /// Search within a crate
    #[instrument(name = "rust_client.search", skip(self))]
    pub async fn search(&self, crate_name: &str, query: &str) -> Result<Vec<RustItem>> {
        let index = self.get_search_index(crate_name).await?;
        let crate_info = self.get_crate(crate_name).await?;

        // Tokenize the query into individual search terms
        let query_terms: Vec<String> = query
            .to_lowercase()
            .split_whitespace()
            .map(String::from)
            .collect();

        let mut results: Vec<(i32, RustItem)> = index
            .items
            .iter()
            .filter_map(|entry| {
                let name_lower = entry.name.to_lowercase();
                let desc_lower = entry.desc.to_lowercase();
                let path_lower = entry.path.to_lowercase();

                // Calculate match score based on how many query terms match
                let mut score = 0i32;
                let mut matched_terms = 0;

                for term in &query_terms {
                    // Exact name match for this term
                    if name_lower == *term {
                        score += 100;
                        matched_terms += 1;
                    } else if name_lower.starts_with(term) {
                        score += 50;
                        matched_terms += 1;
                    } else if name_lower.contains(term) {
                        score += 30;
                        matched_terms += 1;
                    } else if desc_lower.contains(term) {
                        score += 10;
                        matched_terms += 1;
                    } else if path_lower.contains(term) {
                        score += 5;
                        matched_terms += 1;
                    }
                }

                // Only include if at least one term matched
                if matched_terms == 0 {
                    return None;
                }

                // Boost by kind (structs, traits, enums are more important)
                score += match entry.kind {
                    RustItemKind::Struct | RustItemKind::Trait => 15,
                    RustItemKind::Enum => 12,
                    RustItemKind::Function => 10,
                    RustItemKind::Macro => 8,
                    RustItemKind::Module => 5,
                    _ => 0,
                };

                // Bonus for matching multiple terms
                if matched_terms > 1 {
                    score += matched_terms * 5;
                }

                let item = RustItem::from_search_entry(entry, crate_name, &crate_info.version);
                Some((score, item))
            })
            .collect();

        // Sort by score descending
        results.sort_by(|a, b| b.0.cmp(&a.0));

        Ok(results.into_iter().map(|(_, item)| item).take(50).collect())
    }

    /// Search for crates on docs.rs
    #[instrument(name = "rust_client.search_crates", skip(self))]
    pub async fn search_crates(&self, query: &str) -> Result<Vec<RustCrate>> {
        let cache_key = format!("search_{}.json", query.replace(' ', "_"));

        // Check cache first
        if let Ok(Some(entry)) = self
            .disk_cache
            .load::<DocsRsReleasesResponse>(&cache_key)
            .await
        {
            return Ok(releases_to_crates(&entry.value.results));
        }

        // Fetch from docs.rs
        let url = format!("{}?query={}", DOCS_RS_RELEASES_SEARCH, urlencoding::encode(query));
        debug!(url = %url, "Searching docs.rs for crates");

        let response = self
            .http
            .get(&url)
            .send()
            .await
            .context("Failed to search docs.rs")?;

        if !response.status().is_success() {
            anyhow::bail!("docs.rs search failed: {}", response.status());
        }

        let data: DocsRsReleasesResponse = response
            .json()
            .await
            .context("Failed to parse docs.rs search results")?;

        // Cache the result
        let _ = self.disk_cache.store(&cache_key, data.clone()).await;

        Ok(releases_to_crates(&data.results))
    }

    /// Get or fetch the search index for a crate
    async fn get_search_index(&self, crate_name: &str) -> Result<RustSearchIndex> {
        let is_std = STD_CRATES.iter().any(|(n, _)| *n == crate_name);

        // Check in-memory cache first
        if is_std {
            if let Some(index) = self.std_indexes.read().await.get(crate_name) {
                return Ok(index.clone());
            }
        } else if let Some(index) = self.crate_indexes.read().await.get(crate_name) {
            return Ok(index.clone());
        }

        // Check disk cache
        let cache_key = format!("index_{}.json", crate_name);
        if let Ok(Some(entry)) = self.disk_cache.load::<RustSearchIndex>(&cache_key).await {
            let index = entry.value;
            if is_std {
                self.std_indexes
                    .write()
                    .await
                    .insert(crate_name.to_string(), index.clone());
            } else {
                self.crate_indexes
                    .write()
                    .await
                    .insert(crate_name.to_string(), index.clone());
            }
            return Ok(index);
        }

        // Fetch the search index
        let index = if is_std {
            self.fetch_std_search_index(crate_name).await?
        } else {
            self.fetch_docs_rs_search_index(crate_name).await?
        };

        // Cache to disk
        let _ = self.disk_cache.store(&cache_key, index.clone()).await;

        // Cache in memory
        if is_std {
            self.std_indexes
                .write()
                .await
                .insert(crate_name.to_string(), index.clone());
        } else {
            self.crate_indexes
                .write()
                .await
                .insert(crate_name.to_string(), index.clone());
        }

        Ok(index)
    }

    /// Fetch and parse the std library search index
    async fn fetch_std_search_index(&self, crate_name: &str) -> Result<RustSearchIndex> {
        let _lock = self.std_lock.lock().await;

        debug!("Fetching std library search index");
        let response = self
            .http
            .get(STD_SEARCH_INDEX_URL)
            .send()
            .await
            .context("Failed to fetch std search index")?;

        if response.status().is_success() {
            let text = response.text().await?;
            let index = parse_search_index_js(&text, crate_name)?;
            if !index.items.is_empty() {
                return Ok(index);
            }
        }

        // Fallback: scrape the std documentation page directly
        // The modern rustdoc uses a binary database format, so we scrape HTML instead
        debug!("Search index unavailable or empty, falling back to HTML scraping for std");
        self.scrape_std_index(crate_name).await
    }

    /// Scrape the std library documentation page to build a search index
    async fn scrape_std_index(&self, crate_name: &str) -> Result<RustSearchIndex> {
        use scraper::{Html, Selector};

        // Use the "all items" page which has a comprehensive listing
        let url = format!("https://doc.rust-lang.org/{}/all.html", crate_name);
        debug!(url = %url, "Scraping std all items page for search index");

        let response = self
            .http
            .get(&url)
            .send()
            .await
            .context("Failed to fetch std all items page")?;

        if !response.status().is_success() {
            // Fallback to index.html if all.html doesn't exist
            return self.scrape_std_index_fallback(crate_name).await;
        }

        let html = response.text().await?;
        let document = Html::parse_document(&html);

        let mut items = Vec::new();

        // The all.html page has sections like "Structs", "Enums", etc.
        // with links to each item
        if let Ok(selector) = Selector::parse("section a, .all-items a, ul.all-items-list a, main a") {
            for element in document.select(&selector) {
                let name = element.text().collect::<String>().trim().to_string();
                if name.is_empty() || name.contains("All Items") || name.len() > 100 {
                    continue;
                }

                let href = element.value().attr("href").unwrap_or("");

                // Determine item kind from href
                let kind = if href.contains("struct.") {
                    RustItemKind::Struct
                } else if href.contains("enum.") {
                    RustItemKind::Enum
                } else if href.contains("trait.") {
                    RustItemKind::Trait
                } else if href.contains("fn.") {
                    RustItemKind::Function
                } else if href.contains("macro.") {
                    RustItemKind::Macro
                } else if href.contains("type.") {
                    RustItemKind::Type
                } else if href.contains("constant.") {
                    RustItemKind::Constant
                } else if href.contains("static.") {
                    RustItemKind::Static
                } else if href.ends_with("/index.html") || (href.ends_with("/") && !href.contains('.')) {
                    RustItemKind::Module
                } else {
                    continue; // Skip unknown items
                };

                // Extract module path from href
                let path = href
                    .trim_end_matches(".html")
                    .split('/')
                    .filter(|s| !s.is_empty() && !s.contains('.'))
                    .collect::<Vec<_>>()
                    .join("::");

                items.push(RustSearchIndexEntry {
                    name: name.clone(),
                    path,
                    kind,
                    desc: String::new(),
                    parent: None,
                });
            }
        }

        // Deduplicate items by name
        items.sort_by(|a, b| a.name.cmp(&b.name));
        items.dedup_by(|a, b| a.name == b.name && a.kind == b.kind);

        debug!(count = items.len(), "Scraped items from std all items page");

        Ok(RustSearchIndex {
            crate_name: crate_name.to_string(),
            crate_version: "latest".to_string(),
            items,
        })
    }

    /// Fallback scraper for std index when all.html is unavailable
    async fn scrape_std_index_fallback(&self, crate_name: &str) -> Result<RustSearchIndex> {
        use scraper::{Html, Selector};

        let url = format!("https://doc.rust-lang.org/{}/index.html", crate_name);
        debug!(url = %url, "Scraping std index.html as fallback");

        let response = self
            .http
            .get(&url)
            .send()
            .await
            .context("Failed to fetch std documentation")?;

        if !response.status().is_success() {
            anyhow::bail!("Failed to fetch std docs: {}", response.status());
        }

        let html = response.text().await?;
        let document = Html::parse_document(&html);

        let mut items = Vec::new();

        // Parse items from the modern rustdoc structure
        if let Ok(dt_selector) = Selector::parse("dl.item-table dt") {
            for dt_element in document.select(&dt_selector) {
                if let Some(item) = self.parse_item_table_dt(&dt_element, crate_name) {
                    items.push(item);
                }
            }
        }

        debug!(count = items.len(), "Scraped items from std index.html fallback");

        Ok(RustSearchIndex {
            crate_name: crate_name.to_string(),
            crate_version: "latest".to_string(),
            items,
        })
    }

    /// Fetch and parse a docs.rs crate's search index
    async fn fetch_docs_rs_search_index(&self, crate_name: &str) -> Result<RustSearchIndex> {
        // First get the crate version
        let crate_info = self.get_crate(crate_name).await?;

        // Try to fetch search-index.js from docs.rs
        let url = format!(
            "https://docs.rs/{}/{}/search-index.js",
            crate_name, crate_info.version
        );

        debug!(url = %url, "Fetching docs.rs search index");

        let response = self.http.get(&url).send().await;

        match response {
            Ok(resp) if resp.status().is_success() => {
                let text = resp.text().await?;
                parse_search_index_js(&text, crate_name)
            }
            _ => {
                // Fall back to scraping the crate's main documentation page
                debug!("Search index not available, scraping crate documentation");
                self.scrape_crate_index(crate_name, &crate_info.version).await
            }
        }
    }

    /// Scrape a crate's documentation page to build a basic search index
    async fn scrape_crate_index(&self, crate_name: &str, version: &str) -> Result<RustSearchIndex> {
        use scraper::{Html, Selector};

        // Fetch the main crate documentation page
        let url = format!("https://docs.rs/{}/{}/{}/", crate_name, version, crate_name);
        debug!(url = %url, "Scraping crate documentation for search index");

        let response = self
            .http
            .get(&url)
            .send()
            .await
            .context("Failed to fetch crate documentation")?;

        if !response.status().is_success() {
            anyhow::bail!("Failed to fetch crate docs: {}", response.status());
        }

        let html = response.text().await?;
        let document = Html::parse_document(&html);

        let mut items = Vec::new();

        // Extract items from the main documentation page
        // The modern rustdoc structure uses: <dl class="item-table"><dt><a>...</a></dt></dl>

        // Try the modern rustdoc structure (item-table with dt > a)
        if let Ok(dt_selector) = Selector::parse("dl.item-table dt") {
            for dt_element in document.select(&dt_selector) {
                if let Some(item) = self.parse_item_table_dt(&dt_element, crate_name) {
                    items.push(item);
                }
            }
        }

        // If no items found, try legacy selectors
        if items.is_empty() {
            if let Ok(selector) = Selector::parse(".module-item") {
                for element in document.select(&selector) {
                    if let Some(item) = self.parse_module_item(&element, crate_name) {
                        items.push(item);
                    }
                }
            }
        }

        // Last resort: try very generic selectors
        if items.is_empty() {
            if let Ok(selector) = Selector::parse(".item-left, .item-right") {
                for element in document.select(&selector) {
                    if let Some(item) = self.parse_legacy_item(&element, crate_name) {
                        items.push(item);
                    }
                }
            }
        }

        debug!(count = items.len(), "Scraped items from crate documentation");

        Ok(RustSearchIndex {
            crate_name: crate_name.to_string(),
            crate_version: version.to_string(),
            items,
        })
    }

    /// Parse a module item element into a search index entry
    fn parse_module_item(&self, element: &scraper::ElementRef, _crate_name: &str) -> Option<RustSearchIndexEntry> {
        use scraper::Selector;

        // Extract link and text
        let link_selector = Selector::parse("a").ok()?;
        let link = element.select(&link_selector).next()?;

        let name = link.text().collect::<String>().trim().to_string();
        if name.is_empty() {
            return None;
        }

        // Get the item kind from the class or span
        let kind = if let Ok(kind_selector) = Selector::parse(".stab, .item-kind") {
            element
                .select(&kind_selector)
                .next()
                .map_or_else(
                    || "module".to_string(),
                    |e| e.text().collect::<String>().trim().to_lowercase(),
                )
        } else {
            "module".to_string()
        };

        let item_kind = match kind.as_str() {
            "struct" => RustItemKind::Struct,
            "enum" => RustItemKind::Enum,
            "trait" => RustItemKind::Trait,
            "fn" | "function" => RustItemKind::Function,
            "type" => RustItemKind::Type,
            "macro" => RustItemKind::Macro,
            "constant" | "const" => RustItemKind::Constant,
            "static" => RustItemKind::Static,
            _ => RustItemKind::Module,
        };

        // Get description
        let desc = if let Ok(desc_selector) = Selector::parse(".item-desc, .item-right") {
            element
                .select(&desc_selector)
                .next()
                .map(|e| e.text().collect::<String>().trim().to_string())
                .unwrap_or_default()
        } else {
            String::new()
        };

        Some(RustSearchIndexEntry {
            name,
            path: String::new(), // Root level items have empty path
            kind: item_kind,
            desc,
            parent: None,
        })
    }

    /// Parse an item from a <dt> element in the item-table structure
    fn parse_item_table_dt(&self, dt_element: &scraper::ElementRef, _crate_name: &str) -> Option<RustSearchIndexEntry> {
        use scraper::Selector;

        // Find the <a> tag inside the <dt>
        let link_selector = Selector::parse("a").ok()?;
        let link = dt_element.select(&link_selector).next()?;

        // Extract name from link text
        let name = link.text().collect::<String>().trim().to_string();
        if name.is_empty() {
            return None;
        }

        // Determine kind from the link's class (e.g., "mod", "struct", "fn", "macro", "attr")
        let class = link.value().attr("class").unwrap_or("");
        let kind = match class {
            c if c.contains("mod") => RustItemKind::Module,
            c if c.contains("struct") => RustItemKind::Struct,
            c if c.contains("enum") => RustItemKind::Enum,
            c if c.contains("trait") => RustItemKind::Trait,
            c if c.contains("fn") => RustItemKind::Function,
            c if c.contains("type") => RustItemKind::Type,
            c if c.contains("macro") => RustItemKind::Macro,
            c if c.contains("constant") => RustItemKind::Constant,
            c if c.contains("static") => RustItemKind::Static,
            c if c.contains("attr") => RustItemKind::Macro, // Attribute macros
            _ => {
                // Fallback: try to infer from href
                let href = link.value().attr("href").unwrap_or("");
                if href.contains("struct.") {
                    RustItemKind::Struct
                } else if href.contains("enum.") {
                    RustItemKind::Enum
                } else if href.contains("trait.") {
                    RustItemKind::Trait
                } else if href.contains("fn.") {
                    RustItemKind::Function
                } else {
                    RustItemKind::Module
                }
            }
        };

        // Get description from the next <dd> sibling
        let desc = dt_element
            .next_siblings()
            .filter_map(scraper::ElementRef::wrap)
            .find(|e| e.value().name() == "dd")
            .map(|dd| dd.text().collect::<String>().trim().to_string())
            .unwrap_or_default();

        Some(RustSearchIndexEntry {
            name,
            path: String::new(),
            kind,
            desc,
            parent: None,
        })
    }

    /// Parse items using legacy selectors
    fn parse_legacy_item(&self, element: &scraper::ElementRef, _crate_name: &str) -> Option<RustSearchIndexEntry> {
        use scraper::Selector;

        let link_selector = Selector::parse("a").ok()?;
        let link = element.select(&link_selector).next()?;

        let name = link.text().collect::<String>().trim().to_string();
        if name.is_empty() {
            return None;
        }

        let href = link.value().attr("href").unwrap_or("");
        let kind = if href.contains("struct.") {
            RustItemKind::Struct
        } else if href.contains("enum.") {
            RustItemKind::Enum
        } else if href.contains("trait.") {
            RustItemKind::Trait
        } else if href.contains("fn.") {
            RustItemKind::Function
        } else {
            RustItemKind::Module
        };

        Some(RustSearchIndexEntry {
            name,
            path: String::new(),
            kind,
            desc: String::new(),
            parent: None,
        })
    }

    /// Build the documentation URL for an item
    fn build_item_url(&self, crate_name: &str, version: &str, path: &str) -> String {
        let path_parts: Vec<&str> = path.split("::").collect();
        let html_path = if path_parts.len() > 1 {
            path_parts[1..].join("/")
        } else {
            String::new()
        };

        if STD_CRATES.iter().any(|(n, _)| *n == crate_name) {
            format!("https://doc.rust-lang.org/{}/{}.html", crate_name, html_path)
        } else {
            format!("https://docs.rs/{}/{}/{}.html", crate_name, version, html_path)
        }
    }

    pub fn cache_dir(&self) -> &PathBuf {
        &self.cache_dir
    }
}

/// Convert docs.rs releases to RustCrate structs
fn releases_to_crates(releases: &[DocsRsRelease]) -> Vec<RustCrate> {
    releases
        .iter()
        .filter(|r| r.rustdoc_status)
        .map(|r| RustCrate {
            name: r.name.clone(),
            version: r.version.clone(),
            description: r.description.clone().unwrap_or_default(),
            documentation_url: format!("https://docs.rs/{}/{}/", r.name, r.version),
            repository_url: None,
            is_std: false,
        })
        .collect()
}

/// Parse the rustdoc search-index.js format
fn parse_search_index_js(js_content: &str, target_crate: &str) -> Result<RustSearchIndex> {
    // The search-index.js file contains JavaScript that assigns search index data
    // Format varies but typically looks like:
    // var searchIndex = {...};
    // or searchState.loadedDescShards = {...};

    let mut items = Vec::new();
    let crate_version = "latest".to_string();

    // Try to find the JSON-like data for our target crate
    // Look for patterns like: "crate_name":{"doc":"...","t":[...],"n":[...],...}

    // Find the crate's data block
    let crate_pattern = format!("\"{}\":", target_crate);
    if let Some(start) = js_content.find(&crate_pattern) {
        let content_start = start + crate_pattern.len();

        // Find the matching closing brace
        if let Some(data_start) = js_content[content_start..].find('{') {
            let data_start = content_start + data_start;
            let mut brace_count = 0;
            let mut data_end = data_start;

            for (i, c) in js_content[data_start..].chars().enumerate() {
                match c {
                    '{' => brace_count += 1,
                    '}' => {
                        brace_count -= 1;
                        if brace_count == 0 {
                            data_end = data_start + i + 1;
                            break;
                        }
                    }
                    _ => {}
                }
            }

            let json_str = &js_content[data_start..data_end];

            // Try to parse as JSON
            if let Ok(data) = serde_json::from_str::<Value>(json_str) {
                items = parse_rustdoc_index_format(&data, target_crate);
            }
        }
    }

    Ok(RustSearchIndex {
        crate_name: target_crate.to_string(),
        crate_version,
        items,
    })
}

/// Parse the modern rustdoc search index format
fn parse_rustdoc_index_format(data: &Value, _crate_name: &str) -> Vec<RustSearchIndexEntry> {
    let mut items = Vec::new();

    // Modern format has arrays: t (types), n (names), q (paths), d (descriptions)
    let types = data.get("t").and_then(|v| v.as_array());
    let names = data.get("n").and_then(|v| v.as_array());
    let paths = data.get("q").and_then(|v| v.as_array());
    let descs = data.get("d").and_then(|v| v.as_array());

    if let (Some(types), Some(names)) = (types, names) {
        let len = types.len().min(names.len());

        for i in 0..len {
            let type_id = types[i]
                .as_u64()
                .and_then(|value| u8::try_from(value).ok())
                .unwrap_or(0);
            let name = names[i].as_str().unwrap_or("").to_string();
            let path = paths
                .and_then(|p| p.get(i))
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let desc = descs
                .and_then(|d| d.get(i))
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            if name.is_empty() {
                continue;
            }

            let kind = RustItemKind::from_type_id(type_id).unwrap_or(RustItemKind::Function);

            items.push(RustSearchIndexEntry {
                name,
                path,
                kind,
                desc,
                parent: None,
            });
        }
    }

    items
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_creation() {
        let _client = RustClient::new();
    }

    #[test]
    fn test_rust_item_kind_from_type_id() {
        assert_eq!(RustItemKind::from_type_id(0), Some(RustItemKind::Module));
        assert_eq!(RustItemKind::from_type_id(3), Some(RustItemKind::Struct));
        assert_eq!(RustItemKind::from_type_id(5), Some(RustItemKind::Function));
        assert_eq!(RustItemKind::from_type_id(8), Some(RustItemKind::Trait));
        assert_eq!(RustItemKind::from_type_id(255), None);
    }
}
