use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio::io::{self, AsyncBufReadExt, AsyncWriteExt, BufReader};
use tracing::{debug, info, warn};

use crate::executor::{ToolExecutor, ToolExecutorError};

const SERVER_INSTRUCTIONS: &str = r#"You are connected to the Apple Developer Documentation MCP server. Use the provided tools to ground every answer in official documentation before responding to the user.

Workflow guidance:
1. Establish context. Call `discover_technologies` to browse or filter available technologies, then invoke `choose_technology` to lock the framework the user cares about. Use `current_technology` whenever you need to confirm or reset that selection.
2. Locate symbols. Prefer `search_symbols` with clear queries and optional filters (`scope`, `platform`, `symbolType`) to surface relevant APIs or articles. If the user already supplied a full documentation path, skip directly to `get_documentation`.
3. Retrieve details. `get_documentation` returns summaries, availability, code listings, and design guidance. Extract information from this payload instead of inventing answers. When the user wants guided steps or best practices, call `how_do_i` for curated recipes.

Response expectations:
- Synthesize tool results into a concise Markdown answer with descriptive headings. Highlight platform availability, usage notes, and design considerations when the data is present.
- Cite the documentation path or symbol name you relied on so the user knows where the information originated.
- If a tool returns no results, explain what you tried, suggest alternative queries, or ask the user for clarification rather than guessing.
- Stay within Apple platform topics; if the request is out of scope, say so and offer relevant alternatives if possible.

These instructions remain in effect for the entire session. Re-check the active technology when the conversation shifts topics, and prefer incremental tool calls over large speculative queries."#;

pub async fn serve_stdio(executor: ToolExecutor) -> Result<()> {
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
            Ok(request) => handle_request(executor.clone(), request).await,
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

async fn handle_request(executor: ToolExecutor, request: RpcRequest) -> Option<RpcResponse> {
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
                },
                "instructions": SERVER_INSTRUCTIONS,
            }),
        )),
        "list_tools" | "tools/list" => {
            let definitions = executor.list_tools().await;
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

                    match executor.call_tool(&name, arguments).await {
                        Ok(response) => Some(RpcResponse::result(
                            Some(id_value.clone()),
                            serde_json::to_value(response).unwrap(),
                        )),
                        Err(ToolExecutorError::UnknownTool(_)) => Some(RpcResponse::error(
                            Some(id_value.clone()),
                            -32601,
                            format!("Unknown tool: {}", name),
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
                    -32602,
                    "Missing tool name",
                )),
            }
        }
        _ => {
            if let Some(response) = handle_tool_passthrough(
                executor.clone(),
                method,
                request.params.clone(),
                id_value.clone(),
            )
            .await
            {
                return Some(response);
            }

            Some(RpcResponse::error(
                Some(id_value),
                -32601,
                format!("Unknown method: {}", method),
            ))
        }
    }
}

async fn handle_tool_passthrough(
    executor: ToolExecutor,
    method: &str,
    params: Option<serde_json::Value>,
    id: serde_json::Value,
) -> Option<RpcResponse> {
    let arguments = match params {
        None | Some(serde_json::Value::Null) => serde_json::json!({}),
        Some(serde_json::Value::Object(map)) => serde_json::Value::Object(map),
        Some(_) => {
            return Some(RpcResponse::error(
                Some(id),
                -32602,
                "Tool arguments must be an object",
            ));
        }
    };

    match executor.call_tool(method, arguments).await {
        Ok(response) => Some(RpcResponse::result(
            Some(id),
            serde_json::to_value(response).expect("tool response is serializable"),
        )),
        Err(ToolExecutorError::UnknownTool(_)) => None,
        Err(error) => Some(RpcResponse::error(Some(id), -32000, error.to_string())),
    }
}
