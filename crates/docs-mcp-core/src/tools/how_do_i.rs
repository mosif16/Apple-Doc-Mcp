use std::sync::Arc;

use anyhow::{Context, Result};
use multi_provider_client::types::ProviderType;
use serde::Deserialize;
use serde_json::json;

use crate::{
    markdown,
    services::knowledge,
    state::{AppContext, SearchQueryLog, ToolDefinition, ToolHandler, ToolResponse},
    tools::{parse_args, text_response, wrap_handler},
};

#[derive(Debug, Deserialize)]
struct Args {
    task: String,
}

pub fn definition() -> (ToolDefinition, ToolHandler) {
    (
        ToolDefinition {
            name: "how_do_i".to_string(),
            description:
                "Provide guided multi-step recipes for common tasks within the active technology"
                    .to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "required": ["task"],
                "properties": {
                    "task": {
                        "type": "string",
                        "description": "Describe the outcome you're trying to implement (e.g. \"add search suggestions\")."
                    }
                }
            }),
            // Examples showing how to phrase task descriptions for best recipe matching
            input_examples: Some(vec![
                // UI implementation task
                json!({"task": "add search suggestions"}),
                // Navigation pattern
                json!({"task": "implement tab-based navigation"}),
                // Data handling task
                json!({"task": "fetch data from an API"}),
                // State management
                json!({"task": "share state between views"}),
                // Animation task
                json!({"task": "animate view transitions"}),
            ]),
            // Recipe lookup - returns structured guidance, less useful for batch processing
            allowed_callers: None,
        },
        wrap_handler(|context, value| async move {
            let args: Args = parse_args(value)?;
            handle(context, args).await
        }),
    )
}

async fn handle(context: Arc<AppContext>, args: Args) -> Result<ToolResponse> {
    // Get active provider
    let provider = *context.state.active_provider.read().await;

    // Get the active technology title based on provider
    let active_title = match provider {
        ProviderType::Apple => {
            context
                .state
                .active_technology
                .read()
                .await
                .clone()
                .map(|t| t.title)
                .context("No technology selected. Use `choose_technology` before requesting a recipe.")?
        }
        ProviderType::Telegram | ProviderType::TON | ProviderType::Cocoon | ProviderType::Rust
        | ProviderType::Mdn | ProviderType::WebFrameworks | ProviderType::Mlx | ProviderType::HuggingFace
        | ProviderType::QuickNode => {
            context
                .state
                .active_unified_technology
                .read()
                .await
                .clone()
                .map(|t| t.title)
                .context("No technology selected. Use `choose_technology` before requesting a recipe.")?
        }
    };
    let task_trimmed = args.task.trim().to_string();

    if let Some(recipe) = knowledge::find_recipe(&active_title, &args.task) {
        let mut lines = vec![
            markdown::header(1, &format!("🧩 Recipe: {}", recipe.title)),
            String::new(),
            markdown::bold("Technology", &active_title),
            markdown::bold("Summary", recipe.summary),
            String::new(),
            markdown::header(2, "Steps"),
        ];

        for (index, step) in recipe.steps.iter().enumerate() {
            lines.push(format!("{}. {}", index + 1, step));
        }

        if !recipe.references.is_empty() {
            lines.push(String::new());
            lines.push(markdown::header(2, "References"));
            for reference in recipe.references {
                lines.push(format!(
                    "• **{}** — {} (`get_documentation {{ \"path\": \"{}\" }}`)",
                    reference.title, reference.note, reference.path
                ));
            }
        }

        lines.push(String::new());
        lines.push("Tip: Run `search_symbols` with the related APIs above to explore deeper implementation details.".to_string());

        let metadata = serde_json::json!({
            "found": true,
            "task": task_trimmed,
            "recipeId": recipe.id,
            "steps": recipe.steps.len(),
            "references": recipe.references.len(),
        });

        Ok(text_response(lines).with_metadata(metadata))
    } else {
        let lines = vec![
            markdown::header(1, "No recipe available yet"),
            String::new(),
            format!(
                "I couldn't find a curated recipe for \"{}\" in {}.",
                task_trimmed,
                active_title
            ),
            "Try adjusting the description (for example, \"add search suggestions\"), or search directly with `search_symbols`."
                .to_string(),
        ];
        let mut metadata = json!({
            "found": false,
            "task": task_trimmed,
        });

        let fallback = build_fallback_recipe(context.clone(), &active_title, &task_trimmed).await;
        if let Some(fallback) = fallback {
            if let Some(obj) = metadata.as_object_mut() {
                obj.insert("fallback".to_string(), fallback.metadata.clone());
            }
            let mut augmented = lines;
            augmented.push(String::new());
            augmented.push(markdown::header(2, "Suggested Plan"));
            augmented.extend(fallback.lines);
            return Ok(text_response(augmented).with_metadata(metadata));
        }

        Ok(text_response(lines).with_metadata(metadata))
    }
}

struct FallbackRecipe {
    lines: Vec<String>,
    metadata: serde_json::Value,
}

async fn build_fallback_recipe(
    context: Arc<AppContext>,
    technology: &str,
    task: &str,
) -> Option<FallbackRecipe> {
    let queries = context.state.recent_queries.lock().await.clone();
    let mut related_queries: Vec<&SearchQueryLog> = queries
        .iter()
        .filter(|item| {
            item.technology
                .as_deref()
                .map(|tech| tech.eq_ignore_ascii_case(technology))
                .unwrap_or(false)
        })
        .collect();
    related_queries.sort_by(|a, b| b.matches.cmp(&a.matches));
    let top_query = related_queries.first()?;

    let knowledge_matches = knowledge::lookup(technology, task)
        .map(|entry| knowledge::related_items(entry).to_vec())
        .unwrap_or_default();

    let mut lines = Vec::new();
    lines.push(format!(
        "1. Re-run `search_symbols {{ \"query\": \"{}\" }}` to gather symbol matches (previous run returned {}).",
        top_query.query, top_query.matches
    ));
    lines.push("2. Inspect the strongest matches with `get_documentation` to review parameters, relationships, and sample code.".to_string());
    if !knowledge_matches.is_empty() {
        lines.push("3. Cross-reference related APIs for a working baseline:".to_string());
        for related in knowledge_matches.iter().take(3) {
            lines.push(format!(
                "   • **{}** — {} (`get_documentation {{ \"path\": \"{}\" }}`)",
                related.title, related.note, related.path
            ));
        }
    }
    lines.push(
        "4. Prototype the workflow and capture notes to convert into a curated recipe later."
            .to_string(),
    );

    let metadata = json!({
        "suggestedQuery": top_query.query,
        "matchesObserved": top_query.matches,
        "relatedKnowledge": knowledge_matches.len(),
    });

    Some(FallbackRecipe { lines, metadata })
}
