use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::cocoon::types::{CocoonDocument, CocoonSection, CocoonTechnology};
use crate::rust::types::{RustCategory, RustItem, RustTechnology};
use crate::telegram::types::{TelegramCategory, TelegramItem, TelegramTechnology};
use crate::ton::types::{TonCategory, TonEndpoint, TonTechnology};

/// Provider type enum for identifying documentation sources
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum ProviderType {
    #[default]
    Apple,
    Telegram,
    TON,
    Cocoon,
    Rust,
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
}

impl UnifiedTechnology {
    pub fn from_apple(tech: apple_docs_client::types::Technology) -> Self {
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
    pub fn from_apple(data: apple_docs_client::types::FrameworkData) -> Self {
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
    pub fn from_apple(data: apple_docs_client::types::SymbolData) -> Self {
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
}
