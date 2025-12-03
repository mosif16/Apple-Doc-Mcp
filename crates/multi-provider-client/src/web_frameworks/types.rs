use serde::{Deserialize, Serialize};

/// Web framework identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum WebFramework {
    #[default]
    React,
    NextJs,
    NodeJs,
}

impl WebFramework {
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::React => "react",
            Self::NextJs => "nextjs",
            Self::NodeJs => "nodejs",
        }
    }

    #[must_use]
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::React => "React",
            Self::NextJs => "Next.js",
            Self::NodeJs => "Node.js",
        }
    }

    #[must_use]
    pub fn base_url(&self) -> &'static str {
        match self {
            Self::React => "https://react.dev",
            Self::NextJs => "https://nextjs.org",
            Self::NodeJs => "https://nodejs.org",
        }
    }

    /// Parse framework from string
    #[must_use]
    pub fn from_str_opt(s: &str) -> Option<Self> {
        let lower = s.to_lowercase();
        if lower.contains("react") {
            Some(Self::React)
        } else if lower.contains("next") {
            Some(Self::NextJs)
        } else if lower.contains("node") {
            Some(Self::NodeJs)
        } else {
            None
        }
    }
}

impl std::fmt::Display for WebFramework {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.display_name())
    }
}

/// A code example with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeExample {
    pub code: String,
    pub language: String,
    pub filename: Option<String>,
    pub description: Option<String>,
    pub is_complete: bool,
    pub has_output: bool,
}

impl CodeExample {
    /// Calculate quality score for ranking
    #[must_use]
    pub fn quality_score(&self) -> i32 {
        let mut score = 0;

        // Completeness indicators
        let has_imports = self.code.contains("import ") || self.code.contains("require(");
        let has_function =
            self.code.contains("function ") || self.code.contains("=>") || self.code.contains("fn ");
        let has_export = self.code.contains("export ");

        // Boost complete examples
        if has_imports {
            score += 10;
        }
        if has_function {
            score += 5;
        }
        if has_export {
            score += 5;
        }
        if self.is_complete {
            score += 20;
        }
        if self.has_output {
            score += 15;
        }

        // Boost examples with descriptions
        if self.description.is_some() {
            score += 10;
        }

        // Boost examples with filenames (shows context)
        if self.filename.is_some() {
            score += 5;
        }

        // Penalize very short snippets
        if self.code.len() < 50 {
            score -= 10;
        }

        // Boost longer, more complete examples
        if self.code.len() > 200 {
            score += 5;
        }

        score
    }
}

/// Documentation article from a web framework
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebFrameworkArticle {
    pub framework: WebFramework,
    pub slug: String,
    pub title: String,
    pub description: String,
    pub content: String,
    pub examples: Vec<CodeExample>,
    pub api_signature: Option<String>,
    pub related: Vec<String>,
    pub url: String,
}

/// Technology representation for unified interface
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebFrameworkTechnology {
    pub identifier: String,
    pub framework: WebFramework,
    pub title: String,
    pub description: String,
    pub url: String,
    pub version: String,
}

impl WebFrameworkTechnology {
    /// Create predefined technologies
    #[must_use]
    pub fn predefined() -> Vec<Self> {
        vec![
            Self {
                identifier: "webfw:react".to_string(),
                framework: WebFramework::React,
                title: "React".to_string(),
                description: "A JavaScript library for building user interfaces".to_string(),
                url: "https://react.dev".to_string(),
                version: "19".to_string(),
            },
            Self {
                identifier: "webfw:nextjs".to_string(),
                framework: WebFramework::NextJs,
                title: "Next.js".to_string(),
                description: "The React Framework for the Web".to_string(),
                url: "https://nextjs.org".to_string(),
                version: "15".to_string(),
            },
            Self {
                identifier: "webfw:nodejs".to_string(),
                framework: WebFramework::NodeJs,
                title: "Node.js".to_string(),
                description: "JavaScript runtime built on Chrome's V8 engine".to_string(),
                url: "https://nodejs.org".to_string(),
                version: "22".to_string(),
            },
        ]
    }
}

/// Search entry for web framework documentation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebFrameworkSearchEntry {
    pub framework: WebFramework,
    pub slug: String,
    pub title: String,
    pub description: String,
    pub url: String,
    pub category: Option<String>,
}

/// Node.js API documentation structure
#[derive(Debug, Clone, Deserialize)]
pub struct NodeApiModule {
    pub name: String,
    #[serde(default)]
    pub displayName: Option<String>,
    #[serde(default)]
    pub desc: Option<String>,
    #[serde(default)]
    pub stability: Option<i32>,
    #[serde(default)]
    pub methods: Vec<NodeApiMethod>,
    #[serde(default)]
    pub classes: Vec<NodeApiClass>,
    #[serde(default)]
    pub properties: Vec<NodeApiProperty>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct NodeApiMethod {
    pub name: String,
    #[serde(default)]
    pub desc: Option<String>,
    #[serde(default)]
    pub signatures: Vec<NodeApiSignature>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct NodeApiSignature {
    #[serde(default)]
    pub params: Vec<NodeApiParam>,
    #[serde(default)]
    pub return_type: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct NodeApiParam {
    pub name: String,
    #[serde(default)]
    pub type_name: Option<String>,
    #[serde(default)]
    pub desc: Option<String>,
    #[serde(default)]
    pub optional: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct NodeApiClass {
    pub name: String,
    #[serde(default)]
    pub desc: Option<String>,
    #[serde(default)]
    pub methods: Vec<NodeApiMethod>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct NodeApiProperty {
    pub name: String,
    #[serde(default)]
    pub desc: Option<String>,
}

/// React documentation page structure (from react.dev)
#[derive(Debug, Clone, Deserialize)]
pub struct ReactDocPage {
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub content: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_framework_from_str() {
        assert_eq!(WebFramework::from_str_opt("React"), Some(WebFramework::React));
        assert_eq!(WebFramework::from_str_opt("nextjs"), Some(WebFramework::NextJs));
        assert_eq!(WebFramework::from_str_opt("Node.js"), Some(WebFramework::NodeJs));
        assert_eq!(WebFramework::from_str_opt("python"), None);
    }

    #[test]
    fn test_code_example_score() {
        let complete_example = CodeExample {
            code: "import React from 'react';\n\nexport function App() {\n  return <div>Hello</div>;\n}".to_string(),
            language: "jsx".to_string(),
            filename: Some("App.jsx".to_string()),
            description: Some("A simple React component".to_string()),
            is_complete: true,
            has_output: false,
        };

        let snippet = CodeExample {
            code: "<div>Hi</div>".to_string(),
            language: "jsx".to_string(),
            filename: None,
            description: None,
            is_complete: false,
            has_output: false,
        };

        assert!(complete_example.quality_score() > snippet.quality_score());
    }

    #[test]
    fn test_predefined_technologies() {
        let techs = WebFrameworkTechnology::predefined();
        assert_eq!(techs.len(), 3);
        assert!(techs.iter().any(|t| t.framework == WebFramework::React));
        assert!(techs.iter().any(|t| t.framework == WebFramework::NextJs));
        assert!(techs.iter().any(|t| t.framework == WebFramework::NodeJs));
    }
}
