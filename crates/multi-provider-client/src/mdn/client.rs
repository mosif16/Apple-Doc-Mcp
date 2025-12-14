use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::time::Duration as StdDuration;

use anyhow::{Context, Result};
use directories::ProjectDirs;
use once_cell::sync::Lazy;
use regex::Regex;
use reqwest::Client;
use scraper::{Html, Selector};
use tokio::sync::RwLock;
use tracing::{debug, instrument, warn};

use super::types::{
    MdnArticle, MdnCategory, MdnDocument, MdnDocumentResponse, MdnExample, MdnParameter,
    MdnSearchDocument, MdnSearchEntry, MdnSearchResponse, MdnTechnology,
};
use docs_mcp_client::cache::{DiskCache, MemoryCache};

const MDN_SEARCH_API: &str = "https://developer.mozilla.org/api/v1/search";
const MDN_DOCUMENT_API: &str = "https://developer.mozilla.org";
const MDN_BASE_URL: &str = "https://developer.mozilla.org/en-US/docs";
const ARTICLE_CACHE_VERSION: u32 = 2;

static PRE_BLOCK_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?is)<pre[^>]*>.*?</pre>").expect("pre block regex"));

#[derive(Debug)]
pub struct MdnClient {
    http: Client,
    disk_cache: DiskCache,
    memory_cache: MemoryCache<Vec<u8>>,
    /// Cached search results by query
    search_cache: RwLock<HashMap<String, Vec<MdnSearchEntry>>>,
    cache_dir: PathBuf,
}

impl Default for MdnClient {
    fn default() -> Self {
        Self::new()
    }
}

impl MdnClient {
    #[must_use]
    pub fn new() -> Self {
        let project_dirs = ProjectDirs::from("com", "RecordAndLearn", "multi-docs-mcp")
            .expect("unable to resolve project directories");

        let cache_dir = project_dirs.cache_dir().join("mdn");
        if let Err(e) = std::fs::create_dir_all(&cache_dir) {
            warn!(error = %e, "Failed to create MDN cache directory");
        }

        let http = Client::builder()
            .user_agent("MultiDocsMCP/1.0 (Documentation Search Tool)")
            .timeout(StdDuration::from_secs(30))
            .gzip(true)
            .build()
            .expect("failed to build reqwest client");

        Self {
            http,
            disk_cache: DiskCache::new(&cache_dir),
            memory_cache: MemoryCache::new(time::Duration::hours(1)),
            search_cache: RwLock::new(HashMap::new()),
            cache_dir,
        }
    }

    /// Get available MDN technologies
    #[instrument(name = "mdn_client.get_technologies", skip(self))]
    pub async fn get_technologies(&self) -> Result<Vec<MdnTechnology>> {
        Ok(MdnTechnology::predefined())
    }

    /// Search MDN documentation
    #[instrument(name = "mdn_client.search", skip(self))]
    pub async fn search(&self, query: &str) -> Result<Vec<MdnSearchEntry>> {
        let cache_key = format!("search_{}", query.replace(' ', "_").to_lowercase());

        // Check memory cache
        if let Some(results) = self.search_cache.read().await.get(&cache_key) {
            debug!(query = %query, "Using cached MDN search results");
            return Ok(results.clone());
        }

        // Check disk cache
        if let Ok(Some(entry)) = self.disk_cache.load::<Vec<MdnSearchEntry>>(&cache_key).await {
            let results = entry.value;
            self.search_cache
                .write()
                .await
                .insert(cache_key.clone(), results.clone());
            return Ok(results);
        }

        // Fetch from MDN API
        let url = format!(
            "{}?q={}&locale=en-US&size=20",
            MDN_SEARCH_API,
            urlencoding::encode(query)
        );
        debug!(url = %url, "Searching MDN");

        let response = self
            .http
            .get(&url)
            .send()
            .await
            .context("Failed to search MDN")?;

        if !response.status().is_success() {
            anyhow::bail!("MDN search failed: {}", response.status());
        }

        let search_response: MdnSearchResponse = response
            .json()
            .await
            .context("Failed to parse MDN search response")?;

        let results: Vec<MdnSearchEntry> = search_response
            .documents
            .into_iter()
            .map(|doc| self.document_to_entry(doc))
            .collect();

        // Cache results
        let _ = self.disk_cache.store(&cache_key, results.clone()).await;
        self.search_cache
            .write()
            .await
            .insert(cache_key, results.clone());

        Ok(results)
    }

    /// Get a specific MDN article by slug
    #[instrument(name = "mdn_client.get_article", skip(self))]
    pub async fn get_article(&self, slug: &str) -> Result<MdnArticle> {
        let cache_key = format!("article_v{ARTICLE_CACHE_VERSION}_{}", slug.replace('/', "_"));

        // Check disk cache
        if let Ok(Some(entry)) = self.disk_cache.load::<MdnArticle>(&cache_key).await {
            debug!(slug = %slug, "Using cached MDN article");
            return Ok(entry.value);
        }

        // Fetch from MDN
        let url = format!("{}/{}/index.json", MDN_DOCUMENT_API, slug);
        debug!(url = %url, "Fetching MDN article");

        let response = self
            .http
            .get(&url)
            .send()
            .await
            .context("Failed to fetch MDN article")?;

        if !response.status().is_success() {
            // Try HTML fallback
            return self.fetch_article_html(slug).await;
        }

        let doc_response: MdnDocumentResponse = response
            .json()
            .await
            .context("Failed to parse MDN document response")?;

        let article = self.document_to_article(doc_response.doc, slug);

        // Cache the result
        let _ = self.disk_cache.store(&cache_key, article.clone()).await;

        Ok(article)
    }

    /// Fetch article via HTML scraping (fallback)
    async fn fetch_article_html(&self, slug: &str) -> Result<MdnArticle> {
        let url = format!("{}/{}", MDN_BASE_URL, slug);
        debug!(url = %url, "Fetching MDN article via HTML");

        let response = self
            .http
            .get(&url)
            .send()
            .await
            .context("Failed to fetch MDN HTML page")?;

        if !response.status().is_success() {
            anyhow::bail!("MDN page not found: {}", slug);
        }

        let html = response.text().await?;
        let document = Html::parse_document(&html);

        // Extract title
        let title = self
            .extract_selector_text(&document, "h1")
            .unwrap_or_else(|| slug.split('/').last().unwrap_or("Unknown").to_string());

        // Extract summary
        let summary = self
            .extract_selector_text(&document, ".seoSummary, article > p:first-of-type")
            .unwrap_or_default();

        // Extract examples
        let examples = self.extract_examples_from_html(&document);

        // Extract syntax
        let syntax = self.extract_selector_text(&document, ".syntaxbox, pre.syntaxbox, .brush.js");

        // Extract parameters
        let parameters = self.extract_parameters_from_html(&document);

        Ok(MdnArticle {
            slug: slug.to_string(),
            title,
            summary,
            category: MdnCategory::from_slug(slug),
            url: format!("{}/{}", MDN_BASE_URL, slug),
            examples,
            syntax,
            parameters,
            return_value: self.extract_return_value_from_html(&document),
            browser_compat: None,
            content: self.extract_content_from_html(&document),
        })
    }

    /// Extract code examples from HTML document
    fn extract_examples_from_html(&self, document: &Html) -> Vec<MdnExample> {
        let mut examples = Vec::new();

        // Try various code block selectors
        let selectors = [
            "pre.brush",
            "pre[class*='brush']",
            ".code-example pre",
            "#examples pre",
            "pre.js",
            "pre.javascript",
            "pre.notranslate",
        ];

        for selector_str in selectors {
            if let Ok(selector) = Selector::parse(selector_str) {
                for element in document.select(&selector) {
                    let code = element.text().collect::<String>().trim().to_string();
                    if code.is_empty() || code.len() < 10 {
                        continue;
                    }

                    // Determine language from class
                    let class = element.value().attr("class").unwrap_or("");
                    let language = if class.contains("js") || class.contains("javascript") {
                        "javascript"
                    } else if class.contains("css") {
                        "css"
                    } else if class.contains("html") {
                        "html"
                    } else if class.contains("ts") || class.contains("typescript") {
                        "typescript"
                    } else {
                        "javascript"
                    };

                    // Get description from preceding element
                    let description = element
                        .prev_siblings()
                        .filter_map(scraper::ElementRef::wrap)
                        .find(|e| e.value().name() == "p")
                        .map(|e| e.text().collect::<String>().trim().to_string());

                    // Check if it's a runnable example
                    let is_runnable = code.contains("function ")
                        || code.contains("const ")
                        || code.contains("let ")
                        || code.contains("=>");

                    examples.push(MdnExample {
                        code,
                        language: language.to_string(),
                        description,
                        is_runnable,
                    });

                    // Limit to 5 examples per article
                    if examples.len() >= 5 {
                        break;
                    }
                }
            }

            if examples.len() >= 5 {
                break;
            }
        }

        examples
    }

    /// Extract parameters from HTML document
    fn extract_parameters_from_html(&self, document: &Html) -> Vec<MdnParameter> {
        let mut params = Vec::new();

        // Try to find parameters section
        if let Ok(selector) = Selector::parse("#parameters + dl dt, #parameters ~ dl dt") {
            for dt in document.select(&selector) {
                let name = dt.text().collect::<String>().trim().to_string();
                if name.is_empty() {
                    continue;
                }

                // Get description from next dd element
                let description = dt
                    .next_siblings()
                    .filter_map(scraper::ElementRef::wrap)
                    .find(|e| e.value().name() == "dd")
                    .map(|e| e.text().collect::<String>().trim().to_string())
                    .unwrap_or_default();

                let optional = name.contains("optional") || description.contains("Optional");

                params.push(MdnParameter {
                    name: name.replace("Optional", "").trim().to_string(),
                    description,
                    param_type: None,
                    optional,
                });
            }
        }

        params
    }

    /// Extract return value from HTML document
    fn extract_return_value_from_html(&self, document: &Html) -> Option<String> {
        self.extract_selector_text(document, "#return_value + p, #return_value ~ p:first-of-type")
    }

    /// Extract main content from HTML document
    fn extract_content_from_html(&self, document: &Html) -> Option<String> {
        self.extract_selector_text(document, "article.main-page-content")
            .map(|s| {
                // Truncate if too long
                if s.len() > 4000 {
                    format!("{}...", &s[..4000])
                } else {
                    s
                }
            })
    }

    /// Helper to extract text from selector
    fn extract_selector_text(&self, document: &Html, selector_str: &str) -> Option<String> {
        if let Ok(selector) = Selector::parse(selector_str) {
            document
                .select(&selector)
                .next()
                .map(|e| e.text().collect::<String>().trim().to_string())
                .filter(|s| !s.is_empty())
        } else {
            None
        }
    }

    /// Convert search document to entry
    fn document_to_entry(&self, doc: MdnSearchDocument) -> MdnSearchEntry {
        let slug = doc
            .slug
            .unwrap_or_else(|| {
                doc.mdn_url
                    .trim_start_matches("/en-US/docs/")
                    .to_string()
            });

        MdnSearchEntry {
            slug: slug.clone(),
            title: doc.title,
            summary: doc.summary,
            category: MdnCategory::from_slug(&slug),
            url: format!("{}{}", MDN_BASE_URL, doc.mdn_url),
        }
    }

    /// Convert document response to article
    fn document_to_article(&self, doc: MdnDocument, slug: &str) -> MdnArticle {
        let mut examples = Vec::new();
        let mut syntax = None;
        let mut content_parts = Vec::new();
        let mut example_dedupe = HashSet::<String>::new();
        let pre_selector = Selector::parse("pre").ok();

        for section in &doc.body {
            match &section.value {
                Some(super::types::MdnSectionValue::Code { code, language }) => {
                    if !code.is_empty() {
                        let lang = language.as_deref().unwrap_or("javascript");
                        examples.push(MdnExample {
                            code: code.clone(),
                            language: lang.to_string(),
                            description: None,
                            is_runnable: code.contains("function ")
                                || code.contains("const ")
                                || code.contains("=>"),
                        });
                    }
                }
                Some(super::types::MdnSectionValue::Prose { content }) => {
                    if examples.len() < 5 {
                        if let Some(selector) = &pre_selector {
                            let fragment = Html::parse_fragment(content);
                            for pre in fragment.select(selector) {
                                let code = pre.text().collect::<String>().trim().to_string();
                                if code.len() < 10 || !example_dedupe.insert(code.clone()) {
                                    continue;
                                }

                                let class = pre.value().attr("class").unwrap_or_default();
                                let language = guess_language_for_snippet(slug, class).to_string();
                                let is_runnable = code.contains("function ")
                                    || code.contains("const ")
                                    || code.contains("let ")
                                    || code.contains("=>");

                                if syntax.is_none() && looks_like_syntax_snippet(&code) {
                                    syntax = Some(code.clone());
                                }

                                examples.push(MdnExample {
                                    code,
                                    language,
                                    description: None,
                                    is_runnable,
                                });

                                if examples.len() >= 5 {
                                    break;
                                }
                            }
                        }
                    }

                    // Add prose text (excluding preformatted code blocks)
                    let cleaned = PRE_BLOCK_RE.replace_all(content, "");
                    let fragment = Html::parse_fragment(cleaned.as_ref());
                    let text = fragment
                        .root_element()
                        .text()
                        .collect::<Vec<_>>()
                        .join("")
                        .trim()
                        .to_string();
                    if !text.is_empty() {
                        content_parts.push(text);
                    }
                }
                Some(super::types::MdnSectionValue::Text(text)) => {
                    content_parts.push(text.clone());
                }
                Some(super::types::MdnSectionValue::Other(_)) | None => {}
            }
        }

        let parameters = syntax
            .as_deref()
            .map(extract_parameters_from_syntax)
            .unwrap_or_default();

        MdnArticle {
            slug: slug.to_string(),
            title: doc.title,
            summary: doc.summary,
            category: MdnCategory::from_slug(slug),
            url: doc.url,
            examples,
            syntax,
            parameters,
            return_value: None,
            browser_compat: None,
            content: if content_parts.is_empty() {
                None
            } else {
                Some(content_parts.join("\n\n"))
            },
        }
    }

    pub fn cache_dir(&self) -> &PathBuf {
        &self.cache_dir
    }
}

fn guess_language_for_snippet(slug: &str, class: &str) -> &'static str {
    let class_lower = class.to_lowercase();
    if class_lower.contains("css") {
        return "css";
    }
    if class_lower.contains("html") {
        return "html";
    }
    if class_lower.contains("json") {
        return "json";
    }
    if class_lower.contains("ts") || class_lower.contains("typescript") {
        return "typescript";
    }
    if class_lower.contains("js") || class_lower.contains("javascript") {
        return "javascript";
    }

    match MdnCategory::from_slug(slug) {
        MdnCategory::JavaScript | MdnCategory::WebApi => "javascript",
        MdnCategory::Css => "css",
        MdnCategory::Html => "html",
    }
}

fn looks_like_syntax_snippet(code: &str) -> bool {
    let code = code.trim();
    if code.len() > 160 {
        return false;
    }
    if !code.contains('(') || !code.contains(')') {
        return false;
    }
    if code.contains('{')
        || code.contains(';')
        || code.contains("=>")
        || code.contains("const ")
        || code.contains("let ")
        || code.contains("var ")
    {
        return false;
    }
    true
}

fn extract_parameters_from_syntax(syntax: &str) -> Vec<MdnParameter> {
    let mut params = Vec::new();
    let mut seen = HashSet::<String>::new();

    let mut remaining = syntax;
    while let Some(open) = remaining.find('(') {
        let Some(close) = remaining[open + 1..].find(')') else {
            break;
        };
        let inside = &remaining[open + 1..open + 1 + close];
        for raw in inside.split(',') {
            let candidate = raw.trim();
            if candidate.is_empty() {
                continue;
            }

            let candidate = candidate
                .trim_start_matches("...")
                .trim_matches(|c: char| !c.is_alphanumeric() && c != '_' && c != '$')
                .to_string();

            if candidate.is_empty() || !seen.insert(candidate.clone()) {
                continue;
            }

            params.push(MdnParameter {
                name: candidate,
                description: String::new(),
                param_type: None,
                optional: false,
            });
        }
        remaining = &remaining[open + 1 + close + 1..];
    }

    params
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mdn::types::{MdnSection, MdnSectionValue, MdnSource};

    #[test]
    fn test_client_creation() {
        let _client = MdnClient::new();
    }

    #[test]
    fn test_document_to_article_extracts_syntax_parameters_examples_and_content() {
        let client = MdnClient::new();
        let slug = "Web/JavaScript/Reference/Global_Objects/Array/map";

        let doc = MdnDocument {
            url: "https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/Array/map"
                .to_string(),
            title: "Array.prototype.map()".to_string(),
            summary: "Returns a new array populated with the results of calling a provided function."
                .to_string(),
            body: vec![
                MdnSection {
                    section_type: Some("prose".to_string()),
                    value: Some(MdnSectionValue::Prose {
                        content: "<p>Overview</p><pre>map(callbackFn, thisArg)</pre><pre class=\"language-js\">const xs = [1, 2, 3];</pre>".to_string(),
                    }),
                },
                MdnSection {
                    section_type: Some("browser_compatibility".to_string()),
                    value: Some(MdnSectionValue::Other(serde_json::json!({
                        "browser_compatibility": {}
                    }))),
                },
            ],
            source: MdnSource::default(),
        };

        let article = client.document_to_article(doc, slug);

        assert_eq!(article.syntax.as_deref(), Some("map(callbackFn, thisArg)"));
        let names: Vec<&str> = article.parameters.iter().map(|p| p.name.as_str()).collect();
        assert!(names.contains(&"callbackFn"));
        assert!(names.contains(&"thisArg"));
        assert!(article.examples.iter().any(|ex| ex.code.contains("const xs")));

        let content = article.content.unwrap_or_default();
        assert!(content.contains("Overview"));
        assert!(!content.contains("callbackFn"));
        assert!(!content.contains("const xs"));
    }

    #[test]
    fn test_document_deserialization_tolerates_unknown_section_values() {
        let payload = serde_json::json!({
            "doc": {
                "mdn_url": "https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/Array/map",
                "title": "Array.prototype.map()",
                "summary": "",
                "body": [
                    {
                        "type": "prose",
                        "value": { "content": "<p>Hi</p>" }
                    },
                    {
                        "type": "browser_compatibility",
                        "value": { "specifications": [] }
                    }
                ],
                "source": {}
            }
        });

        let doc_response: MdnDocumentResponse = serde_json::from_value(payload).unwrap();
        let client = MdnClient::new();
        let article = client.document_to_article(doc_response.doc, "Web/JavaScript/Reference/Global_Objects/Array/map");

        assert!(article.content.unwrap_or_default().contains("Hi"));
    }
}
