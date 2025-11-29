use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
    time::Instant,
};

use anyhow::{bail, Context, Result};
use apple_docs_client::types::{
    extract_text, format_platforms, FrameworkData, PlatformInfo, ReferenceData, Technology,
};
use once_cell::sync::Lazy;
use regex::Regex;
use serde::Deserialize;
use serde_json::json;

use crate::{
    markdown,
    services::{
        design_guidance, ensure_framework_index, ensure_global_framework_index, expand_identifiers,
        knowledge, load_active_framework,
    },
    state::{
        AppContext, FrameworkIndexEntry, SearchQueryLog, ToolDefinition, ToolHandler, ToolResponse,
    },
    tools::{parse_args, text_response, wrap_handler},
};
use futures::future;
use time::OffsetDateTime;
use tracing::{debug, warn};

const MAX_DESIGN_GUIDANCE_LOOKUPS: usize = 3;

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

#[derive(Clone)]
struct QueryConfig {
    raw: String,
    compact: String,
    terms: Vec<String>,
    synonyms: HashMap<String, Vec<String>>,
}

impl QueryConfig {
    fn term_count(&self) -> usize {
        self.terms.len()
    }

    fn synonyms_applied(&self) -> bool {
        self.synonyms.values().any(|values| !values.is_empty())
    }
}

#[derive(Clone)]
struct RankedEntry {
    score: i32,
    entry: FrameworkIndexEntry,
    matched_terms: usize,
    synonym_hits: usize,
    proximity_bonus: i32,
}

static QUERY_SYNONYMS: Lazy<HashMap<&'static str, Vec<&'static str>>> = Lazy::new(|| {
    HashMap::from([
        // List/Collection family
        ("list", vec!["table", "collection", "outline", "foreach", "lazyvstack"]),
        ("table", vec!["list", "grid", "datagrid"]),
        ("grid", vec!["collection", "layout", "lazygrid", "lazyhgrid", "lazyvgrid"]),
        ("collection", vec!["list", "grid", "foreach"]),
        ("foreach", vec!["list", "collection", "loop"]),

        // Text family
        ("text", vec!["label", "string", "typography", "font"]),
        ("label", vec!["text", "string", "caption"]),
        ("textfield", vec!["input", "formfield", "textinput", "edittext", "entry"]),
        ("field", vec!["input", "textfield", "entry"]),
        ("texteditor", vec!["textarea", "multiline", "textview"]),
        ("string", vec!["text", "label", "attributedstring"]),

        // Search family
        ("search", vec!["find", "lookup", "query", "searchable", "filter"]),
        ("searchable", vec!["search", "filter", "find"]),
        ("filter", vec!["search", "predicate", "query"]),

        // Navigation family
        ("toolbar", vec!["navigationbar", "actions", "bar", "menu"]),
        ("tab", vec!["segmented", "page", "tabview", "tabbar"]),
        ("tabview", vec!["tab", "page", "segmented"]),
        ("navigation", vec!["routing", "stack", "navigator", "navigationstack", "navigationview"]),
        ("navigationstack", vec!["navigation", "stack", "router"]),
        ("navigationsplitview", vec!["sidebar", "splitview", "master", "detail"]),
        ("sidebar", vec!["navigationsplitview", "menu", "drawer"]),

        // Button/Control family
        ("button", vec!["control", "action", "tap", "press", "click"]),
        ("toggle", vec!["switch", "checkbox", "boolean"]),
        ("switch", vec!["toggle", "checkbox"]),
        ("picker", vec!["dropdown", "select", "menu", "selection", "wheel"]),
        ("menu", vec!["picker", "dropdown", "contextmenu", "popover"]),
        ("slider", vec!["range", "progress", "scrubber"]),
        ("stepper", vec!["increment", "counter", "spinner"]),

        // Alert/Modal family
        ("alert", vec!["dialog", "notification", "popup", "message"]),
        ("sheet", vec!["modal", "presentation", "bottomsheet", "popover"]),
        ("modal", vec!["sheet", "presentation", "dialog", "fullscreen"]),
        ("popover", vec!["tooltip", "popup", "menu", "contextmenu"]),

        // Link/URL family
        ("link", vec!["url", "address", "href", "navigation"]),
        ("url", vec!["link", "address", "uri"]),

        // Image/Media family
        ("image", vec!["photo", "picture", "icon", "graphic", "asyncimage"]),
        ("asyncimage", vec!["image", "remote", "url", "photo"]),
        ("icon", vec!["symbol", "sfsymbol", "image", "glyph"]),
        ("symbol", vec!["icon", "sfsymbol", "glyph"]),

        // Layout family
        ("stack", vec!["vstack", "hstack", "zstack", "layout", "container"]),
        ("vstack", vec!["vertical", "column", "stack"]),
        ("hstack", vec!["horizontal", "row", "stack"]),
        ("zstack", vec!["overlay", "layer", "stack"]),
        ("spacer", vec!["padding", "gap", "margin"]),
        ("frame", vec!["size", "bounds", "dimension", "layout"]),
        ("padding", vec!["spacing", "margin", "inset"]),

        // State/Data family
        ("state", vec!["binding", "observable", "published", "data"]),
        ("binding", vec!["state", "twoway", "observable"]),
        ("observable", vec!["state", "published", "observed", "observableobject"]),
        ("environment", vec!["environmentobject", "injection", "dependency"]),

        // Animation family
        ("animation", vec!["transition", "animate", "motion", "effect"]),
        ("transition", vec!["animation", "enter", "exit", "appear"]),
        ("gesture", vec!["tap", "drag", "swipe", "touch", "interaction"]),

        // Form family
        ("form", vec!["settings", "preferences", "input", "section"]),
        ("section", vec!["group", "form", "container"]),

        // Color/Style family
        ("color", vec!["tint", "foreground", "background", "fill"]),
        ("style", vec!["modifier", "appearance", "theme"]),

        // View lifecycle
        ("onappear", vec!["viewdidappear", "load", "appear", "lifecycle"]),
        ("task", vec!["async", "await", "background", "concurrent"]),

        // Accessibility
        ("accessibility", vec!["a11y", "voiceover", "assistive", "label"]),
        ("voiceover", vec!["accessibility", "screenreader", "assistive"]),
    ])
});

/// Common abbreviations expanded to full terms for better matching
static ABBREVIATIONS: Lazy<HashMap<&'static str, &'static str>> = Lazy::new(|| {
    HashMap::from([
        // Common UI abbreviations
        ("nav", "navigation"),
        ("btn", "button"),
        ("img", "image"),
        ("txt", "text"),
        ("lbl", "label"),
        ("bg", "background"),
        ("fg", "foreground"),
        ("ctx", "context"),
        ("cfg", "config"),
        ("config", "configuration"),
        ("prefs", "preferences"),
        ("pref", "preference"),
        ("auth", "authentication"),
        ("async", "asynchronous"),
        ("sync", "synchronous"),
        ("init", "initialization"),
        ("deinit", "deinitialization"),
        ("func", "function"),
        ("fn", "function"),
        ("prop", "property"),
        ("attr", "attribute"),
        ("attrs", "attributes"),
        ("elem", "element"),
        ("elems", "elements"),
        ("comp", "component"),
        ("comps", "components"),
        ("param", "parameter"),
        ("params", "parameters"),
        ("arg", "argument"),
        ("args", "arguments"),
        ("val", "value"),
        ("vals", "values"),
        ("var", "variable"),
        ("vars", "variables"),
        ("obj", "object"),
        ("objs", "objects"),
        ("ref", "reference"),
        ("refs", "references"),
        ("ptr", "pointer"),
        ("str", "string"),
        ("int", "integer"),
        ("bool", "boolean"),
        ("num", "number"),
        ("idx", "index"),
        ("len", "length"),
        ("cnt", "count"),
        ("sz", "size"),
        ("min", "minimum"),
        ("max", "maximum"),
        ("avg", "average"),
        ("err", "error"),
        ("errs", "errors"),
        ("msg", "message"),
        ("msgs", "messages"),
        ("info", "information"),
        ("desc", "description"),
        ("docs", "documentation"),
        ("doc", "documentation"),
        ("spec", "specification"),
        ("specs", "specifications"),
        ("impl", "implementation"),
        ("util", "utility"),
        ("utils", "utilities"),
        ("lib", "library"),
        ("libs", "libraries"),
        ("pkg", "package"),
        ("pkgs", "packages"),
        ("mod", "module"),
        ("mods", "modules"),
        ("env", "environment"),
        ("src", "source"),
        ("dst", "destination"),
        ("dest", "destination"),
        ("tmp", "temporary"),
        ("temp", "temporary"),
        ("prev", "previous"),
        ("curr", "current"),
        ("nxt", "next"),
        // Apple/SwiftUI specific
        ("hstack", "horizontalstack"),
        ("vstack", "verticalstack"),
        ("zstack", "depthstack"),
        ("navstack", "navigationstack"),
        ("navsplit", "navigationsplitview"),
        ("tabbar", "tabview"),
        ("sb", "storyboard"),
        ("vc", "viewcontroller"),
        ("vm", "viewmodel"),
        ("ui", "userinterface"),
        ("ux", "userexperience"),
        ("sf", "sfsymbol"),
        ("a11y", "accessibility"),
        ("i18n", "internationalization"),
        ("l10n", "localization"),
    ])
});

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
    let query = prepare_query(&args.query);

    let mut ranked_matches =
        collect_matches(&index, &args, &query, Some(technology.title.as_str()));
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
            ranked_matches =
                collect_matches(&index, &args, &query, Some(technology.title.as_str()));
        }
    }

    let mut deduped_matches: Vec<RankedEntry> = Vec::new();
    if !ranked_matches.is_empty() {
        let mut seen_paths = HashSet::new();
        for ranked in ranked_matches {
            let path = ranked
                .entry
                .reference
                .url
                .clone()
                .unwrap_or_else(|| "(unknown path)".to_string());
            let title = ranked
                .entry
                .reference
                .title
                .clone()
                .unwrap_or_else(|| "Symbol".to_string());
            let key = dedup_key(&path, &title);
            if seen_paths.insert(key) {
                deduped_matches.push(ranked);
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

    let design_targets: Vec<FrameworkIndexEntry> = deduped_matches
        .iter()
        .take(MAX_DESIGN_GUIDANCE_LOOKUPS)
        .map(|ranked| ranked.entry.clone())
        .collect();
    let design_sections: HashMap<String, Vec<design_guidance::DesignSection>> =
        if design_targets.is_empty() {
            HashMap::new()
        } else {
            gather_design_guidance(&context, &design_targets, MAX_DESIGN_GUIDANCE_LOOKUPS).await
        };
    let mut knowledge_hits = 0usize;
    let mut design_hits = 0usize;
    let mut synonym_match_total = 0usize;
    let mut full_term_match_count = 0usize;
    let mut total_score = 0i32;
    let mut total_proximity_bonus = 0i32;
    let term_count = query.term_count();

    if deduped_matches.is_empty() {
        lines.push("No symbols matched those terms within this technology.".to_string());
        lines.push("Try broader keywords (e.g. \"tab\"), explore synonyms, or run `discover_technologies` again.".to_string());
        if !fallback.is_empty() {
            lines.push(String::new());
            lines.push(markdown::header(2, "Fallback suggestions"));
            lines.push(String::new());
            for result in &fallback {
                lines.push(format!(
                    "• **{}** — {}",
                    result.title.as_str(),
                    trim_with_ellipsis(&result.description, 120)
                ));
                lines.push(format!(
                    "  `get_documentation {{ \"path\": \"{}\" }}`",
                    result.path.as_str()
                ));
                lines.push(format!("  Platforms: {}", result.platforms));
                lines.push(format!("  Found via: {}", result.found_via));
                lines.push(String::new());
            }
        }
    } else {
        for ranked in &deduped_matches {
            total_score += ranked.score;
            synonym_match_total += ranked.synonym_hits;
            total_proximity_bonus += ranked.proximity_bonus;
            if term_count > 0 && ranked.matched_terms >= term_count {
                full_term_match_count += 1;
            }

            let entry = &ranked.entry;
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
                .as_deref();
            let (platform_label, availability) = classify_platforms(&path, platform_slice);
            let key = dedup_key(&path, &title);
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
                knowledge_hits += 1;
            }
            if let Some(sections) = design_sections.get(&key) {
                let mut highlights = Vec::new();
                for section in sections {
                    if let Some(bullet) = section.bullets.first() {
                        highlights.push(format!("{}: {}", bullet.category, bullet.text));
                    }
                }
                if !highlights.is_empty() {
                    let summary = highlights
                        .into_iter()
                        .take(2)
                        .collect::<Vec<_>>()
                        .join(" · ");
                    lines.push(format!("  Design checklist: {}", summary));
                }
                if let Some(section) = sections.first() {
                    lines.push(format!(
                        "  HIG reference: `get_documentation {{ \"path\": \"{}\" }}`",
                        section.slug
                    ));
                }
                if !sections.is_empty() {
                    design_hits += 1;
                }
            }
            lines.push(String::new());
        }
    }

    let metadata = json!({
        "scope": "technology",
        "query": args.query,
        "matchCount": match_count,
        "maxResults": max_results,
        "matchDensity": if max_results == 0 { 0.0 } else { match_count as f64 / max_results as f64 },
        "fallbackCount": fallback.len(),
        "designAnnotated": design_hits,
        "knowledgeAnnotated": knowledge_hits,
        "designSectionsFetched": design_sections.len(),
        "synonymsApplied": query.synonyms_applied(),
        "synonymMatches": synonym_match_total,
        "fullMatchCount": full_term_match_count,
        "avgScore": if match_count == 0 { 0.0 } else { total_score as f64 / match_count as f64 },
        "queryTerms": term_count,
        "proximityBonus": total_proximity_bonus,
    });
    log_search_query(
        &context,
        Some(technology.title.clone()),
        "technology",
        &query.raw,
        match_count,
    )
    .await;

    Ok(text_response(lines).with_metadata(metadata))
}

async fn search_all_technologies(context: Arc<AppContext>, args: Args) -> Result<ToolResponse> {
    let max_results = args.max_results.unwrap_or(20).max(1);
    let query = prepare_query(&args.query);

    let technologies = context.client.get_technologies().await?;
    let frameworks: Vec<Technology> = technologies
        .values()
        .filter(|tech| tech.kind == "symbol" && tech.role == "collection")
        .cloned()
        .collect();

    let mut aggregate = Vec::new();
    let mut skipped_frameworks = 0usize;
    for technology in &frameworks {
        // Gracefully handle framework loading errors - skip broken frameworks
        // instead of failing the entire search
        let index = match ensure_global_framework_index(&context, technology).await {
            Ok(idx) => idx,
            Err(e) => {
                warn!(
                    target: "search_symbols.global",
                    tech = %technology.title,
                    "Skipping framework due to load error: {e:#}"
                );
                skipped_frameworks += 1;
                continue;
            }
        };

        let mut matches = collect_matches(
            &index,
            &args,
            &query,
            Some(technology.title.as_str()),
        );
        matches.truncate(max_results);

        for ranked in matches {
            aggregate.push(GlobalMatch {
                score: ranked.score,
                entry: ranked.entry,
                technology_title: technology.title.clone(),
                technology_identifier: technology.identifier.clone(),
                matched_terms: ranked.matched_terms,
                synonym_hits: ranked.synonym_hits,
                proximity_bonus: ranked.proximity_bonus,
            });
        }
    }

    if skipped_frameworks > 0 {
        debug!(
            target: "search_symbols.global",
            skipped = skipped_frameworks,
            total = frameworks.len(),
            "Some frameworks were skipped due to load errors"
        );
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

    let design_targets: Vec<FrameworkIndexEntry> = matches
        .iter()
        .take(MAX_DESIGN_GUIDANCE_LOOKUPS)
        .map(|item| item.entry.clone())
        .collect();

    let design_sections: HashMap<String, Vec<design_guidance::DesignSection>> =
        if design_targets.is_empty() {
            HashMap::new()
        } else {
            gather_design_guidance(&context, &design_targets, MAX_DESIGN_GUIDANCE_LOOKUPS).await
        };
    let mut knowledge_hits = 0usize;
    let mut design_hits = 0usize;
    let mut synonym_match_total = 0usize;
    let mut full_term_match_count = 0usize;
    let mut total_score = 0i32;
    let mut total_proximity_bonus = 0i32;
    let term_count = query.term_count();

    if matches.is_empty() {
        lines.push("No symbols matched those terms across Apple documentation.".to_string());
        lines.push("Try alternative keywords or switch back to a specific technology.".to_string());
        let metadata = json!({
            "scope": "global",
            "query": args.query,
            "matchCount": 0,
            "maxResults": max_results,
            "matchDensity": 0.0,
            "designAnnotated": 0,
            "knowledgeAnnotated": 0,
            "technologiesScanned": frameworks.len(),
            "technologiesSkipped": skipped_frameworks,
            "synonymsApplied": query.synonyms_applied(),
            "synonymMatches": 0,
            "fullMatchCount": 0,
            "avgScore": 0.0,
            "queryTerms": term_count,
            "proximityBonus": 0,
        });
        return Ok(text_response(lines).with_metadata(metadata));
    }

    for matched in &matches {
        total_score += matched.score;
        synonym_match_total += matched.synonym_hits;
        total_proximity_bonus += matched.proximity_bonus;
        if term_count > 0 && matched.matched_terms >= term_count {
            full_term_match_count += 1;
        }

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
            .as_deref();
        let (platform_label, availability) = classify_platforms(&path, platform_slice);
        let key = dedup_key(&path, &title);

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
            knowledge_hits += 1;
        }
        if let Some(sections) = design_sections.get(&key) {
            let mut highlights = Vec::new();
            for section in sections {
                if let Some(bullet) = section.bullets.first() {
                    highlights.push(format!("{}: {}", bullet.category, bullet.text));
                }
            }
            if !highlights.is_empty() {
                let summary = highlights
                    .into_iter()
                    .take(2)
                    .collect::<Vec<_>>()
                    .join(" · ");
                lines.push(format!("  Design checklist: {}", summary));
            }
            if let Some(section) = sections.first() {
                lines.push(format!(
                    "  HIG reference: `get_documentation {{ \"path\": \"{}\" }}`",
                    section.slug
                ));
            }
            if !sections.is_empty() {
                design_hits += 1;
            }
        }
        lines.push(String::new());
    }

    let metadata = json!({
        "scope": "global",
        "query": args.query,
        "matchCount": matches.len(),
        "maxResults": max_results,
        "matchDensity": if max_results == 0 { 0.0 } else { matches.len() as f64 / max_results as f64 },
        "designAnnotated": design_hits,
        "knowledgeAnnotated": knowledge_hits,
        "designSectionsFetched": design_sections.len(),
        "technologiesScanned": frameworks.len(),
        "technologiesSkipped": skipped_frameworks,
        "synonymsApplied": query.synonyms_applied(),
        "synonymMatches": synonym_match_total,
        "fullMatchCount": full_term_match_count,
        "avgScore": if matches.is_empty() { 0.0 } else { total_score as f64 / matches.len() as f64 },
        "queryTerms": term_count,
        "proximityBonus": total_proximity_bonus,
    });
    log_search_query(&context, None, "global", &query.raw, matches.len()).await;

    Ok(text_response(lines).with_metadata(metadata))
}

fn collect_matches(
    entries: &[FrameworkIndexEntry],
    args: &Args,
    query: &QueryConfig,
    knowledge_tech: Option<&str>,
) -> Vec<RankedEntry> {
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

        if let Some(score) = score_entry(entry, query, knowledge_tech) {
            ranked.push(RankedEntry {
                score: score.score,
                entry: entry.clone(),
                matched_terms: score.matched_terms,
                synonym_hits: score.synonym_hits,
                proximity_bonus: score.proximity_bonus,
            });
        }
    }

    ranked.sort_by(|a, b| {
        b.score
            .cmp(&a.score)
            .then_with(|| a.entry.reference.title.cmp(&b.entry.reference.title))
    });
    ranked
}

struct MatchScore {
    score: i32,
    matched_terms: usize,
    synonym_hits: usize,
    proximity_bonus: i32,
}

/// Symbol kind priority - higher values rank better for general searches
fn symbol_kind_boost(kind: Option<&str>) -> i32 {
    match kind.map(|k| k.to_lowercase()).as_deref() {
        // Primary types - developers usually search for these first
        Some("struct") | Some("class") | Some("protocol") | Some("actor") => 6,
        // Views and important UI types
        Some("view") | Some("typealias") => 5,
        // Enums are often important for configuration
        Some("enum") | Some("enumeration") => 4,
        // Functions and methods
        Some("func") | Some("method") | Some("function") | Some("init") => 3,
        // Properties and variables
        Some("property") | Some("var") | Some("let") | Some("variable") => 2,
        // Type members
        Some("case") | Some("associatedtype") => 1,
        // Operators and extensions
        Some("op") | Some("operator") | Some("extension") => 0,
        // Unknown or other
        _ => 0,
    }
}

/// Boost for common/important UI symbols that are frequently searched
fn common_symbol_boost(title: &str) -> i32 {
    static COMMON_SYMBOLS: Lazy<HashMap<&'static str, i32>> = Lazy::new(|| {
        HashMap::from([
            // Core UI components
            ("button", 8),
            ("text", 8),
            ("image", 8),
            ("list", 8),
            ("view", 7),
            ("navigationstack", 7),
            ("tabview", 7),
            ("textfield", 7),
            ("label", 6),
            ("toggle", 6),
            ("picker", 6),
            ("slider", 6),
            ("form", 6),
            ("sheet", 6),
            ("alert", 6),
            ("menu", 6),
            ("link", 5),
            ("section", 5),
            ("spacer", 5),
            ("divider", 5),
            ("scrollview", 5),
            ("vstack", 5),
            ("hstack", 5),
            ("zstack", 5),
            ("lazyvstack", 5),
            ("lazyhstack", 5),
            ("grid", 5),
            ("asyncimage", 5),
            ("progressview", 5),
            ("color", 4),
            ("font", 4),
            ("gesture", 4),
        ])
    });

    let title_lower = title.to_lowercase();
    *COMMON_SYMBOLS.get(title_lower.as_str()).unwrap_or(&0)
}

/// Calculate edit distance between two strings (Levenshtein distance)
/// Returns None if distance exceeds max_distance for efficiency
fn edit_distance(a: &str, b: &str, max_distance: usize) -> Option<usize> {
    let a_len = a.len();
    let b_len = b.len();

    // Early exit if length difference exceeds max_distance
    if a_len.abs_diff(b_len) > max_distance {
        return None;
    }

    // For short strings, use exact matching
    if a_len == 0 {
        return if b_len <= max_distance { Some(b_len) } else { None };
    }
    if b_len == 0 {
        return if a_len <= max_distance { Some(a_len) } else { None };
    }

    let mut prev_row: Vec<usize> = (0..=b_len).collect();
    let mut curr_row: Vec<usize> = vec![0; b_len + 1];

    for (i, a_char) in a.chars().enumerate() {
        curr_row[0] = i + 1;
        let mut min_in_row = curr_row[0];

        for (j, b_char) in b.chars().enumerate() {
            let cost = if a_char == b_char { 0 } else { 1 };
            curr_row[j + 1] = (prev_row[j + 1] + 1)
                .min(curr_row[j] + 1)
                .min(prev_row[j] + cost);
            min_in_row = min_in_row.min(curr_row[j + 1]);
        }

        // Early exit if minimum in row exceeds max_distance
        if min_in_row > max_distance {
            return None;
        }

        std::mem::swap(&mut prev_row, &mut curr_row);
    }

    let distance = prev_row[b_len];
    if distance <= max_distance {
        Some(distance)
    } else {
        None
    }
}

/// Calculate proximity bonus based on matched token positions
/// Awards points when query terms appear close together in the symbol
fn calculate_proximity_bonus(positions: &[usize]) -> i32 {
    if positions.len() < 2 {
        return 0;
    }

    let mut sorted_positions = positions.to_vec();
    sorted_positions.sort_unstable();

    let mut total_bonus = 0;

    // Check consecutive pairs of matched positions
    for window in sorted_positions.windows(2) {
        let distance = window[1] - window[0];

        let bonus = match distance {
            1 => 5,      // Adjacent tokens: +5 points
            2 => 3,      // Within 2 tokens: +3 points
            3..=4 => 1,  // Within 4 tokens: +1 point
            _ => 0,
        };

        total_bonus += bonus;
    }

    total_bonus
}

fn score_entry(
    entry: &FrameworkIndexEntry,
    query: &QueryConfig,
    knowledge_tech: Option<&str>,
) -> Option<MatchScore> {
    let mut score = 0;
    let mut matched_terms = 0usize;
    let mut synonym_hits = 0usize;
    let mut matched_positions: Vec<usize> = Vec::new();

    let title_lower = entry
        .reference
        .title
        .as_deref()
        .unwrap_or_default()
        .to_lowercase();
    let id_lower = entry.id.to_lowercase();
    let url_lower = entry
        .reference
        .url
        .as_deref()
        .unwrap_or_default()
        .to_lowercase();

    // Very strong boost for exact title match - ensures exact matches appear first
    if title_lower == query.raw || title_lower == query.compact {
        score += 30;
        matched_terms = query.term_count();
        // Extra boost for primary types (struct, class, protocol) with exact match
        if matches!(
            entry.reference.kind.as_deref().map(|k| k.to_lowercase()).as_deref(),
            Some("struct") | Some("class") | Some("protocol") | Some("actor") | Some("enum")
        ) {
            score += 15;
        }
    }

    for term in &query.terms {
        let mut term_score = 0;
        let mut matched_position: Option<usize> = None;

        // Check for exact match
        for (idx, token) in entry.tokens.iter().enumerate() {
            if token == term {
                term_score = 6;
                matched_position = Some(idx);
                break;
            }
        }

        // Check for prefix match if no exact match
        if term_score == 0 {
            for (idx, token) in entry.tokens.iter().enumerate() {
                if token.starts_with(term) {
                    term_score = 4;
                    matched_position = Some(idx);
                    break;
                }
            }
        }

        // Check for contains match if still no match
        if term_score == 0 {
            for (idx, token) in entry.tokens.iter().enumerate() {
                if token.contains(term) {
                    term_score = 2;
                    matched_position = Some(idx);
                    break;
                }
            }
        }

        // Check synonyms if still no match
        if term_score == 0 {
            if let Some(synonyms) = query.synonyms.get(term) {
                let mut synonym_hit = false;
                for synonym in synonyms {
                    for (idx, token) in entry.tokens.iter().enumerate() {
                        if token == synonym {
                            term_score = 3;
                            matched_position = Some(idx);
                            synonym_hit = true;
                            break;
                        }
                    }
                    if synonym_hit {
                        break;
                    }
                    for (idx, token) in entry.tokens.iter().enumerate() {
                        if token.starts_with(synonym) {
                            term_score = 2;
                            matched_position = Some(idx);
                            synonym_hit = true;
                            break;
                        }
                    }
                    if synonym_hit {
                        break;
                    }
                    for (idx, token) in entry.tokens.iter().enumerate() {
                        if token.contains(synonym) {
                            term_score = 1;
                            matched_position = Some(idx);
                            synonym_hit = true;
                            break;
                        }
                    }
                    if synonym_hit {
                        break;
                    }
                }
                if synonym_hit {
                    synonym_hits += 1;
                }
            }
        }

        // Typo tolerance: if no match found, try edit distance on title
        if term_score == 0 && term.len() >= 3 {
            // Only for terms 3+ chars
            let max_typos = if term.len() <= 4 { 1 } else { 2 };
            for (idx, token) in entry.tokens.iter().enumerate() {
                if token.len() >= 3 {
                    if let Some(distance) = edit_distance(term, token, max_typos) {
                        // Score based on edit distance
                        term_score = match distance {
                            0 => 6, // Exact (shouldn't happen, already matched above)
                            1 => 3, // One typo
                            2 => 1, // Two typos
                            _ => 0,
                        };
                        if term_score > 0 {
                            matched_position = Some(idx);
                            break;
                        }
                    }
                }
            }
        }

        if term_score > 0 {
            matched_terms += 1;
            score += term_score;
            if let Some(pos) = matched_position {
                matched_positions.push(pos);
            }
        }
    }

    // Boost for title containing the full query phrase
    if !query.raw.is_empty() && title_lower.contains(&query.raw) {
        score += 5;
    }

    // Boost for title starting with the query
    if !query.raw.is_empty() && title_lower.starts_with(&query.raw) {
        score += 3;
    }

    if !query.compact.is_empty()
        && (id_lower.contains(&query.compact) || url_lower.contains(&query.compact))
    {
        score += 2;
    }

    // Knowledge base boost
    if let Some(tech) = knowledge_tech {
        if let Some(title) = entry.reference.title.as_deref() {
            if knowledge::lookup(tech, title).is_some() {
                score += 3;
            }
        }
    }

    // All terms matched bonus
    if matched_terms == query.term_count() && query.term_count() > 0 {
        score += 4;
    }

    // Symbol kind boost - promote types over properties
    if score > 0 {
        score += symbol_kind_boost(entry.reference.kind.as_deref());

        // Common symbol boost - prioritize frequently searched symbols
        if let Some(title) = entry.reference.title.as_deref() {
            score += common_symbol_boost(title);
        }
    }

    // Calculate proximity bonus based on matched token positions
    let proximity_bonus = calculate_proximity_bonus(&matched_positions);
    score += proximity_bonus;

    if score > 0 {
        Some(MatchScore {
            score,
            matched_terms,
            synonym_hits,
            proximity_bonus,
        })
    } else {
        None
    }
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
    matched_terms: usize,
    synonym_hits: usize,
    proximity_bonus: i32,
}

async fn gather_design_guidance(
    context: &Arc<AppContext>,
    entries: &[FrameworkIndexEntry],
    limit: usize,
) -> HashMap<String, Vec<design_guidance::DesignSection>> {
    let capped = limit.min(entries.len());
    if capped == 0 {
        return HashMap::new();
    }

    let mut tasks = Vec::with_capacity(capped);
    for entry in entries.iter().take(capped) {
        let path = match entry.reference.url.clone() {
            Some(value) if value != "(unknown path)" => value,
            _ => continue,
        };
        let title = entry
            .reference
            .title
            .clone()
            .unwrap_or_else(|| "Symbol".to_string());
        let key = dedup_key(&path, &title);
        let context = Arc::clone(context);

        tasks.push(async move {
            let started = Instant::now();
            let result = design_guidance::guidance_for(context.as_ref(), &title, &path).await;
            match &result {
                Ok(sections) => {
                    debug!(
                        target: "search_symbols.design_guidance",
                        path = %path,
                        ms = started.elapsed().as_millis(),
                        entries = sections.len(),
                        "loaded design guidance"
                    );
                }
                Err(error) => {
                    warn!(
                        target: "search_symbols.design_guidance",
                        path = %path,
                        ms = started.elapsed().as_millis(),
                        "failed to load design guidance: {error:#}"
                    );
                }
            }
            (key, result)
        });
    }

    let mut map = HashMap::new();
    for (key, result) in future::join_all(tasks).await {
        match result {
            Ok(sections) if !sections.is_empty() => {
                map.insert(key, sections);
            }
            _ => {}
        }
    }

    map
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

fn prepare_query(raw: &str) -> QueryConfig {
    let normalized = raw.trim().to_lowercase();
    let compact = normalized
        .chars()
        .filter(|c| !c.is_whitespace() && *c != '-')
        .collect::<String>();

    let mut terms = Vec::new();
    for token in raw
        .split(|c: char| {
            c.is_whitespace()
                || matches!(
                    c,
                    '/' | '.' | '_' | '-' | ':' | '(' | ')' | '[' | ']' | '{' | '}' | ','
                )
        })
        .filter(|token| !token.is_empty())
    {
        let term = token.to_lowercase();
        if !terms.contains(&term) {
            terms.push(term.clone());
        }

        // Expand abbreviations - add both the original and expanded form
        if let Some(expanded) = ABBREVIATIONS.get(term.as_str()) {
            let expanded_term = expanded.to_string();
            if !terms.contains(&expanded_term) {
                terms.push(expanded_term);
            }
        }
    }

    let mut synonyms = HashMap::new();
    for term in &terms {
        if let Some(values) = QUERY_SYNONYMS.get(term.as_str()) {
            let mapped = values
                .iter()
                .map(|value| value.to_string())
                .collect::<Vec<_>>();
            if !mapped.is_empty() {
                synonyms.insert(term.clone(), mapped);
            }
        }
    }

    QueryConfig {
        raw: normalized,
        compact,
        terms,
        synonyms,
    }
}
async fn log_search_query(
    context: &Arc<AppContext>,
    technology: Option<String>,
    scope: &str,
    query: &str,
    matches: usize,
) {
    let entry = SearchQueryLog {
        technology,
        scope: scope.to_string(),
        query: query.to_string(),
        matches,
        timestamp: Some(OffsetDateTime::now_utc()),
    };
    let mut log = context.state.recent_queries.lock().await;
    log.push(entry);
    const MAX_QUERIES: usize = 50;
    if log.len() > MAX_QUERIES {
        let overflow = log.len() - MAX_QUERIES;
        log.drain(0..overflow);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_proximity_bonus_adjacent_tokens() {
        // Adjacent tokens (distance = 1) should get +5 bonus
        let positions = vec![0, 1];
        let bonus = calculate_proximity_bonus(&positions);
        assert_eq!(bonus, 5, "Adjacent tokens should get +5 bonus");
    }

    #[test]
    fn test_proximity_bonus_two_apart() {
        // Tokens 2 apart (distance = 2) should get +3 bonus
        let positions = vec![0, 2];
        let bonus = calculate_proximity_bonus(&positions);
        assert_eq!(bonus, 3, "Tokens 2 apart should get +3 bonus");
    }

    #[test]
    fn test_proximity_bonus_three_apart() {
        // Tokens 3 apart (distance = 3) should get +1 bonus
        let positions = vec![0, 3];
        let bonus = calculate_proximity_bonus(&positions);
        assert_eq!(bonus, 1, "Tokens 3 apart should get +1 bonus");
    }

    #[test]
    fn test_proximity_bonus_four_apart() {
        // Tokens 4 apart (distance = 4) should get +1 bonus
        let positions = vec![0, 4];
        let bonus = calculate_proximity_bonus(&positions);
        assert_eq!(bonus, 1, "Tokens 4 apart should get +1 bonus");
    }

    #[test]
    fn test_proximity_bonus_scattered_tokens() {
        // Tokens far apart (distance > 4) should get no bonus
        let positions = vec![0, 10];
        let bonus = calculate_proximity_bonus(&positions);
        assert_eq!(bonus, 0, "Scattered tokens should get no bonus");
    }

    #[test]
    fn test_proximity_bonus_multiple_pairs() {
        // Multiple consecutive pairs should accumulate bonuses
        // Positions: [0, 1, 3] -> pairs: (0,1)=+5, (1,3)=+3 = +8 total
        let positions = vec![0, 1, 3];
        let bonus = calculate_proximity_bonus(&positions);
        assert_eq!(bonus, 8, "Multiple pairs should accumulate bonuses");
    }

    #[test]
    fn test_proximity_bonus_all_adjacent() {
        // All adjacent tokens: [0, 1, 2, 3] -> pairs: (0,1)=+5, (1,2)=+5, (2,3)=+5 = +15 total
        let positions = vec![0, 1, 2, 3];
        let bonus = calculate_proximity_bonus(&positions);
        assert_eq!(bonus, 15, "All adjacent tokens should maximize bonus");
    }

    #[test]
    fn test_proximity_bonus_single_token() {
        // Single token should get no bonus (no pairs)
        let positions = vec![5];
        let bonus = calculate_proximity_bonus(&positions);
        assert_eq!(bonus, 0, "Single token should get no bonus");
    }

    #[test]
    fn test_proximity_bonus_empty_positions() {
        // Empty positions should get no bonus
        let positions = vec![];
        let bonus = calculate_proximity_bonus(&positions);
        assert_eq!(bonus, 0, "Empty positions should get no bonus");
    }

    #[test]
    fn test_proximity_bonus_unsorted_positions() {
        // Function should handle unsorted positions correctly
        // [5, 1, 3] -> sorted [1, 3, 5] -> pairs: (1,3)=+3, (3,5)=+3 = +6 total
        let positions = vec![5, 1, 3];
        let bonus = calculate_proximity_bonus(&positions);
        assert_eq!(bonus, 6, "Unsorted positions should be handled correctly");
    }

    #[test]
    fn test_edit_distance_exact_match() {
        // Exact match should return 0
        let distance = edit_distance("hello", "hello", 2);
        assert_eq!(distance, Some(0), "Exact match should have distance 0");
    }

    #[test]
    fn test_edit_distance_one_char_difference() {
        // One character difference
        let distance = edit_distance("hello", "hallo", 2);
        assert_eq!(distance, Some(1), "One char difference should have distance 1");
    }

    #[test]
    fn test_edit_distance_two_chars_difference() {
        // Two character differences
        let distance = edit_distance("hello", "hxllx", 2);
        assert_eq!(distance, Some(2), "Two char difference should have distance 2");
    }

    #[test]
    fn test_edit_distance_exceeds_max() {
        // Distance exceeds max_distance, should return None
        let distance = edit_distance("hello", "world", 2);
        assert_eq!(distance, None, "Distance exceeding max should return None");
    }

    #[test]
    fn test_edit_distance_insertion() {
        // Insertion: "hello" -> "helllo" (distance = 1)
        let distance = edit_distance("hello", "helllo", 2);
        assert_eq!(distance, Some(1), "Insertion should have distance 1");
    }

    #[test]
    fn test_edit_distance_deletion() {
        // Deletion: "hello" -> "helo" (distance = 1)
        let distance = edit_distance("hello", "helo", 2);
        assert_eq!(distance, Some(1), "Deletion should have distance 1");
    }

    #[test]
    fn test_edit_distance_empty_strings() {
        // Empty string comparisons
        let distance = edit_distance("", "", 2);
        assert_eq!(distance, Some(0), "Two empty strings should have distance 0");

        let distance = edit_distance("abc", "", 5);
        assert_eq!(distance, Some(3), "Empty vs non-empty should equal length");

        let distance = edit_distance("", "abc", 5);
        assert_eq!(distance, Some(3), "Non-empty vs empty should equal length");
    }

    #[test]
    fn test_edit_distance_length_diff_exceeds_max() {
        // If length difference alone exceeds max_distance, should return None early
        let distance = edit_distance("a", "abcdef", 2);
        assert_eq!(distance, None, "Large length difference should return None");
    }

    #[test]
    fn test_prepare_query_normalization() {
        let query = prepare_query("  Hello World  ");
        assert_eq!(query.raw, "hello world");
        assert_eq!(query.compact, "helloworld");
        assert!(query.terms.contains(&"hello".to_string()));
        assert!(query.terms.contains(&"world".to_string()));
    }

    #[test]
    fn test_prepare_query_removes_punctuation() {
        let query = prepare_query("navigation-stack/view");
        assert!(query.terms.contains(&"navigation".to_string()));
        assert!(query.terms.contains(&"stack".to_string()));
        assert!(query.terms.contains(&"view".to_string()));
    }

    #[test]
    fn test_prepare_query_expands_abbreviations() {
        let query = prepare_query("nav btn");
        // Should have both original and expanded forms
        assert!(query.terms.contains(&"nav".to_string()));
        assert!(query.terms.contains(&"navigation".to_string()));
        assert!(query.terms.contains(&"btn".to_string()));
        assert!(query.terms.contains(&"button".to_string()));
    }

    #[test]
    fn test_prepare_query_synonym_expansion() {
        let query = prepare_query("list");
        assert!(query.synonyms.contains_key("list"));
        let synonyms = query.synonyms.get("list").unwrap();
        assert!(synonyms.iter().any(|s| s == "table"));
        assert!(synonyms.iter().any(|s| s == "collection"));
    }

    #[test]
    fn test_symbol_kind_boost_struct() {
        assert_eq!(symbol_kind_boost(Some("struct")), 6);
        assert_eq!(symbol_kind_boost(Some("class")), 6);
        assert_eq!(symbol_kind_boost(Some("protocol")), 6);
        assert_eq!(symbol_kind_boost(Some("actor")), 6);
    }

    #[test]
    fn test_symbol_kind_boost_view() {
        assert_eq!(symbol_kind_boost(Some("view")), 5);
        assert_eq!(symbol_kind_boost(Some("typealias")), 5);
    }

    #[test]
    fn test_symbol_kind_boost_enum() {
        assert_eq!(symbol_kind_boost(Some("enum")), 4);
        assert_eq!(symbol_kind_boost(Some("enumeration")), 4);
    }

    #[test]
    fn test_symbol_kind_boost_function() {
        assert_eq!(symbol_kind_boost(Some("func")), 3);
        assert_eq!(symbol_kind_boost(Some("method")), 3);
        assert_eq!(symbol_kind_boost(Some("function")), 3);
    }

    #[test]
    fn test_symbol_kind_boost_unknown() {
        assert_eq!(symbol_kind_boost(Some("unknown")), 0);
        assert_eq!(symbol_kind_boost(None), 0);
    }

    #[test]
    fn test_common_symbol_boost_high_priority() {
        assert_eq!(common_symbol_boost("Button"), 8);
        assert_eq!(common_symbol_boost("Text"), 8);
        assert_eq!(common_symbol_boost("List"), 8);
        assert_eq!(common_symbol_boost("Image"), 8);
    }

    #[test]
    fn test_common_symbol_boost_medium_priority() {
        assert_eq!(common_symbol_boost("NavigationStack"), 7);
        assert_eq!(common_symbol_boost("TabView"), 7);
        assert_eq!(common_symbol_boost("TextField"), 7);
    }

    #[test]
    fn test_common_symbol_boost_low_priority() {
        assert_eq!(common_symbol_boost("Color"), 4);
        assert_eq!(common_symbol_boost("Font"), 4);
    }

    #[test]
    fn test_common_symbol_boost_case_insensitive() {
        assert_eq!(common_symbol_boost("button"), 8);
        assert_eq!(common_symbol_boost("BUTTON"), 8);
        assert_eq!(common_symbol_boost("BuTtOn"), 8);
    }

    #[test]
    fn test_common_symbol_boost_unknown() {
        assert_eq!(common_symbol_boost("UnknownSymbol"), 0);
        assert_eq!(common_symbol_boost("CustomView"), 0);
    }

    #[test]
    fn test_trim_with_ellipsis_short_text() {
        let result = trim_with_ellipsis("short", 100);
        assert_eq!(result, "short");
    }

    #[test]
    fn test_trim_with_ellipsis_long_text() {
        let text = "This is a very long text that should be trimmed";
        let result = trim_with_ellipsis(text, 20);
        assert_eq!(result, "This is a very long ...");
        assert_eq!(result.len(), 23); // 20 chars + "..."
    }

    #[test]
    fn test_trim_with_ellipsis_exact_length() {
        let text = "exactly twenty chars";
        let result = trim_with_ellipsis(text, 20);
        assert_eq!(result, "exactly twenty chars");
    }

    #[test]
    fn test_dedup_key_normal_path() {
        let key = dedup_key("/documentation/swiftui/button", "Button");
        assert_eq!(key, "/documentation/swiftui/button");
    }

    #[test]
    fn test_dedup_key_unknown_path() {
        let key = dedup_key("(unknown path)", "Button");
        assert_eq!(key, "unknown::button");
    }

    #[test]
    fn test_dedup_key_case_normalization() {
        let key1 = dedup_key("/Documentation/SwiftUI/Button", "Button");
        let key2 = dedup_key("/documentation/swiftui/button", "Button");
        assert_eq!(key1, key2, "Dedup keys should be case-normalized");
    }

    #[test]
    fn test_is_design_material() {
        assert!(is_design_material("/design/human-interface-guidelines/buttons"));
        assert!(is_design_material("/documentation/design/components"));
        assert!(!is_design_material("/documentation/swiftui/button"));
        assert!(!is_design_material("/tutorials/swiftui"));
    }
}
