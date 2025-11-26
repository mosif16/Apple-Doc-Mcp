use std::{collections::HashMap, sync::Arc};

use anyhow::Result;
use apple_docs_client::types::{extract_text, Technology};
use once_cell::sync::Lazy;
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
    category: Option<String>,
    #[serde(rename = "sortBy")]
    sort_by: Option<String>,
}

/// Technology categories for filtering
static CATEGORIES: Lazy<HashMap<&'static str, Vec<&'static str>>> = Lazy::new(|| {
    HashMap::from([
        (
            "ui",
            vec![
                "swiftui",
                "uikit",
                "appkit",
                "watchkit",
                "tvuikit",
                "widgetkit",
                "activitykit",
            ],
        ),
        (
            "data",
            vec![
                "foundation",
                "coredata",
                "swiftdata",
                "cloudkit",
                "combine",
                "observation",
            ],
        ),
        (
            "network",
            vec![
                "network",
                "urlsession",
                "multipeerconnectivity",
                "networkextension",
            ],
        ),
        (
            "media",
            vec![
                "avfoundation",
                "avkit",
                "coremedia",
                "coreimage",
                "coregraphics",
                "metal",
                "realitykit",
                "arkit",
                "scenekit",
                "spritekit",
                "vision",
                "photosui",
                "photokit",
            ],
        ),
        (
            "system",
            vec![
                "security",
                "corelocation",
                "mapkit",
                "eventkit",
                "contacts",
                "usernotifications",
                "backgroundtasks",
                "storekit",
                "gamekit",
            ],
        ),
        (
            "accessibility",
            vec!["accessibility", "voiceover", "assistiveaccess"],
        ),
        ("testing", vec!["xctest", "xctestui", "testing"]),
        (
            "developer",
            vec!["xcode", "instruments", "swift", "objectivec", "playground"],
        ),
    ])
});

/// Popularity/relevance scores for common frameworks
static POPULARITY: Lazy<HashMap<&'static str, i32>> = Lazy::new(|| {
    HashMap::from([
        // Tier 1 - Most commonly searched
        ("swiftui", 100),
        ("uikit", 95),
        ("foundation", 90),
        ("combine", 85),
        ("swift", 85),
        // Tier 2 - Very common
        ("coredata", 80),
        ("cloudkit", 75),
        ("avfoundation", 75),
        ("mapkit", 75),
        ("corelocation", 70),
        // Tier 3 - Common
        ("appkit", 65),
        ("widgetkit", 65),
        ("storekit", 65),
        ("usernotifications", 60),
        ("metal", 60),
        ("arkit", 60),
        ("realitykit", 55),
        // Tier 4 - Specialized
        ("watchkit", 50),
        ("tvuikit", 45),
        ("scenekit", 45),
        ("spritekit", 45),
        ("gamekit", 40),
        ("vision", 50),
        ("contacts", 40),
        ("eventkit", 40),
    ])
});

pub fn definition() -> (ToolDefinition, ToolHandler) {
    (
        ToolDefinition {
            name: "discover_technologies".to_string(),
            description: "Explore and filter available Apple technologies/frameworks".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "Filter by technology name or description"
                    },
                    "category": {
                        "type": "string",
                        "enum": ["ui", "data", "network", "media", "system", "accessibility", "testing", "developer"],
                        "description": "Filter by category: ui (SwiftUI, UIKit), data (CoreData, CloudKit), network, media (AV, Metal), system (Location, Notifications), accessibility, testing, developer"
                    },
                    "sortBy": {
                        "type": "string",
                        "enum": ["alphabetical", "relevance"],
                        "description": "Sort results: alphabetical (default) or relevance (most popular first)"
                    },
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
    let category_lower = args.category.as_ref().map(|c| c.to_lowercase());
    let sort_by = args.sort_by.as_deref().unwrap_or("alphabetical");

    let technologies = context.client.get_technologies().await?;
    let mut frameworks: Vec<Technology> = technologies
        .values()
        .cloned()
        .filter(|tech| tech.kind == "symbol" && tech.role == "collection")
        .collect();

    // Apply category filter
    if let Some(category) = &category_lower {
        if let Some(category_frameworks) = CATEGORIES.get(category.as_str()) {
            frameworks.retain(|tech| {
                let title_lower = tech.title.to_lowercase();
                category_frameworks
                    .iter()
                    .any(|cf| title_lower.contains(cf))
            });
        }
    }

    // Apply query filter
    if let Some(query_lower) = &query_lower {
        frameworks.retain(|tech| {
            tech.title.to_lowercase().contains(query_lower)
                || extract_text(&tech.r#abstract)
                    .to_lowercase()
                    .contains(query_lower)
        });
    }

    // Sort based on preference
    match sort_by {
        "relevance" => {
            frameworks.sort_by(|a, b| {
                let score_a = get_relevance_score(&a.title, &query_lower);
                let score_b = get_relevance_score(&b.title, &query_lower);
                score_b.cmp(&score_a).then_with(|| a.title.cmp(&b.title))
            });
        }
        _ => {
            frameworks.sort_by(|a, b| a.title.cmp(&b.title));
        }
    }

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

    // Build filter description
    let mut filter_parts = Vec::new();
    if let Some(query) = &args.query {
        filter_parts.push(format!("\"{}\"", query));
    }
    if let Some(category) = &args.category {
        filter_parts.push(format!("category: {}", category));
    }
    let filter_desc = if filter_parts.is_empty() {
        String::new()
    } else {
        format!(" ({})", filter_parts.join(", "))
    };

    let mut lines = vec![
        markdown::header(1, &format!("Discover Apple Technologies{}", filter_desc)),
        String::new(),
        markdown::bold("Matches", &frameworks.len().to_string()),
        markdown::bold(
            "Page",
            &format!("{} / {}", current_page, total_pages.max(1)),
        ),
        markdown::bold("Sort", if sort_by == "relevance" { "by relevance" } else { "alphabetical" }),
        String::new(),
    ];

    // Show available categories hint when no filter applied
    if args.query.is_none() && args.category.is_none() {
        lines.push("*Tip: Filter by category: `discover_technologies { \"category\": \"ui\" }`*".to_string());
        lines.push("*Categories: ui, data, network, media, system, accessibility, testing, developer*".to_string());
        lines.push(String::new());
    }

    lines.push(markdown::header(2, "Available Frameworks"));

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
        "category": args.category,
        "sortBy": sort_by,
    });

    Ok(text_response(lines).with_metadata(metadata))
}

/// Calculate relevance score for a technology based on popularity and query match
fn get_relevance_score(title: &str, query: &Option<String>) -> i32 {
    let title_lower = title.to_lowercase();

    // Base popularity score
    let mut score = POPULARITY
        .iter()
        .find(|(name, _)| title_lower.contains(*name))
        .map(|(_, s)| *s)
        .unwrap_or(30); // Default score for unknown frameworks

    // Query match boost
    if let Some(q) = query {
        let q_lower = q.to_lowercase();
        if title_lower == q_lower {
            score += 50; // Exact match
        } else if title_lower.starts_with(&q_lower) {
            score += 30; // Starts with query
        } else if title_lower.contains(&q_lower) {
            score += 15; // Contains query
        }
    }

    // Design guidance availability boost
    if design_guidance::has_primer_mapping_by_title(&title_lower) {
        score += 5;
    }

    // Recipe availability boost
    let recipe_count = knowledge::recipes_for(title).len();
    if recipe_count > 0 {
        score += recipe_count as i32 * 3;
    }

    score
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
