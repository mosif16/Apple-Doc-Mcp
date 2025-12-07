use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::claude_agent_sdk::types::{
    AgentSdkArticle, AgentSdkCategory, AgentSdkLanguage, AgentSdkTechnology,
};
use crate::cocoon::types::{CocoonDocument, CocoonSection, CocoonTechnology};
use crate::huggingface::types::{HfArticle, HfCategory, HfTechnology, HfTechnologyKind};
use crate::mdn::types::{MdnArticle, MdnCategory, MdnTechnology};
use crate::mlx::types::{MlxArticle, MlxCategory, MlxLanguage, MlxTechnology};
use crate::quicknode::types::{QuickNodeCategory, QuickNodeMethod, QuickNodeTechnology};
use crate::rust::types::{RustCategory, RustItem, RustTechnology};
use crate::telegram::types::{TelegramCategory, TelegramItem, TelegramTechnology};
use crate::ton::types::{TonCategory, TonEndpoint, TonTechnology};
use crate::web_frameworks::types::{
    CodeExample, WebFramework, WebFrameworkArticle, WebFrameworkTechnology,
};

/// Provider type enum for identifying documentation sources
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum ProviderType {
    #[default]
    Apple,
    Telegram,
    TON,
    Cocoon,
    Rust,
    Mdn,
    WebFrameworks,
    /// MLX - Apple Silicon ML framework
    Mlx,
    /// Hugging Face - LLM models and transformers
    HuggingFace,
    /// QuickNode - Solana blockchain RPC documentation
    QuickNode,
    /// Claude Agent SDK - TypeScript and Python SDKs for building AI agents
    ClaudeAgentSdk,
}

impl ProviderType {
    #[must_use]
    pub fn name(&self) -> &'static str {
        match self {
            Self::Apple => "Apple",
            Self::Telegram => "Telegram",
            Self::TON => "TON",
            Self::Cocoon => "Cocoon",
            Self::Rust => "Rust",
            Self::Mdn => "MDN",
            Self::WebFrameworks => "Web Frameworks",
            Self::Mlx => "MLX",
            Self::HuggingFace => "Hugging Face",
            Self::QuickNode => "QuickNode",
            Self::ClaudeAgentSdk => "Claude Agent SDK",
        }
    }

    #[must_use]
    pub fn description(&self) -> &'static str {
        match self {
            Self::Apple => "Apple Developer Documentation",
            Self::Telegram => "Telegram Bot API Documentation",
            Self::TON => "TON Blockchain API Documentation",
            Self::Cocoon => "Cocoon Verifiable AI Documentation",
            Self::Rust => "Rust Language and Crate Documentation",
            Self::Mdn => "MDN Web Documentation (JavaScript, Web APIs, CSS)",
            Self::WebFrameworks => "React, Next.js, and Node.js Documentation",
            Self::Mlx => "MLX Machine Learning Framework for Apple Silicon",
            Self::HuggingFace => "Hugging Face Transformers and Model Documentation",
            Self::QuickNode => "QuickNode Solana RPC Documentation",
            Self::ClaudeAgentSdk => "Claude Agent SDK for TypeScript and Python",
        }
    }
}

impl std::fmt::Display for ProviderType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name())
    }
}

/// Unified technology representation across all providers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedTechnology {
    pub provider: ProviderType,
    pub identifier: String,
    pub title: String,
    pub description: String,
    pub url: Option<String>,
    pub kind: TechnologyKind,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TechnologyKind {
    /// Apple framework (SwiftUI, UIKit, etc.)
    Framework,
    /// Telegram category (Methods, Types, etc.)
    ApiCategory,
    /// TON API tag (Accounts, NFTs, etc.)
    BlockchainApi,
    /// Cocoon documentation section
    DocSection,
    /// Rust crate (std, serde, tokio, etc.)
    RustCrate,
    /// MDN category (JavaScript, Web API, CSS, HTML)
    MdnCategory,
    /// Web framework (React, Next.js, Node.js)
    WebFramework,
    /// MLX framework (Swift or Python)
    MlxFramework,
    /// Hugging Face library (Transformers, Hub, etc.)
    HfLibrary,
    /// QuickNode Solana API (HTTP, WebSocket, Marketplace)
    QuickNodeApi,
    /// Claude Agent SDK library (TypeScript or Python)
    AgentSdkLibrary,
}

impl UnifiedTechnology {
    pub fn from_apple(tech: docs_mcp_client::types::Technology) -> Self {
        let description = tech
            .r#abstract
            .iter()
            .filter_map(|rt| rt.text.clone())
            .collect::<Vec<_>>()
            .join(" ");

        Self {
            provider: ProviderType::Apple,
            identifier: tech.identifier.clone(),
            title: tech.title,
            description,
            url: Some(tech.url),
            kind: TechnologyKind::Framework,
        }
    }

    pub fn from_telegram(tech: TelegramTechnology) -> Self {
        Self {
            provider: ProviderType::Telegram,
            identifier: tech.identifier,
            title: tech.title,
            description: tech.description,
            url: Some(tech.url),
            kind: TechnologyKind::ApiCategory,
        }
    }

    pub fn from_ton(tech: TonTechnology) -> Self {
        Self {
            provider: ProviderType::TON,
            identifier: tech.identifier,
            title: tech.title,
            description: tech.description,
            url: Some(tech.url),
            kind: TechnologyKind::BlockchainApi,
        }
    }

    pub fn from_cocoon(tech: CocoonTechnology) -> Self {
        Self {
            provider: ProviderType::Cocoon,
            identifier: tech.identifier,
            title: tech.title,
            description: tech.description,
            url: tech.url,
            kind: TechnologyKind::DocSection,
        }
    }

    pub fn from_rust(tech: RustTechnology) -> Self {
        Self {
            provider: ProviderType::Rust,
            identifier: tech.identifier,
            title: tech.title,
            description: tech.description,
            url: Some(tech.url),
            kind: TechnologyKind::RustCrate,
        }
    }

    pub fn from_mdn(tech: MdnTechnology) -> Self {
        Self {
            provider: ProviderType::Mdn,
            identifier: tech.identifier,
            title: tech.title,
            description: tech.description,
            url: Some(tech.url),
            kind: TechnologyKind::MdnCategory,
        }
    }

    pub fn from_web_framework(tech: WebFrameworkTechnology) -> Self {
        Self {
            provider: ProviderType::WebFrameworks,
            identifier: tech.identifier,
            title: tech.title,
            description: tech.description,
            url: Some(tech.url),
            kind: TechnologyKind::WebFramework,
        }
    }

    pub fn from_mlx(tech: MlxTechnology) -> Self {
        Self {
            provider: ProviderType::Mlx,
            identifier: tech.identifier,
            title: tech.title,
            description: tech.description,
            url: Some(tech.url),
            kind: TechnologyKind::MlxFramework,
        }
    }

    pub fn from_huggingface(tech: HfTechnology) -> Self {
        Self {
            provider: ProviderType::HuggingFace,
            identifier: tech.identifier,
            title: tech.title,
            description: tech.description,
            url: Some(tech.url),
            kind: TechnologyKind::HfLibrary,
        }
    }

    pub fn from_quicknode(tech: QuickNodeTechnology) -> Self {
        Self {
            provider: ProviderType::QuickNode,
            identifier: tech.identifier,
            title: tech.title,
            description: tech.description,
            url: Some(tech.url),
            kind: TechnologyKind::QuickNodeApi,
        }
    }

    pub fn from_claude_agent_sdk(tech: AgentSdkTechnology) -> Self {
        Self {
            provider: ProviderType::ClaudeAgentSdk,
            identifier: tech.identifier,
            title: tech.title,
            description: tech.description,
            url: Some(tech.url),
            kind: TechnologyKind::AgentSdkLibrary,
        }
    }
}

/// Unified framework/category data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedFrameworkData {
    pub provider: ProviderType,
    pub title: String,
    pub description: String,
    pub items: Vec<UnifiedReference>,
    pub sections: Vec<UnifiedSection>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedReference {
    pub identifier: String,
    pub title: String,
    pub description: Option<String>,
    pub kind: Option<String>,
    pub url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedSection {
    pub title: String,
    pub identifiers: Vec<String>,
}

impl UnifiedFrameworkData {
    pub fn from_apple(data: docs_mcp_client::types::FrameworkData) -> Self {
        let description = data
            .r#abstract
            .iter()
            .filter_map(|rt| rt.text.clone())
            .collect::<Vec<_>>()
            .join(" ");

        let items: Vec<UnifiedReference> = data
            .references
            .into_iter()
            .map(|(id, r)| {
                let desc = r
                    .r#abstract
                    .as_ref()
                    .map(|abs| {
                        abs.iter()
                            .filter_map(|rt| rt.text.clone())
                            .collect::<Vec<_>>()
                            .join(" ")
                    });
                UnifiedReference {
                    identifier: id,
                    title: r.title.unwrap_or_default(),
                    description: desc,
                    kind: r.kind,
                    url: r.url,
                }
            })
            .collect();

        let sections = data
            .topic_sections
            .into_iter()
            .map(|s| UnifiedSection {
                title: s.title,
                identifiers: s.identifiers,
            })
            .collect();

        Self {
            provider: ProviderType::Apple,
            title: data.metadata.title,
            description,
            items,
            sections,
        }
    }

    pub fn from_telegram(data: TelegramCategory) -> Self {
        let items = data
            .items
            .into_iter()
            .map(|item| UnifiedReference {
                identifier: item.name.clone(),
                title: item.name,
                description: Some(item.description),
                kind: Some(item.kind),
                url: Some(item.href),
            })
            .collect();

        Self {
            provider: ProviderType::Telegram,
            title: data.title,
            description: data.description,
            items,
            sections: vec![],
        }
    }

    pub fn from_ton(data: TonCategory) -> Self {
        let items = data
            .endpoints
            .into_iter()
            .map(|ep| UnifiedReference {
                identifier: ep.operation_id.clone(),
                title: ep.summary.unwrap_or_else(|| ep.operation_id.clone()),
                description: ep.description,
                kind: Some(ep.method.to_uppercase()),
                url: Some(ep.path),
            })
            .collect();

        Self {
            provider: ProviderType::TON,
            title: data.tag,
            description: data.description,
            items,
            sections: vec![],
        }
    }

    pub fn from_cocoon(data: CocoonSection) -> Self {
        let items = data
            .documents
            .into_iter()
            .map(|doc| UnifiedReference {
                identifier: doc.path.clone(),
                title: doc.title,
                description: Some(doc.summary),
                kind: Some("document".to_string()),
                url: Some(doc.url),
            })
            .collect();

        Self {
            provider: ProviderType::Cocoon,
            title: data.title,
            description: data.description,
            items,
            sections: vec![],
        }
    }

    pub fn from_rust(data: RustCategory) -> Self {
        let items = data
            .items
            .into_iter()
            .map(|item| UnifiedReference {
                identifier: item.path.clone(),
                title: item.name,
                description: Some(item.description),
                kind: Some(item.kind.to_string()),
                url: Some(item.url),
            })
            .collect();

        Self {
            provider: ProviderType::Rust,
            title: data.title,
            description: data.description,
            items,
            sections: vec![],
        }
    }

    pub fn from_mlx(data: MlxCategory) -> Self {
        let items = data
            .items
            .into_iter()
            .map(|item| UnifiedReference {
                identifier: item.path.clone(),
                title: item.name,
                description: Some(item.description),
                kind: Some(item.kind.to_string()),
                url: Some(item.url),
            })
            .collect();

        Self {
            provider: ProviderType::Mlx,
            title: data.title,
            description: data.description,
            items,
            sections: vec![],
        }
    }

    pub fn from_huggingface(data: HfCategory) -> Self {
        let items = data
            .items
            .into_iter()
            .map(|item| UnifiedReference {
                identifier: item.path.clone(),
                title: item.name,
                description: Some(item.description),
                kind: Some(item.kind.to_string()),
                url: Some(item.url),
            })
            .collect();

        Self {
            provider: ProviderType::HuggingFace,
            title: data.title,
            description: data.description,
            items,
            sections: vec![],
        }
    }

    pub fn from_quicknode(data: QuickNodeCategory) -> Self {
        let items = data
            .items
            .into_iter()
            .map(|item| UnifiedReference {
                identifier: item.name.clone(),
                title: item.name,
                description: Some(item.description),
                kind: Some(item.kind.to_string()),
                url: Some(item.url),
            })
            .collect();

        Self {
            provider: ProviderType::QuickNode,
            title: data.title,
            description: data.description,
            items,
            sections: vec![],
        }
    }

    pub fn from_claude_agent_sdk(data: AgentSdkCategory) -> Self {
        let items = data
            .items
            .into_iter()
            .map(|item| UnifiedReference {
                identifier: item.path.clone(),
                title: item.name,
                description: Some(item.description),
                kind: Some(item.kind.to_string()),
                url: Some(item.url),
            })
            .collect();

        Self {
            provider: ProviderType::ClaudeAgentSdk,
            title: data.title,
            description: data.description,
            items,
            sections: vec![],
        }
    }
}

/// Unified symbol/item data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedSymbolData {
    pub provider: ProviderType,
    pub title: String,
    pub description: String,
    pub kind: Option<String>,
    pub content: SymbolContent,
    pub related: Vec<UnifiedReference>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SymbolContent {
    /// Apple symbol with sections
    Apple {
        platforms: Vec<String>,
        sections: Vec<serde_json::Value>,
    },
    /// Telegram method or type
    Telegram {
        fields: Vec<TelegramField>,
        returns: Option<Vec<String>>,
    },
    /// TON endpoint
    Ton {
        method: String,
        path: String,
        parameters: Vec<TonParameter>,
        responses: HashMap<String, String>,
    },
    /// Cocoon markdown document
    Cocoon { markdown: String },
    /// Rust documentation item
    Rust {
        crate_name: String,
        crate_version: String,
        module_path: String,
        signature: Option<String>,
        documentation: String,
        source_url: Option<String>,
    },
    /// MDN article content
    Mdn {
        category: String,
        syntax: Option<String>,
        parameters: Vec<MdnParamInfo>,
        return_value: Option<String>,
        browser_compat: Option<String>,
        examples: Vec<MdnExampleInfo>,
    },
    /// Web framework documentation
    WebFramework {
        framework: String,
        api_signature: Option<String>,
        examples: Vec<WebFrameworkExampleInfo>,
        content: String,
    },
    /// MLX documentation
    Mlx {
        language: String,
        declaration: Option<String>,
        documentation: String,
        examples: Vec<MlxExampleInfo>,
        platforms: Vec<String>,
    },
    /// Hugging Face documentation
    HuggingFace {
        technology: String,
        declaration: Option<String>,
        documentation: String,
        examples: Vec<HfExampleInfo>,
        parameters: Vec<HfParamInfo>,
    },
    /// QuickNode Solana RPC documentation
    QuickNode {
        method_kind: String,
        parameters: Vec<QuickNodeParamInfo>,
        returns: Option<QuickNodeReturnInfo>,
        examples: Vec<QuickNodeExampleInfo>,
    },
    /// Claude Agent SDK documentation
    ClaudeAgentSdk {
        language: String,
        declaration: Option<String>,
        documentation: String,
        examples: Vec<AgentSdkExampleInfo>,
        parameters: Vec<AgentSdkParamInfo>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuickNodeParamInfo {
    pub name: String,
    pub description: String,
    pub param_type: String,
    pub required: bool,
    pub default_value: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuickNodeReturnInfo {
    pub type_name: String,
    pub description: String,
    pub fields: Vec<QuickNodeFieldInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuickNodeFieldInfo {
    pub name: String,
    pub field_type: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuickNodeExampleInfo {
    pub code: String,
    pub language: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MdnParamInfo {
    pub name: String,
    pub description: String,
    pub param_type: Option<String>,
    pub optional: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MdnExampleInfo {
    pub code: String,
    pub language: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebFrameworkExampleInfo {
    pub code: String,
    pub language: String,
    pub filename: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MlxExampleInfo {
    pub code: String,
    pub language: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HfExampleInfo {
    pub code: String,
    pub language: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HfParamInfo {
    pub name: String,
    pub description: String,
    pub param_type: Option<String>,
    pub default_value: Option<String>,
    pub required: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentSdkExampleInfo {
    pub code: String,
    pub language: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentSdkParamInfo {
    pub name: String,
    pub description: String,
    pub param_type: Option<String>,
    pub default_value: Option<String>,
    pub required: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelegramField {
    pub name: String,
    pub types: Vec<String>,
    pub required: bool,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TonParameter {
    pub name: String,
    pub location: String, // "path", "query", "body"
    pub required: bool,
    pub description: Option<String>,
    pub schema_type: Option<String>,
}

impl UnifiedSymbolData {
    pub fn from_apple(data: docs_mcp_client::types::SymbolData) -> Self {
        let description = data
            .r#abstract
            .iter()
            .filter_map(|rt| rt.text.clone())
            .collect::<Vec<_>>()
            .join(" ");

        let platforms = data
            .metadata
            .platforms
            .iter()
            .map(|p| p.name.clone())
            .collect();

        let related = data
            .references
            .into_iter()
            .map(|(id, r)| {
                let desc = r.r#abstract.as_ref().map(|abs| {
                    abs.iter()
                        .filter_map(|rt| rt.text.clone())
                        .collect::<Vec<_>>()
                        .join(" ")
                });
                UnifiedReference {
                    identifier: id,
                    title: r.title.unwrap_or_default(),
                    description: desc,
                    kind: r.kind,
                    url: r.url,
                }
            })
            .collect();

        Self {
            provider: ProviderType::Apple,
            title: data.metadata.title.unwrap_or_default(),
            description,
            kind: data.metadata.symbol_kind,
            content: SymbolContent::Apple {
                platforms,
                sections: data.primary_content_sections,
            },
            related,
        }
    }

    pub fn from_telegram(data: TelegramItem) -> Self {
        let fields = data
            .fields
            .into_iter()
            .map(|f| TelegramField {
                name: f.name,
                types: f.types,
                required: f.required,
                description: f.description,
            })
            .collect();

        Self {
            provider: ProviderType::Telegram,
            title: data.name,
            description: data.description,
            kind: Some(data.kind),
            content: SymbolContent::Telegram {
                fields,
                returns: data.returns,
            },
            related: vec![],
        }
    }

    pub fn from_ton(data: TonEndpoint) -> Self {
        let parameters = data
            .parameters
            .into_iter()
            .map(|p| TonParameter {
                name: p.name,
                location: p.location,
                required: p.required,
                description: p.description,
                schema_type: p.schema_type,
            })
            .collect();

        Self {
            provider: ProviderType::TON,
            title: data.summary.unwrap_or_else(|| data.operation_id.clone()),
            description: data.description.unwrap_or_default(),
            kind: Some(data.method.to_uppercase()),
            content: SymbolContent::Ton {
                method: data.method,
                path: data.path,
                parameters,
                responses: data.responses,
            },
            related: vec![],
        }
    }

    pub fn from_cocoon(data: CocoonDocument) -> Self {
        Self {
            provider: ProviderType::Cocoon,
            title: data.title,
            description: data.summary,
            kind: Some("document".to_string()),
            content: SymbolContent::Cocoon {
                markdown: data.content,
            },
            related: vec![],
        }
    }

    pub fn from_rust(data: RustItem) -> Self {
        Self {
            provider: ProviderType::Rust,
            title: data.name,
            description: data.summary,
            kind: Some(data.kind.to_string()),
            content: SymbolContent::Rust {
                crate_name: data.crate_name,
                crate_version: data.crate_version,
                module_path: data.path,
                signature: data.declaration,
                documentation: data.documentation.unwrap_or_default(),
                source_url: data.source_url,
            },
            related: vec![],
        }
    }

    pub fn from_mdn(data: MdnArticle) -> Self {
        let parameters = data
            .parameters
            .into_iter()
            .map(|p| MdnParamInfo {
                name: p.name,
                description: p.description,
                param_type: p.param_type,
                optional: p.optional,
            })
            .collect();

        let examples = data
            .examples
            .into_iter()
            .map(|e| MdnExampleInfo {
                code: e.code,
                language: e.language,
                description: e.description,
            })
            .collect();

        Self {
            provider: ProviderType::Mdn,
            title: data.title,
            description: data.summary,
            kind: Some(data.category.to_string()),
            content: SymbolContent::Mdn {
                category: data.category.to_string(),
                syntax: data.syntax,
                parameters,
                return_value: data.return_value,
                browser_compat: data.browser_compat,
                examples,
            },
            related: vec![],
        }
    }

    pub fn from_web_framework(data: WebFrameworkArticle) -> Self {
        let examples = data
            .examples
            .into_iter()
            .map(|e| WebFrameworkExampleInfo {
                code: e.code,
                language: e.language,
                filename: e.filename,
                description: e.description,
            })
            .collect();

        Self {
            provider: ProviderType::WebFrameworks,
            title: data.title,
            description: data.description,
            kind: Some(data.framework.to_string()),
            content: SymbolContent::WebFramework {
                framework: data.framework.to_string(),
                api_signature: data.api_signature,
                examples,
                content: data.content,
            },
            related: data
                .related
                .into_iter()
                .map(|r| UnifiedReference {
                    identifier: r.clone(),
                    title: r,
                    description: None,
                    kind: None,
                    url: None,
                })
                .collect(),
        }
    }

    pub fn from_mlx(data: MlxArticle) -> Self {
        let examples = data
            .examples
            .into_iter()
            .map(|e| MlxExampleInfo {
                code: e.code,
                language: e.language,
                description: e.description,
            })
            .collect();

        Self {
            provider: ProviderType::Mlx,
            title: data.title,
            description: data.description,
            kind: Some(data.kind.to_string()),
            content: SymbolContent::Mlx {
                language: data.language.to_string(),
                declaration: data.declaration,
                documentation: data.content,
                examples,
                platforms: data.platforms,
            },
            related: data
                .related
                .into_iter()
                .map(|r| UnifiedReference {
                    identifier: r.clone(),
                    title: r,
                    description: None,
                    kind: None,
                    url: None,
                })
                .collect(),
        }
    }

    pub fn from_huggingface(data: HfArticle) -> Self {
        let examples = data
            .examples
            .into_iter()
            .map(|e| HfExampleInfo {
                code: e.code,
                language: e.language,
                description: e.description,
            })
            .collect();

        let parameters = data
            .parameters
            .into_iter()
            .map(|p| HfParamInfo {
                name: p.name,
                description: p.description,
                param_type: p.param_type,
                default_value: p.default_value,
                required: p.required,
            })
            .collect();

        Self {
            provider: ProviderType::HuggingFace,
            title: data.title,
            description: data.description,
            kind: Some(data.kind.to_string()),
            content: SymbolContent::HuggingFace {
                technology: data.technology.to_string(),
                declaration: data.declaration,
                documentation: data.content,
                examples,
                parameters,
            },
            related: data
                .related
                .into_iter()
                .map(|r| UnifiedReference {
                    identifier: r.clone(),
                    title: r,
                    description: None,
                    kind: None,
                    url: None,
                })
                .collect(),
        }
    }

    pub fn from_quicknode(data: QuickNodeMethod) -> Self {
        let parameters = data
            .parameters
            .into_iter()
            .map(|p| QuickNodeParamInfo {
                name: p.name,
                description: p.description,
                param_type: p.param_type,
                required: p.required,
                default_value: p.default_value,
            })
            .collect();

        let returns = data.returns.map(|r| QuickNodeReturnInfo {
            type_name: r.type_name,
            description: r.description,
            fields: r
                .fields
                .into_iter()
                .map(|f| QuickNodeFieldInfo {
                    name: f.name,
                    field_type: f.field_type,
                    description: f.description,
                })
                .collect(),
        });

        let examples = data
            .examples
            .into_iter()
            .map(|e| QuickNodeExampleInfo {
                code: e.code,
                language: e.language,
                description: e.description,
            })
            .collect();

        Self {
            provider: ProviderType::QuickNode,
            title: data.name,
            description: data.description,
            kind: Some(data.kind.to_string()),
            content: SymbolContent::QuickNode {
                method_kind: data.kind.to_string(),
                parameters,
                returns,
                examples,
            },
            related: vec![],
        }
    }

    pub fn from_claude_agent_sdk(data: AgentSdkArticle) -> Self {
        let examples = data
            .examples
            .into_iter()
            .map(|e| AgentSdkExampleInfo {
                code: e.code,
                language: e.language,
                description: e.description,
            })
            .collect();

        let parameters = data
            .parameters
            .into_iter()
            .map(|p| AgentSdkParamInfo {
                name: p.name,
                description: p.description,
                param_type: p.param_type,
                default_value: p.default_value,
                required: p.required,
            })
            .collect();

        Self {
            provider: ProviderType::ClaudeAgentSdk,
            title: data.title,
            description: data.description,
            kind: Some(data.kind.to_string()),
            content: SymbolContent::ClaudeAgentSdk {
                language: data.language.to_string(),
                declaration: data.declaration,
                documentation: data.content,
                examples,
                parameters,
            },
            related: data
                .related
                .into_iter()
                .map(|r| UnifiedReference {
                    identifier: r.clone(),
                    title: r,
                    description: None,
                    kind: None,
                    url: None,
                })
                .collect(),
        }
    }
}
