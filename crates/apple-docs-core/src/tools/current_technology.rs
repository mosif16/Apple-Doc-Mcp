use std::sync::Arc;

use anyhow::Result;
use apple_docs_client::DocsPlatform;
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
            description: "Report the currently selected platform and technology/library".to_string(),
            input_schema: serde_json::json!({"type": "object", "properties": {}}),
        },
        wrap_handler(|context, _value| async move { handle(context).await }),
    )
}

async fn handle(context: Arc<AppContext>) -> Result<ToolResponse> {
    let platform = *context.state.active_platform.read().await;

    // Show platform info at the top
    let lines = vec![
        markdown::header(1, "📘 Current Context"),
        String::new(),
        markdown::bold("Platform", platform.display_name()),
        markdown::bold("Languages", &platform.languages().join(", ")),
        String::new(),
    ];

    match platform {
        DocsPlatform::Apple => {
            return handle_apple(context, lines).await;
        }
        DocsPlatform::Android => {
            return handle_android(context, lines).await;
        }
        DocsPlatform::Flutter => {
            return handle_flutter(context, lines).await;
        }
    }
}

async fn handle_android(context: Arc<AppContext>, mut lines: Vec<String>) -> Result<ToolResponse> {
    if let Some(lib_name) = context.state.active_android_library.read().await.clone() {
        if let Ok(Some(lib)) = context.android_client.get_library(&lib_name).await {
            lines.push(markdown::header(2, "Active Library"));
            lines.push(String::new());
            lines.push(markdown::bold("Name", &lib.name));
            lines.push(markdown::bold("Category", &lib.category.to_string()));
            lines.push(markdown::bold("Artifact", &format!("{}:{}", lib.group_id, lib.artifact_id)));
            if let Some(desc) = &lib.description {
                lines.push(markdown::bold("Description", desc));
            }
            lines.push(String::new());
            lines.push(markdown::header(2, "Next actions"));
            lines.push("• `search_symbols { \"query\": \"keyword\" }` to find APIs".to_string());
            lines.push("• `discover_technologies` to browse other libraries".to_string());
            lines.push("• `switch_platform \"apple\"` or `switch_platform \"flutter\"` to change platform".to_string());

            return Ok(text_response(lines).with_metadata(json!({
                "platform": "android",
                "selected": true,
                "library": lib.name,
                "category": lib.category.to_string(),
            })));
        }
    }

    lines.push("🚦 No Android library selected".to_string());
    lines.push(String::new());
    lines.push("Use `discover_technologies` to browse Android libraries, then `choose_technology` to select one.".to_string());

    Ok(text_response(lines).with_metadata(json!({
        "platform": "android",
        "selected": false
    })))
}

async fn handle_flutter(context: Arc<AppContext>, mut lines: Vec<String>) -> Result<ToolResponse> {
    if let Some(lib_name) = context.state.active_flutter_library.read().await.clone() {
        lines.push(markdown::header(2, "Active Library"));
        lines.push(String::new());
        lines.push(markdown::bold("Name", &lib_name));
        lines.push(String::new());
        lines.push(markdown::header(2, "Next actions"));
        lines.push("• `search_symbols { \"query\": \"keyword\" }` to find APIs".to_string());
        lines.push("• `discover_technologies` to browse other libraries".to_string());
        lines.push("• `switch_platform \"apple\"` or `switch_platform \"android\"` to change platform".to_string());

        return Ok(text_response(lines).with_metadata(json!({
            "platform": "flutter",
            "selected": true,
            "library": lib_name,
        })));
    }

    lines.push("🚦 No Flutter library selected".to_string());
    lines.push(String::new());
    lines.push("Use `discover_technologies` to browse Flutter libraries, then `choose_technology` to select one.".to_string());

    Ok(text_response(lines).with_metadata(json!({
        "platform": "flutter",
        "selected": false
    })))
}

async fn handle_apple(context: Arc<AppContext>, mut lines: Vec<String>) -> Result<ToolResponse> {
    if let Some(active) = context.state.active_technology.read().await.clone() {
        lines.push(markdown::header(2, "Active Technology"));
        lines.push(String::new());
        lines.push(markdown::bold("Name", &active.title));
        lines.push(markdown::bold("Identifier", &active.identifier));
        lines.push(String::new());
        lines.push(markdown::header(2, "Next actions"));
        lines.push("• `search_symbols { \"query\": \"keyword\" }` to find symbols".to_string());
        lines.push("• `get_documentation { \"path\": \"SymbolName\" }` to open docs".to_string());
        lines.push("• `choose_technology \"Another Framework\"` to switch".to_string());
        lines.push("• `switch_platform \"android\"` or `switch_platform \"flutter\"` to change platform".to_string());

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
                let mut fallback_keyword = None;
                let task_hint = recipe.keywords.first().copied().unwrap_or_else(|| {
                    fallback_keyword = Some(recipe.title.to_lowercase());
                    fallback_keyword.as_ref().unwrap().as_str()
                });
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
            "platform": "apple",
            "selected": true,
            "identifier": active.identifier,
            "name": active.title,
            "designPrimerCount": primer_count,
            "recipeCount": recipes.len(),
        });

        Ok(text_response(lines).with_metadata(metadata))
    } else {
        lines.push("🚦 Technology Not Selected".to_string());
        lines.push(String::new());
        lines.push("Use `discover_technologies` then `choose_technology` to get started.".to_string());
        Ok(text_response(lines).with_metadata(json!({
            "platform": "apple",
            "selected": false
        })))
    }
}
