use std::{sync::Arc, time::Instant};

use anyhow::Result;
use serde_json::Value;
use thiserror::Error;
use time::OffsetDateTime;
use tracing::{info, warn};

use crate::state::{AppContext, TelemetryEntry, ToolDefinition, ToolResponse};

#[derive(Clone)]
pub struct ToolExecutor {
    context: Arc<AppContext>,
    options: ExecutorOptions,
}

#[derive(Clone)]
struct ExecutorOptions {
    record_telemetry: bool,
}

#[derive(Clone)]
pub struct ToolExecutorBuilder {
    context: Arc<AppContext>,
    options: ExecutorOptions,
}

impl ToolExecutorBuilder {
    pub fn new(context: Arc<AppContext>) -> Self {
        Self {
            context,
            options: ExecutorOptions {
                record_telemetry: true,
            },
        }
    }

    #[must_use]
    pub fn record_telemetry(mut self, enabled: bool) -> Self {
        self.options.record_telemetry = enabled;
        self
    }

    pub fn build(self) -> ToolExecutor {
        ToolExecutor {
            context: self.context,
            options: self.options,
        }
    }
}

impl ToolExecutor {
    pub fn builder(context: Arc<AppContext>) -> ToolExecutorBuilder {
        ToolExecutorBuilder::new(context)
    }

    pub fn context(&self) -> Arc<AppContext> {
        self.context.clone()
    }

    pub async fn list_tools(&self) -> Vec<ToolDefinition> {
        self.context.tools.definitions().await
    }

    pub async fn call_tool(
        &self,
        name: &str,
        arguments: Value,
    ) -> Result<ToolResponse, ToolExecutorError> {
        let Some(entry) = self.context.tools.get(name).await else {
            return Err(ToolExecutorError::UnknownTool(name.to_string()));
        };

        let handler = entry.handler.clone();
        let started = Instant::now();
        match handler(self.context.clone(), arguments).await {
            Ok(response) => {
                if self.options.record_telemetry {
                    self.record_success(name, started.elapsed().as_millis() as u64, &response)
                        .await;
                }
                Ok(response)
            }
            Err(source) => {
                if self.options.record_telemetry {
                    self.record_failure(
                        name,
                        started.elapsed().as_millis() as u64,
                        source.to_string(),
                    )
                    .await;
                }
                Err(ToolExecutorError::Execution {
                    name: name.to_string(),
                    source,
                })
            }
        }
    }

    async fn record_success(&self, name: &str, latency_ms: u64, response: &ToolResponse) {
        let metadata = response.metadata.clone();
        let entry = TelemetryEntry {
            tool: name.to_string(),
            timestamp: OffsetDateTime::now_utc(),
            latency_ms,
            success: true,
            metadata: metadata.clone(),
            error: None,
        };
        self.context.record_telemetry(entry).await;
        info!(
            target: "apple_docs_executor",
            tool = %name,
            latency_ms,
            success = true,
            metadata = metadata
                .as_ref()
                .map(|value| value.to_string())
                .unwrap_or_else(|| "null".to_string()),
            "tool completed"
        );
    }

    async fn record_failure(&self, name: &str, latency_ms: u64, message: String) {
        let entry = TelemetryEntry {
            tool: name.to_string(),
            timestamp: OffsetDateTime::now_utc(),
            latency_ms,
            success: false,
            metadata: None,
            error: Some(message.clone()),
        };
        self.context.record_telemetry(entry).await;
        warn!(
            target: "apple_docs_executor",
            tool = %name,
            latency_ms,
            error = %message,
            "tool failed"
        );
    }
}

#[derive(Debug, Error)]
pub enum ToolExecutorError {
    #[error("unknown tool: {0}")]
    UnknownTool(String),
    #[error("tool `{name}` failed: {source}")]
    Execution {
        name: String,
        #[source]
        source: anyhow::Error,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::{ToolContent, ToolDefinition, ToolEntry, ToolResponse};
    use serde_json::json;

    #[tokio::test]
    async fn executor_invokes_registered_tool() {
        let client = apple_docs_client::AppleDocsClient::new();
        let context = Arc::new(AppContext::new(client));

        let handler = Arc::new(
            move |_ctx: Arc<AppContext>, value: Value| -> crate::state::ToolFuture {
                Box::pin(async move {
                    let message = value
                        .get("message")
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                        .to_string();
                    Ok(ToolResponse {
                        content: vec![ToolContent {
                            r#type: "text".to_string(),
                            text: message,
                        }],
                        metadata: None,
                    })
                })
            },
        );

        let definition = ToolDefinition {
            name: "echo".to_string(),
            description: "Echo back a message".to_string(),
            input_schema: json!({}),
        };

        context
            .tools
            .insert(ToolEntry {
                definition,
                handler,
            })
            .await;

        let executor = ToolExecutor::builder(context.clone()).build();

        let response = executor
            .call_tool("echo", json!({"message": "hello"}))
            .await
            .expect("tool succeeds");

        assert_eq!(response.content.len(), 1);
        assert_eq!(response.content[0].text, "hello");
    }

    #[tokio::test]
    async fn executor_reports_unknown_tool() {
        let client = apple_docs_client::AppleDocsClient::new();
        let context = Arc::new(AppContext::new(client));
        let executor = ToolExecutor::builder(context).build();

        let error = executor
            .call_tool("missing", Value::Null)
            .await
            .expect_err("unknown tool should fail");
        assert!(error.to_string().contains("unknown tool"));
    }
}
