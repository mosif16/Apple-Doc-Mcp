use std::sync::Arc;

use anyhow::Result;
use serde_json::json;

use crate::{
    markdown,
    services::{design_guidance, knowledge},
    state::{AppContext, ToolDefinition, ToolHandler, ToolResponse},
    tools::{text_response, wrap_handler},
};

pub fn definition() -> (ToolDefinition, ToolHandler) {
    (
        ToolDefinition {
            name: "current_technology".to_string(),
            description: "Report the currently selected technology".to_string(),
            input_schema: serde_json::json!({"type": "object", "properties": {}}),
            // No parameters needed - this tool just reports current state
            input_examples: None,
            // State-reporting tool - no batch benefit from programmatic calling
            allowed_callers: None,
        },
        wrap_handler(|context, _value| async move { handle(context).await }),
    )
}

async fn handle(context: Arc<AppContext>) -> Result<ToolResponse> {
    if let Some(active) = context.state.active_technology.read().await.clone() {
        let mut lines = vec![
            markdown::header(1, "📘 Current Technology"),
            String::new(),
            markdown::bold("Name", &active.title),
            markdown::bold("Identifier", &active.identifier),
            String::new(),
            markdown::header(2, "Next actions"),
            "• `search_symbols { \"query\": \"keyword\" }` to find symbols".to_string(),
            "• `get_documentation { \"path\": \"SymbolName\" }` to open docs".to_string(),
            "• `choose_technology \"Another Framework\"` to switch".to_string(),
        ];

        let mut primer_count = 0usize;
        let recipes = knowledge::recipes_for(&active.title);
        if let Ok(sections) = design_guidance::primers_for_technology(&context, &active).await {
            primer_count = sections.len();
            if !sections.is_empty() {
                lines.push(String::new());
                lines.push(markdown::header(2, "Design primers"));
                for section in sections.iter().take(3) {
                    if let Some(bullet) = section.bullets.first() {
                        lines.push(format!("• {} — {}", section.title, bullet.text));
                    } else if let Some(summary) = section.summary.as_ref() {
                        lines.push(format!("• {} — {}", section.title, summary));
                    } else {
                        lines.push(format!("• {}", section.title));
                    }
                }
                lines.push(format!(
                    "• Deep dive: `get_documentation {{ \"path\": \"{}\" }}`",
                    sections[0].slug
                ));
            }
        }

        if !recipes.is_empty() {
            lines.push(String::new());
            lines.push(markdown::header(2, "Curated recipes"));
            for recipe in recipes.iter().take(3) {
                let task_hint = recipe
                    .keywords
                    .first()
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| recipe.title.to_lowercase());
                lines.push(format!(
                    "• **{}** — {} (`how_do_i {{ \"task\": \"{}\" }}`)",
                    recipe.title, recipe.summary, task_hint
                ));
            }
            if recipes.len() > 3 {
                lines.push(format!(
                    "• …and {} more recipes available via `how_do_i`",
                    recipes.len() - 3
                ));
            }
        }

        if let Some(last_query) =
            context
                .state
                .recent_queries
                .lock()
                .await
                .iter()
                .rev()
                .find(|entry| {
                    entry
                        .technology
                        .as_deref()
                        .map(|tech| tech.eq_ignore_ascii_case(&active.title))
                        .unwrap_or(false)
                })
        {
            lines.push(String::new());
            lines.push(markdown::header(2, "Recent search"));
            lines.push(format!(
                "• `search_symbols {{ \"query\": \"{}\" }}` — {} matches",
                last_query.query, last_query.matches
            ));
        }

        let metadata = json!({
            "selected": true,
            "identifier": active.identifier,
            "name": active.title,
            "designPrimerCount": primer_count,
            "recipeCount": recipes.len(),
        });

        Ok(text_response(lines).with_metadata(metadata))
    } else {
        let lines = [
            "🚦 Technology Not Selected".to_string(),
            "Use `discover_technologies` then `choose_technology` to get started.".to_string(),
        ];
        Ok(text_response(lines).with_metadata(json!({
            "selected": false
        })))
    }
}
