use std::{path::PathBuf, sync::Arc};

use anyhow::Result;
use apple_docs_client::{AppleDocsClient, ClientConfig};

pub mod executor;
pub mod markdown;
pub mod services;
pub mod state;
pub mod tools;
pub mod transport;
use state::AppContext;
use time::OffsetDateTime;
use tracing::{debug, info};

/// Configuration inputs required to bootstrap the MCP server core.
#[derive(Debug, Clone)]
pub struct ServerConfig {
    /// Optional override for on-disk cache location.
    pub cache_dir: Option<PathBuf>,
    /// Timestamp captured during process initialization for diagnostics.
    pub boot_timestamp: OffsetDateTime,
    /// How the server transports requests/responses.
    pub mode: ServerMode,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServerMode {
    Stdio,
    Headless,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            cache_dir: None,
            boot_timestamp: OffsetDateTime::now_utc(),
            mode: ServerMode::Stdio,
        }
    }
}

#[derive(Clone)]
pub struct CoreRuntime {
    config: ServerConfig,
    executor: executor::ToolExecutor,
}

impl CoreRuntime {
    pub fn executor(&self) -> executor::ToolExecutor {
        self.executor.clone()
    }

    pub fn config(&self) -> &ServerConfig {
        &self.config
    }

    pub async fn serve(&self) -> Result<()> {
        match self.config.mode {
            ServerMode::Stdio => transport::serve_stdio(self.executor.clone()).await?,
            ServerMode::Headless => {
                debug!(target: "apple_docs_core", "Headless mode: skipping transport loop")
            }
        }
        Ok(())
    }
}

pub async fn bootstrap(config: ServerConfig) -> Result<CoreRuntime> {
    let client = match &config.cache_dir {
        Some(dir) => AppleDocsClient::with_config(ClientConfig {
            cache_dir: dir.clone(),
            ..ClientConfig::default()
        }),
        None => AppleDocsClient::new(),
    };

    let context = Arc::new(AppContext::new(client));
    tools::register_tools(context.clone()).await;

    debug!(
        target: "apple_docs_core",
        cache_dir = %context.client.cache_dir().display(),
        "AppleDocsClient initialized"
    );

    info!(
        target: "apple_docs_core",
        cache_dir = ?config.cache_dir,
        boot_timestamp = %config.boot_timestamp,
        mode = ?config.mode,
        "Core server starting"
    );

    let executor = executor::ToolExecutor::builder(context).build();
    Ok(CoreRuntime { config, executor })
}

/// Placeholder entry point for the core server runtime.
///
/// Later phases will replace this stub with the full MCP event loop.
pub async fn run(config: ServerConfig) -> Result<()> {
    bootstrap(config).await?.serve().await
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn stub_run_completes() {
        let tmp = tempdir().expect("tempdir");
        let mut config = ServerConfig::default();
        config.cache_dir = Some(tmp.path().to_path_buf());
        config.mode = ServerMode::Headless;
        let runtime = bootstrap(config).await.expect("bootstrap succeeds");
        let result = runtime.serve().await;
        assert!(result.is_ok());
    }
}

pub use executor::{ToolExecutor, ToolExecutorBuilder, ToolExecutorError};
