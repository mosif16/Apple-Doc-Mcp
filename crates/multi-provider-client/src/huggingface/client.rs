//! Hugging Face documentation client.
//!
//! Provides access to Hugging Face transformers library documentation,
//! swift-transformers, and model information from the Hub.

use std::path::PathBuf;
use std::time::Duration as StdDuration;

use anyhow::{Context, Result};
use directories::ProjectDirs;
use reqwest::Client;
use scraper::{Html, Selector};
use tracing::{debug, instrument, warn};

use super::types::{
    HfArticle, HfCategory, HfCategoryItem, HfExample, HfItemKind, HfModelInfo,
    HfParameter, HfSearchResult, HfTechnology, HfTechnologyKind,
    LLM_MODEL_FAMILIES, SWIFT_TRANSFORMERS_TOPICS, TRANSFORMERS_TOPICS,
};
use docs_mcp_client::cache::{DiskCache, MemoryCache};

const TRANSFORMERS_DOCS_BASE: &str = "https://huggingface.co/docs/transformers/main/en";
const SWIFT_TRANSFORMERS_BASE: &str = "https://huggingface.co/docs/swift-transformers/main/en";
const HF_HUB_API: &str = "https://huggingface.co/api";

#[derive(Debug)]
pub struct HuggingFaceClient {
    http: Client,
    disk_cache: DiskCache,
    #[allow(dead_code)]
    memory_cache: MemoryCache<Vec<u8>>,
    cache_dir: PathBuf,
}

impl Default for HuggingFaceClient {
    fn default() -> Self {
        Self::new()
    }
}

impl HuggingFaceClient {
    #[must_use]
    pub fn new() -> Self {
        let project_dirs = ProjectDirs::from("com", "RecordAndLearn", "multi-docs-mcp")
            .expect("unable to resolve project directories");

        let cache_dir = project_dirs.cache_dir().join("huggingface");
        if let Err(e) = std::fs::create_dir_all(&cache_dir) {
            warn!(error = %e, "Failed to create HuggingFace cache directory");
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
            cache_dir,
        }
    }

    /// Get available HF technologies
    #[instrument(name = "hf_client.get_technologies", skip(self))]
    pub async fn get_technologies(&self) -> Result<Vec<HfTechnology>> {
        Ok(vec![
            HfTechnology {
                identifier: "hf:transformers".to_string(),
                title: "Transformers".to_string(),
                description: "State-of-the-art ML for PyTorch, TensorFlow, JAX".to_string(),
                url: TRANSFORMERS_DOCS_BASE.to_string(),
                kind: HfTechnologyKind::Transformers,
            },
            HfTechnology {
                identifier: "hf:swift-transformers".to_string(),
                title: "Swift Transformers".to_string(),
                description: "Transformers models for Swift/iOS/macOS development".to_string(),
                url: SWIFT_TRANSFORMERS_BASE.to_string(),
                kind: HfTechnologyKind::SwiftTransformers,
            },
            HfTechnology {
                identifier: "hf:models".to_string(),
                title: "Model Hub".to_string(),
                description: "Browse and use pretrained models".to_string(),
                url: "https://huggingface.co/models".to_string(),
                kind: HfTechnologyKind::Models,
            },
            HfTechnology {
                identifier: "hf:tokenizers".to_string(),
                title: "Tokenizers".to_string(),
                description: "Fast tokenization library".to_string(),
                url: "https://huggingface.co/docs/tokenizers".to_string(),
                kind: HfTechnologyKind::Tokenizers,
            },
        ])
    }

    /// Get category listing
    #[instrument(name = "hf_client.get_category", skip(self))]
    pub async fn get_category(&self, identifier: &str) -> Result<HfCategory> {
        let (topics, kind, base_url, title, description) = if identifier.contains("swift") {
            (
                SWIFT_TRANSFORMERS_TOPICS,
                HfTechnologyKind::SwiftTransformers,
                SWIFT_TRANSFORMERS_BASE,
                "Swift Transformers",
                "ML models for Swift/iOS/macOS development",
            )
        } else {
            (
                TRANSFORMERS_TOPICS,
                HfTechnologyKind::Transformers,
                TRANSFORMERS_DOCS_BASE,
                "Transformers Library",
                "State-of-the-art ML models and utilities",
            )
        };

        let items: Vec<HfCategoryItem> = topics
            .iter()
            .map(|(name, path, desc, item_kind)| HfCategoryItem {
                name: (*name).to_string(),
                description: (*desc).to_string(),
                kind: *item_kind,
                path: (*path).to_string(),
                url: format!("{}/{}", base_url, path),
            })
            .collect();

        Ok(HfCategory {
            identifier: identifier.to_string(),
            title: title.to_string(),
            description: description.to_string(),
            items,
            kind,
        })
    }

    /// Search HF documentation
    #[instrument(name = "hf_client.search", skip(self))]
    pub async fn search(
        &self,
        query: &str,
        technology: Option<HfTechnologyKind>,
    ) -> Result<Vec<HfSearchResult>> {
        let query_lower = query.to_lowercase();
        let query_terms: Vec<&str> = query_lower.split_whitespace().collect();

        let mut results = Vec::new();

        // Search transformers topics
        if technology.is_none() || technology == Some(HfTechnologyKind::Transformers) {
            for (name, path, desc, item_kind) in TRANSFORMERS_TOPICS {
                let score = calculate_score(name, desc, &query_terms);
                if score > 0 {
                    results.push(HfSearchResult {
                        name: (*name).to_string(),
                        path: (*path).to_string(),
                        url: format!("{}/{}", TRANSFORMERS_DOCS_BASE, path),
                        kind: *item_kind,
                        technology: HfTechnologyKind::Transformers,
                        description: (*desc).to_string(),
                        score,
                    });
                }
            }
        }

        // Search Swift transformers topics
        if technology.is_none() || technology == Some(HfTechnologyKind::SwiftTransformers) {
            for (name, path, desc, item_kind) in SWIFT_TRANSFORMERS_TOPICS {
                let score = calculate_score(name, desc, &query_terms);
                if score > 0 {
                    results.push(HfSearchResult {
                        name: (*name).to_string(),
                        path: (*path).to_string(),
                        url: format!("{}/{}", SWIFT_TRANSFORMERS_BASE, path),
                        kind: *item_kind,
                        technology: HfTechnologyKind::SwiftTransformers,
                        description: (*desc).to_string(),
                        score,
                    });
                }
            }
        }

        // Search model families
        if technology.is_none() || technology == Some(HfTechnologyKind::Models) {
            for (family, desc) in LLM_MODEL_FAMILIES {
                if query_terms.iter().any(|t| family.contains(t) || t.contains(family)) {
                    results.push(HfSearchResult {
                        name: (*family).to_string(),
                        path: format!("models/{}", family),
                        url: format!("https://huggingface.co/models?search={}", family),
                        kind: HfItemKind::Model,
                        technology: HfTechnologyKind::Models,
                        description: (*desc).to_string(),
                        score: 80,
                    });
                }
            }
        }

        // Sort by score descending
        results.sort_by(|a, b| b.score.cmp(&a.score));
        results.truncate(20);

        Ok(results)
    }

    /// Search models on Hugging Face Hub
    #[instrument(name = "hf_client.search_models", skip(self))]
    pub async fn search_models(&self, query: &str, limit: usize) -> Result<Vec<HfModelInfo>> {
        let cache_key = format!("models_search_{}.json", query.replace(' ', "_"));

        // Check cache
        if let Ok(Some(entry)) = self.disk_cache.load::<Vec<HfModelInfo>>(&cache_key).await {
            return Ok(entry.value);
        }

        // Fetch from Hub API
        let url = format!(
            "{}/models?search={}&sort=downloads&direction=-1&limit={}",
            HF_HUB_API,
            urlencoding::encode(query),
            limit
        );

        debug!(url = %url, "Searching Hugging Face models");

        let response = self.http.get(&url).send().await;

        match response {
            Ok(resp) if resp.status().is_success() => {
                let models: Vec<HfModelInfo> = resp.json().await?;
                let _ = self.disk_cache.store(&cache_key, models.clone()).await;
                Ok(models)
            }
            Ok(resp) => {
                anyhow::bail!("Hub API returned error: {}", resp.status())
            }
            Err(e) => {
                anyhow::bail!("Failed to search models: {}", e)
            }
        }
    }

    /// Get detailed article documentation
    #[instrument(name = "hf_client.get_article", skip(self))]
    pub async fn get_article(
        &self,
        path: &str,
        technology: HfTechnologyKind,
    ) -> Result<HfArticle> {
        let (base_url, topics): (&str, &[(&str, &str, &str, HfItemKind)]) =
            if technology == HfTechnologyKind::SwiftTransformers {
                (SWIFT_TRANSFORMERS_BASE, SWIFT_TRANSFORMERS_TOPICS)
            } else {
                (TRANSFORMERS_DOCS_BASE, TRANSFORMERS_TOPICS)
            };

        // Find in predefined topics
        let topic = topics
            .iter()
            .find(|(_, p, _, _)| *p == path || path.ends_with(p))
            .or_else(|| topics.iter().find(|(n, _, _, _)| path.contains(n)));

        let (name, url, desc, kind) = match topic {
            Some((n, p, d, k)) => ((*n).to_string(), format!("{}/{}", base_url, p), (*d).to_string(), *k),
            None => {
                let clean_path = path.strip_prefix("hf:").unwrap_or(path);
                (
                    clean_path.split('/').last().unwrap_or(clean_path).to_string(),
                    format!("{}/{}", base_url, clean_path),
                    String::new(),
                    HfItemKind::Class,
                )
            }
        };

        // Check cache
        let cache_key = format!("article_{}_{}.json", technology, path.replace('/', "_"));

        if let Ok(Some(entry)) = self.disk_cache.load::<HfArticle>(&cache_key).await {
            return Ok(entry.value);
        }

        // Fetch and parse documentation
        let article = self.fetch_article(&url, &name, &desc, kind, technology).await?;

        // Cache result
        let _ = self.disk_cache.store(&cache_key, article.clone()).await;

        Ok(article)
    }

    /// Fetch and parse documentation page
    async fn fetch_article(
        &self,
        url: &str,
        name: &str,
        default_desc: &str,
        kind: HfItemKind,
        technology: HfTechnologyKind,
    ) -> Result<HfArticle> {
        debug!(url = %url, "Fetching HuggingFace documentation");

        let response = self.http.get(url).send().await;

        match response {
            Ok(resp) if resp.status().is_success() => {
                let html = resp.text().await?;
                self.parse_hf_docs_html(&html, name, url, default_desc, kind, technology)
            }
            _ => {
                // Return basic article with predefined info
                Ok(HfArticle {
                    title: name.to_string(),
                    description: default_desc.to_string(),
                    path: url.split('/').last().unwrap_or(name).to_string(),
                    url: url.to_string(),
                    kind,
                    technology,
                    declaration: None,
                    content: default_desc.to_string(),
                    examples: vec![],
                    parameters: vec![],
                    return_value: None,
                    related: vec![],
                    languages: vec!["python".to_string()],
                })
            }
        }
    }

    /// Parse HuggingFace documentation HTML
    fn parse_hf_docs_html(
        &self,
        html: &str,
        name: &str,
        url: &str,
        default_desc: &str,
        kind: HfItemKind,
        technology: HfTechnologyKind,
    ) -> Result<HfArticle> {
        let document = Html::parse_document(html);

        // Extract title
        let title = extract_text(&document, "h1, .prose h1")
            .unwrap_or_else(|| name.to_string());

        // Extract description
        let description = extract_text(&document, ".prose > p:first-of-type, article > p:first-of-type")
            .unwrap_or_else(|| default_desc.to_string());

        // Extract declaration/signature
        let declaration = extract_text(&document, ".docstring-signature, pre.highlight");

        // Extract content
        let content = extract_text(&document, ".prose, article, .doc-content")
            .unwrap_or_else(|| description.clone());

        // Extract code examples
        let examples = extract_code_examples(&document);

        // Extract parameters
        let parameters = extract_parameters(&document);

        // Extract return value
        let return_value = extract_text(&document, ".returns, .field-body:contains('Returns')");

        // Extract related
        let related = extract_related(&document);

        let languages = if technology == HfTechnologyKind::SwiftTransformers {
            vec!["swift".to_string()]
        } else {
            vec!["python".to_string()]
        };

        Ok(HfArticle {
            title,
            description,
            path: url.split('/').last().unwrap_or(name).to_string(),
            url: url.to_string(),
            kind,
            technology,
            declaration,
            content,
            examples,
            parameters,
            return_value,
            related,
            languages,
        })
    }

    /// Get model documentation from Hub
    #[instrument(name = "hf_client.get_model_info", skip(self))]
    pub async fn get_model_info(&self, model_id: &str) -> Result<HfModelInfo> {
        let cache_key = format!("model_{}.json", model_id.replace('/', "_"));

        if let Ok(Some(entry)) = self.disk_cache.load::<HfModelInfo>(&cache_key).await {
            return Ok(entry.value);
        }

        let url = format!("{}/models/{}", HF_HUB_API, model_id);
        debug!(url = %url, "Fetching model info");

        let response = self
            .http
            .get(&url)
            .send()
            .await
            .context("Failed to fetch model info")?;

        if !response.status().is_success() {
            anyhow::bail!("Model not found: {}", model_id);
        }

        let info: HfModelInfo = response.json().await?;
        let _ = self.disk_cache.store(&cache_key, info.clone()).await;

        Ok(info)
    }

    pub fn cache_dir(&self) -> &PathBuf {
        &self.cache_dir
    }
}

/// Calculate search score
fn calculate_score(name: &str, desc: &str, query_terms: &[&str]) -> i32 {
    let name_lower = name.to_lowercase();
    let desc_lower = desc.to_lowercase();

    let mut score = 0;

    for term in query_terms {
        if name_lower == *term {
            score += 100;
        } else if name_lower.starts_with(term) {
            score += 50;
        } else if name_lower.contains(term) {
            score += 30;
        } else if desc_lower.contains(term) {
            score += 10;
        }
    }

    // Boost important terms
    let boost_terms = [
        "automodel", "pipeline", "trainer", "tokenizer", "generate",
        "llama", "mistral", "gemma", "bert", "gpt", "transformer",
    ];
    for boost in boost_terms {
        if name_lower.contains(boost) && query_terms.iter().any(|t| t.contains(boost)) {
            score += 25;
        }
    }

    score
}

/// Extract text from selector
fn extract_text(document: &Html, selector_str: &str) -> Option<String> {
    let selector = Selector::parse(selector_str).ok()?;
    document
        .select(&selector)
        .next()
        .map(|el| el.text().collect::<String>().trim().to_string())
        .filter(|s| !s.is_empty())
}

/// Extract code examples
fn extract_code_examples(document: &Html) -> Vec<HfExample> {
    let mut examples = Vec::new();

    if let Ok(selector) = Selector::parse("pre code, .highlight pre, .code-block") {
        for element in document.select(&selector).take(5) {
            let code = element.text().collect::<String>().trim().to_string();
            if !code.is_empty() && code.len() > 20 {
                // Detect language from class
                let classes = element
                    .value()
                    .attr("class")
                    .unwrap_or("");
                let language = if classes.contains("python") || classes.contains("py") {
                    "python"
                } else if classes.contains("swift") {
                    "swift"
                } else if classes.contains("bash") || classes.contains("shell") {
                    "bash"
                } else {
                    "python" // Default for HF docs
                };

                examples.push(HfExample {
                    code,
                    language: language.to_string(),
                    description: None,
                });
            }
        }
    }

    examples
}

/// Extract parameters
fn extract_parameters(document: &Html) -> Vec<HfParameter> {
    let mut params = Vec::new();

    if let Ok(selector) = Selector::parse(".docstring-args li, .field-list .field") {
        for element in document.select(&selector) {
            let text = element.text().collect::<String>();

            // Parse format: "name (type) – description" or "name: description"
            if let Some(dash_pos) = text.find(" – ").or_else(|| text.find(": ")) {
                let (name_part, desc) = text.split_at(dash_pos);
                let desc = desc.trim_start_matches([' ', '–', ':']);

                // Extract name and type
                let (name, param_type) = if let Some(paren_pos) = name_part.find('(') {
                    let name = name_part[..paren_pos].trim();
                    let ptype = name_part[paren_pos..]
                        .trim_matches(|c| c == '(' || c == ')')
                        .trim();
                    (name.to_string(), Some(ptype.to_string()))
                } else {
                    (name_part.trim().to_string(), None)
                };

                // Check for default value
                let (desc_clean, default_value) = if let Some(default_pos) = desc.find("Defaults to") {
                    let default_str = desc[default_pos + 11..].split('.').next().unwrap_or("");
                    (
                        desc[..default_pos].trim().to_string(),
                        Some(default_str.trim().to_string()),
                    )
                } else {
                    (desc.trim().to_string(), None)
                };

                params.push(HfParameter {
                    name,
                    description: desc_clean,
                    param_type,
                    default_value,
                    required: !text.contains("optional"),
                });
            }
        }
    }

    params
}

/// Extract related links
fn extract_related(document: &Html) -> Vec<String> {
    let mut related = Vec::new();

    if let Ok(selector) = Selector::parse(".see-also a, .related a") {
        for element in document.select(&selector).take(10) {
            let text = element.text().collect::<String>().trim().to_string();
            if !text.is_empty() {
                related.push(text);
            }
        }
    }

    related
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_creation() {
        let _client = HuggingFaceClient::new();
    }

    #[test]
    fn test_calculate_score() {
        let terms = vec!["automodel", "llama"];
        assert!(calculate_score("AutoModelForCausalLM", "Auto class for LLM", &terms) > 0);
        assert!(calculate_score("random", "unrelated", &terms) == 0);
    }
}
