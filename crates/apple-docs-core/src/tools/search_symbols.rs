use std::{collections::HashSet, sync::Arc};

use anyhow::{bail, Context, Result};
use apple_docs_client::types::{
    extract_text, format_platforms, FrameworkData, PlatformInfo, ReferenceData, Technology,
};
use regex::Regex;
use serde::Deserialize;

use crate::{
    markdown,
    services::{
        ensure_framework_index, ensure_global_framework_index, expand_identifiers, knowledge,
        load_active_framework,
    },
    state::{AppContext, FrameworkIndexEntry, ToolDefinition, ToolHandler, ToolResponse},
    tools::{parse_args, text_response, wrap_handler},
};

#[derive(Debug, Deserialize)]
struct Args {
    query: String,
    #[serde(rename = "maxResults")]
    max_results: Option<usize>,
    platform: Option<String>,
    #[serde(rename = "symbolType")]
    symbol_type: Option<String>,
    scope: Option<String>,
}

pub fn definition() -> (ToolDefinition, ToolHandler) {
    (
        ToolDefinition {
            name: "search_symbols".to_string(),
            description:
                "Search symbols within the selected technology or across all Apple documentation"
                    .to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "required": ["query"],
                "properties": {
                    "query": {"type": "string"},
                    "maxResults": {"type": "number"},
                    "platform": {"type": "string"},
                    "symbolType": {"type": "string"},
                    "scope": {
                        "type": "string",
                        "enum": ["technology", "global"],
                        "description": "Set to \"global\" to search every technology instead of only the active one"
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
    let scope = args.scope.as_deref().unwrap_or("technology").to_lowercase();

    match scope.as_str() {
        "technology" => search_active_technology(context, args).await,
        "global" => search_all_technologies(context, args).await,
        _ => bail!("Unsupported search scope \"{}\"", scope),
    }
}

async fn search_active_technology(context: Arc<AppContext>, args: Args) -> Result<ToolResponse> {
    let technology = context
        .state
        .active_technology
        .read()
        .await
        .clone()
        .context("No technology selected. Use `choose_technology` first.")?;

    let mut index = ensure_framework_index(&context).await?;
    let max_results = args.max_results.unwrap_or(20).max(1);

    let mut ranked_matches = collect_matches(&index, &args);
    if ranked_matches.is_empty() {
        let framework = load_active_framework(&context).await?;
        let identifiers: Vec<String> = framework
            .topic_sections
            .iter()
            .flat_map(|section| section.identifiers.iter().cloned())
            .take(200)
            .collect();
        if !identifiers.is_empty() {
            index = expand_identifiers(&context, &identifiers).await?;
            ranked_matches = collect_matches(&index, &args);
        }
    }

    let mut deduped_matches = Vec::new();
    if !ranked_matches.is_empty() {
        let mut seen_paths = HashSet::new();
        for (_, entry) in ranked_matches {
            let path = entry
                .reference
                .url
                .clone()
                .unwrap_or_else(|| "(unknown path)".to_string());
            let title = entry
                .reference
                .title
                .clone()
                .unwrap_or_else(|| "Symbol".to_string());
            let key = dedup_key(&path, &title);
            if seen_paths.insert(key) {
                deduped_matches.push(entry);
            }
            if deduped_matches.len() >= max_results {
                break;
            }
        }
    }

    let match_count = deduped_matches.len();
    let mut fallback = Vec::new();
    if match_count == 0 {
        fallback = perform_fallback_search(&context, &args, max_results).await?;
    }

    let mut lines = vec![
        markdown::header(1, &format!("🔍 Search Results for \"{}\"", args.query)),
        String::new(),
        markdown::bold("Technology", &technology.title),
        markdown::bold("Matches", &match_count.to_string()),
        String::new(),
        markdown::header(2, "Symbols"),
        String::new(),
    ];

    if deduped_matches.is_empty() {
        lines.push("No symbols matched those terms within this technology.".to_string());
        lines.push("Try broader keywords (e.g. \"tab\"), explore synonyms, or run `discover_technologies` again.".to_string());
        if !fallback.is_empty() {
            lines.push(String::new());
            lines.push(markdown::header(2, "Fallback suggestions"));
            lines.push(String::new());
            for result in fallback {
                lines.push(format!(
                    "• **{}** — {}",
                    result.title,
                    trim_with_ellipsis(&result.description, 120)
                ));
                lines.push(format!(
                    "  `get_documentation {{ \"path\": \"{}\" }}`",
                    result.path
                ));
                lines.push(format!("  Platforms: {}", result.platforms));
                lines.push(format!("  Found via: {}", result.found_via));
                lines.push(String::new());
            }
        }
    } else {
        for entry in deduped_matches {
            let title = entry
                .reference
                .title
                .clone()
                .unwrap_or_else(|| "Symbol".to_string());
            let description = entry
                .reference
                .r#abstract
                .as_ref()
                .map(|segments| extract_text(segments))
                .unwrap_or_default();
            let path = entry
                .reference
                .url
                .clone()
                .unwrap_or_else(|| "(unknown path)".to_string());
            let platform_slice = entry
                .reference
                .platforms
                .as_ref()
                .map(|platforms| platforms.as_slice());
            let (platform_label, availability) = classify_platforms(&path, platform_slice);
            lines.push(format!(
                "• **{}** — {}",
                title,
                trim_with_ellipsis(&description, 120)
            ));
            lines.push(format!(
                "  `get_documentation {{ \"path\": \"{}\" }}`",
                path
            ));
            lines.push(format!("  Platforms: {}", platform_label));
            if let Some(introduced) = availability {
                lines.push(format!("  Availability: {}", introduced));
            }
            if let Some(entry) = knowledge::lookup(&technology.title, &title) {
                if let Some(tip) = entry.quick_tip {
                    lines.push(format!("  Tip: {}", tip));
                }
                let related = knowledge::related_items(entry);
                if !related.is_empty() {
                    let summary = related
                        .iter()
                        .map(|item| item.title)
                        .take(3)
                        .collect::<Vec<_>>()
                        .join(" · ");
                    lines.push(format!("  Related: {}", summary));
                }
                let links = knowledge::integration_links(entry);
                if !links.is_empty() {
                    let summary = links
                        .iter()
                        .map(|link| format!("{} {}", link.framework, link.title))
                        .collect::<Vec<_>>()
                        .join(" · ");
                    lines.push(format!("  Bridge: {}", summary));
                }
            }
            lines.push(String::new());
        }
    }

    Ok(text_response(lines))
}

async fn search_all_technologies(context: Arc<AppContext>, args: Args) -> Result<ToolResponse> {
    let max_results = args.max_results.unwrap_or(20).max(1);

    let technologies = context.client.get_technologies().await?;
    let frameworks: Vec<Technology> = technologies
        .values()
        .cloned()
        .filter(|tech| tech.kind == "symbol" && tech.role == "collection")
        .collect();

    let mut aggregate = Vec::new();
    for technology in &frameworks {
        let mut matches = collect_matches(
            &ensure_global_framework_index(&context, technology).await?,
            &args,
        );
        matches.truncate(max_results);

        for (score, entry) in matches {
            aggregate.push(GlobalMatch {
                score,
                entry,
                technology_title: technology.title.clone(),
                technology_identifier: technology.identifier.clone(),
            });
        }
    }

    aggregate.sort_by(|a, b| {
        b.score
            .cmp(&a.score)
            .then_with(|| a.entry.reference.title.cmp(&b.entry.reference.title))
            .then_with(|| a.technology_title.cmp(&b.technology_title))
    });

    let mut seen_paths = HashSet::new();
    let mut matches = Vec::new();
    for item in aggregate {
        let path = item
            .entry
            .reference
            .url
            .clone()
            .unwrap_or_else(|| "(unknown path)".to_string());
        let title = item
            .entry
            .reference
            .title
            .clone()
            .unwrap_or_else(|| "Symbol".to_string());
        if seen_paths.insert(dedup_key(&path, &title)) {
            matches.push(item);
        }
        if matches.len() >= max_results {
            break;
        }
    }

    let mut lines = vec![
        markdown::header(
            1,
            &format!("🔍 Global Search Results for \"{}\"", args.query),
        ),
        String::new(),
        markdown::bold("Scope", "All Apple Technologies"),
        markdown::bold("Matches", &matches.len().to_string()),
        markdown::bold("Technologies Scanned", &frameworks.len().to_string()),
        String::new(),
        markdown::header(2, "Symbols"),
        String::new(),
    ];

    if matches.is_empty() {
        lines.push("No symbols matched those terms across Apple documentation.".to_string());
        lines.push("Try alternative keywords or switch back to a specific technology.".to_string());
        return Ok(text_response(lines));
    }

    for matched in matches {
        let title = matched
            .entry
            .reference
            .title
            .clone()
            .unwrap_or_else(|| "Symbol".to_string());
        let description = matched
            .entry
            .reference
            .r#abstract
            .as_ref()
            .map(|segments| extract_text(segments))
            .unwrap_or_default();
        let path = matched
            .entry
            .reference
            .url
            .clone()
            .unwrap_or_else(|| "(unknown path)".to_string());
        let platform_slice = matched
            .entry
            .reference
            .platforms
            .as_ref()
            .map(|platforms| platforms.as_slice());
        let (platform_label, availability) = classify_platforms(&path, platform_slice);

        lines.push(format!(
            "• **{}** — {}",
            title,
            trim_with_ellipsis(&description, 120)
        ));
        lines.push(format!("  Technology: {}", matched.technology_title));
        lines.push(format!("  Identifier: {}", matched.technology_identifier));
        lines.push(format!(
            "  `get_documentation {{ \"path\": \"{}\" }}`",
            path
        ));
        lines.push(format!("  Platforms: {}", platform_label));
        if let Some(introduced) = availability {
            lines.push(format!("  Availability: {}", introduced));
        }
        if let Some(entry) = knowledge::lookup(&matched.technology_title, &title) {
            if let Some(tip) = entry.quick_tip {
                lines.push(format!("  Tip: {}", tip));
            }
            let related = knowledge::related_items(entry);
            if !related.is_empty() {
                let summary = related
                    .iter()
                    .map(|item| item.title)
                    .take(3)
                    .collect::<Vec<_>>()
                    .join(" · ");
                lines.push(format!("  Related: {}", summary));
            }
            let links = knowledge::integration_links(entry);
            if !links.is_empty() {
                let summary = links
                    .iter()
                    .map(|link| format!("{} {}", link.framework, link.title))
                    .collect::<Vec<_>>()
                    .join(" · ");
                lines.push(format!("  Bridge: {}", summary));
            }
        }
        lines.push(String::new());
    }

    Ok(text_response(lines))
}

fn collect_matches(
    entries: &[FrameworkIndexEntry],
    args: &Args,
) -> Vec<(i32, FrameworkIndexEntry)> {
    let terms = args
        .query
        .to_lowercase()
        .split_whitespace()
        .map(str::to_string)
        .collect::<Vec<_>>();

    let mut ranked = Vec::new();
    for entry in entries {
        if let Some(symbol_type) = &args.symbol_type {
            if !entry
                .reference
                .kind
                .as_ref()
                .map(|kind| kind.eq_ignore_ascii_case(symbol_type))
                .unwrap_or(false)
            {
                continue;
            }
        }

        if let Some(platform) = &args.platform {
            let lower = platform.to_lowercase();
            let matches_platform = entry
                .reference
                .platforms
                .as_ref()
                .map(|platforms| {
                    platforms
                        .iter()
                        .any(|info| info.name.to_lowercase().contains(&lower))
                })
                .unwrap_or(true);
            if !matches_platform {
                continue;
            }
        }

        let score = score_entry(entry, &terms);
        if score > 0 {
            ranked.push((score, entry.clone()));
        }
    }

    ranked.sort_by(|a, b| {
        b.0.cmp(&a.0)
            .then_with(|| a.1.reference.title.cmp(&b.1.reference.title))
    });
    ranked
}

fn score_entry(entry: &FrameworkIndexEntry, terms: &[String]) -> i32 {
    let mut score = 0;
    for term in terms {
        if entry.tokens.iter().any(|token| token == term) {
            score += 3;
        } else if entry.tokens.iter().any(|token| token.contains(term)) {
            score += 1;
        }
    }
    score
}

fn trim_with_ellipsis(text: &str, max: usize) -> String {
    if text.len() <= max {
        text.to_string()
    } else {
        format!("{}...", &text[..max])
    }
}

struct FallbackResult {
    title: String,
    path: String,
    description: String,
    platforms: String,
    found_via: &'static str,
}

struct GlobalMatch {
    score: i32,
    entry: FrameworkIndexEntry,
    technology_title: String,
    technology_identifier: String,
}

fn classify_platforms(path: &str, platforms: Option<&[PlatformInfo]>) -> (String, Option<String>) {
    if is_design_material(path) {
        return ("Design guidance".to_string(), None);
    }

    match platforms {
        Some(slice) if !slice.is_empty() => {
            let availability = summarize_introduced(slice);
            (format_platforms(slice), availability)
        }
        _ => ("All platforms".to_string(), None),
    }
}

fn summarize_introduced(platforms: &[PlatformInfo]) -> Option<String> {
    let mut entries = Vec::new();
    for platform in platforms {
        if let Some(version) = &platform.introduced_at {
            let mut text = format!("{} {}", platform.name, version);
            if platform.beta {
                text.push_str(" (Beta)");
            }
            entries.push(text);
        }
    }
    if entries.is_empty() {
        None
    } else {
        Some(entries.join(" · "))
    }
}

fn is_design_material(path: &str) -> bool {
    path.contains("/design/")
}

fn dedup_key(path: &str, title: &str) -> String {
    if path == "(unknown path)" {
        format!("unknown::{}", title.to_lowercase())
    } else {
        path.to_lowercase()
    }
}

async fn perform_fallback_search(
    context: &Arc<AppContext>,
    args: &Args,
    max_results: usize,
) -> Result<Vec<FallbackResult>> {
    let framework = load_active_framework(context).await?;
    let mut results = hierarchical_fallback(&framework, args, max_results);
    if results.is_empty() {
        results = regex_fallback(&framework, args, max_results)?;
    }
    Ok(results)
}

fn hierarchical_fallback(
    framework: &FrameworkData,
    args: &Args,
    max_results: usize,
) -> Vec<FallbackResult> {
    let query = args.query.to_lowercase();
    let mut results = Vec::new();
    for reference in framework.references.values() {
        let title = reference.title.as_deref().unwrap_or("");
        let url = reference.url.as_deref().unwrap_or("");
        let abstract_text = reference
            .r#abstract
            .as_ref()
            .map(|segments| extract_text(segments))
            .unwrap_or_default();

        if title.to_lowercase().contains(&query)
            || url.to_lowercase().contains(&query)
            || abstract_text.to_lowercase().contains(&query)
        {
            results.push(build_fallback_result(
                reference,
                &framework.metadata.platforms,
                "hierarchical",
            ));
            if results.len() >= max_results {
                break;
            }
        }
    }
    results
}

fn regex_fallback(
    framework: &FrameworkData,
    args: &Args,
    max_results: usize,
) -> Result<Vec<FallbackResult>> {
    if args.query.trim().is_empty() {
        return Ok(Vec::new());
    }

    let escaped = regex::escape(&args.query);
    let mut fuzzy_pattern = String::new();
    for (index, ch) in escaped.chars().enumerate() {
        if index > 0 {
            fuzzy_pattern.push_str(".*?");
        }
        fuzzy_pattern.push(ch);
    }
    let regex = Regex::new(&format!("(?i){}", fuzzy_pattern))?;

    let mut results = Vec::new();
    for reference in framework.references.values() {
        let title = reference.title.as_deref().unwrap_or("");
        let url = reference.url.as_deref().unwrap_or("");
        let abstract_text = reference
            .r#abstract
            .as_ref()
            .map(|segments| extract_text(segments))
            .unwrap_or_default();

        if regex.is_match(title) || regex.is_match(url) || regex.is_match(&abstract_text) {
            results.push(build_fallback_result(
                reference,
                &framework.metadata.platforms,
                "regex",
            ));
            if results.len() >= max_results {
                break;
            }
        }
    }

    Ok(results)
}

fn build_fallback_result(
    reference: &ReferenceData,
    default_platforms: &[PlatformInfo],
    found_via: &'static str,
) -> FallbackResult {
    let title = reference
        .title
        .clone()
        .unwrap_or_else(|| "Symbol".to_string());
    let description = reference
        .r#abstract
        .as_ref()
        .map(|segments| extract_text(segments))
        .unwrap_or_default();
    let platforms = reference
        .platforms
        .as_ref()
        .map(|platforms| format_platforms(platforms))
        .unwrap_or_else(|| format_platforms(default_platforms));
    let path = reference
        .url
        .clone()
        .unwrap_or_else(|| "(unknown path)".to_string());

    FallbackResult {
        title,
        path,
        description,
        platforms,
        found_via,
    }
}
