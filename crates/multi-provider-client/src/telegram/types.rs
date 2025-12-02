use serde::{Deserialize, Serialize};

use std::collections::HashMap;

/// Telegram Bot API specification structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelegramApiSpec {
    pub version: String,
    pub release_date: String,
    pub changelog: String,
    #[serde(default)]
    pub methods: HashMap<String, TelegramMethodSpec>,
    #[serde(default)]
    pub types: HashMap<String, TelegramTypeSpec>,
}

/// Raw method from the Telegram Bot API spec
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelegramMethodSpec {
    pub name: String,
    pub href: String,
    #[serde(default)]
    pub description: Vec<String>,
    #[serde(default)]
    pub returns: Vec<String>,
    #[serde(default)]
    pub fields: Vec<TelegramFieldSpec>,
}

/// Raw type from the Telegram Bot API spec
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelegramTypeSpec {
    pub name: String,
    pub href: String,
    #[serde(default)]
    pub description: Vec<String>,
    #[serde(default)]
    pub fields: Vec<TelegramFieldSpec>,
    #[serde(default)]
    pub subtypes: Vec<String>,
    #[serde(default)]
    pub subtype_of: Vec<String>,
}

/// Field specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelegramFieldSpec {
    pub name: String,
    pub types: Vec<String>,
    pub required: bool,
    pub description: String,
}

/// Normalized technology representation for Telegram
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelegramTechnology {
    pub identifier: String,
    pub title: String,
    pub description: String,
    pub url: String,
    pub item_count: usize,
}

/// Category of Telegram items (Methods or Types)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelegramCategory {
    pub identifier: String,
    pub title: String,
    pub description: String,
    pub items: Vec<TelegramCategoryItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelegramCategoryItem {
    pub name: String,
    pub description: String,
    pub kind: String,
    pub href: String,
}

/// Detailed item (method or type)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelegramItem {
    pub name: String,
    pub description: String,
    pub kind: String, // "method" or "type"
    pub href: String,
    pub fields: Vec<TelegramItemField>,
    pub returns: Option<Vec<String>>,
    pub subtypes: Vec<String>,
    pub subtype_of: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelegramItemField {
    pub name: String,
    pub types: Vec<String>,
    pub required: bool,
    pub description: String,
}

impl TelegramItem {
    /// Create a TelegramItem from a method spec
    pub fn from_method(name: &str, method: &TelegramMethodSpec) -> Self {
        Self {
            name: name.to_string(),
            description: method.description.join(" "),
            kind: "method".to_string(),
            href: method.href.clone(),
            fields: method
                .fields
                .iter()
                .map(|f| TelegramItemField {
                    name: f.name.clone(),
                    types: f.types.clone(),
                    required: f.required,
                    description: f.description.clone(),
                })
                .collect(),
            returns: Some(method.returns.clone()),
            subtypes: vec![],
            subtype_of: vec![],
        }
    }

    /// Create a TelegramItem from a type spec
    pub fn from_type(name: &str, t: &TelegramTypeSpec) -> Self {
        Self {
            name: name.to_string(),
            description: t.description.join(" "),
            kind: "type".to_string(),
            href: t.href.clone(),
            fields: t
                .fields
                .iter()
                .map(|f| TelegramItemField {
                    name: f.name.clone(),
                    types: f.types.clone(),
                    required: f.required,
                    description: f.description.clone(),
                })
                .collect(),
            returns: None,
            subtypes: t.subtypes.clone(),
            subtype_of: t.subtype_of.clone(),
        }
    }
}
