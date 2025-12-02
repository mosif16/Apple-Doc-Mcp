use std::sync::Arc;

use anyhow::{Context, Result};
use apple_docs_client::types::{extract_text, Technology};
use multi_provider_client::types::{ProviderType, TechnologyKind, UnifiedTechnology};
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
            description: "Select the framework/technology to scope all subsequent searches. Supports Apple (SwiftUI, UIKit), Telegram (methods, types), TON (accounts, nft), and Cocoon (architecture, smart-contracts)."
                .to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "identifier": {
                        "type": "string",
                        "description": "Technology identifier. Examples: 'doc://com.apple.documentation/documentation/swiftui' (Apple), 'telegram:methods' (Telegram), 'ton:accounts' (TON), 'cocoon:architecture' (Cocoon)"
                    },
                    "name": {
                        "type": "string",
                        "description": "Technology title (e.g. 'SwiftUI', 'Telegram Bot API Methods', 'TON Accounts')"
                    }
                }
            }),
            // Examples showing how to select technologies from different providers
            input_examples: Some(vec![
                // Apple: by name (simplest)
                json!({"name": "SwiftUI"}),
                // Apple: by full identifier
                json!({"identifier": "doc://com.apple.documentation/documentation/swiftui"}),
                // Telegram: by identifier
                json!({"identifier": "telegram:methods"}),
                // TON: by identifier
                json!({"identifier": "ton:accounts"}),
                // Cocoon: by identifier
                json!({"identifier": "cocoon:architecture"}),
            ]),
            // State-setting tool - typically called once before batch operations.
            // Programmatic calling has limited benefit.
            allowed_callers: None,
        },
        wrap_handler(|context, value| async move {
            let args: Args = parse_args(value)?;
            handle(context, args).await
        }),
    )
}

async fn handle(context: Arc<AppContext>, args: Args) -> Result<ToolResponse> {
    // Determine which provider to use based on identifier prefix
    let identifier = args.identifier.as_deref().unwrap_or("");
    let name = args.name.as_deref().unwrap_or("");

    // Check for provider-specific identifiers
    if identifier.starts_with("telegram:") || name.to_lowercase().contains("telegram") {
        return handle_telegram(&context, &args).await;
    }

    if identifier.starts_with("ton:") || name.to_lowercase().contains("ton ") {
        return handle_ton(&context, &args).await;
    }

    if identifier.starts_with("cocoon:") || name.to_lowercase().contains("cocoon") {
        return handle_cocoon(&context, &args).await;
    }

    // Default to Apple
    handle_apple(&context, &args).await
}

/// Handle Apple technology selection
async fn handle_apple(context: &Arc<AppContext>, args: &Args) -> Result<ToolResponse> {
    let technologies = context
        .client
        .get_technologies()
        .await
        .context("Failed to load Apple technologies")?;

    let candidates: Vec<Technology> = technologies
        .values()
        .filter(|tech| tech.kind == "symbol" && tech.role == "collection")
        .cloned()
        .collect();

    let chosen = resolve_apple_candidate(&candidates, args);
    let input_identifier = args.identifier.clone();
    let input_name = args.name.clone();

    let technology = match chosen {
        Some(tech) => tech,
        None => {
            let not_found = build_apple_not_found(&candidates, args);
            context.state.active_technology.write().await.take();
            context.state.active_unified_technology.write().await.take();
            let metadata = json!({
                "resolved": false,
                "provider": "apple",
                "inputIdentifier": input_identifier,
                "inputName": input_name,
                "searchTerm": not_found.search_term,
                "suggestions": not_found.suggestion_count,
            });
            return Ok(text_response(not_found.lines).with_metadata(metadata));
        }
    };

    // Set both legacy and unified technology state
    *context.state.active_technology.write().await = Some(technology.clone());
    *context.state.active_provider.write().await = ProviderType::Apple;
    *context.state.active_unified_technology.write().await = Some(UnifiedTechnology {
        provider: ProviderType::Apple,
        identifier: technology.identifier.clone(),
        title: technology.title.clone(),
        description: extract_text(&technology.r#abstract),
        url: Some(format!("https://developer.apple.com{}", technology.url)),
        kind: TechnologyKind::Framework,
    });

    context.state.framework_cache.write().await.take();
    context.state.framework_index.write().await.take();
    context.state.expanded_identifiers.lock().await.clear();
    context.state.design_guidance_cache.write().await.clear();

    let has_design_mapping = design_guidance::has_primer_mapping(&technology);

    // Pre-cache design guidance in background
    if has_design_mapping {
        let context_clone = Arc::clone(context);
        let tech_clone = technology.clone();
        tokio::spawn(async move {
            if let Err(e) = design_guidance::precache_for_technology(&context_clone, &tech_clone).await {
                tracing::warn!(
                    target: "choose_technology.precache",
                    technology = %tech_clone.title,
                    "Failed to pre-cache design guidance: {e:#}"
                );
            }
        });
    }

    let lines = vec![
        markdown::header(1, "✅ Apple Technology Selected"),
        String::new(),
        markdown::bold("Provider", "🍎 Apple"),
        markdown::bold("Name", &technology.title),
        markdown::bold("Identifier", &technology.identifier),
        String::new(),
        markdown::header(2, "Next actions"),
        "• `search_symbols { \"query\": \"keyword\" }` — fuzzy search within this framework".to_string(),
        "• `get_documentation { \"path\": \"SymbolName\" }` — open a symbol page".to_string(),
        "• `discover_technologies` — pick another framework".to_string(),
    ];

    let metadata = json!({
        "resolved": true,
        "provider": "apple",
        "identifier": technology.identifier,
        "name": technology.title,
        "designPrimersAvailable": has_design_mapping,
    });

    Ok(text_response(lines).with_metadata(metadata))
}

/// Handle Telegram technology selection
async fn handle_telegram(context: &Arc<AppContext>, args: &Args) -> Result<ToolResponse> {
    let technologies = context
        .providers
        .telegram
        .get_technologies()
        .await
        .context("Failed to load Telegram technologies")?;

    let identifier = args.identifier.as_deref().unwrap_or("");
    let name = args.name.as_deref().unwrap_or("");

    // Find matching technology
    let chosen = technologies.iter().find(|t| {
        t.identifier.to_lowercase() == identifier.to_lowercase()
            || t.title.to_lowercase().contains(&name.to_lowercase())
    });

    let technology = match chosen {
        Some(tech) => tech.clone(),
        None => {
            let mut lines = vec![
                markdown::header(1, "❌ Telegram Technology Not Found"),
                format!("Could not find \"{}\".", args.name.as_deref().or(args.identifier.as_deref()).unwrap_or("unknown")),
                String::new(),
                markdown::header(2, "Available Telegram Categories"),
            ];
            for tech in &technologies {
                lines.push(format!("• {} — `choose_technology {{ \"identifier\": \"{}\" }}`", tech.title, tech.identifier));
            }
            let metadata = json!({
                "resolved": false,
                "provider": "telegram",
                "suggestions": technologies.len(),
            });
            return Ok(text_response(lines).with_metadata(metadata));
        }
    };

    // Set unified technology state
    context.state.active_technology.write().await.take();
    *context.state.active_provider.write().await = ProviderType::Telegram;
    *context.state.active_unified_technology.write().await = Some(UnifiedTechnology::from_telegram(technology.clone()));

    let lines = vec![
        markdown::header(1, "✅ Telegram Technology Selected"),
        String::new(),
        markdown::bold("Provider", "📱 Telegram"),
        markdown::bold("Name", &technology.title),
        markdown::bold("Identifier", &technology.identifier),
        markdown::bold("Items", &technology.item_count.to_string()),
        String::new(),
        markdown::header(2, "Next actions"),
        "• `search_symbols { \"query\": \"send\" }` — search Telegram methods/types".to_string(),
        "• `get_documentation { \"path\": \"sendMessage\" }` — get method/type details".to_string(),
        "• `discover_technologies { \"provider\": \"telegram\" }` — browse categories".to_string(),
    ];

    let metadata = json!({
        "resolved": true,
        "provider": "telegram",
        "identifier": technology.identifier,
        "name": technology.title,
        "itemCount": technology.item_count,
    });

    Ok(text_response(lines).with_metadata(metadata))
}

/// Handle TON technology selection
async fn handle_ton(context: &Arc<AppContext>, args: &Args) -> Result<ToolResponse> {
    let technologies = context
        .providers
        .ton
        .get_technologies()
        .await
        .context("Failed to load TON technologies")?;

    let identifier = args.identifier.as_deref().unwrap_or("");
    let name = args.name.as_deref().unwrap_or("");

    // Find matching technology
    let chosen = technologies.iter().find(|t| {
        t.identifier.to_lowercase() == identifier.to_lowercase()
            || t.title.to_lowercase().contains(&name.to_lowercase())
    });

    let technology = match chosen {
        Some(tech) => tech.clone(),
        None => {
            let mut lines = vec![
                markdown::header(1, "❌ TON Technology Not Found"),
                format!("Could not find \"{}\".", args.name.as_deref().or(args.identifier.as_deref()).unwrap_or("unknown")),
                String::new(),
                markdown::header(2, "Available TON Categories"),
            ];
            for tech in technologies.iter().take(15) {
                lines.push(format!("• {} ({} endpoints) — `choose_technology {{ \"identifier\": \"{}\" }}`",
                    tech.title, tech.endpoint_count, tech.identifier));
            }
            if technologies.len() > 15 {
                lines.push(format!("... and {} more", technologies.len() - 15));
            }
            let metadata = json!({
                "resolved": false,
                "provider": "ton",
                "suggestions": technologies.len(),
            });
            return Ok(text_response(lines).with_metadata(metadata));
        }
    };

    // Set unified technology state
    context.state.active_technology.write().await.take();
    *context.state.active_provider.write().await = ProviderType::TON;
    *context.state.active_unified_technology.write().await = Some(UnifiedTechnology::from_ton(technology.clone()));

    let lines = vec![
        markdown::header(1, "✅ TON Technology Selected"),
        String::new(),
        markdown::bold("Provider", "💎 TON Blockchain"),
        markdown::bold("Name", &technology.title),
        markdown::bold("Identifier", &technology.identifier),
        markdown::bold("Endpoints", &technology.endpoint_count.to_string()),
        String::new(),
        markdown::header(2, "Next actions"),
        "• `search_symbols { \"query\": \"account\" }` — search TON endpoints".to_string(),
        "• `get_documentation { \"path\": \"getAccounts\" }` — get endpoint details".to_string(),
        "• `discover_technologies { \"provider\": \"ton\" }` — browse categories".to_string(),
    ];

    let metadata = json!({
        "resolved": true,
        "provider": "ton",
        "identifier": technology.identifier,
        "name": technology.title,
        "endpointCount": technology.endpoint_count,
    });

    Ok(text_response(lines).with_metadata(metadata))
}

/// Handle Cocoon technology selection
async fn handle_cocoon(context: &Arc<AppContext>, args: &Args) -> Result<ToolResponse> {
    let technologies = context
        .providers
        .cocoon
        .get_technologies()
        .await
        .context("Failed to load Cocoon technologies")?;

    let identifier = args.identifier.as_deref().unwrap_or("");
    let name = args.name.as_deref().unwrap_or("");

    // Find matching technology
    let chosen = technologies.iter().find(|t| {
        t.identifier.to_lowercase() == identifier.to_lowercase()
            || t.title.to_lowercase().contains(&name.to_lowercase())
    });

    let technology = match chosen {
        Some(tech) => tech.clone(),
        None => {
            let mut lines = vec![
                markdown::header(1, "❌ Cocoon Technology Not Found"),
                format!("Could not find \"{}\".", args.name.as_deref().or(args.identifier.as_deref()).unwrap_or("unknown")),
                String::new(),
                markdown::header(2, "Available Cocoon Sections"),
            ];
            for tech in &technologies {
                lines.push(format!("• {} — `choose_technology {{ \"identifier\": \"{}\" }}`", tech.title, tech.identifier));
            }
            let metadata = json!({
                "resolved": false,
                "provider": "cocoon",
                "suggestions": technologies.len(),
            });
            return Ok(text_response(lines).with_metadata(metadata));
        }
    };

    // Set unified technology state
    context.state.active_technology.write().await.take();
    *context.state.active_provider.write().await = ProviderType::Cocoon;
    *context.state.active_unified_technology.write().await = Some(UnifiedTechnology::from_cocoon(technology.clone()));

    let lines = vec![
        markdown::header(1, "✅ Cocoon Technology Selected"),
        String::new(),
        markdown::bold("Provider", "🥥 Cocoon"),
        markdown::bold("Name", &technology.title),
        markdown::bold("Identifier", &technology.identifier),
        markdown::bold("Documents", &technology.doc_count.to_string()),
        String::new(),
        markdown::header(2, "Next actions"),
        "• `search_symbols { \"query\": \"tdx\" }` — search Cocoon documentation".to_string(),
        "• `get_documentation { \"path\": \"architecture\" }` — get document details".to_string(),
        "• `discover_technologies { \"provider\": \"cocoon\" }` — browse sections".to_string(),
    ];

    let metadata = json!({
        "resolved": true,
        "provider": "cocoon",
        "identifier": technology.identifier,
        "name": technology.title,
        "docCount": technology.doc_count,
    });

    Ok(text_response(lines).with_metadata(metadata))
}

fn resolve_apple_candidate(candidates: &[Technology], args: &Args) -> Option<Technology> {
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

    // Fuzzy match
    candidates
        .iter()
        .map(|tech| {
            let score = fuzzy_score(&tech.title, args.name.as_deref().unwrap_or_default());
            (score, tech)
        })
        .filter(|(score, _)| *score < u32::MAX)
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
        u32::MAX
    }
}

struct NotFoundDetails {
    lines: Vec<String>,
    search_term: String,
    suggestion_count: usize,
}

fn build_apple_not_found(candidates: &[Technology], args: &Args) -> NotFoundDetails {
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
        .map(|tech| format!("• {} — `choose_technology {{ \"name\": \"{}\" }}`", tech.title, tech.title))
        .collect();
    let suggestion_count = suggestions_list.len();

    let mut lines = vec![
        markdown::header(1, "❌ Apple Technology Not Found"),
        format!("Could not resolve \"{}\".", search_term),
        String::new(),
        markdown::header(2, "Suggestions"),
    ];

    if suggestions_list.is_empty() {
        lines.push("• Use `discover_technologies { \"provider\": \"apple\" }` to find candidates".to_string());
        lines.push("• Or try other providers: telegram, ton, cocoon".to_string());
    } else {
        lines.extend(suggestions_list);
    }

    NotFoundDetails {
        lines,
        search_term: search_term.to_string(),
        suggestion_count,
    }
}
