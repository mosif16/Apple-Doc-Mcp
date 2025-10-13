use std::sync::Arc;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio::io::{self, AsyncBufReadExt, AsyncWriteExt, BufReader};
use tracing::{debug, info, warn};

use crate::state::AppContext;

pub async fn serve_stdio(context: Arc<AppContext>) -> Result<()> {
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut reader = BufReader::new(stdin);
    let mut writer = stdout;

    let mut buffer = String::new();
    loop {
        buffer.clear();
        let bytes = reader.read_line(&mut buffer).await?;
        if bytes == 0 {
            info!(target: "apple_docs_transport", "STDIO closed; shutting down");
            break;
        }

        debug!(target: "apple_docs_transport", request = buffer.trim());
        let maybe_response = match serde_json::from_str::<RpcRequest>(&buffer) {
            Ok(request) => handle_request(context.clone(), request).await,
            Err(error) => {
                warn!(target: "apple_docs_transport", error = %error, "Failed to parse request");
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
                info!(target: "apple_docs_transport", "Client signaled initialized");
            }
            other => {
                debug!(
                    target: "apple_docs_transport",
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
                "protocolVersion": "0.1.0",
                "serverInfo": {
                    "name": "apple-docs",
                    "version": env!("CARGO_PKG_VERSION"),
                },
                "capabilities": {
                    "tools": {}
                }
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
                            match handler(context.clone(), arguments).await {
                                Ok(response) => Some(RpcResponse::result(
                                    Some(id_value.clone()),
                                    serde_json::to_value(response).unwrap(),
                                )),
                                Err(error) => Some(RpcResponse::error(
                                    Some(id_value.clone()),
                                    -32000,
                                    error.to_string(),
                                )),
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
