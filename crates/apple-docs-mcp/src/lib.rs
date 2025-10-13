use std::path::PathBuf;

use anyhow::Result;
use apple_docs_core::{run, ServerConfig, ServerMode};

const CACHE_DIR_ENV: &str = "APPLEDOC_CACHE_DIR";
const HEADLESS_ENV: &str = "APPLEDOC_HEADLESS";

/// Launches the MCP server using environment-informed defaults.
///
/// Phase 2 provides scaffolding only; the concrete implementation lands in later phases.
pub async fn run_server() -> Result<()> {
    let mut config = ServerConfig::default();
    config.cache_dir = resolve_cache_dir();
    config.mode = resolve_mode();

    tracing::info!(
        target: "apple_docs_mcp",
        cache_dir = ?config.cache_dir,
        mode = ?config.mode,
        "Starting MCP server"
    );
    run(config).await
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
        std::env::set_var(CACHE_DIR_ENV, "/tmp/apple-docs-cache");
        std::env::set_var(HEADLESS_ENV, "1");
        let result = run_server().await;
        assert!(result.is_ok());
    }
}
