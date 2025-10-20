use std::sync::Arc;

use anyhow::Result;
use apple_docs_client::types::{extract_text, Technology};
use serde::Deserialize;
use serde_json::json;

use crate::{
    markdown,
    services::{design_guidance, knowledge},
    state::{AppContext, DiscoverySnapshot, ToolDefinition, ToolHandler, ToolResponse},
    tools::{parse_args, text_response, wrap_handler},
};

#[derive(Debug, Deserialize)]
struct Args {
    query: Option<String>,
    page: Option<usize>,
    #[serde(rename = "pageSize")]
    page_size: Option<usize>,
}

pub fn definition() -> (ToolDefinition, ToolHandler) {
    (
        ToolDefinition {
            name: "discover_technologies".to_string(),
            description: "Explore and filter available Apple technologies/frameworks".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "query": {"type": "string"},
                    "page": {"type": "number"},
                    "pageSize": {"type": "number"}
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
    let page = args.page.unwrap_or(1).max(1);
    let page_size = args.page_size.unwrap_or(25).clamp(1, 100);
    let query_lower = args.query.as_ref().map(|q| q.to_lowercase());

    let technologies = context.client.get_technologies().await?;
    let mut frameworks: Vec<Technology> = technologies
        .values()
        .cloned()
        .filter(|tech| tech.kind == "symbol" && tech.role == "collection")
        .collect();

    if let Some(query_lower) = &query_lower {
        frameworks.retain(|tech| {
            tech.title.to_lowercase().contains(query_lower)
                || extract_text(&tech.r#abstract)
                    .to_lowercase()
                    .contains(query_lower)
        });
    }

    frameworks.sort_by(|a, b| a.title.cmp(&b.title));

    let total_pages = (frameworks.len().max(1) + page_size - 1) / page_size;
    let current_page = page.min(total_pages);
    let start = (current_page - 1) * page_size;
    let page_items = frameworks
        .iter()
        .skip(start)
        .take(page_size)
        .cloned()
        .collect::<Vec<_>>();

    *context.state.last_discovery.write().await = Some(DiscoverySnapshot {
        query: args.query.clone(),
        results: page_items.clone(),
    });

    let mut lines = vec![
        markdown::header(
            1,
            &format!(
                "Discover Apple Technologies{}",
                args.query
                    .as_ref()
                    .map(|query| format!(" (filtered by \"{}\")", query))
                    .unwrap_or_default()
            ),
        ),
        String::new(),
        markdown::bold("Matches", &frameworks.len().to_string()),
        markdown::bold(
            "Page",
            &format!("{} / {}", current_page, total_pages.max(1)),
        ),
        String::new(),
        markdown::header(2, "Available Frameworks"),
    ];

    for framework in &page_items {
        let description = extract_text(&framework.r#abstract);
        let is_design = framework
            .url
            .to_ascii_lowercase()
            .starts_with("/design/human-interface-guidelines");
        let has_primers = design_guidance::has_primer_mapping(framework);
        let recipe_count = knowledge::recipes_for(&framework.title).len();
        let mut title_line = format!("### {}", framework.title);
        if is_design || has_primers {
            title_line.push_str(" · [Design]");
        }
        lines.push(title_line);
        if !description.is_empty() {
            lines.push(format!("   {}", trim_with_ellipsis(&description, 180)));
        }
        if is_design {
            lines.push(
                "   • Focus: Human Interface Guidelines primers for multi-platform design."
                    .to_string(),
            );
        } else if has_primers {
            lines.push(
                "   • Design support: SwiftUI/UIKit mappings include layout, typography, and color guidance."
                    .to_string(),
            );
        }
        lines.push(format!("   • **Identifier:** {}", framework.identifier));
        if recipe_count > 0 {
            lines.push(format!(
                "   • Recipes available: {} (`how_do_i {{ \"task\": \"...\" }}`)",
                recipe_count
            ));
        }
        lines.push(format!(
            "   • **Select:** `choose_technology \"{}\"`",
            framework.title
        ));
        lines.push(String::new());
    }

    lines.extend(build_pagination(
        args.query.as_deref(),
        current_page,
        total_pages,
    ));
    lines.push(String::new());
    lines.push("## Next Step".to_string());
    let design_badged = page_items
        .iter()
        .filter(|framework| {
            framework
                .url
                .to_ascii_lowercase()
                .starts_with("/design/human-interface-guidelines")
                || design_guidance::has_primer_mapping(framework)
        })
        .count();
    let recipes_on_page: usize = page_items
        .iter()
        .map(|framework| knowledge::recipes_for(&framework.title).len())
        .sum();
    let metadata = json!({
        "totalMatches": frameworks.len(),
        "page": current_page,
        "pageSize": page_size,
        "pageItems": page_items.len(),
        "designFlaggedOnPage": design_badged,
        "recipesOnPage": recipes_on_page,
        "query": args.query,
    });

    Ok(text_response(lines).with_metadata(metadata))
}

fn build_pagination(query: Option<&str>, current: usize, total: usize) -> Vec<String> {
    if total <= 1 {
        return vec![];
    }

    let query = query.unwrap_or("");
    let mut items = Vec::new();
    if current > 1 {
        items.push(format!(
            "• Previous: `discover_technologies {{ \"query\": \"{}\", \"page\": {} }}`",
            query,
            current - 1
        ));
    }
    if current < total {
        items.push(format!(
            "• Next: `discover_technologies {{ \"query\": \"{}\", \"page\": {} }}`",
            query,
            current + 1
        ));
    }

    if items.is_empty() {
        Vec::new()
    } else {
        let mut lines = vec!["*Pagination*".to_string()];
        lines.extend(items);
        lines
    }
}

fn trim_with_ellipsis(text: &str, max: usize) -> String {
    if text.len() <= max {
        text.to_string()
    } else {
        format!("{}...", &text[..max])
    }
}
