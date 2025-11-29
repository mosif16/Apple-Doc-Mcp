use std::sync::Arc;

use anyhow::{Context, Result};
use apple_docs_client::{types::Technology, DocsPlatform};
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
            description: "Select the framework/library to scope all subsequent searches (works with current platform)"
                .to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "identifier": {
                        "type": "string",
                        "description": "Technology identifier (doc://...) for Apple, or library name for Android/Flutter"
                    },
                    "name": {
                        "type": "string",
                        "description": "Technology/library title (e.g. SwiftUI, Compose UI, widgets)"
                    }
                }
            }),
        },
        wrap_handler(|context, value| async move {
            let args: Args = parse_args(value)?;
            let platform = *context.state.active_platform.read().await;
            match platform {
                DocsPlatform::Apple => handle_apple(context, args).await,
                DocsPlatform::Android => handle_android(context, args).await,
                DocsPlatform::Flutter => handle_flutter(context, args).await,
            }
        }),
    )
}

async fn handle_android(context: Arc<AppContext>, args: Args) -> Result<ToolResponse> {
    let name = args.name.as_ref().or(args.identifier.as_ref())
        .ok_or_else(|| anyhow::anyhow!("Please provide a library name"))?;

    if let Some(lib) = context.android_client.get_library(name).await? {
        *context.state.active_android_library.write().await = Some(lib.name.clone());

        let lines = vec![
            markdown::header(1, "✅ Android Library Selected"),
            String::new(),
            markdown::bold("Name", &lib.name),
            markdown::bold("Category", &lib.category.to_string()),
            markdown::bold("Artifact", &format!("{}:{}", lib.group_id, lib.artifact_id)),
            String::new(),
            markdown::header(2, "Next actions"),
            "• `search_symbols { \"query\": \"keyword\" }` — search Android APIs".to_string(),
            "• `discover_technologies` — browse other libraries".to_string(),
        ];

        let metadata = json!({
            "platform": "android",
            "resolved": true,
            "name": lib.name,
            "category": lib.category.to_string(),
            "artifact": format!("{}:{}", lib.group_id, lib.artifact_id),
        });

        return Ok(text_response(lines).with_metadata(metadata));
    }

    // Not found - show suggestions
    let libraries = context.android_client.get_libraries().await?;
    let name_lower = name.to_lowercase();
    let suggestions: Vec<_> = libraries
        .iter()
        .filter(|lib| lib.name.to_lowercase().contains(&name_lower) || lib.artifact_id.to_lowercase().contains(&name_lower))
        .take(5)
        .map(|lib| format!("• {} — `choose_technology \"{}\"`", lib.name, lib.name))
        .collect();

    let mut lines = vec![
        markdown::header(1, "❌ Library Not Found"),
        format!("Could not find Android library \"{}\".", name),
        String::new(),
        markdown::header(2, "Suggestions"),
    ];

    if suggestions.is_empty() {
        lines.push("• Use `discover_technologies` to browse available libraries".to_string());
    } else {
        lines.extend(suggestions);
    }

    Ok(text_response(lines).with_metadata(json!({
        "platform": "android",
        "resolved": false,
        "searchTerm": name,
    })))
}

async fn handle_flutter(context: Arc<AppContext>, args: Args) -> Result<ToolResponse> {
    let name = args.name.as_ref().or(args.identifier.as_ref())
        .ok_or_else(|| anyhow::anyhow!("Please provide a library name"))?;

    let index = context.flutter_client.get_index().await?;
    let name_lower = name.to_lowercase();

    // Find matching library
    let library = index.iter()
        .find(|item| {
            item.kind.as_deref() == Some("library") &&
            (item.name.to_lowercase() == name_lower || item.qualified_name.to_lowercase() == name_lower)
        });

    if let Some(lib) = library {
        *context.state.active_flutter_library.write().await = Some(lib.name.clone());

        let lines = vec![
            markdown::header(1, "✅ Flutter Library Selected"),
            String::new(),
            markdown::bold("Name", &lib.name),
            markdown::bold("Path", &lib.href),
            String::new(),
            markdown::header(2, "Next actions"),
            "• `search_symbols { \"query\": \"keyword\" }` — search Flutter APIs".to_string(),
            "• `discover_technologies` — browse other libraries".to_string(),
        ];

        let metadata = json!({
            "platform": "flutter",
            "resolved": true,
            "name": lib.name,
            "href": lib.href,
        });

        return Ok(text_response(lines).with_metadata(metadata));
    }

    // Not found - show suggestions
    let suggestions: Vec<_> = index.iter()
        .filter(|item| {
            item.kind.as_deref() == Some("library") &&
            (item.name.to_lowercase().contains(&name_lower) || item.qualified_name.to_lowercase().contains(&name_lower))
        })
        .take(5)
        .map(|lib| format!("• {} — `choose_technology \"{}\"`", lib.name, lib.name))
        .collect();

    let mut lines = vec![
        markdown::header(1, "❌ Library Not Found"),
        format!("Could not find Flutter library \"{}\".", name),
        String::new(),
        markdown::header(2, "Suggestions"),
    ];

    if suggestions.is_empty() {
        lines.push("• Use `discover_technologies` to browse available libraries".to_string());
    } else {
        lines.extend(suggestions);
    }

    Ok(text_response(lines).with_metadata(json!({
        "platform": "flutter",
        "resolved": false,
        "searchTerm": name,
    })))
}

async fn handle_apple(context: Arc<AppContext>, args: Args) -> Result<ToolResponse> {
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

    let has_design_mapping = design_guidance::has_primer_mapping(&technology);
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
