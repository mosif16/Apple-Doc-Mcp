use anyhow::{Context, Result};
use apple_docs_client::types::{FrameworkData, ReferenceData, SymbolData, Technology};

use crate::state::{AppContext, FrameworkIndexEntry};

pub mod design_guidance;
pub mod knowledge;

pub async fn load_active_framework(context: &AppContext) -> Result<FrameworkData> {
    let maybe_cached = context.state.framework_cache.read().await.clone();
    if let Some(cached) = maybe_cached {
        return Ok(cached);
    }

    let technology = context
        .state
        .active_technology
        .read()
        .await
        .clone()
        .context(
            "No technology selected. Call discover_technologies then choose_technology first.",
        )?;

    let identifier = technology
        .identifier
        .split('/')
        .last()
        .context("Invalid technology identifier")?;
    let data = context
        .client
        .get_framework(identifier)
        .await
        .context("Failed to load framework data")?;

    *context.state.framework_cache.write().await = Some(data.clone());
    context.state.framework_index.write().await.take();

    Ok(data)
}

pub async fn ensure_framework_index(context: &AppContext) -> Result<Vec<FrameworkIndexEntry>> {
    if let Some(index) = context.state.framework_index.read().await.clone() {
        return Ok(index);
    }

    let framework = load_active_framework(context).await?;
    let entries = build_framework_index(&framework);

    *context.state.framework_index.write().await = Some(entries.clone());
    Ok(entries)
}

pub async fn ensure_global_framework_index(
    context: &AppContext,
    technology: &Technology,
) -> Result<Vec<FrameworkIndexEntry>> {
    if let Some(index) = context
        .state
        .global_indexes
        .read()
        .await
        .get(&technology.identifier)
        .cloned()
    {
        return Ok(index);
    }

    let identifier = technology
        .identifier
        .split('/')
        .last()
        .context("Invalid technology identifier")?;
    let framework = context
        .client
        .get_framework(identifier)
        .await
        .with_context(|| format!("Failed to load framework data for {}", technology.title))?;

    let entries = build_framework_index(&framework);
    context
        .state
        .global_indexes
        .write()
        .await
        .insert(technology.identifier.clone(), entries.clone());

    Ok(entries)
}

fn build_framework_index(framework: &FrameworkData) -> Vec<FrameworkIndexEntry> {
    let mut entries = Vec::with_capacity(framework.references.len());
    for (id, reference) in framework.references.iter() {
        entries.push(build_entry(id, reference));
    }
    entries
}

fn build_entry(id: &str, reference: &ReferenceData) -> FrameworkIndexEntry {
    let mut tokens = Vec::new();
    tokenize_into(reference.title.as_deref().unwrap_or_default(), &mut tokens);
    tokenize_into(id, &mut tokens);

    let mut normalized_reference = reference.clone();
    if let Some(url) = &normalized_reference.url {
        let normalized = normalize_reference_link(url);
        if normalized.is_empty() {
            normalized_reference.url = derive_path_from_identifier(id);
        } else {
            tokenize_into(&normalized, &mut tokens);
            normalized_reference.url = Some(normalized);
        }
    } else if let Some(normalized) = derive_path_from_identifier(id) {
        tokenize_into(&normalized, &mut tokens);
        normalized_reference.url = Some(normalized);
    }

    if let Some(abstract_text) = &reference.r#abstract {
        let text: String = abstract_text
            .iter()
            .filter_map(|segment| segment.text.as_deref())
            .collect();
        tokenize_into(&text, &mut tokens);
    }

    FrameworkIndexEntry {
        id: id.to_string(),
        tokens,
        reference: normalized_reference,
    }
}

fn build_symbol_entry(identifier: &str, symbol: &SymbolData) -> FrameworkIndexEntry {
    let mut tokens = Vec::new();
    if let Some(title) = &symbol.metadata.title {
        tokenize_into(title, &mut tokens);
    }
    tokenize_into(identifier, &mut tokens);
    let normalized_path = normalize_reference_link(identifier);
    if !normalized_path.is_empty() {
        tokenize_into(&normalized_path, &mut tokens);
    }
    FrameworkIndexEntry {
        id: identifier.to_string(),
        tokens,
        reference: ReferenceData {
            title: symbol.metadata.title.clone(),
            kind: symbol.metadata.symbol_kind.clone(),
            r#abstract: Some(symbol.r#abstract.clone()),
            platforms: Some(symbol.metadata.platforms.clone()),
            url: if normalized_path.is_empty() {
                None
            } else {
                Some(normalized_path)
            },
        },
    }
}

fn tokenize_into(value: &str, tokens: &mut Vec<String>) {
    for token in value
        .split(|c: char| {
            c.is_whitespace()
                || matches!(
                    c,
                    '/' | '.' | '_' | '-' | '(' | ')' | ':' | ';' | ',' | '[' | ']' | '{' | '}'
                )
        })
        .filter(|token| !token.is_empty())
    {
        insert_token(tokens, token);
        for piece in split_camel_case(token) {
            insert_token(tokens, &piece);
        }
    }
}

fn insert_token(tokens: &mut Vec<String>, token: &str) {
    if token.is_empty() {
        return;
    }
    let lower = token.to_lowercase();
    if !tokens.contains(&lower) {
        tokens.push(lower);
    }
}

fn split_camel_case(token: &str) -> Vec<String> {
    if token.chars().all(|c| !c.is_alphabetic()) {
        return Vec::new();
    }

    let chars: Vec<char> = token.chars().collect();
    let mut pieces = Vec::new();
    let mut start = 0;

    for i in 1..chars.len() {
        let current = chars[i];
        let previous = chars[i - 1];
        let next_is_lowercase = chars
            .get(i + 1)
            .map(|next| next.is_lowercase())
            .unwrap_or(false);

        let boundary = (previous.is_lowercase() && current.is_uppercase())
            || (previous.is_uppercase() && current.is_uppercase() && next_is_lowercase);

        if boundary {
            if let Some(slice) = token.get(start..i) {
                if !slice.is_empty() {
                    pieces.push(slice.to_string());
                }
            }
            start = i;
        }
    }

    if let Some(slice) = token.get(start..) {
        if !slice.is_empty() {
            pieces.push(slice.to_string());
        }
    }

    pieces
}

fn normalize_reference_link(input: &str) -> String {
    let trimmed = input.trim();
    let without_doc = trimmed
        .strip_prefix("doc://com.apple.documentation/")
        .or_else(|| trimmed.strip_prefix("doc://com.apple.SwiftUI/"))
        .or_else(|| trimmed.strip_prefix("doc://com.apple.HIG/"))
        .unwrap_or(trimmed);
    let without_leading_slash = without_doc.trim_start_matches('/');

    if without_leading_slash.starts_with("design/")
        || without_leading_slash.starts_with("human-interface-guidelines/")
        || without_leading_slash.starts_with("documentation/")
    {
        without_leading_slash.to_string()
    } else if without_leading_slash.is_empty() {
        String::new()
    } else {
        format!("documentation/{}", without_leading_slash)
    }
}

fn derive_path_from_identifier(identifier: &str) -> Option<String> {
    let normalized = normalize_reference_link(identifier);
    if normalized.is_empty() {
        None
    } else {
        Some(normalized)
    }
}

pub async fn expand_identifiers(
    context: &AppContext,
    identifiers: &[String],
) -> Result<Vec<FrameworkIndexEntry>> {
    let mut needed = Vec::new();
    {
        let mut expanded = context.state.expanded_identifiers.lock().await;
        for identifier in identifiers {
            if expanded.insert(identifier.clone()) {
                needed.push(identifier.clone());
            }
        }
    }

    if needed.is_empty() {
        return ensure_framework_index(context).await;
    }

    for identifier in needed {
        let normalized = identifier
            .trim()
            .strip_prefix("doc://com.apple.documentation/")
            .or_else(|| identifier.trim().strip_prefix("doc://com.apple.SwiftUI/"))
            .unwrap_or(identifier.trim())
            .trim_start_matches('/');

        if !normalized.starts_with("documentation/") {
            continue;
        }

        let path = normalized.to_string();

        let symbol: SymbolData = context
            .client
            .get_symbol(&path)
            .await
            .with_context(|| format!("Failed to expand identifier {path}"))?;

        let mut index_guard = context.state.framework_index.write().await;
        let entries = index_guard.get_or_insert_with(Vec::new);
        entries.push(build_symbol_entry(&identifier, &symbol));
        for (id, reference) in symbol.references.iter() {
            entries.push(build_entry(id, reference));
        }
    }

    Ok(context
        .state
        .framework_index
        .read()
        .await
        .clone()
        .unwrap_or_default())
}
