use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use apple_docs_client::{
    types::{FrameworkData, ReferenceData, SymbolData, Technology},
    AppleDocsClient, AndroidDocsClient, FlutterDocsClient, DocsPlatform,
};
use futures::future::BoxFuture;
use serde::Serialize;
use serde_json::Value;
use time::OffsetDateTime;
use tokio::sync::{Mutex, RwLock};

#[derive(Clone)]
pub struct AppContext {
    pub client: Arc<AppleDocsClient>,
    pub android_client: Arc<AndroidDocsClient>,
    pub flutter_client: Arc<FlutterDocsClient>,
    pub state: Arc<ServerState>,
    pub tools: Arc<ToolRegistry>,
}

impl AppContext {
    pub fn new(client: AppleDocsClient) -> Self {
        Self {
            client: Arc::new(client),
            android_client: Arc::new(AndroidDocsClient::new()),
            flutter_client: Arc::new(FlutterDocsClient::new()),
            state: Arc::new(ServerState::default()),
            tools: Arc::new(ToolRegistry::default()),
        }
    }

    pub fn with_all_clients(
        apple: AppleDocsClient,
        android: AndroidDocsClient,
        flutter: FlutterDocsClient,
    ) -> Self {
        Self {
            client: Arc::new(apple),
            android_client: Arc::new(android),
            flutter_client: Arc::new(flutter),
            state: Arc::new(ServerState::default()),
            tools: Arc::new(ToolRegistry::default()),
        }
    }

    pub async fn record_telemetry(&self, entry: TelemetryEntry) {
        let mut guard = self.state.telemetry_log.lock().await;
        guard.push(entry);
        const MAX_ENTRIES: usize = 200;
        if guard.len() > MAX_ENTRIES {
            let overflow = guard.len() - MAX_ENTRIES;
            guard.drain(0..overflow);
        }
    }

    pub async fn telemetry_snapshot(&self) -> Vec<TelemetryEntry> {
        self.state.telemetry_log.lock().await.clone()
    }
}

#[derive(Default)]
pub struct ServerState {
    /// Currently active documentation platform
    pub active_platform: RwLock<DocsPlatform>,
    /// Apple: Currently active technology/framework
    pub active_technology: RwLock<Option<Technology>>,
    pub framework_cache: RwLock<Option<FrameworkData>>,
    pub framework_index: RwLock<Option<Vec<FrameworkIndexEntry>>>,
    pub global_indexes: RwLock<HashMap<String, Vec<FrameworkIndexEntry>>>,
    pub expanded_identifiers: Mutex<HashSet<String>>,
    pub last_symbol: RwLock<Option<SymbolData>>,
    pub last_discovery: RwLock<Option<DiscoverySnapshot>>,
    pub telemetry_log: Mutex<Vec<TelemetryEntry>>,
    pub recent_queries: Mutex<Vec<SearchQueryLog>>,
    /// Android: Currently active library
    pub active_android_library: RwLock<Option<String>>,
    /// Flutter: Currently active library
    pub active_flutter_library: RwLock<Option<String>>,
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
pub struct SearchQueryLog {
    pub technology: Option<String>,
    pub scope: String,
    pub query: String,
    pub matches: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<OffsetDateTime>,
}

#[derive(Clone, Serialize)]
pub struct TelemetryEntry {
    pub tool: String,
    #[serde(with = "time::serde::rfc3339")]
    pub timestamp: OffsetDateTime,
    pub latency_ms: u64,
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<Value>,
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

impl ToolResponse {
    pub fn with_metadata(mut self, metadata: Value) -> Self {
        self.metadata = Some(metadata);
        self
    }
}
