//! Batch documentation tool optimized for programmatic calling.
//!
//! This tool fetches documentation for multiple symbols in a single call,
//! returning compact summaries suitable for aggregation in code.

use std::sync::Arc;

use anyhow::{Context, Result};
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

    let active = context
        .state
        .active_technology
        .read()
        .await
        .clone()
        .context("No technology selected. Use `choose_technology` first.")?;

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
        match fetch_symbol_info(&context, &active.identifier, path).await {
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

/// Fetch basic symbol information for a given path
async fn fetch_symbol_info(
    context: &Arc<AppContext>,
    technology_id: &str,
    path: &str,
) -> Result<SymbolInfo> {
    use apple_docs_client::types::extract_text;

    // Normalize the path
    let normalized = normalize_path(technology_id, path);

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

/// Normalize a symbol path for the Apple documentation API
fn normalize_path(technology_id: &str, path: &str) -> String {
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
