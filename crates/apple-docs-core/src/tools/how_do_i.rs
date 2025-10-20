use std::sync::Arc;

use anyhow::{Context, Result};
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
        },
        wrap_handler(|context, value| async move {
            let args: Args = parse_args(value)?;
            handle(context, args).await
        }),
    )
}

async fn handle(context: Arc<AppContext>, args: Args) -> Result<ToolResponse> {
    let active = context
        .state
        .active_technology
        .read()
        .await
        .clone()
        .context("No technology selected. Use `choose_technology` before requesting a recipe.")?;
    let task_trimmed = args.task.trim().to_string();

    if let Some(recipe) = knowledge::find_recipe(&active.title, &args.task) {
        let mut lines = vec![
            markdown::header(1, &format!("🧩 Recipe: {}", recipe.title)),
            String::new(),
            markdown::bold("Technology", &active.title),
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
                active.title
            ),
            "Try adjusting the description (for example, \"add search suggestions\"), or search directly with `search_symbols`."
                .to_string(),
        ];
        let mut metadata = json!({
            "found": false,
            "task": task_trimmed,
        });

        let fallback = build_fallback_recipe(context.clone(), &active.title, &task_trimmed).await;
        if let Some(fallback) = fallback {
            metadata
                .as_object_mut()
                .unwrap()
                .insert("fallback".to_string(), fallback.metadata.clone());
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
