use std::{collections::HashSet, sync::Arc};

use anyhow::{Context, Result};
use apple_docs_client::types::Technology;
use serde::Deserialize;
use serde_json::json;

use crate::{
    markdown,
    services::design_guidance,
    state::{AppContext, ToolDefinition, ToolHandler, ToolResponse},
    tools::{parse_args, text_response, wrap_handler},
};

#[derive(Debug, Deserialize)]
struct Args {
    identifier: Option<String>,
    name: Option<String>,
    technology: Option<String>,
}

pub fn definition() -> (ToolDefinition, ToolHandler) {
    (
        ToolDefinition {
            name: "choose_technology".to_string(),
            description: "Select the framework/technology to scope all subsequent searches"
                .to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "identifier": {
                        "type": "string",
                        "description": "Technology identifier (doc://...)"
                    },
                    "name": {
                        "type": "string",
                        "description": "Technology title (e.g. SwiftUI)"
                    },
                    "technology": {
                        "type": "string",
                        "description": "Technology identifier, slug, or name"
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
    let technologies = context
        .client
        .get_technologies()
        .await
        .context("Failed to load technologies")?;

    let candidates: Vec<Technology> = technologies
        .values()
        .cloned()
        .filter(|tech| tech.kind == "symbol" && tech.role == "collection")
        .collect();

    let input_identifier = args.identifier.clone();
    let input_name = args.name.clone();
    let input_technology = args.technology.clone();

    let resolution = resolve_candidate(&candidates, &args);

    let (technology, strategy) = match resolution {
        Resolution::Match {
            technology,
            strategy,
        } => (technology, strategy),
        Resolution::Ambiguous {
            search_term,
            candidates,
        } => {
            context.state.active_technology.write().await.take();
            let metadata = json!({
                "resolved": false,
                "ambiguous": true,
                "inputIdentifier": input_identifier,
                "inputName": input_name,
                "inputTechnology": input_technology,
                "searchTerm": search_term,
                "candidateCount": candidates.len(),
                "candidates": candidates
                    .iter()
                    .map(|tech| json!({
                        "identifier": tech.identifier,
                        "name": tech.title,
                    }))
                    .collect::<Vec<_>>(),
            });

            let lines = build_ambiguous(&search_term, &candidates);
            return Ok(text_response(lines).with_metadata(metadata));
        }
        Resolution::NotFound(not_found) => {
            context.state.active_technology.write().await.take();
            let metadata = json!({
                "resolved": false,
                "ambiguous": false,
                "inputIdentifier": input_identifier,
                "inputName": input_name,
                "inputTechnology": input_technology,
                "searchTerm": not_found.search_term,
                "suggestions": not_found.suggestion_count,
            });
            return Ok(text_response(not_found.lines).with_metadata(metadata));
        }
    };

    *context.state.active_technology.write().await = Some(technology.clone());
    context.state.framework_cache.write().await.take();
    context.state.framework_index.write().await.take();
    context.state.expanded_identifiers.lock().await.clear();

    let has_design_mapping = design_guidance::has_primer_mapping(&technology);
    let lines = vec![
        markdown::header(1, "✅ Technology Selected"),
        String::new(),
        markdown::bold("Name", &technology.title),
        markdown::bold("Identifier", &technology.identifier),
        String::new(),
        markdown::header(2, "Next actions"),
        "• `search_symbols { \"query\": \"keyword\" }` — fuzzy search within this framework"
            .to_string(),
        "• `get_documentation { \"path\": \"SymbolName\" }` — open a symbol page".to_string(),
        "• `discover_technologies` — pick another framework".to_string(),
    ];

    let metadata = json!({
        "resolved": true,
        "identifier": technology.identifier,
        "name": technology.title,
        "designPrimersAvailable": has_design_mapping,
        "matchStrategy": strategy.as_str(),
    });

    Ok(text_response(lines).with_metadata(metadata))
}

fn resolve_candidate(candidates: &[Technology], args: &Args) -> Resolution {
    let queries = gather_queries(args);
    if queries.is_empty() {
        return Resolution::NotFound(build_not_found(candidates, args));
    }

    let identifier_matches: Vec<_> = candidates
        .iter()
        .filter_map(|tech| {
            queries.iter().find_map(|query| {
                if tech.identifier.eq_ignore_ascii_case(query) {
                    Some((MatchStrategy::Identifier, tech.clone()))
                } else if tech
                    .identifier
                    .rsplit('/')
                    .next()
                    .map(|slug| slug.eq_ignore_ascii_case(query))
                    .unwrap_or(false)
                {
                    Some((MatchStrategy::Slug, tech.clone()))
                } else {
                    None
                }
            })
        })
        .collect();

    if let Some(resolution) = finalize_matches(identifier_matches, &queries) {
        return resolution;
    }

    let title_matches: Vec<_> = candidates
        .iter()
        .filter_map(|tech| {
            queries
                .iter()
                .find(|query| tech.title.eq_ignore_ascii_case(query))
                .map(|_| (MatchStrategy::Title, tech.clone()))
        })
        .collect();

    if let Some(resolution) = finalize_matches(title_matches, &queries) {
        return resolution;
    }

    let mut scored = Vec::new();
    for tech in candidates {
        let mut best = u32::MAX;
        for query in &queries {
            let score = fuzzy_score(&tech.title, query);
            best = best.min(score);
        }
        if best != u32::MAX {
            scored.push((best, tech.clone()));
        }
    }

    scored.sort_by(|(left_score, left_tech), (right_score, right_tech)| {
        left_score.cmp(right_score).then_with(|| {
            left_tech
                .title
                .to_lowercase()
                .cmp(&right_tech.title.to_lowercase())
        })
    });

    if let Some((best_score, best_tech)) = scored.first().cloned() {
        let shared = scored
            .iter()
            .take_while(|(score, _)| *score == best_score)
            .count();

        if best_score <= 2 && shared == 1 {
            return Resolution::Match {
                technology: best_tech,
                strategy: MatchStrategy::Fuzzy,
            };
        }

        let search_term = queries.first().cloned().unwrap_or_default();
        let limit = scored.len().min(5);
        let suggestions = scored
            .into_iter()
            .take(limit)
            .map(|(_, tech)| tech)
            .collect();
        return Resolution::Ambiguous {
            search_term,
            candidates: suggestions,
        };
    }

    Resolution::NotFound(build_not_found(candidates, args))
}

fn fuzzy_score(candidate: &str, target: &str) -> u32 {
    if target.is_empty() {
        return u32::MAX;
    }

    let candidate_lower = candidate.to_lowercase();
    let target_lower = target.to_lowercase();

    if candidate_lower == target_lower {
        0
    } else if candidate_lower.starts_with(&target_lower)
        || target_lower.starts_with(&candidate_lower)
    {
        1
    } else if candidate_lower.contains(&target_lower) || target_lower.contains(&candidate_lower) {
        2
    } else {
        3
    }
}

struct NotFoundDetails {
    lines: Vec<String>,
    search_term: String,
    suggestion_count: usize,
}

fn build_not_found(candidates: &[Technology], args: &Args) -> NotFoundDetails {
    let search_term = args
        .identifier
        .as_ref()
        .or(args.name.as_ref())
        .or(args.technology.as_ref())
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .unwrap_or("unknown");

    let suggestions_list: Vec<String> = candidates
        .iter()
        .filter(|tech| {
            tech.title
                .to_lowercase()
                .contains(&search_term.to_lowercase())
        })
        .take(5)
        .map(|tech| {
            format!(
                "• {} — `choose_technology {{ \"identifier\": \"{}\" }}`",
                tech.title, tech.identifier
            )
        })
        .collect::<Vec<_>>();
    let suggestion_count = suggestions_list.len();

    let mut lines = vec![
        markdown::header(1, "❌ Technology Not Found"),
        format!("Could not resolve \"{}\".", search_term),
        String::new(),
        markdown::header(2, "Suggestions"),
    ];

    if suggestions_list.is_empty() {
        lines.push(
            "• Use `discover_technologies { \"query\": \"keyword\" }` to find candidates"
                .to_string(),
        );
    } else {
        lines.extend(suggestions_list.iter().cloned());
    }

    NotFoundDetails {
        lines,
        search_term: search_term.to_string(),
        suggestion_count,
    }
}

#[derive(Clone, Copy)]
enum MatchStrategy {
    Identifier,
    Slug,
    Title,
    Fuzzy,
}

impl MatchStrategy {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Identifier => "identifier",
            Self::Slug => "slug",
            Self::Title => "title",
            Self::Fuzzy => "fuzzy",
        }
    }
}

enum Resolution {
    Match {
        technology: Technology,
        strategy: MatchStrategy,
    },
    Ambiguous {
        search_term: String,
        candidates: Vec<Technology>,
    },
    NotFound(NotFoundDetails),
}

fn finalize_matches(
    matches: Vec<(MatchStrategy, Technology)>,
    queries: &[String],
) -> Option<Resolution> {
    if matches.is_empty() {
        return None;
    }

    let mut seen = HashSet::new();
    let mut unique = Vec::new();
    for (strategy, technology) in matches {
        if seen.insert(technology.identifier.clone()) {
            unique.push((strategy, technology));
        }
    }

    if unique.is_empty() {
        return None;
    }

    if unique.len() == 1 {
        let (strategy, technology) = unique.into_iter().next().expect("single match");
        return Some(Resolution::Match {
            technology,
            strategy,
        });
    }

    let search_term = queries.first().cloned().unwrap_or_default();
    let candidates = unique
        .into_iter()
        .map(|(_, technology)| technology)
        .collect();
    Some(Resolution::Ambiguous {
        search_term,
        candidates,
    })
}

fn gather_queries(args: &Args) -> Vec<String> {
    let mut queries = Vec::new();
    let mut seen = HashSet::new();
    if let Some(identifier) = args.identifier.as_ref() {
        push_if_present(&mut queries, &mut seen, identifier);
    }
    if let Some(technology) = args.technology.as_ref() {
        push_if_present(&mut queries, &mut seen, technology);
    }
    if let Some(name) = args.name.as_ref() {
        push_if_present(&mut queries, &mut seen, name);
    }
    queries
}

fn push_if_present(queries: &mut Vec<String>, seen: &mut HashSet<String>, value: &str) {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return;
    }
    let lower = trimmed.to_lowercase();
    if seen.insert(lower) {
        queries.push(trimmed.to_string());
    }
}

fn build_ambiguous(search_term: &str, candidates: &[Technology]) -> Vec<String> {
    let mut lines = vec![
        markdown::header(1, "⚠️ Multiple Technologies Matched"),
        format!(
            "`{}` matches more than one technology. Choose one by identifier to proceed.",
            search_term
        ),
        String::new(),
        markdown::header(2, "Candidates"),
    ];

    for tech in candidates {
        lines.push(format!(
            "• {} — `choose_technology {{ \"identifier\": \"{}\" }}`",
            tech.title, tech.identifier
        ));
    }

    if candidates.is_empty() {
        lines.push("No overlapping technologies were returned.".to_string());
    }

    lines
}
