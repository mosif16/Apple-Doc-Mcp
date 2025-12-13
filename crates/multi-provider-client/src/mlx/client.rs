//! MLX documentation client for Apple Silicon ML framework.
//!
//! Provides access to MLX-Swift and MLX Python documentation.

use std::path::PathBuf;
use std::time::Duration as StdDuration;

use anyhow::Result;
use directories::ProjectDirs;
use reqwest::Client;
use scraper::{Html, Selector};
use tracing::{debug, instrument, warn};

use super::types::{
    MlxArticle, MlxCategory, MlxCategoryItem, MlxExample, MlxItemKind, MlxLanguage,
    MlxParameter, MlxSearchResult, MlxTechnology, MLX_PYTHON_TOPICS, MLX_SWIFT_TOPICS,
};
use docs_mcp_client::cache::{DiskCache, MemoryCache};

const MLX_SWIFT_BASE: &str = "https://ml-explore.github.io/mlx-swift/documentation/mlx";
const MLX_PYTHON_BASE: &str = "https://ml-explore.github.io/mlx/build/html";

#[derive(Debug)]
pub struct MlxClient {
    http: Client,
    disk_cache: DiskCache,
    memory_cache: MemoryCache<Vec<u8>>,
    cache_dir: PathBuf,
}

impl Default for MlxClient {
    fn default() -> Self {
        Self::new()
    }
}

impl MlxClient {
    #[must_use]
    pub fn new() -> Self {
        let project_dirs = ProjectDirs::from("com", "RecordAndLearn", "multi-docs-mcp")
            .expect("unable to resolve project directories");

        let cache_dir = project_dirs.cache_dir().join("mlx");
        if let Err(e) = std::fs::create_dir_all(&cache_dir) {
            warn!(error = %e, "Failed to create MLX cache directory");
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

    /// Get available MLX technologies (Swift and Python)
    #[instrument(name = "mlx_client.get_technologies", skip(self))]
    pub async fn get_technologies(&self) -> Result<Vec<MlxTechnology>> {
        Ok(vec![
            MlxTechnology {
                identifier: "mlx:swift".to_string(),
                title: "MLX Swift".to_string(),
                description: "MLX for Swift - Machine learning on Apple Silicon for iOS/macOS".to_string(),
                url: MLX_SWIFT_BASE.to_string(),
                language: MlxLanguage::Swift,
            },
            MlxTechnology {
                identifier: "mlx:python".to_string(),
                title: "MLX Python".to_string(),
                description: "MLX for Python - NumPy-like ML framework for Apple Silicon".to_string(),
                url: MLX_PYTHON_BASE.to_string(),
                language: MlxLanguage::Python,
            },
        ])
    }

    /// Get category listing
    #[instrument(name = "mlx_client.get_category", skip(self))]
    pub async fn get_category(&self, identifier: &str) -> Result<MlxCategory> {
        let language = if identifier.contains("python") {
            MlxLanguage::Python
        } else {
            MlxLanguage::Swift
        };

        let topics = if language == MlxLanguage::Swift {
            MLX_SWIFT_TOPICS
        } else {
            MLX_PYTHON_TOPICS
        };

        let items: Vec<MlxCategoryItem> = topics
            .iter()
            .map(|(name, path, desc)| {
                let base = if language == MlxLanguage::Swift {
                    MLX_SWIFT_BASE
                } else {
                    MLX_PYTHON_BASE
                };

                MlxCategoryItem {
                    name: (*name).to_string(),
                    description: (*desc).to_string(),
                    kind: infer_item_kind(name),
                    path: (*path).to_string(),
                    url: format!("{}/{}", base, path),
                }
            })
            .collect();

        Ok(MlxCategory {
            identifier: identifier.to_string(),
            title: if language == MlxLanguage::Swift {
                "MLX Swift API".to_string()
            } else {
                "MLX Python API".to_string()
            },
            description: if language == MlxLanguage::Swift {
                "MLX-Swift machine learning framework for Apple Silicon".to_string()
            } else {
                "MLX Python machine learning framework for Apple Silicon".to_string()
            },
            items,
            language,
        })
    }

    /// Search MLX documentation
    #[instrument(name = "mlx_client.search", skip(self))]
    pub async fn search(&self, query: &str, language: Option<MlxLanguage>) -> Result<Vec<MlxSearchResult>> {
        let query_lower = query.to_lowercase();
        let query_terms: Vec<&str> = query_lower.split_whitespace().collect();

        let mut results = Vec::new();

        // Search Swift topics
        if language.is_none() || language == Some(MlxLanguage::Swift) {
            for (name, path, desc) in MLX_SWIFT_TOPICS {
                let score = calculate_score(name, desc, &query_terms);
                if score > 0 {
                    results.push(MlxSearchResult {
                        name: (*name).to_string(),
                        path: (*path).to_string(),
                        url: format!("{}/{}", MLX_SWIFT_BASE, path),
                        kind: infer_item_kind(name),
                        description: (*desc).to_string(),
                        language: MlxLanguage::Swift,
                        score,
                    });
                }
            }
        }

        // Search Python topics
        if language.is_none() || language == Some(MlxLanguage::Python) {
            for (name, path, desc) in MLX_PYTHON_TOPICS {
                let score = calculate_score(name, desc, &query_terms);
                if score > 0 {
                    results.push(MlxSearchResult {
                        name: (*name).to_string(),
                        path: (*path).to_string(),
                        url: format!("{}/{}", MLX_PYTHON_BASE, path),
                        kind: infer_item_kind(name),
                        description: (*desc).to_string(),
                        language: MlxLanguage::Python,
                        score,
                    });
                }
            }
        }

        // Sort by score descending
        results.sort_by(|a, b| b.score.cmp(&a.score));
        results.truncate(20);

        Ok(results)
    }

    /// Get detailed article documentation
    #[instrument(name = "mlx_client.get_article", skip(self))]
    pub async fn get_article(&self, path: &str, language: MlxLanguage) -> Result<MlxArticle> {
        let (base_url, topics) = if language == MlxLanguage::Swift {
            (MLX_SWIFT_BASE, MLX_SWIFT_TOPICS)
        } else {
            (MLX_PYTHON_BASE, MLX_PYTHON_TOPICS)
        };

        // Find in predefined topics
        let topic = topics
            .iter()
            .find(|(_, p, _)| *p == path || path.ends_with(p))
            .or_else(|| topics.iter().find(|(n, _, _)| path.contains(n)));

        let (name, url, desc) = match topic {
            Some((n, p, d)) => ((*n).to_string(), format!("{}/{}", base_url, p), (*d).to_string()),
            None => {
                // Try to construct URL directly
                let clean_path = path.strip_prefix("mlx:").unwrap_or(path);
                (
                    clean_path.split('/').last().unwrap_or(clean_path).to_string(),
                    format!("{}/{}", base_url, clean_path),
                    String::new(),
                )
            }
        };

        // Try to fetch live documentation
        let cache_key = format!("article_{}_{}.json", language, path.replace('/', "_"));

        if let Ok(Some(entry)) = self.disk_cache.load::<MlxArticle>(&cache_key).await {
            return Ok(entry.value);
        }

        // Fetch and parse the documentation page
        let article = if language == MlxLanguage::Swift {
            self.fetch_swift_article(&url, &name, &desc).await?
        } else {
            self.fetch_python_article(&url, &name, &desc).await?
        };

        // Cache the result
        let _ = self.disk_cache.store(&cache_key, article.clone()).await;

        Ok(article)
    }

    /// Fetch and parse MLX-Swift DocC documentation
    async fn fetch_swift_article(&self, url: &str, name: &str, default_desc: &str) -> Result<MlxArticle> {
        debug!(url = %url, "Fetching MLX-Swift documentation");

        let response = self.http.get(url).send().await;

        match response {
            Ok(resp) if resp.status().is_success() => {
                let html = resp.text().await?;
                self.parse_docc_article(&html, name, url, default_desc)
            }
            _ => {
                // Return a basic article with predefined info
                Ok(MlxArticle {
                    title: name.to_string(),
                    description: default_desc.to_string(),
                    path: url.replace(MLX_SWIFT_BASE, ""),
                    url: url.to_string(),
                    kind: infer_item_kind(name),
                    language: MlxLanguage::Swift,
                    declaration: None,
                    content: default_desc.to_string(),
                    examples: vec![],
                    parameters: vec![],
                    return_value: None,
                    related: vec![],
                    platforms: vec!["macOS 14.0+".to_string(), "iOS 17.0+".to_string()],
                })
            }
        }
    }

    /// Parse DocC HTML format (used by MLX-Swift)
    fn parse_docc_article(&self, html: &str, name: &str, url: &str, default_desc: &str) -> Result<MlxArticle> {
        let document = Html::parse_document(html);

        // Extract title
        let title = extract_text(&document, "h1.title, .primary-content-header h1")
            .unwrap_or_else(|| name.to_string());

        // Extract description/abstract
        let description = extract_text(&document, ".abstract, .topic-abstract, .contenttable-section p")
            .unwrap_or_else(|| default_desc.to_string());

        // Extract declaration
        let declaration = extract_text(&document, ".declaration code, pre.highlight, .source code");

        // Extract full content
        let content = extract_text(&document, ".content, .primary-content, .topic-content")
            .unwrap_or_else(|| description.clone());

        // Extract code examples
        let examples = extract_code_examples(&document, MlxLanguage::Swift);

        // Extract parameters
        let parameters = extract_parameters(&document);

        // Extract return value
        let return_value = extract_text(&document, ".returns p, .return-value");

        // Extract related links
        let related = extract_related_links(&document);

        Ok(MlxArticle {
            title,
            description,
            path: url.replace(MLX_SWIFT_BASE, "").trim_start_matches('/').to_string(),
            url: url.to_string(),
            kind: infer_item_kind(name),
            language: MlxLanguage::Swift,
            declaration,
            content,
            examples,
            parameters,
            return_value,
            related,
            platforms: vec!["macOS 14.0+".to_string(), "iOS 17.0+".to_string()],
        })
    }

    /// Fetch and parse MLX Python Sphinx documentation
    async fn fetch_python_article(&self, url: &str, name: &str, default_desc: &str) -> Result<MlxArticle> {
        debug!(url = %url, "Fetching MLX Python documentation");

        let response = self.http.get(url).send().await;

        match response {
            Ok(resp) if resp.status().is_success() => {
                let html = resp.text().await?;
                self.parse_sphinx_article(&html, name, url, default_desc)
            }
            _ => {
                // Return a basic article with predefined info
                Ok(MlxArticle {
                    title: name.to_string(),
                    description: default_desc.to_string(),
                    path: url.replace(MLX_PYTHON_BASE, ""),
                    url: url.to_string(),
                    kind: infer_item_kind(name),
                    language: MlxLanguage::Python,
                    declaration: None,
                    content: default_desc.to_string(),
                    examples: vec![],
                    parameters: vec![],
                    return_value: None,
                    related: vec![],
                    platforms: vec!["macOS with Apple Silicon".to_string()],
                })
            }
        }
    }

    /// Parse Sphinx HTML format (used by MLX Python)
    fn parse_sphinx_article(&self, html: &str, name: &str, url: &str, default_desc: &str) -> Result<MlxArticle> {
        let document = Html::parse_document(html);

        // Extract title
        let title = extract_text(&document, "h1, .document h1")
            .unwrap_or_else(|| name.to_string());

        // Extract description
        let description = extract_text(&document, ".section > p:first-of-type, dd > p:first-of-type")
            .unwrap_or_else(|| default_desc.to_string());

        // Extract declaration/signature
        let declaration = extract_text(&document, "dt.sig, .sig-prename, pre.literal-block");

        // Extract full content
        let content = extract_text(&document, ".section, .body")
            .unwrap_or_else(|| description.clone());

        // Extract code examples
        let examples = extract_code_examples(&document, MlxLanguage::Python);

        // Extract parameters from docstring
        let parameters = extract_sphinx_parameters(&document);

        // Extract return value
        let return_value = extract_text(&document, ".field-list .field:contains('Returns') dd");

        Ok(MlxArticle {
            title: title.trim_end_matches('¶').trim().to_string(),
            description,
            path: url.replace(MLX_PYTHON_BASE, "").trim_start_matches('/').to_string(),
            url: url.to_string(),
            kind: infer_item_kind(name),
            language: MlxLanguage::Python,
            declaration,
            content,
            examples,
            parameters,
            return_value,
            related: vec![],
            platforms: vec!["macOS with Apple Silicon".to_string()],
        })
    }

    pub fn cache_dir(&self) -> &PathBuf {
        &self.cache_dir
    }
}

/// Calculate search relevance score
fn calculate_score(name: &str, desc: &str, query_terms: &[&str]) -> i32 {
    let name_lower = name.to_lowercase();
    let desc_lower = desc.to_lowercase();

    let mut score = 0;

    for term in query_terms {
        // Exact name match
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

    // Boost for important ML terms
    let boost_terms = ["array", "linear", "conv", "attention", "transformer", "adam", "sgd", "loss", "grad", "compile", "module"];
    for boost in boost_terms {
        if name_lower.contains(boost) && query_terms.iter().any(|t| t.contains(boost)) {
            score += 20;
        }
    }

    score
}

/// Infer item kind from name
fn infer_item_kind(name: &str) -> MlxItemKind {
    let name_lower = name.to_lowercase();

    if name.starts_with(|c: char| c.is_uppercase()) && !name.contains("::") && !name.contains('.') {
        // Likely a class/type name
        if name_lower.contains("protocol") || name_lower.ends_with("able") {
            MlxItemKind::Protocol
        } else if name_lower.contains("enum") {
            MlxItemKind::Enum
        } else {
            MlxItemKind::Class
        }
    } else if name.contains("(") || name_lower.starts_with("mlx.") {
        MlxItemKind::Function
    } else if name_lower.contains("guide") || name_lower.contains("tutorial") {
        MlxItemKind::Guide
    } else if name.contains("::") || name.contains('.') {
        MlxItemKind::Module
    } else {
        MlxItemKind::Function
    }
}

/// Extract text from first matching selector
fn extract_text(document: &Html, selector_str: &str) -> Option<String> {
    let selector = Selector::parse(selector_str).ok()?;
    document
        .select(&selector)
        .next()
        .map(|el| el.text().collect::<String>().trim().to_string())
        .filter(|s| !s.is_empty())
}

/// Extract code examples from documentation
fn extract_code_examples(document: &Html, language: MlxLanguage) -> Vec<MlxExample> {
    let mut examples = Vec::new();

    let selector_str = if language == MlxLanguage::Swift {
        "pre code, .code-listing pre, .highlight pre"
    } else {
        "pre, .highlight-python pre, .doctest pre"
    };

    if let Ok(selector) = Selector::parse(selector_str) {
        for element in document.select(&selector).take(5) {
            let code = element.text().collect::<String>().trim().to_string();
            if !code.is_empty() && code.len() > 10 {
                examples.push(MlxExample {
                    code,
                    language: if language == MlxLanguage::Swift {
                        "swift".to_string()
                    } else {
                        "python".to_string()
                    },
                    description: None,
                });
            }
        }
    }

    examples
}

/// Extract parameters from DocC format
fn extract_parameters(document: &Html) -> Vec<MlxParameter> {
    let mut params = Vec::new();

    if let Ok(selector) = Selector::parse(".parameters li, .param dt, dl.field-list dd") {
        for element in document.select(&selector) {
            let text = element.text().collect::<String>();
            if let Some((name, desc)) = text.split_once(':') {
                params.push(MlxParameter {
                    name: name.trim().to_string(),
                    description: desc.trim().to_string(),
                    param_type: None,
                    default_value: None,
                });
            }
        }
    }

    params
}

/// Extract parameters from Sphinx format
fn extract_sphinx_parameters(document: &Html) -> Vec<MlxParameter> {
    let mut params = Vec::new();

    if let Ok(selector) = Selector::parse(".field-list .field-body ul li, dl.simple dt") {
        for element in document.select(&selector) {
            let text = element.text().collect::<String>();
            let parts: Vec<&str> = text.splitn(2, " – ").collect();
            if parts.len() == 2 {
                // Extract name and type from format like "name (type)"
                let name_part = parts[0].trim();
                let (name, param_type) = if let Some(paren_pos) = name_part.find('(') {
                    let name = name_part[..paren_pos].trim();
                    let ptype = name_part[paren_pos..].trim_matches(|c| c == '(' || c == ')');
                    (name.to_string(), Some(ptype.to_string()))
                } else {
                    (name_part.to_string(), None)
                };

                params.push(MlxParameter {
                    name,
                    description: parts[1].trim().to_string(),
                    param_type,
                    default_value: None,
                });
            }
        }
    }

    params
}

/// Extract related links
fn extract_related_links(document: &Html) -> Vec<String> {
    let mut related = Vec::new();

    if let Ok(selector) = Selector::parse(".see-also a, .related a, .seealso a") {
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
        let _client = MlxClient::new();
    }

    #[test]
    fn test_infer_item_kind() {
        assert_eq!(infer_item_kind("MLXArray"), MlxItemKind::Class);
        assert_eq!(infer_item_kind("softmax"), MlxItemKind::Function);
        assert_eq!(infer_item_kind("mlx.core.array"), MlxItemKind::Function);
    }

    #[test]
    fn test_calculate_score() {
        let terms = vec!["array", "mlx"];
        assert!(calculate_score("MLXArray", "Core array type", &terms) > 0);
        assert!(calculate_score("unrelated", "nothing here", &terms) == 0);
    }
}
