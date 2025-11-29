use std::sync::Arc;

use anyhow::Result;
use apple_docs_client::DocsPlatform;
use serde::Deserialize;
use serde_json::json;

use crate::state::{AppContext, ToolDefinition, ToolHandler, ToolResponse};

use super::{parse_args, text_response, wrap_handler};

#[derive(Deserialize)]
struct SwitchPlatformArgs {
    platform: String,
}

pub fn definition() -> (ToolDefinition, ToolHandler) {
    let def = ToolDefinition {
        name: "switch_platform".to_string(),
        description: "Switch to a different documentation platform (Apple, Android, or Flutter). Use this to change which platform's documentation you're searching and browsing.".to_string(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "platform": {
                    "type": "string",
                    "description": "The platform to switch to. Options: 'apple' (iOS/macOS/Swift), 'android' (Kotlin/Jetpack Compose), 'flutter' (Dart/Flutter)",
                    "enum": ["apple", "android", "flutter"]
                }
            },
            "required": ["platform"]
        }),
    };

    (def, wrap_handler(handle_switch_platform))
}

async fn handle_switch_platform(
    context: Arc<AppContext>,
    value: serde_json::Value,
) -> Result<ToolResponse> {
    let args: SwitchPlatformArgs = parse_args(value)?;

    let platform = DocsPlatform::from_str_loose(&args.platform)
        .ok_or_else(|| anyhow::anyhow!(
            "Unknown platform '{}'. Use 'apple', 'android', or 'flutter'.",
            args.platform
        ))?;

    let previous = {
        let mut guard = context.state.active_platform.write().await;
        let prev = *guard;
        *guard = platform;
        prev
    };

    let mut lines = vec![
        format!("## Switched to {} Documentation", platform.display_name()),
        String::new(),
        format!("**Platform:** {}", platform.display_name()),
        format!("**Languages:** {}", platform.languages().join(", ")),
        format!("**Description:** {}", platform.description()),
        String::new(),
    ];

    if previous != platform {
        lines.push(format!("*Previously using {} documentation.*", previous.display_name()));
        lines.push(String::new());
    }

    lines.push("### Available Tools".to_string());
    lines.push(String::new());

    match platform {
        DocsPlatform::Apple => {
            lines.push("- **discover_technologies** - Browse Apple frameworks (SwiftUI, UIKit, Foundation, etc.)".to_string());
            lines.push("- **choose_technology** - Select a framework to explore".to_string());
            lines.push("- **search_symbols** - Search for APIs within the selected framework".to_string());
            lines.push("- **get_documentation** - Get detailed documentation for a symbol".to_string());
            lines.push("- **how_do_i** - Get guided recipes for common tasks".to_string());
        }
        DocsPlatform::Android => {
            lines.push("- **discover_technologies** - Browse Android libraries (Compose, Room, ViewModel, etc.)".to_string());
            lines.push("- **choose_technology** - Select a library to explore".to_string());
            lines.push("- **search_symbols** - Search for APIs within Android documentation".to_string());
            lines.push("- **get_documentation** - Get reference links for Android APIs".to_string());
        }
        DocsPlatform::Flutter => {
            lines.push("- **discover_technologies** - Browse Flutter/Dart libraries".to_string());
            lines.push("- **choose_technology** - Select a library to explore".to_string());
            lines.push("- **search_symbols** - Search Flutter API documentation".to_string());
            lines.push("- **get_documentation** - Get detailed Flutter API documentation".to_string());
        }
    }

    lines.push(String::new());
    lines.push("### Quick Start".to_string());
    lines.push(String::new());
    lines.push("Use `discover_technologies` to see available frameworks/libraries, or `search_symbols` to search directly.".to_string());

    let response = text_response(lines);
    Ok(response.with_metadata(json!({
        "platform": platform.display_name(),
        "previous_platform": previous.display_name(),
        "switched": previous != platform
    })))
}
