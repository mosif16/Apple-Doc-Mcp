use std::path::PathBuf;
use std::sync::Arc;

use anyhow::{Context, Result};
use docs_mcp_client::{AppleDocsClient, ClientConfig};
use docs_mcp_core::{run, state::AppContext, ServerConfig, ServerMode};
use serde_json::json;

const CACHE_DIR_ENV: &str = "DOCSMCP_CACHE_DIR";
const HEADLESS_ENV: &str = "DOCSMCP_HEADLESS";

/// Launches the MCP server using environment-informed defaults.
///
/// Phase 2 provides scaffolding only; the concrete implementation lands in later phases.
pub async fn run_server() -> Result<()> {
    let config = ServerConfig {
        cache_dir: resolve_cache_dir(),
        mode: resolve_mode(),
        ..Default::default()
    };

    tracing::info!(
        target: "docs_mcp",
        cache_dir = ?config.cache_dir,
        mode = ?config.mode,
        "Starting MCP server"
    );
    run(config).await
}

pub async fn oneshot_query(query: &str, max_results: Option<usize>) -> Result<docs_mcp_core::state::ToolResponse> {
    let client = match resolve_cache_dir() {
        Some(dir) => AppleDocsClient::with_config(ClientConfig {
            cache_dir: dir,
            ..ClientConfig::default()
        }),
        None => AppleDocsClient::new(),
    };

    let context = Arc::new(AppContext::new(client));
    docs_mcp_core::tools::register_tools(context.clone()).await;

    let tool = context
        .tools
        .get("query")
        .await
        .context("query tool not registered")?;

    let mut args = json!({ "query": query });
    if let Some(max) = max_results {
        args["maxResults"] = json!(max);
    }

    (tool.handler)(context, args).await
}

fn resolve_cache_dir() -> Option<PathBuf> {
    std::env::var_os(CACHE_DIR_ENV).map(PathBuf::from)
}

fn resolve_mode() -> ServerMode {
    match std::env::var_os(HEADLESS_ENV) {
        Some(value) if value == "1" || value.eq_ignore_ascii_case("true") => ServerMode::Headless,
        _ => ServerMode::Stdio,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn run_server_scaffold_succeeds() {
        std::env::set_var(CACHE_DIR_ENV, "/tmp/docs-mcp-cache");
        std::env::set_var(HEADLESS_ENV, "1");
        let result = run_server().await;
        assert!(result.is_ok());
    }
}
