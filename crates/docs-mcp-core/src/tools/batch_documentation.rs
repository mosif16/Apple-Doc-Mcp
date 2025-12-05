//! Batch documentation tool optimized for programmatic calling.
//!
//! This tool fetches documentation for multiple symbols in a single call,
//! returning compact summaries suitable for aggregation in code.

use std::sync::Arc;

use anyhow::{Context, Result};
use multi_provider_client::types::ProviderType;
use serde::Deserialize;
use serde_json::json;

use crate::{
    state::{AppContext, ToolDefinition, ToolHandler, ToolResponse},
    tools::{parse_args, text_response, wrap_handler},
};

/// Code execution caller identifier for programmatic tool calling.
const CODE_EXECUTION_CALLER: &str = "code_execution_20250825";

/// Maximum number of paths allowed per batch request.
const MAX_BATCH_SIZE: usize = 10;

#[derive(Debug, Deserialize)]
struct Args {
    /// Array of symbol paths to fetch (max 10)
    paths: Vec<String>,
    /// Which fields to include in response
    fields: Option<Vec<String>>,
}

/// Result for a single symbol in the batch
#[derive(Debug, serde::Serialize)]
struct BatchResult {
    path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    summary: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    platforms: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    kind: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

pub fn definition() -> (ToolDefinition, ToolHandler) {
    (
        ToolDefinition {
            name: "batch_documentation".to_string(),
            description: "Retrieve documentation for multiple symbols in a single call. \
                         Returns compact summaries optimized for batch processing. \
                         Use when comparing multiple APIs or gathering information across symbols. \
                         Designed for programmatic orchestration - call once, process results in code."
                .to_string(),
            input_schema: json!({
                "type": "object",
                "required": ["paths"],
                "properties": {
                    "paths": {
                        "type": "array",
                        "items": {"type": "string"},
                        "description": "Array of symbol paths to fetch (max 10)",
                        "maxItems": MAX_BATCH_SIZE
                    },
                    "fields": {
                        "type": "array",
                        "items": {
                            "type": "string",
                            "enum": ["title", "summary", "platforms", "kind"]
                        },
                        "description": "Which fields to include (default: all). Options: title, summary, platforms, kind"
                    }
                }
            }),
            input_examples: Some(vec![
                // Simple: fetch multiple symbols
                json!({"paths": ["Button", "Toggle", "Picker"]}),
                // With field selection for compact output
                json!({"paths": ["NavigationStack", "TabView", "NavigationSplitView"], "fields": ["summary", "platforms"]}),
                // Comparing similar APIs
                json!({"paths": ["List", "LazyVStack", "ScrollView"], "fields": ["summary", "kind"]}),
            ]),
            // Primary use case is programmatic calling - designed for code orchestration
            allowed_callers: Some(vec![CODE_EXECUTION_CALLER.to_string()]),
        },
        wrap_handler(|context, value| async move {
            let args: Args = parse_args(value)?;
            handle(context, args).await
        }),
    )
}

async fn handle(context: Arc<AppContext>, args: Args) -> Result<ToolResponse> {
    // Validate batch size
    if args.paths.is_empty() {
        anyhow::bail!("paths array cannot be empty");
    }
    if args.paths.len() > MAX_BATCH_SIZE {
        anyhow::bail!(
            "paths array exceeds maximum size of {} (got {})",
            MAX_BATCH_SIZE,
            args.paths.len()
        );
    }

    // Get active provider
    let provider = *context.state.active_provider.read().await;

    // Get active technology based on provider
    let active = match provider {
        ProviderType::Apple => {
            context
                .state
                .active_technology
                .read()
                .await
                .clone()
                .context("No technology selected. Use `choose_technology` first.")?
        }
        _ => {
            // For non-Apple providers, use active_unified_technology
            let unified = context
                .state
                .active_unified_technology
                .read()
                .await
                .clone()
                .context("No technology selected. Use `choose_technology` first.")?;

            docs_mcp_client::types::Technology {
                identifier: unified.identifier,
                title: unified.title,
                r#abstract: vec![],
                kind: String::new(),
                role: String::new(),
                url: String::new(),
            }
        }
    };

    // Determine which fields to include
    let fields = args.fields.unwrap_or_else(|| {
        vec![
            "title".to_string(),
            "summary".to_string(),
            "platforms".to_string(),
            "kind".to_string(),
        ]
    });
    let include_title = fields.iter().any(|f| f == "title");
    let include_summary = fields.iter().any(|f| f == "summary");
    let include_platforms = fields.iter().any(|f| f == "platforms");
    let include_kind = fields.iter().any(|f| f == "kind");

    // Fetch documentation for each path
    let mut results: Vec<BatchResult> = Vec::with_capacity(args.paths.len());
    let mut success_count = 0;
    let mut error_count = 0;

    for path in &args.paths {
        let fetch_result = match provider {
            ProviderType::Apple => fetch_apple_info(&context, &active.identifier, path).await,
            ProviderType::Telegram => fetch_telegram_info(&context, path).await,
            ProviderType::TON => fetch_ton_info(&context, path).await,
            ProviderType::Cocoon => fetch_cocoon_info(&context, &active.identifier, path).await,
            ProviderType::Rust => fetch_rust_info(&context, &active.identifier, path).await,
            // MDN, WebFrameworks, Mlx, HuggingFace, and QuickNode not supported in batch documentation
            ProviderType::Mdn | ProviderType::WebFrameworks | ProviderType::Mlx | ProviderType::HuggingFace
            | ProviderType::QuickNode => {
                Err(anyhow::anyhow!("Provider {} does not support batch documentation", provider.name()))
            }
        };

        match fetch_result {
            Ok(info) => {
                success_count += 1;
                results.push(BatchResult {
                    path: path.clone(),
                    title: if include_title { info.title } else { None },
                    summary: if include_summary { info.summary } else { None },
                    platforms: if include_platforms { info.platforms } else { None },
                    kind: if include_kind { info.kind } else { None },
                    error: None,
                });
            }
            Err(e) => {
                error_count += 1;
                results.push(BatchResult {
                    path: path.clone(),
                    title: None,
                    summary: None,
                    platforms: None,
                    kind: None,
                    error: Some(e.to_string()),
                });
            }
        }
    }

    // Build response as JSON for easy programmatic parsing
    let response_json = json!({
        "technology": active.title,
        "requested": args.paths.len(),
        "succeeded": success_count,
        "failed": error_count,
        "results": results
    });

    let metadata = json!({
        "batchSize": args.paths.len(),
        "successCount": success_count,
        "errorCount": error_count,
        "fieldsIncluded": fields,
    });

    Ok(text_response([serde_json::to_string_pretty(&response_json)?]).with_metadata(metadata))
}

/// Compact symbol information for batch responses
struct SymbolInfo {
    title: Option<String>,
    summary: Option<String>,
    platforms: Option<Vec<String>>,
    kind: Option<String>,
}

/// Fetch Apple documentation info for a given path
async fn fetch_apple_info(
    context: &Arc<AppContext>,
    technology_id: &str,
    path: &str,
) -> Result<SymbolInfo> {
    use docs_mcp_client::types::extract_text;

    // Normalize the path
    let normalized = normalize_apple_path(technology_id, path);

    // Try to fetch the symbol data
    let symbol = context
        .client
        .get_symbol(&normalized)
        .await
        .with_context(|| format!("Failed to fetch documentation for '{}'", path))?;

    // Extract title from metadata or fallback to path
    let title = symbol
        .metadata
        .title
        .clone()
        .or_else(|| path.split('/').last().map(|s| s.to_string()));

    // Extract summary from abstract
    let summary = {
        let text = extract_text(&symbol.r#abstract);
        if text.is_empty() {
            None
        } else {
            Some(text)
        }
    };

    // Extract platforms as Vec<String> for programmatic processing
    let platforms = if symbol.metadata.platforms.is_empty() {
        None
    } else {
        Some(
            symbol
                .metadata
                .platforms
                .iter()
                .map(|p| p.name.clone())
                .collect(),
        )
    };

    // Extract kind from symbol_kind
    let kind = symbol.metadata.symbol_kind.clone();

    Ok(SymbolInfo {
        title,
        summary,
        platforms,
        kind,
    })
}

/// Fetch Telegram Bot API info for a given path
async fn fetch_telegram_info(context: &Arc<AppContext>, path: &str) -> Result<SymbolInfo> {
    let item = context
        .providers
        .telegram
        .get_item(path)
        .await
        .with_context(|| format!("Failed to fetch Telegram docs for '{}'", path))?;

    Ok(SymbolInfo {
        title: Some(item.name.clone()),
        summary: Some(item.description.clone()),
        platforms: Some(vec!["Telegram Bot API".to_string()]),
        kind: Some(item.kind.clone()),
    })
}

/// Fetch TON API info for a given path
async fn fetch_ton_info(context: &Arc<AppContext>, path: &str) -> Result<SymbolInfo> {
    let endpoint = context
        .providers
        .ton
        .get_endpoint(path)
        .await
        .with_context(|| format!("Failed to fetch TON docs for '{}'", path))?;

    Ok(SymbolInfo {
        title: Some(endpoint.path.clone()),
        summary: endpoint.summary.clone().or(endpoint.description.clone()),
        platforms: Some(vec!["TON API".to_string()]),
        kind: Some(format!("{} endpoint", endpoint.method)),
    })
}

/// Fetch Cocoon documentation info for a given path
async fn fetch_cocoon_info(
    context: &Arc<AppContext>,
    section_id: &str,
    path: &str,
) -> Result<SymbolInfo> {
    // Try to find the document in the section
    if let Ok(section) = context.providers.cocoon.get_section(section_id).await {
        if let Some(doc) = section.documents.iter().find(|d| {
            d.path.eq_ignore_ascii_case(path)
                || d.title.to_lowercase().contains(&path.to_lowercase())
        }) {
            return Ok(SymbolInfo {
                title: Some(doc.title.clone()),
                summary: Some(doc.summary.clone()),
                platforms: Some(vec!["Cocoon".to_string()]),
                kind: Some("document".to_string()),
            });
        }
    }

    // Try to get the full document
    if let Ok(doc) = context.providers.cocoon.get_document(path).await {
        return Ok(SymbolInfo {
            title: Some(doc.title.clone()),
            summary: Some(doc.summary.clone()),
            platforms: Some(vec!["Cocoon".to_string()]),
            kind: Some("document".to_string()),
        });
    }

    anyhow::bail!("Cocoon documentation not found for '{}'", path)
}

/// Fetch Rust documentation info for a given path
/// Uses minimal fetch for batch operations to avoid slow HTTP requests
async fn fetch_rust_info(
    context: &Arc<AppContext>,
    technology_id: &str,
    path: &str,
) -> Result<SymbolInfo> {
    // Extract crate name from technology identifier (e.g., "rust:std" -> "std")
    let crate_name = technology_id.strip_prefix("rust:").unwrap_or(technology_id);

    // Try to get the item (minimal version for batch operations)
    if let Ok(item) = context.providers.rust.get_item_minimal(path).await {
        return Ok(SymbolInfo {
            title: Some(item.name.clone()),
            summary: if item.summary.is_empty() {
                None
            } else {
                Some(item.summary.clone())
            },
            platforms: Some(vec![format!("Rust ({} v{})", item.crate_name, item.crate_version)]),
            kind: Some(format!("{:?}", item.kind)),
        });
    }

    // Fallback: search for the item
    if let Ok(results) = context.providers.rust.search(crate_name, path).await {
        if let Some(item) = results.first() {
            return Ok(SymbolInfo {
                title: Some(item.name.clone()),
                summary: if item.summary.is_empty() {
                    None
                } else {
                    Some(item.summary.clone())
                },
                platforms: Some(vec![format!("Rust ({} v{})", item.crate_name, item.crate_version)]),
                kind: Some(format!("{:?}", item.kind)),
            });
        }
    }

    anyhow::bail!("Rust documentation not found for '{}' in crate '{}'", path, crate_name)
}

/// Normalize a symbol path for the Apple documentation API
fn normalize_apple_path(technology_id: &str, path: &str) -> String {
    // Strip doc:// prefix if present
    let path = path
        .strip_prefix("doc://com.apple.documentation/")
        .unwrap_or(path);

    // If already starts with documentation/, use as-is
    if path.starts_with("documentation/") || path.starts_with("design/") {
        return path.to_string();
    }

    // Extract technology name from identifier
    let tech_name = technology_id
        .split('/')
        .last()
        .unwrap_or("swiftui")
        .to_lowercase();

    // Build full path
    format!("documentation/{}/{}", tech_name, path.to_lowercase())
}
