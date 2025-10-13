use anyhow::{Context, Result};
use apple_docs_client::types::{FrameworkData, ReferenceData, SymbolData, Technology};

use crate::state::{AppContext, FrameworkIndexEntry};

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
    if let Some(url) = &reference.url {
        tokenize_into(url, &mut tokens);
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
        reference: reference.clone(),
    }
}

fn build_symbol_entry(identifier: &str, symbol: &SymbolData) -> FrameworkIndexEntry {
    let mut tokens = Vec::new();
    if let Some(title) = &symbol.metadata.title {
        tokenize_into(title, &mut tokens);
    }
    tokenize_into(identifier, &mut tokens);
    FrameworkIndexEntry {
        id: identifier.to_string(),
        tokens,
        reference: ReferenceData {
            title: symbol.metadata.title.clone(),
            kind: symbol.metadata.symbol_kind.clone(),
            r#abstract: Some(symbol.r#abstract.clone()),
            platforms: Some(symbol.metadata.platforms.clone()),
            url: Some(format!(
                "/{}",
                identifier
                    .trim_start_matches("doc://com.apple.documentation/")
                    .trim_start_matches("doc://com.apple.SwiftUI/")
                    .trim_start_matches('/')
            )),
        },
    }
}

fn tokenize_into(value: &str, tokens: &mut Vec<String>) {
    for token in value
        .split(|c: char| c.is_whitespace() || matches!(c, '/' | '.' | '_' | '-'))
        .filter(|token| !token.is_empty())
    {
        let lower = token.to_lowercase();
        if !tokens.contains(&lower) {
            tokens.push(lower);
        }
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
