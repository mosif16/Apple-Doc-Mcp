use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use apple_docs_client::{
    types::{FrameworkData, ReferenceData, SymbolData, Technology},
    AppleDocsClient,
};
use futures::future::BoxFuture;
use serde::Serialize;
use tokio::sync::{Mutex, RwLock};

#[derive(Clone)]
pub struct AppContext {
    pub client: Arc<AppleDocsClient>,
    pub state: Arc<ServerState>,
    pub tools: Arc<ToolRegistry>,
}

impl AppContext {
    pub fn new(client: AppleDocsClient) -> Self {
        Self {
            client: Arc::new(client),
            state: Arc::new(ServerState::default()),
            tools: Arc::new(ToolRegistry::default()),
        }
    }
}

#[derive(Default)]
pub struct ServerState {
    pub active_technology: RwLock<Option<Technology>>,
    pub framework_cache: RwLock<Option<FrameworkData>>,
    pub framework_index: RwLock<Option<Vec<FrameworkIndexEntry>>>,
    pub global_indexes: RwLock<HashMap<String, Vec<FrameworkIndexEntry>>>,
    pub expanded_identifiers: Mutex<HashSet<String>>,
    pub last_symbol: RwLock<Option<SymbolData>>,
    pub last_discovery: RwLock<Option<DiscoverySnapshot>>,
}

#[derive(Clone)]
pub struct FrameworkIndexEntry {
    pub id: String,
    pub tokens: Vec<String>,
    pub reference: ReferenceData,
}

#[derive(Clone)]
pub struct DiscoverySnapshot {
    pub query: Option<String>,
    pub results: Vec<Technology>,
}

#[derive(Clone, Serialize)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    #[serde(rename = "inputSchema")]
    pub input_schema: serde_json::Value,
}

#[derive(Clone, Serialize)]
pub struct ToolResponse {
    pub content: Vec<ToolContent>,
}

#[derive(Clone, Serialize)]
pub struct ToolContent {
    pub r#type: String,
    pub text: String,
}

pub type ToolFuture = BoxFuture<'static, anyhow::Result<ToolResponse>>;
pub type ToolHandler = Arc<dyn Fn(Arc<AppContext>, serde_json::Value) -> ToolFuture + Send + Sync>;

#[derive(Clone)]
pub struct ToolEntry {
    pub definition: ToolDefinition,
    pub handler: ToolHandler,
}

#[derive(Clone, Default)]
pub struct ToolRegistry {
    inner: Arc<RwLock<HashMap<String, ToolEntry>>>,
}

impl ToolRegistry {
    pub async fn insert(&self, entry: ToolEntry) {
        self.inner
            .write()
            .await
            .insert(entry.definition.name.clone(), entry);
    }

    pub async fn get(&self, name: &str) -> Option<ToolEntry> {
        self.inner.read().await.get(name).cloned()
    }

    pub async fn definitions(&self) -> Vec<ToolDefinition> {
        self.inner
            .read()
            .await
            .values()
            .map(|entry| entry.definition.clone())
            .collect()
    }
}
