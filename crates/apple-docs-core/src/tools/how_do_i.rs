use std::sync::Arc;

use anyhow::{Context, Result};
use serde::Deserialize;

use crate::{
    markdown,
    services::knowledge,
    state::{AppContext, ToolDefinition, ToolHandler, ToolResponse},
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

        Ok(text_response(lines))
    } else {
        Ok(text_response([
            markdown::header(1, "No recipe available yet"),
            String::new(),
            format!(
                "I couldn't find a curated recipe for \"{}\" in {}.",
                args.task.trim(),
                active.title
            ),
            "Try adjusting the description (for example, \"add search suggestions\"), or search directly with `search_symbols`."
                .to_string(),
        ]))
    }
}
