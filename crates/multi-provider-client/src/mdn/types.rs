use serde::{Deserialize, Serialize};

/// MDN documentation category
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum MdnCategory {
    #[default]
    JavaScript,
    WebApi,
    Css,
    Html,
}

impl MdnCategory {
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::JavaScript => "JavaScript",
            Self::WebApi => "Web API",
            Self::Css => "CSS",
            Self::Html => "HTML",
        }
    }

    /// Infer category from MDN slug
    #[must_use]
    pub fn from_slug(slug: &str) -> Self {
        let slug_lower = slug.to_lowercase();
        if slug_lower.contains("/javascript/") || slug_lower.starts_with("javascript") {
            Self::JavaScript
        } else if slug_lower.contains("/api/") || slug_lower.contains("/web/api") {
            Self::WebApi
        } else if slug_lower.contains("/css/") {
            Self::Css
        } else if slug_lower.contains("/html/") {
            Self::Html
        } else {
            Self::JavaScript // Default
        }
    }
}

impl std::fmt::Display for MdnCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// A code example from MDN documentation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MdnExample {
    pub code: String,
    pub language: String,
    pub description: Option<String>,
    pub is_runnable: bool,
}

/// Parameter information for MDN functions/methods
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MdnParameter {
    pub name: String,
    pub description: String,
    pub param_type: Option<String>,
    pub optional: bool,
}

/// A searchable MDN article
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MdnArticle {
    pub slug: String,
    pub title: String,
    pub summary: String,
    pub category: MdnCategory,
    pub url: String,
    pub examples: Vec<MdnExample>,
    pub syntax: Option<String>,
    pub parameters: Vec<MdnParameter>,
    pub return_value: Option<String>,
    pub browser_compat: Option<String>,
    /// Full markdown/HTML content
    pub content: Option<String>,
}

/// MDN Technology representation for unified interface
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MdnTechnology {
    pub identifier: String,
    pub title: String,
    pub description: String,
    pub url: String,
    pub article_count: usize,
}

impl MdnTechnology {
    /// Create predefined MDN technologies
    #[must_use]
    pub fn predefined() -> Vec<Self> {
        vec![
            Self {
                identifier: "mdn:javascript".to_string(),
                title: "JavaScript".to_string(),
                description: "JavaScript language reference and built-in objects".to_string(),
                url: "https://developer.mozilla.org/en-US/docs/Web/JavaScript".to_string(),
                article_count: 0,
            },
            Self {
                identifier: "mdn:webapi".to_string(),
                title: "Web APIs".to_string(),
                description: "DOM, Fetch, Canvas, and other Web APIs".to_string(),
                url: "https://developer.mozilla.org/en-US/docs/Web/API".to_string(),
                article_count: 0,
            },
            Self {
                identifier: "mdn:css".to_string(),
                title: "CSS".to_string(),
                description: "CSS properties, selectors, and layout".to_string(),
                url: "https://developer.mozilla.org/en-US/docs/Web/CSS".to_string(),
                article_count: 0,
            },
            Self {
                identifier: "mdn:html".to_string(),
                title: "HTML".to_string(),
                description: "HTML elements, attributes, and semantics".to_string(),
                url: "https://developer.mozilla.org/en-US/docs/Web/HTML".to_string(),
                article_count: 0,
            },
        ]
    }
}

/// Search index entry for MDN
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MdnSearchEntry {
    pub slug: String,
    pub title: String,
    pub summary: String,
    pub category: MdnCategory,
    pub url: String,
}

/// MDN search API response
#[derive(Debug, Clone, Deserialize)]
pub struct MdnSearchResponse {
    pub documents: Vec<MdnSearchDocument>,
    #[serde(default)]
    pub metadata: MdnSearchMetadata,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MdnSearchDocument {
    pub mdn_url: String,
    pub slug: Option<String>,
    pub title: String,
    pub summary: String,
    #[serde(default)]
    pub score: f64,
    #[serde(default)]
    pub popularity: f64,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct MdnSearchMetadata {
    #[serde(default)]
    pub total: MdnSearchTotal,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct MdnSearchTotal {
    #[serde(default)]
    pub value: usize,
}

/// MDN document page response (for fetching full content)
#[derive(Debug, Clone, Deserialize)]
pub struct MdnDocumentResponse {
    pub doc: MdnDocument,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MdnDocument {
    #[serde(rename = "mdn_url")]
    pub url: String,
    pub title: String,
    #[serde(default)]
    pub summary: String,
    #[serde(default)]
    pub body: Vec<MdnSection>,
    #[serde(default)]
    pub source: MdnSource,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct MdnSource {
    #[serde(default)]
    pub github_url: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MdnSection {
    #[serde(rename = "type")]
    pub section_type: Option<String>,
    pub value: Option<MdnSectionValue>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum MdnSectionValue {
    Prose { content: String },
    Code { code: String, language: Option<String> },
    Text(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_category_from_slug() {
        assert_eq!(
            MdnCategory::from_slug("Web/JavaScript/Reference/Global_Objects/Array"),
            MdnCategory::JavaScript
        );
        assert_eq!(
            MdnCategory::from_slug("Web/API/Document/querySelector"),
            MdnCategory::WebApi
        );
        assert_eq!(
            MdnCategory::from_slug("Web/CSS/display"),
            MdnCategory::Css
        );
        assert_eq!(
            MdnCategory::from_slug("Web/HTML/Element/div"),
            MdnCategory::Html
        );
    }

    #[test]
    fn test_predefined_technologies() {
        let techs = MdnTechnology::predefined();
        assert_eq!(techs.len(), 4);
        assert!(techs.iter().any(|t| t.identifier == "mdn:javascript"));
    }
}
