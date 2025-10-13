use std::sync::Arc;

use anyhow::Result;

use crate::{
    markdown,
    state::{AppContext, ToolDefinition, ToolHandler, ToolResponse},
    tools::{simple_text, text_response, wrap_handler},
};

pub fn definition() -> (ToolDefinition, ToolHandler) {
    (
        ToolDefinition {
            name: "current_technology".to_string(),
            description: "Report the currently selected technology".to_string(),
            input_schema: serde_json::json!({"type": "object", "properties": {}}),
        },
        wrap_handler(|context, _value| async move { handle(context).await }),
    )
}

async fn handle(context: Arc<AppContext>) -> Result<ToolResponse> {
    if let Some(active) = context.state.active_technology.read().await.clone() {
        let lines = vec![
            markdown::header(1, "📘 Current Technology"),
            String::new(),
            markdown::bold("Name", &active.title),
            markdown::bold("Identifier", &active.identifier),
            String::new(),
            markdown::header(2, "Next actions"),
            "• `search_symbols { \"query\": \"keyword\" }` to find symbols".to_string(),
            "• `get_documentation { \"path\": \"SymbolName\" }` to open docs".to_string(),
            "• `choose_technology \"Another Framework\"` to switch".to_string(),
        ];
        Ok(text_response(lines))
    } else {
        Ok(simple_text(
            "🚦 Technology Not Selected\nUse `discover_technologies` then `choose_technology` to get started.",
        ))
    }
}
