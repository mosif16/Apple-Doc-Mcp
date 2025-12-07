use std::{collections::HashSet, sync::Arc};

use anyhow::{anyhow, Context, Result};
use docs_mcp_client::types::{
    extract_text, format_platforms, PlatformInfo, ReferenceData, SymbolData, TopicData,
    TopicSection,
};
use multi_provider_client::types::ProviderType;
use serde::Deserialize;
use serde_json::{json, Value};

use crate::{
    markdown,
    services::{design_guidance, knowledge},
    state::{AppContext, ToolDefinition, ToolHandler, ToolResponse},
    tools::{parse_args, text_response, wrap_handler},
};

#[derive(Debug, Deserialize)]
struct Args {
    path: String,
}

#[derive(Debug, Clone)]
struct CodeSnippet {
    language: String,
    code: String,
    caption: Option<String>,
}

#[derive(Debug)]
struct RenderOutput {
    lines: Vec<String>,
    metadata: Value,
}

/// Code execution caller identifier for programmatic tool calling.
const CODE_EXECUTION_CALLER: &str = "code_execution_20250825";

pub fn definition() -> (ToolDefinition, ToolHandler) {
    (
        ToolDefinition {
            name: "get_documentation".to_string(),
            description: "Get detailed documentation for symbols within the selected technology. \
                         Supports batch fetching: call multiple times in code to retrieve docs \
                         for several symbols, then compare or aggregate results programmatically. \
                         Returns summaries, platform availability, API references, and design guidance."
                .to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "required": ["path"],
                "properties": {
                    "path": {"type": "string", "description": "Symbol path or relative name"}
                }
            }),
            // Examples showing various path formats accepted by the tool
            input_examples: Some(vec![
                // Simple symbol name (resolved relative to active technology)
                json!({"path": "Button"}),
                // Nested symbol path
                json!({"path": "View/body-swift.property"}),
                // Full documentation path
                json!({"path": "documentation/swiftui/navigationstack"}),
                // Design guidance / HIG content
                json!({"path": "design/human-interface-guidelines/buttons"}),
                // Path with doc:// prefix (automatically stripped)
                json!({"path": "doc://com.apple.documentation/documentation/swiftui/text"}),
            ]),
            // Enable programmatic calling for batch documentation fetching.
            // Allows Claude to write code that fetches multiple symbols and compares them,
            // or extracts specific fields from each without flooding context.
            allowed_callers: Some(vec![CODE_EXECUTION_CALLER.to_string()]),
        },
        wrap_handler(|context, value| async move {
            let args: Args = parse_args(value)?;
            handle(context, args).await
        }),
    )
}

async fn handle(context: Arc<AppContext>, args: Args) -> Result<ToolResponse> {
    // Dispatch based on provider type
    let provider = *context.state.active_provider.read().await;

    match provider {
        ProviderType::Apple => {
            let active = context
                .state
                .active_technology
                .read()
                .await
                .clone()
                .context("No technology selected. Use `choose_technology` first.")?;
            handle_apple(&context, &active, &args).await
        }
        ProviderType::Telegram | ProviderType::TON | ProviderType::Cocoon | ProviderType::Rust
        | ProviderType::Mdn | ProviderType::WebFrameworks | ProviderType::Mlx | ProviderType::HuggingFace
        | ProviderType::QuickNode | ProviderType::ClaudeAgentSdk => {
            // For non-Apple providers, use active_unified_technology
            let unified = context
                .state
                .active_unified_technology
                .read()
                .await
                .clone()
                .context("No technology selected. Use `choose_technology` first.")?;

            // Create a minimal Technology struct for compatibility
            let active = docs_mcp_client::types::Technology {
                identifier: unified.identifier.clone(),
                title: unified.title.clone(),
                r#abstract: vec![],
                kind: String::new(),
                role: String::new(),
                url: String::new(),
            };

            match provider {
                ProviderType::Telegram => handle_telegram(&context, &active, &args).await,
                ProviderType::TON => handle_ton(&context, &active, &args).await,
                ProviderType::Cocoon => handle_cocoon(&context, &active, &args).await,
                ProviderType::Rust => handle_rust(&context, &active, &args).await,
                // Mlx, HuggingFace, QuickNode, and ClaudeAgentSdk use the unified query tool
                ProviderType::Mlx | ProviderType::HuggingFace | ProviderType::QuickNode | ProviderType::ClaudeAgentSdk => {
                    anyhow::bail!("Use the `query` tool for {} documentation", provider.name())
                }
                _ => unreachable!(),
            }
        }
    }
}

async fn handle_apple(
    context: &Arc<AppContext>,
    active: &docs_mcp_client::types::Technology,
    args: &Args,
) -> Result<ToolResponse> {
    let identifier = active
        .identifier
        .split('/')
        .next_back()
        .context("Invalid technology identifier")?;

    let normalized = normalize_path(&args.path, identifier);
    let fallback = fallback_path(&args.path);
    let paths = if normalized == fallback {
        vec![normalized.clone()]
    } else {
        vec![normalized.clone(), fallback.clone()]
    };
    let mut last_error = None;

    for path in paths {
        match context.client.load_document(&path).await {
            Ok(value) => {
                if let Ok(symbol) = serde_json::from_value::<SymbolData>(value.clone()) {
                    *context.state.last_symbol.write().await = Some(symbol.clone());
                    let symbol_title = symbol
                        .metadata
                        .title
                        .clone()
                        .unwrap_or_else(|| "Symbol".to_string());
                    let symbol_path = format!("/{}", normalized);
                    let design_sections =
                        design_guidance::guidance_for(context, &symbol_title, &symbol_path)
                            .await
                            .unwrap_or_default();
                    let render = build_symbol_response(&active.title, &symbol, &design_sections);
                    return Ok(text_response(render.lines).with_metadata(render.metadata));
                }

                match serde_json::from_value::<TopicData>(value) {
                    Ok(topic) => {
                        let topic_title =
                            topic.metadata.title.clone().unwrap_or_else(|| path.clone());
                        let topic_path = if path.starts_with('/') {
                            path.clone()
                        } else {
                            format!("/{path}")
                        };
                        let design_sections =
                            design_guidance::guidance_for(context, &topic_title, &topic_path)
                                .await
                                .unwrap_or_default();
                        let render =
                            build_topic_response(&active.title, &path, &topic, &design_sections);
                        return Ok(text_response(render.lines).with_metadata(render.metadata));
                    }
                    Err(error) => {
                        last_error = Some(anyhow!(
                            "Unsupported documentation format at {}: {}",
                            path,
                            error
                        ));
                    }
                }
            }
            Err(error) => {
                last_error = Some(error);
            }
        }
    }

    Err(last_error.unwrap_or_else(|| {
        anyhow!(
            "Failed to load documentation for {} (and fallback {}).",
            normalized,
            fallback
        )
    }))
}

async fn handle_telegram(
    context: &Arc<AppContext>,
    active: &docs_mcp_client::types::Technology,
    args: &Args,
) -> Result<ToolResponse> {
    let path = args.path.trim();

    // Try to get item by name (searching through all items)
    if let Ok(item) = context.providers.telegram.get_item(path).await {
        let mut lines = vec![
            markdown::header(1, &item.name),
            String::new(),
            markdown::bold("Provider", "Telegram Bot API"),
            markdown::bold("Type", &item.kind),
        ];

        lines.push(String::new());
        lines.push(markdown::header(2, "Description"));
        lines.push(item.description.clone());

        if !item.fields.is_empty() {
            lines.push(String::new());
            lines.push(markdown::header(2, "Fields/Parameters"));
            for field in &item.fields {
                let required_str = if field.required { " (required)" } else { " (optional)" };
                let types_str = field.types.join(" | ");
                lines.push(format!(
                    "• **{}** `{}`{} — {}",
                    field.name, types_str, required_str, field.description
                ));
            }
        }

        if let Some(returns) = &item.returns {
            lines.push(String::new());
            lines.push(markdown::header(2, "Returns"));
            lines.push(returns.join(", "));
        }

        let metadata = json!({
            "provider": "telegram",
            "kind": item.kind,
            "name": item.name,
            "fieldCount": item.fields.len(),
        });

        return Ok(text_response(lines).with_metadata(metadata));
    }

    // Fallback: search for the item
    if let Ok(results) = context.providers.telegram.search(path).await {
        if let Some(item) = results.first() {
            let mut lines = vec![
                markdown::header(1, &item.name),
                String::new(),
                markdown::bold("Provider", "Telegram Bot API"),
                markdown::bold("Type", &item.kind),
            ];

            lines.push(String::new());
            lines.push(markdown::header(2, "Description"));
            lines.push(item.description.clone());

            let metadata = json!({
                "provider": "telegram",
                "kind": item.kind,
                "name": item.name,
            });

            return Ok(text_response(lines).with_metadata(metadata));
        }
    }

    Err(anyhow!("Documentation not found for '{}' in Telegram Bot API. Technology: {}", path, active.title))
}

async fn handle_ton(
    context: &Arc<AppContext>,
    active: &docs_mcp_client::types::Technology,
    args: &Args,
) -> Result<ToolResponse> {
    let path = args.path.trim();

    // Try to get endpoint by operation ID
    if let Ok(endpoint) = context.providers.ton.get_endpoint(path).await {
        let mut lines = vec![
            markdown::header(1, &endpoint.path),
            String::new(),
            markdown::bold("Provider", "TON API"),
            markdown::bold("Method", &endpoint.method),
            markdown::bold("Operation ID", &endpoint.operation_id),
        ];

        lines.push(String::new());
        lines.push(markdown::header(2, "Description"));
        if let Some(summary) = &endpoint.summary {
            lines.push(summary.clone());
        } else if let Some(desc) = &endpoint.description {
            lines.push(desc.clone());
        } else {
            lines.push("No description available.".to_string());
        }

        if !endpoint.parameters.is_empty() {
            lines.push(String::new());
            lines.push(markdown::header(2, "Parameters"));
            for param in &endpoint.parameters {
                let required = if param.required { " (required)" } else { " (optional)" };
                let location = format!(" [{}]", param.location);
                let schema_type = param.schema_type.as_deref().unwrap_or("any");
                let description = param.description.as_deref().unwrap_or("");
                lines.push(format!(
                    "• **{}** `{}`{}{} — {}",
                    param.name, schema_type, location, required, description
                ));
            }
        }

        if !endpoint.responses.is_empty() {
            lines.push(String::new());
            lines.push(markdown::header(2, "Responses"));
            for (code, desc) in &endpoint.responses {
                lines.push(format!("• **{}** — {}", code, desc));
            }
        }

        let metadata = json!({
            "provider": "ton",
            "kind": "endpoint",
            "method": endpoint.method,
            "path": endpoint.path,
            "parameterCount": endpoint.parameters.len(),
        });

        return Ok(text_response(lines).with_metadata(metadata));
    }

    // Fallback: search for the endpoint
    if let Ok(results) = context.providers.ton.search(path).await {
        if let Some(endpoint) = results.first() {
            let mut lines = vec![
                markdown::header(1, &endpoint.path),
                String::new(),
                markdown::bold("Provider", "TON API"),
                markdown::bold("Method", &endpoint.method),
            ];

            lines.push(String::new());
            lines.push(markdown::header(2, "Description"));
            if let Some(summary) = &endpoint.summary {
                lines.push(summary.clone());
            } else {
                lines.push("No description available.".to_string());
            }

            let metadata = json!({
                "provider": "ton",
                "kind": "endpoint",
                "method": endpoint.method,
                "path": endpoint.path,
            });

            return Ok(text_response(lines).with_metadata(metadata));
        }
    }

    Err(anyhow!("Documentation not found for '{}' in TON API. Technology: {}", path, active.title))
}

async fn handle_cocoon(
    context: &Arc<AppContext>,
    active: &docs_mcp_client::types::Technology,
    args: &Args,
) -> Result<ToolResponse> {
    let path = args.path.trim();
    let section_id = active.identifier.as_str();

    if let Ok(section) = context.providers.cocoon.get_section(section_id).await {
        // First check if the path matches the section itself
        if section.identifier.eq_ignore_ascii_case(path) || section.title.to_lowercase().contains(&path.to_lowercase()) {
            let mut lines = vec![
                markdown::header(1, &section.title),
                String::new(),
                markdown::bold("Provider", "Cocoon"),
                markdown::bold("Type", "Documentation Section"),
            ];

            lines.push(String::new());
            lines.push(markdown::header(2, "Overview"));
            lines.push(section.description.clone());

            if !section.documents.is_empty() {
                lines.push(String::new());
                lines.push(markdown::header(2, "Documents"));
                for doc in &section.documents {
                    lines.push(format!("• **{}** — {}", doc.title, trim_with_ellipsis(&doc.summary, 100)));
                    lines.push(format!("  `get_documentation {{ \"path\": \"{}\" }}`", doc.path));
                }
            }

            let metadata = json!({
                "provider": "cocoon",
                "kind": "section",
                "title": section.title,
                "documentCount": section.documents.len(),
            });

            return Ok(text_response(lines).with_metadata(metadata));
        }

        // Look for a specific document within the section
        if let Some(doc) = section.documents.iter().find(|d| {
            d.path.eq_ignore_ascii_case(path)
                || d.title.to_lowercase().contains(&path.to_lowercase())
        }) {
            // Fetch full document content
            if let Ok(full_doc) = context.providers.cocoon.get_document(&doc.path).await {
                let mut lines = vec![
                    markdown::header(1, &full_doc.title),
                    String::new(),
                    markdown::bold("Provider", "Cocoon"),
                    markdown::bold("Section", &section.title),
                ];

                lines.push(String::new());
                lines.push(markdown::header(2, "Content"));
                lines.push(full_doc.content.clone());

                if !full_doc.url.is_empty() {
                    lines.push(String::new());
                    lines.push(format!("**Source:** {}", full_doc.url));
                }

                let metadata = json!({
                    "provider": "cocoon",
                    "kind": "document",
                    "title": full_doc.title,
                    "path": full_doc.path,
                });

                return Ok(text_response(lines).with_metadata(metadata));
            }
        }
    }

    Err(anyhow!("Documentation not found for '{}' in Cocoon. Technology: {}", path, active.title))
}

async fn handle_rust(
    context: &Arc<AppContext>,
    active: &docs_mcp_client::types::Technology,
    args: &Args,
) -> Result<ToolResponse> {
    let path = args.path.trim();

    // Extract crate name from technology identifier (e.g., "rust:std" -> "std")
    let crate_name = active
        .identifier
        .strip_prefix("rust:")
        .unwrap_or(&active.identifier);

    // Try to get the item documentation
    if let Ok(item) = context.providers.rust.get_item(path).await {
        return Ok(build_rust_response(&item));
    }

    // If not found as item, try searching
    if let Ok(results) = context.providers.rust.search(crate_name, path).await {
        if let Some(item) = results.first() {
            // For search results, try to get detailed docs
            if let Ok(detailed) = context.providers.rust.get_item(&item.path).await {
                return Ok(build_rust_response(&detailed));
            }
            return Ok(build_rust_response(item));
        }
    }

    Err(anyhow!(
        "Documentation not found for '{}' in Rust crate '{}'",
        path,
        crate_name
    ))
}

fn build_rust_response(item: &multi_provider_client::rust::RustItem) -> ToolResponse {
    let mut lines = vec![
        markdown::header(1, &item.name),
        String::new(),
        markdown::bold("Provider", "Rust"),
        markdown::bold("Crate", &format!("{} v{}", item.crate_name, item.crate_version)),
        markdown::bold("Kind", &format!("{:?}", item.kind)),
    ];

    if !item.path.is_empty() {
        lines.push(markdown::bold("Path", &format!("`{}`", item.path)));
    }

    // Declaration/Signature
    if let Some(decl) = &item.declaration {
        lines.push(String::new());
        lines.push(markdown::header(2, "Declaration"));
        lines.push(format!("```rust\n{}\n```", decl));
    }

    // Quick Summary
    lines.push(String::new());
    lines.push(markdown::header(2, "Quick Summary"));
    let quick_summary = build_rust_quick_summary(item);
    lines.extend(quick_summary);

    // Documentation
    lines.push(String::new());
    lines.push(markdown::header(2, "Documentation"));
    if let Some(docs) = &item.documentation {
        if !docs.is_empty() {
            lines.push(docs.clone());
        } else if !item.summary.is_empty() {
            lines.push(item.summary.clone());
        } else {
            lines.push("No documentation available.".to_string());
        }
    } else if !item.summary.is_empty() {
        lines.push(item.summary.clone());
    } else {
        lines.push("No documentation available.".to_string());
    }

    // Code Examples
    if !item.examples.is_empty() {
        lines.push(String::new());
        lines.push(markdown::header(2, "Examples"));
        for (i, example) in item.examples.iter().take(3).enumerate() {
            if item.examples.len() > 1 {
                lines.push(format!("### Example {}", i + 1));
            }
            if let Some(desc) = &example.description {
                lines.push(format!("_{}_", desc));
            }
            lines.push(format!("```rust\n{}\n```", example.code));
            lines.push(String::new());
        }
        if item.examples.len() > 3 {
            lines.push(format!("*... and {} more examples*", item.examples.len() - 3));
        }
    }

    // Methods (for structs/enums/traits)
    if !item.methods.is_empty() {
        lines.push(String::new());
        lines.push(markdown::header(2, "Methods"));
        for method in item.methods.iter().take(15) {
            let mut modifiers = Vec::new();
            if method.is_unsafe {
                modifiers.push("unsafe");
            }
            if method.is_const {
                modifiers.push("const");
            }
            if method.is_async {
                modifiers.push("async");
            }
            let modifier_str = if modifiers.is_empty() {
                String::new()
            } else {
                format!(" [{}]", modifiers.join(", "))
            };
            lines.push(format!("• **{}**{}", method.name, modifier_str));
            if !method.signature.is_empty() {
                lines.push(format!("  `{}`", trim_with_ellipsis(&method.signature, 100)));
            }
            if !method.summary.is_empty() {
                lines.push(format!("  {}", trim_with_ellipsis(&method.summary, 120)));
            }
        }
        if item.methods.len() > 15 {
            lines.push(format!("*... and {} more methods*", item.methods.len() - 15));
        }
    }

    // Trait Implementations
    if !item.impl_traits.is_empty() {
        lines.push(String::new());
        lines.push(markdown::header(2, "Trait Implementations"));
        for trait_name in item.impl_traits.iter().take(20) {
            lines.push(format!("• `{}`", trait_name));
        }
        if item.impl_traits.len() > 20 {
            lines.push(format!(
                "*... and {} more traits*",
                item.impl_traits.len() - 20
            ));
        }
    }

    // Associated Types (for traits)
    if !item.associated_types.is_empty() {
        lines.push(String::new());
        lines.push(markdown::header(2, "Associated Types"));
        for assoc in &item.associated_types {
            let bounds = assoc
                .bounds
                .as_ref()
                .map(|b| format!(": {}", b))
                .unwrap_or_default();
            let default = assoc
                .default
                .as_ref()
                .map(|d| format!(" = {}", d))
                .unwrap_or_default();
            lines.push(format!("• **{}**{}{}", assoc.name, bounds, default));
        }
    }

    // Links
    lines.push(String::new());
    if !item.url.is_empty() {
        lines.push(format!("**Documentation:** {}", item.url));
    }
    if let Some(source) = &item.source_url {
        lines.push(format!("**Source:** {}", source));
    }

    let metadata = json!({
        "provider": "rust",
        "kind": format!("{:?}", item.kind),
        "crate": item.crate_name,
        "version": item.crate_version,
        "path": item.path,
        "isDetailed": item.is_detailed,
        "hasDeclaration": item.declaration.is_some(),
        "hasDocumentation": item.documentation.is_some(),
        "exampleCount": item.examples.len(),
        "methodCount": item.methods.len(),
        "traitImplCount": item.impl_traits.len(),
        "associatedTypeCount": item.associated_types.len(),
    });

    text_response(lines).with_metadata(metadata)
}

fn build_rust_quick_summary(item: &multi_provider_client::rust::RustItem) -> Vec<String> {
    let mut summary = Vec::new();

    summary.push(format!("• Kind: {:?}", item.kind));
    summary.push(format!("• Crate: {} v{}", item.crate_name, item.crate_version));

    if !item.summary.is_empty() {
        summary.push(format!("• Summary: {}", trim_with_ellipsis(&item.summary, 140)));
    }

    if item.declaration.is_some() {
        summary.push("• Declaration: See signature above".to_string());
    }

    if !item.examples.is_empty() {
        summary.push(format!("• Examples: {} code sample(s)", item.examples.len()));
    }

    if !item.methods.is_empty() {
        let method_names: Vec<_> = item.methods.iter().take(3).map(|m| m.name.as_str()).collect();
        summary.push(format!(
            "• Methods: {} ({} total)",
            method_names.join(", "),
            item.methods.len()
        ));
    }

    if !item.impl_traits.is_empty() {
        let trait_names: Vec<_> = item.impl_traits.iter().take(3).cloned().collect();
        summary.push(format!(
            "• Implements: {} ({} total)",
            trait_names.join(", "),
            item.impl_traits.len()
        ));
    }

    summary
}

fn normalize_path(path: &str, identifier: &str) -> String {
    let trimmed = path.trim();
    let without_doc = trimmed
        .strip_prefix("doc://com.apple.SwiftUI/")
        .or_else(|| trimmed.strip_prefix("doc://com.apple.documentation/"))
        .or_else(|| trimmed.strip_prefix("doc://com.apple.HIG/"))
        .unwrap_or(trimmed);
    let without_prefix = without_doc.trim_start_matches('/');

    if without_prefix.starts_with("design/")
        || without_prefix.starts_with("Design/")
        || without_prefix.starts_with("human-interface-guidelines/")
    {
        return without_prefix.to_ascii_lowercase();
    }

    if without_prefix.starts_with("documentation/") {
        without_prefix.to_string()
    } else {
        format!("documentation/{}/{}", identifier, without_prefix)
    }
}

fn fallback_path(path: &str) -> String {
    let trimmed_input = path.trim();
    let without_doc = trimmed_input
        .strip_prefix("doc://com.apple.SwiftUI/")
        .or_else(|| trimmed_input.strip_prefix("doc://com.apple.documentation/"))
        .or_else(|| trimmed_input.strip_prefix("doc://com.apple.HIG/"))
        .unwrap_or(trimmed_input);
    let trimmed = without_doc.trim_start_matches('/');

    if trimmed.starts_with("design/")
        || trimmed.starts_with("Design/")
        || trimmed.starts_with("human-interface-guidelines/")
    {
        trimmed.to_ascii_lowercase()
    } else if trimmed.starts_with("documentation/") {
        trimmed.to_string()
    } else {
        format!("documentation/{}", trimmed)
    }
}

fn build_topic_response(
    technology_title: &str,
    path: &str,
    topic: &TopicData,
    design_sections: &[design_guidance::DesignSection],
) -> RenderOutput {
    let title = topic
        .metadata
        .title
        .clone()
        .unwrap_or_else(|| "Topic".to_string());
    let description = extract_text(&topic.r#abstract);
    let parameters = extract_topic_parameters(topic);
    let relationships = extract_topic_relationships(topic);
    let summary = build_topic_summary(
        topic,
        &description,
        design_sections,
        &parameters,
        &relationships,
    );
    let summary_count = summary.len();
    let has_sample_summary = summary.iter().any(|entry| entry.contains("Sample code"));

    let mut lines = vec![
        markdown::header(1, &title),
        String::new(),
        markdown::bold("Technology", technology_title),
        markdown::bold("Path", path),
    ];

    if !summary.is_empty() {
        lines.push(String::new());
        lines.push(markdown::header(2, "Quick Summary"));
        lines.extend(summary);
    }

    lines.push(String::new());
    lines.push(markdown::header(2, "Overview"));
    if description.trim().is_empty() {
        lines.push("No overview available.".to_string());
    } else {
        lines.push(description);
    }

    if !design_sections.is_empty() {
        lines.push(String::new());
        lines.push(markdown::header(2, "Design Guidance"));
        for section in design_sections.iter().take(2) {
            lines.push(format!("### {}", section.title));
            if let Some(summary) = section.summary.as_ref() {
                lines.push(format!("_{summary}_"));
            }
            for bullet in section.bullets.iter().take(4) {
                lines.push(format!("• **{}:** {}", bullet.category, bullet.text));
            }
            lines.push(format!(
                "Read more: `get_documentation {{ \"path\": \"{}\" }}`",
                section.slug
            ));
            lines.push(String::new());
        }
    }

    if !topic.topic_sections.is_empty() {
        lines.push(String::new());
        lines.push(markdown::header(2, "Topics"));
        for section in &topic.topic_sections {
            lines.push(format!("### {}", section.title));
            for identifier in &section.identifiers {
                if let Some(reference) = topic.references.get(identifier) {
                    let title = reference
                        .title
                        .clone()
                        .unwrap_or_else(|| identifier.to_string());
                    let desc = reference
                        .r#abstract
                        .as_ref()
                        .map(|segments| extract_text(segments))
                        .unwrap_or_default();
                    let trimmed = trim_with_ellipsis(&desc, 100);
                    lines.push(format!("• **{}** - {}", title, trimmed));
                }
            }
            lines.push(String::new());
        }
    }

    if !relationships.is_empty() {
        lines.push(String::new());
        lines.push(markdown::header(2, "Related Content"));
        for rel in &relationships {
            lines.push(format!(
                "• **{}** — {} (`get_documentation {{ \"path\": \"{}\" }}`)",
                rel.title, rel.summary, rel.path
            ));
        }
    }

    if !parameters.is_empty() {
        lines.push(String::new());
        lines.push(markdown::header(2, "Key Parameters"));
        for param in &parameters {
            lines.push(format!(
                "• **{}** — {}",
                param.name,
                trim_with_ellipsis(&param.summary, 120)
            ));
        }
    }

    let metadata = json!({
        "kind": "topic",
        "designSections": design_sections.len(),
        "topicSections": topic.topic_sections.len(),
        "summaryCount": summary_count,
        "hasSampleSummary": has_sample_summary,
        "sampleReferences": count_topic_sample_references(topic),
        "relationshipCount": relationships.len(),
        "parameterCount": parameters.len(),
    });

    RenderOutput { lines, metadata }
}

fn build_symbol_response(
    technology_title: &str,
    symbol: &SymbolData,
    design_sections: &[design_guidance::DesignSection],
) -> RenderOutput {
    let title = symbol
        .metadata
        .title
        .clone()
        .unwrap_or_else(|| "Symbol".to_string());
    let kind = symbol
        .metadata
        .symbol_kind
        .clone()
        .unwrap_or_else(|| "Unknown".to_string());
    let platforms = format_platforms(symbol.metadata.platforms.as_slice());
    let description = extract_text(&symbol.r#abstract);
    let knowledge_entry = knowledge::lookup(technology_title, &title);
    let quick_tip = knowledge_entry.and_then(|entry| entry.quick_tip);
    let snippet_from_knowledge =
        knowledge_entry
            .and_then(knowledge::snippet)
            .map(|snippet| CodeSnippet {
                language: snippet.language.to_string(),
                code: snippet.code.to_string(),
                caption: snippet.caption.map(|caption| caption.to_string()),
            });
    let snippet = snippet_from_knowledge.or_else(|| extract_symbol_snippet(symbol));
    let relationships = extract_relationships(symbol);
    let parameters = extract_parameters(symbol);
    let summary = build_symbol_summary(
        symbol,
        &kind,
        &platforms,
        &description,
        snippet.as_ref(),
        quick_tip,
        design_sections,
        &relationships,
        &parameters,
    );
    let summary_count = summary.len();
    let has_sample_summary = summary.iter().any(|line| line.contains("Sample code"));

    let mut lines = vec![
        markdown::header(1, &title),
        String::new(),
        markdown::bold("Technology", technology_title),
        markdown::bold("Type", &kind),
        markdown::bold("Platforms", &platforms),
    ];

    if !summary.is_empty() {
        lines.push(String::new());
        lines.push(markdown::header(2, "Quick Summary"));
        lines.extend(summary);
    }

    if let Some(snippet) = &snippet {
        lines.push(String::new());
        lines.push(markdown::header(3, "Sample Code"));
        if let Some(caption) = &snippet.caption {
            lines.push(format!("_{caption}_"));
        }
        lines.push(format!(
            "```{}\n{}\n```",
            snippet.language,
            snippet.code.trim_end()
        ));
    }

    if !design_sections.is_empty() {
        lines.push(String::new());
        lines.push(markdown::header(2, "Design Guidance"));
        for section in design_sections.iter().take(2) {
            lines.push(format!("### {}", section.title));
            if let Some(summary) = section.summary.as_ref() {
                lines.push(format!("_{summary}_"));
            }
            for bullet in section.bullets.iter().take(4) {
                lines.push(format!("• **{}:** {}", bullet.category, bullet.text));
            }
            lines.push(format!(
                "Read more: `get_documentation {{ \"path\": \"{}\" }}`",
                section.slug
            ));
            lines.push(String::new());
        }
    }

    let has_knowledge = if let Some(entry) = knowledge_entry {
        let related = knowledge::related_items(entry);
        let integration = knowledge::integration_links(entry);
        if !related.is_empty() || !integration.is_empty() {
            lines.push(String::new());
            lines.push(markdown::header(2, "Integration Notes"));
            for link in integration {
                lines.push(format!(
                    "• Bridge to {}: {} — {} (`get_documentation {{ \"path\": \"{}\" }}`)",
                    link.framework, link.title, link.note, link.path
                ));
            }
            for item in related {
                lines.push(format!(
                    "• Related: {} — {} (`get_documentation {{ \"path\": \"{}\" }}`)",
                    item.title, item.note, item.path
                ));
            }
        }
        true
    } else {
        false
    };

    lines.push(String::new());
    lines.push(markdown::header(2, "Overview"));
    if description.trim().is_empty() {
        lines.push("No overview available.".to_string());
    } else {
        lines.push(description);
    }

    if !symbol.topic_sections.is_empty() {
        lines.push(String::new());
        lines.push(markdown::header(2, "API Reference"));
        for section in &symbol.topic_sections {
            lines.push(format!("### {}", section.title));
            for identifier in section.identifiers.iter().take(5) {
                if let Some(reference) = symbol.references.get(identifier) {
                    let desc = reference
                        .r#abstract
                        .as_ref()
                        .map(|segments| extract_text(segments))
                        .unwrap_or_default();
                    let trimmed = trim_with_ellipsis(&desc, 100);
                    let title = reference
                        .title
                        .clone()
                        .unwrap_or_else(|| "Symbol".to_string());
                    lines.push(format!("• **{}** - {}", title, trimmed));
                }
            }
            if section.identifiers.len() > 5 {
                lines.push(format!(
                    "*... and {} more items*",
                    section.identifiers.len() - 5
                ));
            }
            lines.push(String::new());
        }
    }

    if !relationships.is_empty() {
        lines.push(String::new());
        lines.push(markdown::header(2, "Relationships"));
        for rel in &relationships {
            lines.push(format!(
                "• **{}** — {} (`get_documentation {{ \"path\": \"{}\" }}`)",
                rel.title, rel.summary, rel.path
            ));
        }
    }

    if !parameters.is_empty() {
        lines.push(String::new());
        lines.push(markdown::header(2, "Parameters"));
        for param in &parameters {
            lines.push(format!(
                "• **{}** — {}",
                param.name,
                trim_with_ellipsis(&param.summary, 120)
            ));
        }
    }

    let metadata = json!({
        "kind": "symbol",
        "designSections": design_sections.len(),
        "topicSections": symbol.topic_sections.len(),
        "hasSnippet": snippet.is_some(),
        "hasKnowledge": has_knowledge,
        "hasQuickTip": quick_tip.is_some(),
        "platformCount": symbol.metadata.platforms.len(),
        "sampleReferences": count_symbol_sample_references(symbol),
        "relationshipCount": relationships.len(),
        "parameterCount": parameters.len(),
        "summaryCount": summary_count,
        "hasSampleSummary": has_sample_summary,
    });

    RenderOutput { lines, metadata }
}

fn trim_with_ellipsis(text: &str, max: usize) -> String {
    if text.len() <= max {
        text.to_string()
    } else {
        // Find a valid UTF-8 character boundary at or before max
        let mut end = max;
        while end > 0 && !text.is_char_boundary(end) {
            end -= 1;
        }
        format!("{}...", &text[..end])
    }
}

#[allow(clippy::too_many_arguments)]
fn build_symbol_summary(
    symbol: &SymbolData,
    kind: &str,
    platforms: &str,
    overview: &str,
    snippet: Option<&CodeSnippet>,
    quick_tip: Option<&str>,
    design_sections: &[design_guidance::DesignSection],
    relationships: &[RelationshipEntry],
    parameters: &[ParameterEntry],
) -> Vec<String> {
    let mut summary = Vec::new();

    if !kind.is_empty() {
        summary.push(format!("• Kind: {kind}"));
    }

    if let Some(availability) = summarize_availability(symbol.metadata.platforms.as_slice()) {
        summary.push(format!("• Introduced: {availability}"));
    } else if !platforms.is_empty() {
        summary.push(format!("• Platforms: {platforms}"));
    }

    let brief = overview.trim();
    if !brief.is_empty() {
        summary.push(format!("• Summary: {}", trim_with_ellipsis(brief, 140)));
    }

    if let Some(tip) = quick_tip {
        summary.push(format!("• Tip: {tip}"));
    }

    if let Some(highlights) = summarize_sections(&symbol.topic_sections) {
        summary.push(format!("• Sections to explore: {highlights}"));
    }

    if let Some(snippet) = snippet {
        let caption = snippet.caption.as_deref().unwrap_or("See snippet below.");
        summary.push(format!("• Sample code: {}", caption));
    } else if let Some(samples) = summarize_sample_code(&symbol.topic_sections, &symbol.references)
    {
        summary.push(format!("• Sample code: {samples}"));
    } else if has_code_examples(&symbol.primary_content_sections) {
        summary.push("• Sample code: Inline examples available in documentation.".to_string());
    }

    if let Some(design_summary) = summarize_design(design_sections) {
        summary.push(format!("• Design: {design_summary}"));
    }

    if !relationships.is_empty() {
        let highlights = relationships
            .iter()
            .map(|rel| rel.title.clone())
            .take(3)
            .collect::<Vec<_>>()
            .join(" · ");
        summary.push(format!("• Related types: {highlights}"));
    }

    if !parameters.is_empty() {
        let highlights = parameters
            .iter()
            .map(|param| param.name.clone())
            .take(3)
            .collect::<Vec<_>>()
            .join(" · ");
        summary.push(format!("• Parameters: {highlights}"));
    }

    summary
}

fn summarize_design(sections: &[design_guidance::DesignSection]) -> Option<String> {
    let mut highlights = Vec::new();
    for section in sections {
        if let Some(bullet) = section.bullets.first() {
            highlights.push(format!("{}: {}", bullet.category, bullet.text));
        }
        if highlights.len() >= 2 {
            break;
        }
    }

    if highlights.is_empty() {
        None
    } else {
        Some(highlights.join(" · "))
    }
}

fn build_topic_summary(
    topic: &TopicData,
    overview: &str,
    design_sections: &[design_guidance::DesignSection],
    parameters: &[ParameterEntry],
    relationships: &[RelationshipEntry],
) -> Vec<String> {
    let mut summary = Vec::new();

    let brief = overview.trim();
    if !brief.is_empty() {
        summary.push(format!("• Summary: {}", trim_with_ellipsis(brief, 140)));
    }

    if let Some(highlights) = summarize_sections(&topic.topic_sections) {
        summary.push(format!("• Sections to explore: {highlights}"));
    }

    if let Some(samples) = summarize_sample_code(&topic.topic_sections, &topic.references) {
        summary.push(format!("• Sample code: {samples}"));
    }

    if let Some(design_summary) = summarize_design(design_sections) {
        summary.push(format!("• Design: {design_summary}"));
    }

    if !relationships.is_empty() {
        let highlights = relationships
            .iter()
            .map(|rel| rel.title.clone())
            .take(3)
            .collect::<Vec<_>>()
            .join(" · ");
        summary.push(format!("• Related content: {highlights}"));
    }

    if !parameters.is_empty() {
        let highlights = parameters
            .iter()
            .map(|param| param.name.clone())
            .take(3)
            .collect::<Vec<_>>()
            .join(" · ");
        summary.push(format!("• Parameters: {highlights}"));
    }

    summary
}

fn summarize_availability(platforms: &[PlatformInfo]) -> Option<String> {
    let mut availability = Vec::new();

    for platform in platforms {
        if let Some(version) = &platform.introduced_at {
            let mut entry = format!("{} {}", platform.name, version);
            if platform.beta {
                entry.push_str(" (Beta)");
            }
            availability.push(entry);
        }
    }

    if availability.is_empty() {
        None
    } else {
        Some(availability.join(" · "))
    }
}

fn summarize_sections(sections: &[TopicSection]) -> Option<String> {
    let highlights: Vec<String> = sections
        .iter()
        .filter_map(|section| {
            let title = section.title.trim();
            if title.is_empty() {
                None
            } else {
                Some(title.to_string())
            }
        })
        .take(3)
        .collect();

    if highlights.is_empty() {
        None
    } else {
        Some(highlights.join(" · "))
    }
}

fn summarize_sample_code(
    sections: &[TopicSection],
    references: &std::collections::HashMap<String, ReferenceData>,
) -> Option<String> {
    let mut titles = Vec::new();
    let mut seen = HashSet::new();

    for section in sections {
        let title = section.title.to_lowercase();
        let is_sample_section = title.contains("sample") || title.contains("tutorial");

        for identifier in &section.identifiers {
            if let Some(reference) = references.get(identifier) {
                let matches_kind = reference
                    .kind
                    .as_deref()
                    .map(|kind| kind.eq_ignore_ascii_case("samplecode"))
                    .unwrap_or(false);
                if is_sample_section || matches_kind {
                    if let Some(name) = reference.title.clone() {
                        if seen.insert(name.clone()) {
                            titles.push(name);
                        }
                    }
                }
            }
        }

        if titles.len() >= 3 {
            break;
        }
    }

    if titles.is_empty() {
        None
    } else {
        Some(titles.join(" · "))
    }
}

fn has_code_examples(sections: &[Value]) -> bool {
    sections.iter().any(contains_code_listing)
}

fn extract_symbol_snippet(symbol: &SymbolData) -> Option<CodeSnippet> {
    if let Some(snippet) = extract_snippet_from_sections(&symbol.primary_content_sections) {
        return Some(snippet);
    }
    None
}

fn extract_snippet_from_sections(sections: &[Value]) -> Option<CodeSnippet> {
    for value in sections {
        if let Some(snippet) = extract_snippet_from_value(value) {
            return Some(snippet);
        }
    }
    None
}

fn extract_snippet_from_value(value: &Value) -> Option<CodeSnippet> {
    match value {
        Value::Object(map) => {
            if let Some(snippet) = parse_code_listing(map) {
                return Some(snippet);
            }
            for nested in map.values() {
                if let Some(snippet) = extract_snippet_from_value(nested) {
                    return Some(snippet);
                }
            }
            None
        }
        Value::Array(items) => {
            for item in items {
                if let Some(snippet) = extract_snippet_from_value(item) {
                    return Some(snippet);
                }
            }
            None
        }
        _ => None,
    }
}

fn parse_code_listing(map: &serde_json::Map<String, Value>) -> Option<CodeSnippet> {
    let kind = map
        .get("type")
        .or_else(|| map.get("kind"))
        .and_then(Value::as_str)
        .unwrap_or_default();
    if !kind.eq_ignore_ascii_case("codelisting") {
        return None;
    }

    let code_value = map.get("code")?;
    let code = match code_value {
        Value::Array(lines) => lines
            .iter()
            .filter_map(Value::as_str)
            .collect::<Vec<_>>()
            .join("\n"),
        Value::String(text) => text.clone(),
        _ => String::new(),
    };

    if code.trim().is_empty() {
        return None;
    }

    let language = map
        .get("syntax")
        .or_else(|| map.get("language"))
        .and_then(Value::as_str)
        .unwrap_or("swift")
        .to_string();

    let caption = map
        .get("caption")
        .and_then(Value::as_str)
        .map(String::from)
        .or_else(|| map.get("title").and_then(Value::as_str).map(String::from));

    Some(CodeSnippet {
        language,
        code,
        caption,
    })
}

fn contains_code_listing(value: &Value) -> bool {
    match value {
        Value::Object(map) => {
            if let Some(kind) = map.get("type").and_then(Value::as_str) {
                if kind.eq_ignore_ascii_case("codelisting") {
                    return true;
                }
            }
            if let Some(kind) = map.get("kind").and_then(Value::as_str) {
                if kind.eq_ignore_ascii_case("codelisting") {
                    return true;
                }
            }
            map.values().any(contains_code_listing)
        }
        Value::Array(items) => items.iter().any(contains_code_listing),
        _ => false,
    }
}

fn count_symbol_sample_references(symbol: &SymbolData) -> usize {
    symbol
        .references
        .values()
        .filter(|reference| {
            reference
                .kind
                .as_deref()
                .map(|kind| kind.eq_ignore_ascii_case("samplecode"))
                .unwrap_or(false)
        })
        .count()
}

fn count_topic_sample_references(topic: &TopicData) -> usize {
    let mut count = 0;
    for section in &topic.topic_sections {
        for identifier in &section.identifiers {
            if let Some(reference) = topic.references.get(identifier) {
                if reference
                    .kind
                    .as_deref()
                    .map(|kind| kind.eq_ignore_ascii_case("samplecode"))
                    .unwrap_or(false)
                {
                    count += 1;
                }
            }
        }
    }
    count
}

#[derive(Clone)]
struct RelationshipEntry {
    title: String,
    path: String,
    summary: String,
}

#[derive(Clone)]
struct ParameterEntry {
    name: String,
    summary: String,
}

fn extract_relationships(symbol: &SymbolData) -> Vec<RelationshipEntry> {
    let mut items = Vec::new();
    for section in &symbol.topic_sections {
        if !section.title.to_lowercase().contains("relationship") {
            continue;
        }
        for identifier in &section.identifiers {
            if let Some(reference) = symbol.references.get(identifier) {
                let title = reference
                    .title
                    .clone()
                    .unwrap_or_else(|| identifier.to_string());
                let summary = reference
                    .r#abstract
                    .as_ref()
                    .map(|segments| extract_text(segments))
                    .unwrap_or_default();
                let path = reference
                    .url
                    .clone()
                    .unwrap_or_else(|| identifier.to_string());
                items.push(RelationshipEntry {
                    title,
                    path,
                    summary,
                });
            }
        }
    }
    items
}

fn extract_topic_relationships(topic: &TopicData) -> Vec<RelationshipEntry> {
    let mut items = Vec::new();
    for section in &topic.topic_sections {
        if !section.title.to_lowercase().contains("relationship") {
            continue;
        }
        for identifier in &section.identifiers {
            if let Some(reference) = topic.references.get(identifier) {
                let title = reference
                    .title
                    .clone()
                    .unwrap_or_else(|| identifier.to_string());
                let summary = reference
                    .r#abstract
                    .as_ref()
                    .map(|segments| extract_text(segments))
                    .unwrap_or_default();
                let path = reference
                    .url
                    .clone()
                    .unwrap_or_else(|| identifier.to_string());
                items.push(RelationshipEntry {
                    title,
                    path,
                    summary,
                });
            }
        }
    }
    items
}

fn extract_parameters(symbol: &SymbolData) -> Vec<ParameterEntry> {
    let mut items = Vec::new();
    for section in &symbol.topic_sections {
        let title = section.title.to_lowercase();
        if !title.contains("parameter") && !title.contains("argument") {
            continue;
        }
        for identifier in &section.identifiers {
            if let Some(reference) = symbol.references.get(identifier) {
                let name = reference
                    .title
                    .clone()
                    .unwrap_or_else(|| identifier.to_string());
                let summary = reference
                    .r#abstract
                    .as_ref()
                    .map(|segments| extract_text(segments))
                    .unwrap_or_default();
                items.push(ParameterEntry { name, summary });
            }
        }
    }
    if items.is_empty() {
        items.extend(extract_inline_parameters(&symbol.primary_content_sections));
    }
    items
}

fn extract_topic_parameters(topic: &TopicData) -> Vec<ParameterEntry> {
    let mut items = Vec::new();
    for section in &topic.topic_sections {
        let title = section.title.to_lowercase();
        if !title.contains("parameter") && !title.contains("argument") {
            continue;
        }
        for identifier in &section.identifiers {
            if let Some(reference) = topic.references.get(identifier) {
                let name = reference
                    .title
                    .clone()
                    .unwrap_or_else(|| identifier.to_string());
                let summary = reference
                    .r#abstract
                    .as_ref()
                    .map(|segments| extract_text(segments))
                    .unwrap_or_default();
                items.push(ParameterEntry { name, summary });
            }
        }
    }
    items
}

fn extract_inline_parameters(sections: &[Value]) -> Vec<ParameterEntry> {
    let mut items = Vec::new();
    for value in sections {
        collect_parameters_from_value(value, &mut items);
    }
    items
}

fn collect_parameters_from_value(value: &Value, items: &mut Vec<ParameterEntry>) {
    match value {
        Value::Object(map) => {
            if let Some(kind) = map.get("kind").and_then(Value::as_str) {
                if kind.eq_ignore_ascii_case("parameters") || kind.eq_ignore_ascii_case("arguments")
                {
                    if let Some(content) = map.get("content").and_then(Value::as_array) {
                        for entry in content {
                            if let Some(name) = entry
                                .get("name")
                                .or_else(|| entry.get("title"))
                                .and_then(Value::as_str)
                            {
                                let summary = entry
                                    .get("description")
                                    .or_else(|| entry.get("abstract"))
                                    .and_then(Value::as_array)
                                    .map(|segments| extract_rich_text(segments))
                                    .unwrap_or_default();
                                items.push(ParameterEntry {
                                    name: name.to_string(),
                                    summary,
                                });
                            }
                        }
                    }
                }
            }
            for nested in map.values() {
                collect_parameters_from_value(nested, items);
            }
        }
        Value::Array(array) => {
            for item in array {
                collect_parameters_from_value(item, items);
            }
        }
        _ => {}
    }
}

fn extract_rich_text(segments: &[Value]) -> String {
    let mut text = String::new();
    for segment in segments {
        if let Some(content) = segment.get("text").and_then(Value::as_str) {
            if !text.is_empty() {
                text.push(' ');
            }
            text.push_str(content);
        }
    }
    text
}

#[cfg(test)]
mod tests {
    use super::*;
    use docs_mcp_client::types::{
        PlatformInfo, ReferenceData, RichText, SymbolData, SymbolMetadata, TopicData,
        TopicMetadata, TopicSection,
    };
    use serde_json::json;
    use std::collections::HashMap;

    fn sample_symbol() -> SymbolData {
        let mut references = HashMap::new();
        references.insert(
            "sample-1".to_string(),
            ReferenceData {
                title: Some("Animating a View".to_string()),
                kind: Some("sampleCode".to_string()),
                r#abstract: None,
                platforms: None,
                url: None,
            },
        );

        SymbolData {
            r#abstract: vec![RichText {
                text: Some("Displays styled text content.".to_string()),
                kind: "text".to_string(),
            }],
            metadata: SymbolMetadata {
                platforms: vec![
                    PlatformInfo {
                        name: "iOS".to_string(),
                        introduced_at: Some("15.0".to_string()),
                        beta: false,
                    },
                    PlatformInfo {
                        name: "macOS".to_string(),
                        introduced_at: None,
                        beta: false,
                    },
                ],
                symbol_kind: Some("Struct".to_string()),
                title: Some("StyledText".to_string()),
            },
            primary_content_sections: Vec::new(),
            references,
            topic_sections: vec![
                TopicSection {
                    anchor: None,
                    identifiers: vec!["sample-1".to_string()],
                    title: "Sample Code".to_string(),
                },
                TopicSection {
                    anchor: None,
                    identifiers: Vec::new(),
                    title: "Configure Appearance".to_string(),
                },
            ],
        }
    }

    fn symbol_with_inline_examples() -> SymbolData {
        let mut symbol = sample_symbol();
        symbol.references.clear();
        symbol.topic_sections.clear();
        symbol.primary_content_sections = vec![json!({
            "kind": "content",
            "content": [
                { "type": "codeListing", "syntax": "swift", "code": ["Text(\"Hello World\")"] }
            ]
        })];
        symbol
    }

    fn sample_topic() -> TopicData {
        let mut references = HashMap::new();
        references.insert(
            "tutorial-1".to_string(),
            ReferenceData {
                title: Some("Create a Custom View".to_string()),
                kind: Some("tutorial".to_string()),
                r#abstract: None,
                platforms: None,
                url: None,
            },
        );

        TopicData {
            r#abstract: vec![RichText {
                text: Some("Learn how to compose complex SwiftUI views.".to_string()),
                kind: "text".to_string(),
            }],
            topic_sections: vec![
                TopicSection {
                    anchor: None,
                    identifiers: vec!["tutorial-1".to_string()],
                    title: "Tutorials".to_string(),
                },
                TopicSection {
                    anchor: None,
                    identifiers: Vec::new(),
                    title: "Best Practices".to_string(),
                },
            ],
            references,
            metadata: TopicMetadata {
                title: Some("Building Views".to_string()),
            },
        }
    }

    #[test]
    fn symbol_summary_highlights_availability_and_samples() {
        let symbol = sample_symbol();
        let summary = build_symbol_summary(
            &symbol,
            "Struct",
            "iOS 15.0, macOS",
            "Displays styled text content.",
            None,
            None,
            &[],
            &[],
            &[],
        );

        assert!(summary
            .iter()
            .any(|line| line.contains("Introduced: iOS 15.0")));
        assert!(summary
            .iter()
            .any(|line| line.contains("Sections to explore")));
        assert!(summary.iter().any(|line| line.contains("Sample code")));
    }

    #[test]
    fn topic_summary_includes_sections() {
        let topic = sample_topic();
        let summary = build_topic_summary(
            &topic,
            "Learn how to compose complex SwiftUI views.",
            &[],
            &[],
            &[],
        );

        assert!(summary.iter().any(|line| line.contains("Summary:")));
        assert!(summary
            .iter()
            .any(|line| line.contains("Sections to explore")));
        assert!(summary.iter().any(|line| line.contains("Sample code")));
    }

    #[test]
    fn symbol_summary_mentions_inline_examples_when_no_sample_refs() {
        let symbol = symbol_with_inline_examples();
        let snippet = extract_symbol_snippet(&symbol);
        let summary = build_symbol_summary(
            &symbol,
            "Struct",
            "iOS 15.0",
            "Displays styled text.",
            snippet.as_ref(),
            None,
            &[],
            &[],
            &[],
        );

        assert!(summary
            .iter()
            .any(|line| line.contains("Sample code: See snippet below.")));

        let snippet = snippet.expect("snippet should be present");
        assert_eq!(snippet.language, "swift");
        assert!(snippet.code.contains("Text(\"Hello World\")"));
    }
}
