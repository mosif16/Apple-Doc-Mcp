use std::sync::Arc;

use anyhow::{Context, Result};
use apple_docs_client::types::Technology;
use serde::Deserialize;
use serde_json::json;

use crate::{
    markdown,
    services::design_guidance,
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
        .filter(|tech| tech.kind == "symbol" && tech.role == "collection")
        .cloned()
        .collect();

    let chosen = resolve_candidate(&candidates, &args);
    let input_identifier = args.identifier.clone();
    let input_name = args.name.clone();

    let technology = match chosen {
        Some(tech) => tech,
        None => {
            let not_found = build_not_found(&candidates, &args);
            context.state.active_technology.write().await.take();
            let metadata = json!({
                "resolved": false,
                "inputIdentifier": input_identifier,
                "inputName": input_name,
                "searchTerm": not_found.search_term,
                "suggestions": not_found.suggestion_count,
            });
            return Ok(text_response(not_found.lines).with_metadata(metadata));
        }
    };

    *context.state.active_technology.write().await = Some(technology.clone());
    context.state.framework_cache.write().await.take();
    context.state.framework_index.write().await.take();
    context.state.expanded_identifiers.lock().await.clear();

    // Clear design guidance cache when switching technologies
    context.state.design_guidance_cache.write().await.clear();

    let has_design_mapping = design_guidance::has_primer_mapping(&technology);

    // Pre-cache design guidance for this technology in the background
    // This populates both the global CACHE and ServerState cache for fast lookups
    if has_design_mapping {
        let context_clone = Arc::clone(&context);
        let tech_clone = technology.clone();
        tokio::spawn(async move {
            if let Err(e) = design_guidance::precache_for_technology(&context_clone, &tech_clone).await {
                tracing::warn!(
                    target: "choose_technology.precache",
                    technology = %tech_clone.title,
                    "Failed to pre-cache design guidance: {e:#}"
                );
            } else {
                tracing::debug!(
                    target: "choose_technology.precache",
                    technology = %tech_clone.title,
                    "Successfully pre-cached design guidance"
                );
            }
        });
    }
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

    let metadata = json!({
        "resolved": true,
        "identifier": technology.identifier,
        "name": technology.title,
        "designPrimersAvailable": has_design_mapping,
    });

    Ok(text_response(lines).with_metadata(metadata))
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

struct NotFoundDetails {
    lines: Vec<String>,
    search_term: String,
    suggestion_count: usize,
}

fn build_not_found(candidates: &[Technology], args: &Args) -> NotFoundDetails {
    let search_term = args
        .name
        .as_ref()
        .or(args.identifier.as_ref())
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .unwrap_or("unknown");

    let suggestions_list: Vec<String> = candidates
        .iter()
        .filter(|tech| {
            tech.title
                .to_lowercase()
                .contains(&search_term.to_lowercase())
        })
        .take(5)
        .map(|tech| format!("• {} — `choose_technology \"{}\"`", tech.title, tech.title))
        .collect::<Vec<_>>();
    let suggestion_count = suggestions_list.len();

    let mut lines = vec![
        markdown::header(1, "❌ Technology Not Found"),
        format!("Could not resolve \"{}\".", search_term),
        String::new(),
        markdown::header(2, "Suggestions"),
    ];

    if suggestions_list.is_empty() {
        lines.push(
            "• Use `discover_technologies { \"query\": \"keyword\" }` to find candidates"
                .to_string(),
        );
    } else {
        lines.extend(suggestions_list.iter().cloned());
    }

    NotFoundDetails {
        lines,
        search_term: search_term.to_string(),
        suggestion_count,
    }
}
