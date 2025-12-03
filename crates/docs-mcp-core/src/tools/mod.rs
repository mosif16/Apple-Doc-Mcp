use std::sync::Arc;

use anyhow::{anyhow, Result};

use crate::state::{AppContext, ToolContent, ToolEntry, ToolHandler, ToolResponse};

mod batch_documentation;
mod choose_technology;
mod current_technology;
mod discover;
mod get_documentation;
mod how_do_i;
mod query;
mod search_symbols;

pub async fn register_tools(context: Arc<AppContext>) {
    // Register only the unified query tool
    // Other tools are kept in the codebase for reference but not exposed via MCP
    let tools = [
        query::definition(),
    ];

    let registry = context.tools.clone();

    for (definition, handler) in tools {
        let entry = ToolEntry {
            definition,
            handler,
        };
        registry.insert(entry).await;
    }
}

pub(crate) fn text_response(lines: impl IntoIterator<Item = String>) -> ToolResponse {
    ToolResponse {
        content: vec![ToolContent {
            r#type: "text".to_string(),
            text: lines.into_iter().collect::<Vec<_>>().join("\n"),
        }],
        metadata: None,
    }
}

pub(crate) fn wrap_handler<F, Fut>(handler: F) -> ToolHandler
where
    F: Fn(Arc<AppContext>, serde_json::Value) -> Fut + Send + Sync + 'static,
    Fut: std::future::Future<Output = Result<ToolResponse>> + Send + 'static,
{
    Arc::new(move |context, value| {
        let ctx = context.clone();
        let fut = handler(ctx, value);
        Box::pin(fut)
    })
}

pub(crate) fn parse_args<T: serde::de::DeserializeOwned>(value: serde_json::Value) -> Result<T> {
    serde_json::from_value(value).map_err(|error| anyhow!("invalid arguments: {error}"))
}

pub use current_technology::definition as current_technology_definition;
pub use discover::definition as discover_technologies_definition;
pub use get_documentation::definition as get_documentation_definition;
pub use search_symbols::definition as search_symbols_definition;
