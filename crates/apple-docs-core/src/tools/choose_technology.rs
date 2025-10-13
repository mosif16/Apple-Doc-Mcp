use std::sync::Arc;

use anyhow::{Context, Result};
use apple_docs_client::types::Technology;
use serde::Deserialize;

use crate::{
    markdown,
    state::{AppContext, ToolDefinition, ToolHandler, ToolResponse},
    tools::{parse_args, text_response, wrap_handler},
};

#[derive(Debug, Deserialize)]
struct Args {
    identifier: Option<String>,
    name: Option<String>,
}

pub fn definition() -> (ToolDefinition, ToolHandler) {
    (
        ToolDefinition {
            name: "choose_technology".to_string(),
            description: "Select the framework/technology to scope all subsequent searches"
                .to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "identifier": {
                        "type": "string",
                        "description": "Technology identifier (doc://...)"
                    },
                    "name": {
                        "type": "string",
                        "description": "Technology title (e.g. SwiftUI)"
                    }
                }
            }),
        },
        wrap_handler(|context, value| async move {
            let args: Args = parse_args(value)?;
            handle(context, args).await
        }),
    )
}

async fn handle(context: Arc<AppContext>, args: Args) -> Result<ToolResponse> {
    let technologies = context
        .client
        .get_technologies()
        .await
        .context("Failed to load technologies")?;

    let candidates: Vec<Technology> = technologies
        .values()
        .cloned()
        .filter(|tech| tech.kind == "symbol" && tech.role == "collection")
        .collect();

    let chosen = resolve_candidate(&candidates, &args);

    let technology = match chosen {
        Some(tech) => tech,
        None => {
            let message = build_not_found(&candidates, &args);
            context.state.active_technology.write().await.take();
            return Ok(text_response(message));
        }
    };

    *context.state.active_technology.write().await = Some(technology.clone());
    context.state.framework_cache.write().await.take();
    context.state.framework_index.write().await.take();
    context.state.expanded_identifiers.lock().await.clear();

    let lines = vec![
        markdown::header(1, "✅ Technology Selected"),
        String::new(),
        markdown::bold("Name", &technology.title),
        markdown::bold("Identifier", &technology.identifier),
        String::new(),
        markdown::header(2, "Next actions"),
        "• `search_symbols { \"query\": \"keyword\" }` — fuzzy search within this framework"
            .to_string(),
        "• `get_documentation { \"path\": \"SymbolName\" }` — open a symbol page".to_string(),
        "• `discover_technologies` — pick another framework".to_string(),
    ];

    Ok(text_response(lines))
}

fn resolve_candidate(candidates: &[Technology], args: &Args) -> Option<Technology> {
    if let Some(identifier) = &args.identifier {
        let lower = identifier.to_lowercase();
        if let Some(found) = candidates
            .iter()
            .find(|tech| tech.identifier.to_lowercase() == lower)
        {
            return Some(found.clone());
        }
    }

    if let Some(name) = &args.name {
        let lower = name.to_lowercase();
        if let Some(found) = candidates
            .iter()
            .find(|tech| tech.title.to_lowercase() == lower)
        {
            return Some(found.clone());
        }
    }

    candidates
        .iter()
        .map(|tech| {
            let score = fuzzy_score(&tech.title, args.name.as_deref().unwrap_or_default());
            (score, tech)
        })
        .min_by_key(|(score, _)| *score)
        .map(|(_, tech)| tech.clone())
}

fn fuzzy_score(candidate: &str, target: &str) -> u32 {
    if target.is_empty() {
        return u32::MAX;
    }

    let candidate_lower = candidate.to_lowercase();
    let target_lower = target.to_lowercase();

    if candidate_lower == target_lower {
        0
    } else if candidate_lower.starts_with(&target_lower)
        || target_lower.starts_with(&candidate_lower)
    {
        1
    } else if candidate_lower.contains(&target_lower) || target_lower.contains(&candidate_lower) {
        2
    } else {
        3
    }
}

fn build_not_found(candidates: &[Technology], args: &Args) -> Vec<String> {
    let search_term = args
        .name
        .as_ref()
        .or(args.identifier.as_ref())
        .map(|s| s.as_str())
        .unwrap_or("unknown");

    let suggestions = candidates
        .iter()
        .filter(|tech| {
            tech.title
                .to_lowercase()
                .contains(&search_term.to_lowercase())
        })
        .take(5)
        .map(|tech| format!("• {} — `choose_technology \"{}\"`", tech.title, tech.title))
        .collect::<Vec<_>>();

    let mut lines = vec![
        markdown::header(1, "❌ Technology Not Found"),
        format!("Could not resolve \"{}\".", search_term),
        String::new(),
        markdown::header(2, "Suggestions"),
    ];

    if suggestions.is_empty() {
        lines.push(
            "• Use `discover_technologies { \"query\": \"keyword\" }` to find candidates"
                .to_string(),
        );
    } else {
        lines.extend(suggestions);
    }

    lines
}
