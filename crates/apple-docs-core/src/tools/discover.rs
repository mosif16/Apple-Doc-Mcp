use std::{collections::HashMap, sync::Arc};

use anyhow::Result;
use apple_docs_client::{
    types::{extract_text, Technology},
    DocsPlatform, AndroidCategory,
};
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
            description: "Explore and filter available technologies/frameworks for the active platform (Apple, Android, or Flutter). Use switch_platform to change platforms.".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "Filter by technology name or description"
                    },
                    "category": {
                        "type": "string",
                        "description": "Filter by category. Apple: ui, data, network, media, system, accessibility, testing, developer. Android: compose, architecture, ui, core, media, connectivity, security, test. Flutter: widgets, material, cupertino, animation, painting, etc."
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
            let platform = *context.state.active_platform.read().await;
            match platform {
                DocsPlatform::Apple => handle_apple(context, args).await,
                DocsPlatform::Android => handle_android(context, args).await,
                DocsPlatform::Flutter => handle_flutter(context, args).await,
            }
        }),
    )
}

async fn handle_apple(context: Arc<AppContext>, args: Args) -> Result<ToolResponse> {
    let page = args.page.unwrap_or(1).max(1);
    let page_size = args.page_size.unwrap_or(25).clamp(1, 100);
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

    // Apply query filter with normalization support
    if let Some(query) = &args.query {
        frameworks.retain(|tech| {
            matches_framework_query(&tech.title, query)
                || extract_text(&tech.r#abstract)
                    .to_lowercase()
                    .contains(&query.to_lowercase())
        });
    }

    // Sort based on preference
    match sort_by {
        "relevance" => {
            frameworks.sort_by(|a, b| {
                let score_a = get_relevance_score(&a.title, &args.query);
                let score_b = get_relevance_score(&b.title, &args.query);
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

    // Query match boost with normalization
    if let Some(q) = query {
        let q_lower = q.to_lowercase();
        let title_normalized = normalize_framework_query(title);
        let query_normalized = normalize_framework_query(q);
        let title_compact: String = title_lower.chars().filter(|c| !c.is_whitespace()).collect();
        let query_compact: String = q_lower.chars().filter(|c| !c.is_whitespace()).collect();

        // Exact matches (including normalized variants)
        if title_lower == q_lower
            || title_normalized == query_normalized
            || title_compact == query_compact
        {
            score += 50; // Exact match
        } else if title_lower.starts_with(&q_lower)
            || title_normalized.starts_with(&query_normalized)
        {
            score += 30; // Starts with query
        } else if title_lower.contains(&q_lower)
            || title_normalized.contains(&query_normalized)
            || title_compact.contains(&query_compact)
        {
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

/// Normalize a framework query to handle CamelCase variants.
/// Examples:
///   "CoreML" -> "core ml"
///   "SwiftUI" -> "swiftui" (no split, as UI is a single unit)
///   "AVFoundation" -> "av foundation"
///   "CloudKit" -> "cloudkit" (Kit is kept together)
fn normalize_framework_query(query: &str) -> String {
    let mut result = String::new();
    let chars: Vec<char> = query.chars().collect();

    for (i, c) in chars.iter().enumerate() {
        // Insert space before uppercase letter if:
        // 1. Not the first character
        // 2. Previous char was lowercase
        // 3. OR current is uppercase and next is lowercase (for "AVFoundation" -> "AV Foundation")
        if i > 0 && c.is_uppercase() {
            let prev_lower = chars[i - 1].is_lowercase();
            let next_lower = chars.get(i + 1).map(|nc| nc.is_lowercase()).unwrap_or(false);
            let prev_upper = chars[i - 1].is_uppercase();

            if prev_lower || (prev_upper && next_lower) {
                result.push(' ');
            }
        }
        result.push(c.to_lowercase().next().unwrap());
    }
    result
}

/// Check if query matches framework title with normalization.
/// Handles cases like "CoreML" matching "Core ML" and vice versa.
fn matches_framework_query(title: &str, query: &str) -> bool {
    let title_lower = title.to_lowercase();
    let query_lower = query.to_lowercase();

    // Direct match
    if title_lower.contains(&query_lower) {
        return true;
    }

    // Normalized match (handles CamelCase)
    let title_normalized = normalize_framework_query(title);
    let query_normalized = normalize_framework_query(query);

    if title_normalized.contains(&query_normalized) {
        return true;
    }

    // Also try without spaces (for "Core ML" matching "coreml")
    let title_compact: String = title_lower.chars().filter(|c| !c.is_whitespace()).collect();
    let query_compact: String = query_lower.chars().filter(|c| !c.is_whitespace()).collect();

    title_compact.contains(&query_compact)
}

// ============================================================================
// Android Platform Handler
// ============================================================================

async fn handle_android(context: Arc<AppContext>, args: Args) -> Result<ToolResponse> {
    let page = args.page.unwrap_or(1).max(1);
    let page_size = args.page_size.unwrap_or(25).clamp(1, 100);
    let category_filter = args.category.as_ref().map(|c| c.to_lowercase());
    let sort_by = args.sort_by.as_deref().unwrap_or("alphabetical");

    let mut libraries = context.android_client.get_libraries().await?;

    // Apply category filter
    if let Some(cat) = &category_filter {
        let android_cat = match cat.as_str() {
            "compose" => Some(AndroidCategory::Compose),
            "architecture" => Some(AndroidCategory::Architecture),
            "ui" => Some(AndroidCategory::UI),
            "core" => Some(AndroidCategory::Core),
            "media" => Some(AndroidCategory::Media),
            "connectivity" => Some(AndroidCategory::Connectivity),
            "security" => Some(AndroidCategory::Security),
            "test" | "testing" => Some(AndroidCategory::Test),
            _ => None,
        };
        if let Some(category) = android_cat {
            libraries.retain(|lib| lib.category == category);
        }
    }

    // Apply query filter
    if let Some(query) = &args.query {
        let query_lower = query.to_lowercase();
        libraries.retain(|lib| {
            lib.name.to_lowercase().contains(&query_lower)
                || lib.artifact_id.to_lowercase().contains(&query_lower)
                || lib.description.as_ref().map(|d| d.to_lowercase().contains(&query_lower)).unwrap_or(false)
                || lib.packages.iter().any(|p| p.to_lowercase().contains(&query_lower))
        });
    }

    // Sort
    match sort_by {
        "relevance" => {
            libraries.sort_by(|a, b| {
                let score_a = get_android_relevance(&a.name, &args.query);
                let score_b = get_android_relevance(&b.name, &args.query);
                score_b.cmp(&score_a).then_with(|| a.name.cmp(&b.name))
            });
        }
        _ => {
            libraries.sort_by(|a, b| a.name.cmp(&b.name));
        }
    }

    let total_pages = (libraries.len().max(1) + page_size - 1) / page_size;
    let current_page = page.min(total_pages);
    let start = (current_page - 1) * page_size;
    let page_items: Vec<_> = libraries.iter().skip(start).take(page_size).collect();

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
        markdown::header(1, &format!("Discover Android Libraries{}", filter_desc)),
        String::new(),
        markdown::bold("Platform", "Android (Kotlin/Java)"),
        markdown::bold("Matches", &libraries.len().to_string()),
        markdown::bold("Page", &format!("{} / {}", current_page, total_pages.max(1))),
        String::new(),
    ];

    if args.query.is_none() && args.category.is_none() {
        lines.push("*Tip: Filter by category: `discover_technologies { \"category\": \"compose\" }`*".to_string());
        lines.push("*Categories: compose, architecture, ui, core, media, connectivity, security, test*".to_string());
        lines.push(String::new());
    }

    lines.push(markdown::header(2, "Available Libraries"));

    for lib in &page_items {
        lines.push(format!("### {} [{}]", lib.name, lib.category));
        if let Some(desc) = &lib.description {
            lines.push(format!("   {}", trim_with_ellipsis(desc, 180)));
        }
        lines.push(format!("   • **Artifact:** {}:{}", lib.group_id, lib.artifact_id));
        lines.push(format!("   • **Packages:** {}", lib.packages.join(", ")));
        lines.push(format!("   • **Select:** `choose_technology \"{}\"`", lib.name));
        lines.push(String::new());
    }

    lines.push(String::new());
    lines.push("## Next Step".to_string());
    lines.push("Use `choose_technology` to select a library, or `search_symbols` to search across all Android APIs.".to_string());

    let metadata = json!({
        "platform": "android",
        "totalMatches": libraries.len(),
        "page": current_page,
        "pageSize": page_size,
        "pageItems": page_items.len(),
        "query": args.query,
        "category": args.category,
    });

    Ok(text_response(lines).with_metadata(metadata))
}

fn get_android_relevance(name: &str, query: &Option<String>) -> i32 {
    let name_lower = name.to_lowercase();

    // Base scores for popular libraries
    let mut score = match name_lower.as_str() {
        "compose ui" | "compose material3" => 100,
        "compose foundation" | "compose animation" => 95,
        "viewmodel" | "room" | "hilt" => 90,
        "retrofit" | "okhttp" => 85,
        "recyclerview" | "constraintlayout" => 80,
        "navigation" | "datastore" => 75,
        _ if name_lower.contains("compose") => 70,
        _ => 50,
    };

    if let Some(q) = query {
        let q_lower = q.to_lowercase();
        if name_lower == q_lower {
            score += 50;
        } else if name_lower.starts_with(&q_lower) {
            score += 30;
        } else if name_lower.contains(&q_lower) {
            score += 15;
        }
    }

    score
}

// ============================================================================
// Flutter Platform Handler
// ============================================================================

async fn handle_flutter(context: Arc<AppContext>, args: Args) -> Result<ToolResponse> {
    let page = args.page.unwrap_or(1).max(1);
    let page_size = args.page_size.unwrap_or(25).clamp(1, 100);
    let category_filter = args.category.as_ref().map(|c| c.to_lowercase());

    let index = context.flutter_client.get_index().await?;

    // Filter to only libraries
    let mut libraries: Vec<_> = index
        .iter()
        .filter(|item| item.kind.as_deref() == Some("library"))
        .cloned()
        .collect();

    // Apply category filter (Flutter libraries often have category prefixes)
    if let Some(cat) = &category_filter {
        libraries.retain(|lib| {
            let name_lower = lib.name.to_lowercase();
            let qualified_lower = lib.qualified_name.to_lowercase();
            name_lower.contains(cat) || qualified_lower.contains(cat)
        });
    }

    // Apply query filter
    if let Some(query) = &args.query {
        let query_lower = query.to_lowercase();
        libraries.retain(|lib| {
            lib.name.to_lowercase().contains(&query_lower)
                || lib.qualified_name.to_lowercase().contains(&query_lower)
                || lib.description.as_ref().map(|d| d.to_lowercase().contains(&query_lower)).unwrap_or(false)
        });
    }

    // Sort by relevance (package_rank) or alphabetically
    let sort_by = args.sort_by.as_deref().unwrap_or("relevance");
    match sort_by {
        "alphabetical" => {
            libraries.sort_by(|a, b| a.name.cmp(&b.name));
        }
        _ => {
            libraries.sort_by(|a, b| {
                let score_a = a.package_rank.unwrap_or(0);
                let score_b = b.package_rank.unwrap_or(0);
                score_b.cmp(&score_a).then_with(|| a.name.cmp(&b.name))
            });
        }
    }

    let total_pages = (libraries.len().max(1) + page_size - 1) / page_size;
    let current_page = page.min(total_pages);
    let start = (current_page - 1) * page_size;
    let page_items: Vec<_> = libraries.iter().skip(start).take(page_size).collect();

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
        markdown::header(1, &format!("Discover Flutter Libraries{}", filter_desc)),
        String::new(),
        markdown::bold("Platform", "Flutter (Dart)"),
        markdown::bold("Matches", &libraries.len().to_string()),
        markdown::bold("Page", &format!("{} / {}", current_page, total_pages.max(1))),
        String::new(),
    ];

    if args.query.is_none() && args.category.is_none() {
        lines.push("*Tip: Filter by category: `discover_technologies { \"category\": \"widgets\" }`*".to_string());
        lines.push("*Common categories: widgets, material, cupertino, animation, painting, rendering, services*".to_string());
        lines.push(String::new());
    }

    lines.push(markdown::header(2, "Available Libraries"));

    for lib in &page_items {
        lines.push(format!("### {}", lib.name));
        if let Some(desc) = &lib.description {
            lines.push(format!("   {}", trim_with_ellipsis(desc, 180)));
        }
        lines.push(format!("   • **Path:** {}", lib.href));
        lines.push(format!("   • **Select:** `choose_technology \"{}\"`", lib.name));
        lines.push(String::new());
    }

    lines.push(String::new());
    lines.push("## Next Step".to_string());
    lines.push("Use `choose_technology` to select a library, or `search_symbols` to search Flutter APIs.".to_string());

    let metadata = json!({
        "platform": "flutter",
        "totalMatches": libraries.len(),
        "page": current_page,
        "pageSize": page_size,
        "pageItems": page_items.len(),
        "query": args.query,
        "category": args.category,
    });

    Ok(text_response(lines).with_metadata(metadata))
}
