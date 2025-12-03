use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use time::OffsetDateTime;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlatformInfo {
    pub name: String,
    #[serde(default)]
    pub introduced_at: Option<String>,
    #[serde(default)]
    pub beta: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrameworkData {
    #[serde(rename = "abstract")]
    pub r#abstract: Vec<RichText>,
    pub metadata: FrameworkMetadata,
    pub references: HashMap<String, ReferenceData>,
    #[serde(default, rename = "topicSections")]
    pub topic_sections: Vec<TopicSection>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrameworkMetadata {
    pub platforms: Vec<PlatformInfo>,
    pub role: String,
    pub title: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopicSection {
    #[serde(default)]
    pub anchor: Option<String>,
    #[serde(default)]
    pub identifiers: Vec<String>,
    pub title: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]

pub struct ReferenceData {
    pub title: Option<String>,
    #[serde(default)]
    pub kind: Option<String>,
    #[serde(default, rename = "abstract")]
    pub r#abstract: Option<Vec<RichText>>,
    #[serde(default)]
    pub platforms: Option<Vec<PlatformInfo>>,
    pub url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RichText {
    #[serde(default)]
    pub text: Option<String>,
    #[serde(rename = "type")]
    pub kind: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolData {
    #[serde(rename = "abstract")]
    pub r#abstract: Vec<RichText>,
    pub metadata: SymbolMetadata,
    #[serde(default, rename = "primaryContentSections")]
    pub primary_content_sections: Vec<serde_json::Value>,
    pub references: HashMap<String, ReferenceData>,
    #[serde(default, rename = "topicSections")]
    pub topic_sections: Vec<TopicSection>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolMetadata {
    #[serde(default)]
    pub platforms: Vec<PlatformInfo>,
    #[serde(default)]
    pub symbol_kind: Option<String>,
    pub title: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Technology {
    #[serde(rename = "abstract", default)]
    pub r#abstract: Vec<RichText>,
    #[serde(default)]
    pub identifier: String,
    #[serde(default)]
    pub kind: String,
    #[serde(default)]
    pub role: String,
    pub title: String,
    #[serde(default)]
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub description: String,
    pub framework: String,
    pub path: String,
    pub platforms: Option<String>,
    #[serde(default)]
    pub symbol_kind: Option<String>,
    pub title: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheEntry<T> {
    pub value: T,
    pub stored_at: OffsetDateTime,
    #[serde(default = "OffsetDateTime::now_utc")]
    pub last_accessed: OffsetDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopicMetadata {
    #[serde(default)]
    pub title: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopicData {
    #[serde(rename = "abstract", default)]
    pub r#abstract: Vec<RichText>,
    #[serde(default, rename = "topicSections")]
    pub topic_sections: Vec<TopicSection>,
    #[serde(default)]
    pub references: HashMap<String, ReferenceData>,
    pub metadata: TopicMetadata,
}
