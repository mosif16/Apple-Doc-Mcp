use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use apple_docs_client::{
    types::{FrameworkData, ReferenceData, SymbolData, Technology},
    AppleDocsClient,
};
use futures::future::BoxFuture;
use multi_provider_client::{
    types::{ProviderType, UnifiedTechnology},
    ProviderClients,
};
use serde::Serialize;
use serde_json::Value;
use time::OffsetDateTime;
use tokio::sync::{Mutex, RwLock};

use crate::services::design_guidance::DesignSection;

#[derive(Clone)]
pub struct AppContext {
    pub client: Arc<AppleDocsClient>,
    pub providers: Arc<ProviderClients>,
    pub state: Arc<ServerState>,
    pub tools: Arc<ToolRegistry>,
}

impl AppContext {
    pub fn new(client: AppleDocsClient) -> Self {
        Self {
            client: Arc::new(client),
            providers: Arc::new(ProviderClients::new()),
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

    /// Get current cache statistics from the client
    pub fn cache_stats(&self) -> apple_docs_client::CombinedCacheStats {
        self.client.cache_stats()
    }
}

/// Multi-provider aware context for unified documentation access
#[derive(Clone)]
pub struct MultiProviderContext {
    pub providers: Arc<ProviderClients>,
    pub state: Arc<MultiProviderState>,
    pub tools: Arc<ToolRegistry>,
}

impl MultiProviderContext {
    #[must_use]
    pub fn new() -> Self {
        Self {
            providers: Arc::new(ProviderClients::new()),
            state: Arc::new(MultiProviderState::default()),
            tools: Arc::new(ToolRegistry::default()),
        }
    }

    /// Record a telemetry entry
    pub async fn record_telemetry(&self, entry: TelemetryEntry) {
        let mut guard = self.state.telemetry_log.lock().await;
        guard.push(entry);
        const MAX_ENTRIES: usize = 200;
        if guard.len() > MAX_ENTRIES {
            let overflow = guard.len() - MAX_ENTRIES;
            guard.drain(0..overflow);
        }
    }

    /// Get a snapshot of telemetry entries
    pub async fn telemetry_snapshot(&self) -> Vec<TelemetryEntry> {
        self.state.telemetry_log.lock().await.clone()
    }

    /// Get the currently active provider
    pub async fn active_provider(&self) -> ProviderType {
        *self.state.active_provider.read().await
    }

    /// Set the active provider
    pub async fn set_active_provider(&self, provider: ProviderType) {
        *self.state.active_provider.write().await = provider;
    }

    /// Get the active technology (unified)
    pub async fn active_technology(&self) -> Option<UnifiedTechnology> {
        self.state.active_unified_technology.read().await.clone()
    }

    /// Set the active technology
    pub async fn set_active_technology(&self, tech: Option<UnifiedTechnology>) {
        *self.state.active_unified_technology.write().await = tech;
    }
}

impl Default for MultiProviderContext {
    fn default() -> Self {
        Self::new()
    }
}

/// State for multi-provider context
#[derive(Default)]
pub struct MultiProviderState {
    /// Currently active provider
    pub active_provider: RwLock<ProviderType>,
    /// Currently selected technology (unified representation)
    pub active_unified_technology: RwLock<Option<UnifiedTechnology>>,
    /// Legacy: Apple-specific active technology (for backward compatibility)
    pub active_apple_technology: RwLock<Option<Technology>>,
    /// Cache of framework data per provider
    pub framework_cache: RwLock<Option<FrameworkData>>,
    /// Search index entries
    pub framework_index: RwLock<Option<Vec<FrameworkIndexEntry>>>,
    /// Global search indexes by framework
    pub global_indexes: RwLock<HashMap<String, Vec<FrameworkIndexEntry>>>,
    /// Expanded identifiers for navigation
    pub expanded_identifiers: Mutex<HashSet<String>>,
    /// Last fetched symbol
    pub last_symbol: RwLock<Option<SymbolData>>,
    /// Last discovery results
    pub last_discovery: RwLock<Option<MultiProviderDiscoverySnapshot>>,
    /// Telemetry log
    pub telemetry_log: Mutex<Vec<TelemetryEntry>>,
    /// Recent search queries
    pub recent_queries: Mutex<Vec<SearchQueryLog>>,
    /// Design guidance cache
    pub design_guidance_cache: RwLock<HashMap<String, Arc<DesignSection>>>,
}

/// Discovery snapshot for multi-provider results
#[derive(Clone)]
pub struct MultiProviderDiscoverySnapshot {
    pub query: Option<String>,
    pub provider_filter: Option<ProviderType>,
    pub results: Vec<UnifiedTechnology>,
}

#[derive(Default)]
pub struct ServerState {
    /// Currently active provider (Apple by default)
    pub active_provider: RwLock<ProviderType>,
    /// Active technology (Apple-specific for backward compatibility)
    pub active_technology: RwLock<Option<Technology>>,
    /// Active unified technology (provider-agnostic)
    pub active_unified_technology: RwLock<Option<UnifiedTechnology>>,
    pub framework_cache: RwLock<Option<FrameworkData>>,
    pub framework_index: RwLock<Option<Vec<FrameworkIndexEntry>>>,
    pub global_indexes: RwLock<HashMap<String, Vec<FrameworkIndexEntry>>>,
    pub expanded_identifiers: Mutex<HashSet<String>>,
    pub last_symbol: RwLock<Option<SymbolData>>,
    pub last_discovery: RwLock<Option<DiscoverySnapshot>>,
    pub telemetry_log: Mutex<Vec<TelemetryEntry>>,
    pub recent_queries: Mutex<Vec<SearchQueryLog>>,
    /// Pre-cached design guidance for the active technology
    /// Maps design guidance slug (e.g., "design/human-interface-guidelines/buttons") to sections
    pub design_guidance_cache: RwLock<HashMap<String, Arc<DesignSection>>>,
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
    /// Example inputs demonstrating correct tool usage patterns.
    /// These help Claude understand parameter combinations and formatting.
    #[serde(rename = "inputExamples", skip_serializing_if = "Option::is_none")]
    pub input_examples: Option<Vec<serde_json::Value>>,
    /// Enables programmatic tool calling - allows Claude to orchestrate this tool
    /// through code execution rather than sequential API calls.
    /// Set to `["code_execution_20250825"]` to enable.
    /// Benefits: 37% token reduction, 95% fewer inference passes for batch operations.
    #[serde(rename = "allowedCallers", skip_serializing_if = "Option::is_none")]
    pub allowed_callers: Option<Vec<String>>,
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
