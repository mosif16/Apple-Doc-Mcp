use std::{collections::HashMap, sync::Arc};

use anyhow::Result;
use docs_mcp_client::types::extract_text;
use multi_provider_client::types::{ProviderType, TechnologyKind, UnifiedTechnology};
use once_cell::sync::Lazy;
use serde::Deserialize;
use serde_json::json;

use crate::{
    markdown,
    services::{design_guidance, knowledge},
    state::{AppContext, ToolDefinition, ToolHandler, ToolResponse},
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
    /// Filter by provider: "apple", "telegram", "ton", "cocoon", or "all" (default)
    provider: Option<String>,
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

/// Code execution caller identifier for programmatic tool calling.
const CODE_EXECUTION_CALLER: &str = "code_execution_20250825";

pub fn definition() -> (ToolDefinition, ToolHandler) {
    (
        ToolDefinition {
            name: "discover_technologies".to_string(),
            description: "Explore and filter available technologies/frameworks from Apple, Telegram, TON, Cocoon, and Rust. \
                         Supports programmatic iteration: retrieve technology list in code, then loop through \
                         to search or fetch documentation for each. Useful for cross-framework analysis."
                .to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "Filter by technology name or description"
                    },
                    "provider": {
                        "type": "string",
                        "enum": ["apple", "telegram", "ton", "cocoon", "rust", "all"],
                        "description": "Filter by documentation provider (default: all). Use 'apple' for iOS/macOS frameworks, 'telegram' for Bot API, 'ton' for blockchain API, 'cocoon' for confidential computing, 'rust' for Rust std library and crates"
                    },
                    "category": {
                        "type": "string",
                        "enum": ["ui", "data", "network", "media", "system", "accessibility", "testing", "developer"],
                        "description": "Filter by category (Apple only): ui (SwiftUI, UIKit), data (CoreData, CloudKit), network, media (AV, Metal), system (Location, Notifications), accessibility, testing, developer"
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
            // Examples demonstrating various filtering and browsing patterns
            input_examples: Some(vec![
                // Browse all technologies (no filters)
                json!({}),
                // Search by name
                json!({"query": "SwiftUI"}),
                // Filter by provider
                json!({"provider": "telegram"}),
                // Filter by category (Apple frameworks)
                json!({"category": "ui", "sortBy": "relevance"}),
                // Combined filters with pagination
                json!({"query": "data", "provider": "apple", "page": 2, "pageSize": 10}),
                // Browse Rust crates
                json!({"provider": "rust"}),
            ]),
            // Enable programmatic calling for technology enumeration.
            // Allows Claude to iterate through frameworks and perform operations on each.
            allowed_callers: Some(vec![CODE_EXECUTION_CALLER.to_string()]),
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
    let category_lower = args.category.as_ref().map(|c| c.to_lowercase());
    let sort_by = args.sort_by.as_deref().unwrap_or("alphabetical");
    let provider_filter = args.provider.as_deref().unwrap_or("all").to_lowercase();

    // Collect technologies from all requested providers
    let mut unified_techs: Vec<UnifiedTechnology> = Vec::new();

    // Apple technologies
    if provider_filter == "all" || provider_filter == "apple" {
        let technologies = context.client.get_technologies().await?;
        let apple_techs: Vec<UnifiedTechnology> = technologies
            .values()
            .filter(|tech| tech.kind == "symbol" && tech.role == "collection")
            .map(|tech| UnifiedTechnology {
                provider: ProviderType::Apple,
                identifier: tech.identifier.clone(),
                title: tech.title.clone(),
                description: extract_text(&tech.r#abstract),
                url: Some(format!("https://developer.apple.com{}", tech.url)),
                kind: TechnologyKind::Framework,
            })
            .collect();

        // Apply category filter (Apple only)
        let filtered_apple = if let Some(category) = &category_lower {
            if let Some(category_frameworks) = CATEGORIES.get(category.as_str()) {
                apple_techs
                    .into_iter()
                    .filter(|tech| {
                        let title_lower = tech.title.to_lowercase();
                        category_frameworks.iter().any(|cf| title_lower.contains(cf))
                    })
                    .collect()
            } else {
                apple_techs
            }
        } else {
            apple_techs
        };

        unified_techs.extend(filtered_apple);
    }

    // Telegram technologies
    if provider_filter == "all" || provider_filter == "telegram" {
        if let Ok(telegram_techs) = context.providers.telegram.get_technologies().await {
            unified_techs.extend(telegram_techs.into_iter().map(UnifiedTechnology::from_telegram));
        }
    }

    // TON technologies
    if provider_filter == "all" || provider_filter == "ton" {
        if let Ok(ton_techs) = context.providers.ton.get_technologies().await {
            unified_techs.extend(ton_techs.into_iter().map(UnifiedTechnology::from_ton));
        }
    }

    // Cocoon technologies
    if provider_filter == "all" || provider_filter == "cocoon" {
        if let Ok(cocoon_techs) = context.providers.cocoon.get_technologies().await {
            unified_techs.extend(cocoon_techs.into_iter().map(UnifiedTechnology::from_cocoon));
        }
    }

    // Rust technologies
    if provider_filter == "all" || provider_filter == "rust" {
        if let Ok(rust_techs) = context.providers.rust.get_technologies().await {
            unified_techs.extend(rust_techs.into_iter().map(UnifiedTechnology::from_rust));
        }
    }

    // Apply query filter
    if let Some(query) = &args.query {
        let query_lower = query.to_lowercase();
        unified_techs.retain(|tech| {
            matches_framework_query(&tech.title, query)
                || tech.description.to_lowercase().contains(&query_lower)
        });
    }

    // Sort based on preference
    match sort_by {
        "relevance" => {
            unified_techs.sort_by(|a, b| {
                let score_a = get_unified_relevance_score(a, &args.query);
                let score_b = get_unified_relevance_score(b, &args.query);
                score_b.cmp(&score_a).then_with(|| a.title.cmp(&b.title))
            });
        }
        _ => {
            // Sort by provider first, then alphabetically
            unified_techs.sort_by(|a, b| {
                provider_sort_order(&a.provider)
                    .cmp(&provider_sort_order(&b.provider))
                    .then_with(|| a.title.cmp(&b.title))
            });
        }
    }

    let total_pages = unified_techs.len().max(1).div_ceil(page_size);
    let current_page = page.min(total_pages);
    let start = (current_page - 1) * page_size;
    let page_items: Vec<UnifiedTechnology> = unified_techs
        .iter()
        .skip(start)
        .take(page_size)
        .cloned()
        .collect();

    // Build filter description
    let mut filter_parts = Vec::new();
    if let Some(query) = &args.query {
        filter_parts.push(format!("\"{}\"", query));
    }
    if provider_filter != "all" {
        filter_parts.push(format!("provider: {}", provider_filter));
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
        markdown::header(1, &format!("Discover Technologies{}", filter_desc)),
        String::new(),
        markdown::bold("Matches", &unified_techs.len().to_string()),
        markdown::bold(
            "Page",
            &format!("{} / {}", current_page, total_pages.max(1)),
        ),
        markdown::bold("Sort", if sort_by == "relevance" { "by relevance" } else { "by provider" }),
        String::new(),
    ];

    // Show available providers hint when no filter applied
    if args.query.is_none() && provider_filter == "all" {
        lines.push("*Available providers: apple (iOS/macOS), telegram (Bot API), ton (Blockchain), cocoon (Confidential Computing), rust (Rust std & crates)*".to_string());
        lines.push("*Filter: `discover_technologies { \"provider\": \"telegram\" }`*".to_string());
        lines.push(String::new());
    }

    // Group by provider for better display
    let mut current_provider: Option<ProviderType> = None;
    for tech in &page_items {
        // Add provider header if changed
        if current_provider.as_ref() != Some(&tech.provider) {
            current_provider = Some(tech.provider);
            lines.push(markdown::header(2, &format!("{} Technologies", provider_display_name(&tech.provider))));
            lines.push(String::new());
        }

        let mut title_line = format!("### {}", tech.title);

        // Apple-specific badges
        if tech.provider == ProviderType::Apple {
            let recipe_count = knowledge::recipes_for(&tech.title).len();
            if recipe_count > 0 {
                title_line.push_str(" · [Recipes]");
            }
        }

        // Kind badge
        let kind_badge = match &tech.kind {
            TechnologyKind::Framework => "",
            TechnologyKind::ApiCategory => " [API]",
            TechnologyKind::BlockchainApi => " [Blockchain]",
            TechnologyKind::DocSection => " [Docs]",
            TechnologyKind::RustCrate => " [Crate]",
            TechnologyKind::MdnCategory => " [Web]",
            TechnologyKind::WebFramework => " [Framework]",
            TechnologyKind::MlxFramework => " [ML]",
            TechnologyKind::HfLibrary => " [AI]",
            TechnologyKind::QuickNodeApi => " [Solana]",
            TechnologyKind::AgentSdkLibrary => " [SDK]",
            TechnologyKind::VertcoinApi => " [VTC]",
        };
        title_line.push_str(kind_badge);

        lines.push(title_line);
        if !tech.description.is_empty() {
            lines.push(format!("   {}", trim_with_ellipsis(&tech.description, 180)));
        }
        lines.push(format!("   • **Identifier:** {}", tech.identifier));
        lines.push(format!(
            "   • **Select:** `choose_technology {{ \"identifier\": \"{}\" }}`",
            tech.identifier
        ));
        lines.push(String::new());
    }

    lines.extend(build_pagination_with_provider(
        args.query.as_deref(),
        &provider_filter,
        current_page,
        total_pages,
    ));

    // Count by provider
    let apple_count = unified_techs.iter().filter(|t| t.provider == ProviderType::Apple).count();
    let telegram_count = unified_techs.iter().filter(|t| t.provider == ProviderType::Telegram).count();
    let ton_count = unified_techs.iter().filter(|t| t.provider == ProviderType::TON).count();
    let cocoon_count = unified_techs.iter().filter(|t| t.provider == ProviderType::Cocoon).count();
    let rust_count = unified_techs.iter().filter(|t| t.provider == ProviderType::Rust).count();

    let metadata = json!({
        "totalMatches": unified_techs.len(),
        "page": current_page,
        "pageSize": page_size,
        "pageItems": page_items.len(),
        "query": args.query,
        "provider": provider_filter,
        "category": args.category,
        "sortBy": sort_by,
        "providerCounts": {
            "apple": apple_count,
            "telegram": telegram_count,
            "ton": ton_count,
            "cocoon": cocoon_count,
            "rust": rust_count,
        }
    });

    Ok(text_response(lines).with_metadata(metadata))
}

/// Get display name for provider
fn provider_display_name(provider: &ProviderType) -> &'static str {
    match provider {
        ProviderType::Apple => "🍎 Apple",
        ProviderType::Telegram => "📱 Telegram",
        ProviderType::TON => "💎 TON Blockchain",
        ProviderType::Cocoon => "🥥 Cocoon",
        ProviderType::Rust => "🦀 Rust",
        ProviderType::Mdn => "📚 MDN Web Docs",
        ProviderType::WebFrameworks => "⚛️ Web Frameworks",
        ProviderType::Mlx => "🧠 MLX",
        ProviderType::HuggingFace => "🤗 Hugging Face",
        ProviderType::QuickNode => "⚡ QuickNode Solana",
        ProviderType::ClaudeAgentSdk => "🤖 Claude Agent SDK",
        ProviderType::Vertcoin => "💚 Vertcoin",
    }
}

/// Get sort order for provider (Apple first, then alphabetically)
fn provider_sort_order(provider: &ProviderType) -> u8 {
    match provider {
        ProviderType::Apple => 0,
        ProviderType::Telegram => 1,
        ProviderType::TON => 2,
        ProviderType::Cocoon => 3,
        ProviderType::Rust => 4,
        ProviderType::Mdn => 5,
        ProviderType::WebFrameworks => 6,
        ProviderType::Mlx => 7,
        ProviderType::HuggingFace => 8,
        ProviderType::QuickNode => 9,
        ProviderType::ClaudeAgentSdk => 10,
        ProviderType::Vertcoin => 11,
    }
}

/// Calculate relevance score for unified technology
fn get_unified_relevance_score(tech: &UnifiedTechnology, query: &Option<String>) -> i32 {
    let title_lower = tech.title.to_lowercase();

    // Base popularity score (Apple frameworks have predefined scores)
    let mut score = if tech.provider == ProviderType::Apple {
        POPULARITY
            .iter()
            .find(|(name, _)| title_lower.contains(*name))
            .map(|(_, s)| *s)
            .unwrap_or(30)
    } else {
        // Non-Apple providers get base score based on kind
        match &tech.kind {
            TechnologyKind::ApiCategory => 40,
            TechnologyKind::BlockchainApi => 35,
            TechnologyKind::DocSection => 30,
            TechnologyKind::Framework => 50,
            TechnologyKind::RustCrate => 45,
            TechnologyKind::MdnCategory => 48,
            TechnologyKind::WebFramework => 47,
            TechnologyKind::MlxFramework => 46,
            TechnologyKind::HfLibrary => 44,
            TechnologyKind::QuickNodeApi => 42,
            TechnologyKind::AgentSdkLibrary => 43,
            TechnologyKind::VertcoinApi => 41,
        }
    };

    // Query match boost
    if let Some(q) = query {
        let q_lower = q.to_lowercase();
        let title_normalized = normalize_framework_query(&tech.title);
        let query_normalized = normalize_framework_query(q);

        if title_lower == q_lower || title_normalized == query_normalized {
            score += 50;
        } else if title_lower.starts_with(&q_lower) || title_normalized.starts_with(&query_normalized) {
            score += 30;
        } else if title_lower.contains(&q_lower) || title_normalized.contains(&query_normalized) {
            score += 15;
        }
    }

    // Recipe availability boost (Apple only)
    if tech.provider == ProviderType::Apple {
        let recipe_count = knowledge::recipes_for(&tech.title).len();
        if recipe_count > 0 {
            score += recipe_count as i32 * 3;
        }
    }

    score
}

fn build_pagination_with_provider(query: Option<&str>, provider: &str, current: usize, total: usize) -> Vec<String> {
    if total <= 1 {
        return vec![];
    }

    let query = query.unwrap_or("");
    let mut items = Vec::new();
    if current > 1 {
        items.push(format!(
            "• Previous: `discover_technologies {{ \"query\": \"{}\", \"provider\": \"{}\", \"page\": {} }}`",
            query, provider, current - 1
        ));
    }
    if current < total {
        items.push(format!(
            "• Next: `discover_technologies {{ \"query\": \"{}\", \"provider\": \"{}\", \"page\": {} }}`",
            query, provider, current + 1
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
