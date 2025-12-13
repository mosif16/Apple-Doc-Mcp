use std::{sync::Arc, time::Instant};

use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio::io::{self, AsyncBufReadExt, AsyncWrite, AsyncWriteExt, BufReader};
use tracing::{debug, info, warn};

use crate::state::{AppContext, TelemetryEntry};
use time::OffsetDateTime;

const SERVER_INSTRUCTIONS: &str = r#"You are connected to a multi-provider documentation server. Use the `query` tool to retrieve official documentation for Apple platforms, Rust, Telegram Bot API, TON blockchain, Cocoon, MDN Web Docs, Web Frameworks (React, Next.js, Node.js), MLX (Apple Silicon ML), Hugging Face (Transformers), QuickNode (Solana), Claude Agent SDK, and Vertcoin (cryptocurrency).

## How to Use

**Single tool, complete context:** The `query` tool returns full documentation inline—no follow-up calls needed.

## Feedback (Helps Us Improve)

If you notice missing coverage, irrelevant search results, formatting issues, or performance problems, please call the `submit_feedback` tool with:
- a short summary of what happened
- example queries/symbols that failed
- what you'd like to see improved

**Natural language queries work best:**
- "SwiftUI NavigationStack" → Apple SwiftUI docs with code samples
- "Rust tokio spawn async" → Rust crate documentation
- "Telegram sendMessage" → Bot API method details with parameters
- "how to use CoreData fetch requests" → Implementation guidance
- "JavaScript Array map" → MDN Web Docs with examples
- "React useState hook" → React documentation with usage patterns
- "Next.js server components" → Next.js App Router documentation
- "Node.js fs readFile" → Node.js API documentation
- "MLX array operations Swift" → MLX framework documentation
- "Hugging Face AutoModel" → Transformers library documentation
- "Vertcoin getblockchaininfo" → Vertcoin RPC method documentation
- "Verthash mining setup" → Vertcoin mining specifications

## What You Get

For top results, the tool returns:
- **Full documentation content** (not truncated summaries)
- **Code examples** ready to use
- **Declarations/signatures** for API reference
- **Parameters** with descriptions
- **Platform availability** information
- **Related APIs** for further exploration

## Response Guidelines

1. Use the documentation content directly in your answers
2. Cite the symbol name or API when referencing specific features
3. If results are empty, suggest alternative query keywords
4. The tool auto-detects the provider—just describe what you need

## Supported Providers

- **Apple**: SwiftUI, UIKit, Foundation, CoreData, CoreML, Vision, and 60+ frameworks
- **Rust**: Standard library (std, core, alloc) and crates (tokio, serde, etc.)
- **Telegram**: Bot API methods and types
- **TON**: Blockchain API endpoints
- **Cocoon**: Confidential computing documentation
- **MDN**: JavaScript, TypeScript, Web APIs, DOM documentation
- **Web Frameworks**: React, Next.js, Node.js documentation with examples
- **MLX**: Apple Silicon ML framework (Swift and Python)
- **Hugging Face**: Transformers and swift-transformers for LLM development
- **QuickNode**: Solana blockchain RPC documentation
- **Claude Agent SDK**: TypeScript and Python SDKs for AI agents
- **Vertcoin**: GPU-mineable cryptocurrency with Verthash algorithm (80+ RPC methods)"#;

const DISABLE_FEEDBACK_PROMPT_ENV: &str = "DOCSMCP_DISABLE_FEEDBACK_PROMPT";

pub async fn serve_stdio(context: Arc<AppContext>) -> Result<()> {
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut reader = BufReader::new(stdin);
    let mut writer = stdout;

    let mut feedback_prompt_sent = false;
    let mut buffer = String::new();
    loop {
        buffer.clear();
        let bytes = reader.read_line(&mut buffer).await?;
        if bytes == 0 {
            info!(target: "docs_mcp_transport", "STDIO closed; shutting down");
            break;
        }

        debug!(target: "docs_mcp_transport", request = buffer.trim());
        let maybe_response = match serde_json::from_str::<RpcRequest>(&buffer) {
            Ok(request) => {
                if !feedback_prompt_sent
                    && !feedback_prompt_disabled()
                    && request.id.is_none()
                    && request.method == "notifications/initialized"
                {
                    feedback_prompt_sent = true;
                    if let Err(error) = send_feedback_prompt(&mut writer).await {
                        warn!(
                            target: "docs_mcp_transport",
                            error = %error,
                            "Failed to send feedback prompt notification"
                        );
                    }
                }
                handle_request(context.clone(), request).await
            }
            Err(error) => {
                warn!(target: "docs_mcp_transport", error = %error, "Failed to parse request");
                Some(RpcResponse::error(None, -32700, "Parse error"))
            }
        };

        if let Some(response) = maybe_response {
            let payload = serde_json::to_string(&response)?;
            writer.write_all(payload.as_bytes()).await?;
            writer.write_all(b"\n").await?;
            writer.flush().await?;
        }
    }

    Ok(())
}

fn feedback_prompt_disabled() -> bool {
    match std::env::var(DISABLE_FEEDBACK_PROMPT_ENV) {
        Ok(value) => value == "1" || value.eq_ignore_ascii_case("true"),
        Err(_) => false,
    }
}

async fn send_feedback_prompt<W>(writer: &mut W) -> Result<()>
where
    W: AsyncWrite + Unpin,
{
    #[derive(Serialize)]
    struct RpcNotification<'a> {
        jsonrpc: &'static str,
        method: &'a str,
        params: serde_json::Value,
    }

    let notification = RpcNotification {
        jsonrpc: "2.0",
        method: "notifications/message",
        params: json!({
            "level": "info",
            "message": "Help improve docs-mcp: if anything was missing/slow/confusing, call the `submit_feedback` tool with examples (queries/symbols) and suggestions."
        }),
    };

    let payload = serde_json::to_string(&notification)?;
    writer.write_all(payload.as_bytes()).await?;
    writer.write_all(b"\n").await?;
    writer.flush().await?;
    Ok(())
}

#[derive(Debug, Deserialize)]
struct RpcRequest {
    pub id: Option<serde_json::Value>,
    pub method: String,
    pub params: Option<serde_json::Value>,
}

#[derive(Debug, Serialize)]
struct RpcResponse {
    jsonrpc: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    id: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<RpcError>,
}

#[derive(Debug, Serialize)]
struct RpcError {
    code: i32,
    message: String,
}

impl RpcResponse {
    fn result(id: Option<serde_json::Value>, value: serde_json::Value) -> Self {
        Self {
            jsonrpc: "2.0",
            id,
            result: Some(value),
            error: None,
        }
    }

    fn error(id: Option<serde_json::Value>, code: i32, message: impl Into<String>) -> Self {
        Self {
            jsonrpc: "2.0",
            id,
            result: None,
            error: Some(RpcError {
                code,
                message: message.into(),
            }),
        }
    }
}

async fn handle_request(context: Arc<AppContext>, request: RpcRequest) -> Option<RpcResponse> {
    let method = request.method.as_str();

    if request.id.is_none() {
        match method {
            "notifications/initialized" => {
                info!(target: "docs_mcp_transport", "Client signaled initialized");
            }
            other => {
                debug!(
                    target: "docs_mcp_transport",
                    method = other,
                    "Ignoring notification without handler"
                );
            }
        }
        return None;
    }

    let id_value = request
        .id
        .clone()
        .expect("id is present because notifications are handled above");

    match method {
        "initialize" => Some(RpcResponse::result(
            Some(id_value.clone()),
            json!({
                "protocolVersion": "2024-11-05",
                "serverInfo": {
                    "name": "docs-mcp",
                    "version": env!("CARGO_PKG_VERSION"),
                },
                "capabilities": {
                    "tools": {}
                },
                "instructions": SERVER_INSTRUCTIONS,
            }),
        )),
        "list_tools" | "tools/list" => {
            let definitions = context.tools.definitions().await;
            Some(RpcResponse::result(
                Some(id_value.clone()),
                json!({"tools": definitions}),
            ))
        }
        "call_tool" | "tools/call" => {
            let params = request.params.unwrap_or_else(|| serde_json::json!({}));

            let name_value = params.get("name").cloned();
            let arguments = params
                .get("arguments")
                .cloned()
                .unwrap_or_else(|| serde_json::json!({}));

            match name_value {
                Some(name_value) => {
                    let name = match name_value.as_str() {
                        Some(name) => name.to_string(),
                        None => {
                            return Some(RpcResponse::error(
                                Some(id_value.clone()),
                                -32602,
                                "Tool name must be a string",
                            ))
                        }
                    };

                    match context.tools.get(&name).await {
                        Some(entry) => {
                            let handler = entry.handler.clone();
                            let started = Instant::now();
                            match handler(context.clone(), arguments).await {
                                Ok(response) => {
                                    let latency_ms = started.elapsed().as_millis() as u64;
                                    let metadata = response.metadata.clone();
                                    let entry = TelemetryEntry {
                                        tool: name.clone(),
                                        timestamp: OffsetDateTime::now_utc(),
                                        latency_ms,
                                        success: true,
                                        metadata: metadata.clone(),
                                        error: None,
                                    };
                                    context.record_telemetry(entry).await;
                                    info!(
                                        target: "docs_mcp_transport",
                                        tool = %name,
                                        latency_ms,
                                        success = true,
                                        metadata = metadata.as_ref().map(|value| value.to_string()).unwrap_or_else(|| "null".to_string()),
                                        "tool completed"
                                    );
                                    match serde_json::to_value(response) {
                                        Ok(value) => Some(RpcResponse::result(
                                            Some(id_value.clone()),
                                            value,
                                        )),
                                        Err(e) => Some(RpcResponse::error(
                                            Some(id_value.clone()),
                                            -32603,
                                            format!("Internal error: failed to serialize response: {}", e),
                                        )),
                                    }
                                }
                                Err(error) => {
                                    let latency_ms = started.elapsed().as_millis() as u64;
                                    let message = error.to_string();
                                    let entry = TelemetryEntry {
                                        tool: name.clone(),
                                        timestamp: OffsetDateTime::now_utc(),
                                        latency_ms,
                                        success: false,
                                        metadata: None,
                                        error: Some(message.clone()),
                                    };
                                    context.record_telemetry(entry).await;
                                    warn!(
                                        target: "docs_mcp_transport",
                                        tool = %name,
                                        latency_ms,
                                        error = %message,
                                        "tool failed"
                                    );
                                    Some(RpcResponse::error(
                                        Some(id_value.clone()),
                                        -32000,
                                        message,
                                    ))
                                }
                            }
                        }
                        None => Some(RpcResponse::error(
                            Some(id_value.clone()),
                            -32601,
                            format!("Unknown tool: {}", name),
                        )),
                    }
                }
                None => Some(RpcResponse::error(
                    Some(id_value.clone()),
                    -32602,
                    "Missing tool name",
                )),
            }
        }
        _ => Some(RpcResponse::error(
            Some(id_value),
            -32601,
            format!("Unknown method: {}", method),
        )),
    }
}
