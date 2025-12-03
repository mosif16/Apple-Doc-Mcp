use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use anyhow::{Context, Result};
use docs_mcp_client::types::Technology;
use once_cell::sync::Lazy;
use serde_json::Value;
use tokio::sync::RwLock;

use crate::state::AppContext;

#[derive(Clone)]
pub struct DesignBullet {
    pub category: &'static str,
    pub text: String,
}

#[derive(Clone)]
pub struct DesignSection {
    pub slug: String,
    pub url: String,
    pub title: String,
    pub summary: Option<String>,
    pub bullets: Vec<DesignBullet>,
}

struct Mapping {
    path_prefix: &'static str,
    topics: &'static [&'static str],
}

struct PrimerMapping {
    identifier_prefix: Option<&'static str>,
    title_keyword: Option<&'static str>,
    topics: &'static [&'static str],
}

static CACHE: Lazy<RwLock<HashMap<String, Arc<DesignSection>>>> =
    Lazy::new(|| RwLock::new(HashMap::new()));

const TEXT_TOPICS: &[&str] = &[
    "design/human-interface-guidelines/typography",
    "design/human-interface-guidelines/color",
];
const TEXT_FIELD_TOPICS: &[&str] = &[
    "design/human-interface-guidelines/text-fields",
    "design/human-interface-guidelines/inputs",
];
const LIST_TOPICS: &[&str] = &["design/human-interface-guidelines/lists-and-tables"];
const SEARCH_TOPICS: &[&str] = &["design/human-interface-guidelines/search-fields"];
const BUTTON_TOPICS: &[&str] = &[
    "design/human-interface-guidelines/buttons",
    "design/human-interface-guidelines/inputs",
];
const TOGGLE_TOPICS: &[&str] = &["design/human-interface-guidelines/toggles"];
const TOOLBAR_TOPICS: &[&str] = &[
    "design/human-interface-guidelines/toolbars",
    "design/human-interface-guidelines/navigation-and-search",
];
const TAB_TOPICS: &[&str] = &["design/human-interface-guidelines/tab-bars"];
const SPLIT_TOPICS: &[&str] = &["design/human-interface-guidelines/split-views"];
const ACCESSIBILITY_TOPICS: &[&str] = &["design/human-interface-guidelines/accessibility"];
const GENERAL_FOUNDATION_TOPICS: &[&str] = &[
    "design/human-interface-guidelines/layout",
    "design/human-interface-guidelines/foundations",
];

// Additional component mappings
const PICKER_TOPICS: &[&str] = &[
    "design/human-interface-guidelines/pickers",
    "design/human-interface-guidelines/menus",
];
const MENU_TOPICS: &[&str] = &[
    "design/human-interface-guidelines/menus",
    "design/human-interface-guidelines/context-menus",
];
const SHEET_TOPICS: &[&str] = &[
    "design/human-interface-guidelines/sheets",
    "design/human-interface-guidelines/modality",
];
const ALERT_TOPICS: &[&str] = &[
    "design/human-interface-guidelines/alerts",
    "design/human-interface-guidelines/modality",
];
const PROGRESS_TOPICS: &[&str] = &[
    "design/human-interface-guidelines/progress-indicators",
    "design/human-interface-guidelines/feedback",
];
const SLIDER_TOPICS: &[&str] = &[
    "design/human-interface-guidelines/sliders",
    "design/human-interface-guidelines/inputs",
];
const STEPPER_TOPICS: &[&str] = &[
    "design/human-interface-guidelines/steppers",
    "design/human-interface-guidelines/inputs",
];
const IMAGE_TOPICS: &[&str] = &[
    "design/human-interface-guidelines/images",
    "design/human-interface-guidelines/icons",
];
const FORM_TOPICS: &[&str] = &[
    "design/human-interface-guidelines/settings",
    "design/human-interface-guidelines/inputs",
];
const SCROLL_TOPICS: &[&str] = &[
    "design/human-interface-guidelines/scroll-views",
    "design/human-interface-guidelines/layout",
];
const NAVIGATION_TOPICS: &[&str] = &[
    "design/human-interface-guidelines/navigation-and-search",
    "design/human-interface-guidelines/sidebars",
];
const POPOVER_TOPICS: &[&str] = &[
    "design/human-interface-guidelines/popovers",
    "design/human-interface-guidelines/modality",
];
const COLOR_TOPICS: &[&str] = &["design/human-interface-guidelines/color"];
const GESTURE_TOPICS: &[&str] = &[
    "design/human-interface-guidelines/gestures",
    "design/human-interface-guidelines/inputs",
];
const ANIMATION_TOPICS: &[&str] = &[
    "design/human-interface-guidelines/motion",
    "design/human-interface-guidelines/feedback",
];
const STACK_TOPICS: &[&str] = &[
    "design/human-interface-guidelines/layout",
    "design/human-interface-guidelines/foundations",
];
const LABEL_TOPICS: &[&str] = &[
    "design/human-interface-guidelines/labels",
    "design/human-interface-guidelines/typography",
];
const LINK_TOPICS: &[&str] = &[
    "design/human-interface-guidelines/buttons",
    "design/human-interface-guidelines/color",
];

const SWIFTUI_PRIMERS: &[&str] = &[
    "design/human-interface-guidelines/layout",
    "design/human-interface-guidelines/typography",
    "design/human-interface-guidelines/color",
    "design/human-interface-guidelines/accessibility",
];
const UIKIT_PRIMERS: &[&str] = &[
    "design/human-interface-guidelines/buttons",
    "design/human-interface-guidelines/text-fields",
    "design/human-interface-guidelines/navigation-and-search",
    "design/human-interface-guidelines/color",
];
const APPKIT_PRIMERS: &[&str] = &[
    "design/human-interface-guidelines/layout-and-organization",
    "design/human-interface-guidelines/multitasking",
    "design/human-interface-guidelines/inputs",
];
const WATCHOS_PRIMERS: &[&str] = &[
    "design/human-interface-guidelines/complications",
    "design/human-interface-guidelines/notifications",
    "design/human-interface-guidelines/color",
];
const TVOS_PRIMERS: &[&str] = &[
    "design/human-interface-guidelines/focus-and-selection",
    "design/human-interface-guidelines/layout",
    "design/human-interface-guidelines/menus-and-actions",
];
const VISIONOS_PRIMERS: &[&str] = &[
    "design/human-interface-guidelines/visionos",
    "design/human-interface-guidelines/immersive-experiences",
    "design/human-interface-guidelines/color",
];

static PRIMER_MAPPINGS: &[PrimerMapping] = &[
    PrimerMapping {
        identifier_prefix: Some("doc://com.apple.documentation/documentation/SwiftUI"),
        title_keyword: Some("swiftui"),
        topics: SWIFTUI_PRIMERS,
    },
    PrimerMapping {
        identifier_prefix: Some("doc://com.apple.documentation/documentation/UIKit"),
        title_keyword: Some("uikit"),
        topics: UIKIT_PRIMERS,
    },
    PrimerMapping {
        identifier_prefix: Some("doc://com.apple.documentation/documentation/AppKit"),
        title_keyword: Some("appkit"),
        topics: APPKIT_PRIMERS,
    },
    PrimerMapping {
        identifier_prefix: Some("doc://com.apple.documentation/documentation/WatchKit"),
        title_keyword: Some("watch"),
        topics: WATCHOS_PRIMERS,
    },
    PrimerMapping {
        identifier_prefix: Some("doc://com.apple.documentation/documentation/TVMLKit"),
        title_keyword: Some("tvos"),
        topics: TVOS_PRIMERS,
    },
    PrimerMapping {
        identifier_prefix: Some("doc://com.apple.documentation/documentation/VisionOS"),
        title_keyword: Some("vision"),
        topics: VISIONOS_PRIMERS,
    },
];

static MAPPINGS: &[Mapping] = &[
    // Text components
    Mapping {
        path_prefix: "/documentation/swiftui/textfield",
        topics: TEXT_FIELD_TOPICS,
    },
    Mapping {
        path_prefix: "/documentation/swiftui/texteditor",
        topics: TEXT_FIELD_TOPICS,
    },
    Mapping {
        path_prefix: "/documentation/swiftui/securefield",
        topics: TEXT_FIELD_TOPICS,
    },
    Mapping {
        path_prefix: "/documentation/swiftui/text",
        topics: TEXT_TOPICS,
    },
    Mapping {
        path_prefix: "/documentation/swiftui/attributedstring",
        topics: TEXT_TOPICS,
    },
    Mapping {
        path_prefix: "/documentation/swiftui/label",
        topics: LABEL_TOPICS,
    },
    // List/Collection components
    Mapping {
        path_prefix: "/documentation/swiftui/list",
        topics: LIST_TOPICS,
    },
    Mapping {
        path_prefix: "/documentation/swiftui/outlinegroup",
        topics: LIST_TOPICS,
    },
    Mapping {
        path_prefix: "/documentation/swiftui/foreach",
        topics: LIST_TOPICS,
    },
    Mapping {
        path_prefix: "/documentation/swiftui/lazyvstack",
        topics: LIST_TOPICS,
    },
    Mapping {
        path_prefix: "/documentation/swiftui/lazyvgrid",
        topics: LIST_TOPICS,
    },
    Mapping {
        path_prefix: "/documentation/swiftui/lazyhgrid",
        topics: LIST_TOPICS,
    },
    Mapping {
        path_prefix: "/documentation/swiftui/grid",
        topics: LIST_TOPICS,
    },
    // Search components
    Mapping {
        path_prefix: "/documentation/swiftui/view/searchable",
        topics: SEARCH_TOPICS,
    },
    Mapping {
        path_prefix: "/documentation/swiftui/searchable",
        topics: SEARCH_TOPICS,
    },
    Mapping {
        path_prefix: "/documentation/swiftui/view/searchsuggestions",
        topics: SEARCH_TOPICS,
    },
    // Button/Control components
    Mapping {
        path_prefix: "/documentation/swiftui/button",
        topics: BUTTON_TOPICS,
    },
    Mapping {
        path_prefix: "/documentation/swiftui/editbutton",
        topics: BUTTON_TOPICS,
    },
    Mapping {
        path_prefix: "/documentation/swiftui/pastebutton",
        topics: BUTTON_TOPICS,
    },
    Mapping {
        path_prefix: "/documentation/swiftui/toggle",
        topics: TOGGLE_TOPICS,
    },
    Mapping {
        path_prefix: "/documentation/swiftui/link",
        topics: LINK_TOPICS,
    },
    // Picker/Selection components
    Mapping {
        path_prefix: "/documentation/swiftui/picker",
        topics: PICKER_TOPICS,
    },
    Mapping {
        path_prefix: "/documentation/swiftui/datepicker",
        topics: PICKER_TOPICS,
    },
    Mapping {
        path_prefix: "/documentation/swiftui/colorpicker",
        topics: PICKER_TOPICS,
    },
    Mapping {
        path_prefix: "/documentation/swiftui/multidatepicker",
        topics: PICKER_TOPICS,
    },
    // Menu components
    Mapping {
        path_prefix: "/documentation/swiftui/menu",
        topics: MENU_TOPICS,
    },
    Mapping {
        path_prefix: "/documentation/swiftui/contextmenu",
        topics: MENU_TOPICS,
    },
    Mapping {
        path_prefix: "/documentation/swiftui/view/contextmenu",
        topics: MENU_TOPICS,
    },
    // Modal/Sheet components
    Mapping {
        path_prefix: "/documentation/swiftui/view/sheet",
        topics: SHEET_TOPICS,
    },
    Mapping {
        path_prefix: "/documentation/swiftui/view/fullscreencover",
        topics: SHEET_TOPICS,
    },
    Mapping {
        path_prefix: "/documentation/swiftui/view/popover",
        topics: POPOVER_TOPICS,
    },
    Mapping {
        path_prefix: "/documentation/swiftui/view/alert",
        topics: ALERT_TOPICS,
    },
    Mapping {
        path_prefix: "/documentation/swiftui/view/confirmationdialog",
        topics: ALERT_TOPICS,
    },
    // Progress/Feedback components
    Mapping {
        path_prefix: "/documentation/swiftui/progressview",
        topics: PROGRESS_TOPICS,
    },
    Mapping {
        path_prefix: "/documentation/swiftui/gauge",
        topics: PROGRESS_TOPICS,
    },
    // Input components
    Mapping {
        path_prefix: "/documentation/swiftui/slider",
        topics: SLIDER_TOPICS,
    },
    Mapping {
        path_prefix: "/documentation/swiftui/stepper",
        topics: STEPPER_TOPICS,
    },
    // Navigation components
    Mapping {
        path_prefix: "/documentation/swiftui/tabview",
        topics: TAB_TOPICS,
    },
    Mapping {
        path_prefix: "/documentation/swiftui/navigationsplitview",
        topics: SPLIT_TOPICS,
    },
    Mapping {
        path_prefix: "/documentation/swiftui/navigationstack",
        topics: NAVIGATION_TOPICS,
    },
    Mapping {
        path_prefix: "/documentation/swiftui/navigationview",
        topics: NAVIGATION_TOPICS,
    },
    Mapping {
        path_prefix: "/documentation/swiftui/navigationlink",
        topics: NAVIGATION_TOPICS,
    },
    Mapping {
        path_prefix: "/documentation/swiftui/view/toolbar",
        topics: TOOLBAR_TOPICS,
    },
    Mapping {
        path_prefix: "/documentation/swiftui/toolbaritem",
        topics: TOOLBAR_TOPICS,
    },
    // Layout components
    Mapping {
        path_prefix: "/documentation/swiftui/vstack",
        topics: STACK_TOPICS,
    },
    Mapping {
        path_prefix: "/documentation/swiftui/hstack",
        topics: STACK_TOPICS,
    },
    Mapping {
        path_prefix: "/documentation/swiftui/zstack",
        topics: STACK_TOPICS,
    },
    Mapping {
        path_prefix: "/documentation/swiftui/scrollview",
        topics: SCROLL_TOPICS,
    },
    Mapping {
        path_prefix: "/documentation/swiftui/scrollviewreader",
        topics: SCROLL_TOPICS,
    },
    Mapping {
        path_prefix: "/documentation/swiftui/form",
        topics: FORM_TOPICS,
    },
    // Image components
    Mapping {
        path_prefix: "/documentation/swiftui/image",
        topics: IMAGE_TOPICS,
    },
    Mapping {
        path_prefix: "/documentation/swiftui/asyncimage",
        topics: IMAGE_TOPICS,
    },
    // Accessibility
    Mapping {
        path_prefix: "/documentation/swiftui/view/accessibility",
        topics: ACCESSIBILITY_TOPICS,
    },
    Mapping {
        path_prefix: "/documentation/swiftui/accessibilitylabel",
        topics: ACCESSIBILITY_TOPICS,
    },
    // Color
    Mapping {
        path_prefix: "/documentation/swiftui/color",
        topics: COLOR_TOPICS,
    },
    Mapping {
        path_prefix: "/documentation/swiftui/view/foregroundstyle",
        topics: COLOR_TOPICS,
    },
    Mapping {
        path_prefix: "/documentation/swiftui/view/backgroundstyle",
        topics: COLOR_TOPICS,
    },
    // Gesture
    Mapping {
        path_prefix: "/documentation/swiftui/gesture",
        topics: GESTURE_TOPICS,
    },
    Mapping {
        path_prefix: "/documentation/swiftui/tapgesture",
        topics: GESTURE_TOPICS,
    },
    Mapping {
        path_prefix: "/documentation/swiftui/draggesture",
        topics: GESTURE_TOPICS,
    },
    Mapping {
        path_prefix: "/documentation/swiftui/longpressgesture",
        topics: GESTURE_TOPICS,
    },
    // Animation
    Mapping {
        path_prefix: "/documentation/swiftui/animation",
        topics: ANIMATION_TOPICS,
    },
    Mapping {
        path_prefix: "/documentation/swiftui/view/animation",
        topics: ANIMATION_TOPICS,
    },
    Mapping {
        path_prefix: "/documentation/swiftui/withanimation",
        topics: ANIMATION_TOPICS,
    },
    Mapping {
        path_prefix: "/documentation/swiftui/transition",
        topics: ANIMATION_TOPICS,
    },
];

pub async fn guidance_for(
    context: &AppContext,
    title: &str,
    path: &str,
) -> Result<Vec<DesignSection>> {
    let topics = topics_for(path, title);
    if topics.is_empty() {
        return Ok(Vec::new());
    }

    let mut sections = Vec::new();
    for slug in topics {
        if let Some(section) = fetch_or_load(context, slug).await? {
            sections.push(section);
        }
    }
    Ok(sections)
}

/// Pre-cache design guidance for a technology during framework load
/// This populates both the global CACHE and the ServerState cache for fast lookups
pub async fn precache_for_technology(
    context: &AppContext,
    technology: &Technology,
) -> Result<()> {
    let topics = primer_topics_for_technology(technology);
    if topics.is_empty() {
        return Ok(());
    }

    // Pre-fetch all primer topics and populate both caches
    for slug in topics {
        if let Some(section) = fetch_or_load(context, slug).await? {
            // Insert into ServerState cache for O(1) lookups
            context
                .state
                .design_guidance_cache
                .write()
                .await
                .insert(slug.to_string(), Arc::new(section));
        }
    }

    Ok(())
}

pub async fn primers_for_technology(
    context: &AppContext,
    technology: &Technology,
) -> Result<Vec<DesignSection>> {
    let topics = primer_topics_for_technology(technology);
    if topics.is_empty() {
        return Ok(Vec::new());
    }

    let mut sections = Vec::new();
    for slug in topics {
        if let Some(section) = fetch_or_load(context, slug).await? {
            sections.push(section);
        }
    }
    Ok(sections)
}

pub fn has_primer_mapping(technology: &Technology) -> bool {
    !primer_topics_for_technology(technology).is_empty()
}

/// Check if a technology title has primer mappings (for relevance scoring)
pub fn has_primer_mapping_by_title(title_lower: &str) -> bool {
    PRIMER_MAPPINGS.iter().any(|mapping| {
        mapping
            .title_keyword
            .map(|keyword| title_lower.contains(keyword))
            .unwrap_or(false)
    })
}

fn topics_for(path: &str, title: &str) -> Vec<&'static str> {
    let normalized_path = path.to_ascii_lowercase();
    let mut matches = Vec::new();

    for mapping in MAPPINGS {
        if normalized_path.starts_with(mapping.path_prefix) {
            matches.extend_from_slice(mapping.topics);
        }
    }

    // Title-based fallback matching for components not matched by path
    if matches.is_empty() {
        let lowered_title = title.to_ascii_lowercase();

        // Text input components
        if lowered_title.contains("text field")
            || lowered_title.contains("textfield")
            || lowered_title.contains("securefield")
            || lowered_title.contains("texteditor")
        {
            matches.extend_from_slice(TEXT_FIELD_TOPICS);
        } else if lowered_title.contains("text")
            || lowered_title.contains("font")
            || lowered_title.contains("typography")
        {
            matches.extend_from_slice(TEXT_TOPICS);
        } else if lowered_title.contains("label") {
            matches.extend_from_slice(LABEL_TOPICS);
        }

        // List/Collection components
        if lowered_title.contains("list")
            || lowered_title.contains("table")
            || lowered_title.contains("grid")
            || lowered_title.contains("foreach")
            || lowered_title.contains("collection")
        {
            matches.extend_from_slice(LIST_TOPICS);
        }

        // Search
        if lowered_title.contains("search") || lowered_title.contains("filter") {
            matches.extend_from_slice(SEARCH_TOPICS);
        }

        // Buttons and controls
        if lowered_title.contains("button") || lowered_title.contains("action") {
            matches.extend_from_slice(BUTTON_TOPICS);
        }
        if lowered_title.contains("toggle") || lowered_title.contains("switch") {
            matches.extend_from_slice(TOGGLE_TOPICS);
        }
        if lowered_title.contains("link") && !lowered_title.contains("navigation") {
            matches.extend_from_slice(LINK_TOPICS);
        }

        // Pickers
        if lowered_title.contains("picker")
            || lowered_title.contains("datepicker")
            || lowered_title.contains("colorpicker")
        {
            matches.extend_from_slice(PICKER_TOPICS);
        }

        // Menus
        if lowered_title.contains("menu") || lowered_title.contains("contextmenu") {
            matches.extend_from_slice(MENU_TOPICS);
        }

        // Modal presentations
        if lowered_title.contains("sheet") || lowered_title.contains("fullscreen") {
            matches.extend_from_slice(SHEET_TOPICS);
        }
        if lowered_title.contains("alert") || lowered_title.contains("dialog") {
            matches.extend_from_slice(ALERT_TOPICS);
        }
        if lowered_title.contains("popover") || lowered_title.contains("tooltip") {
            matches.extend_from_slice(POPOVER_TOPICS);
        }

        // Progress and feedback
        if lowered_title.contains("progress")
            || lowered_title.contains("gauge")
            || lowered_title.contains("indicator")
        {
            matches.extend_from_slice(PROGRESS_TOPICS);
        }

        // Input controls
        if lowered_title.contains("slider") || lowered_title.contains("range") {
            matches.extend_from_slice(SLIDER_TOPICS);
        }
        if lowered_title.contains("stepper") || lowered_title.contains("increment") {
            matches.extend_from_slice(STEPPER_TOPICS);
        }

        // Navigation
        if lowered_title.contains("tab") || lowered_title.contains("tabview") {
            matches.extend_from_slice(TAB_TOPICS);
        }
        if lowered_title.contains("split") || lowered_title.contains("column") {
            matches.extend_from_slice(SPLIT_TOPICS);
        }
        if lowered_title.contains("navigation")
            || lowered_title.contains("router")
            || lowered_title.contains("stack")
        {
            matches.extend_from_slice(NAVIGATION_TOPICS);
        }
        if lowered_title.contains("toolbar") || lowered_title.contains("bar") {
            matches.extend_from_slice(TOOLBAR_TOPICS);
        }

        // Layout
        if lowered_title.contains("scroll") || lowered_title.contains("scrollview") {
            matches.extend_from_slice(SCROLL_TOPICS);
        }
        if lowered_title.contains("form") || lowered_title.contains("settings") {
            matches.extend_from_slice(FORM_TOPICS);
        }
        if lowered_title.contains("stack")
            || lowered_title.contains("vstack")
            || lowered_title.contains("hstack")
            || lowered_title.contains("zstack")
        {
            matches.extend_from_slice(STACK_TOPICS);
        }

        // Images
        if lowered_title.contains("image")
            || lowered_title.contains("photo")
            || lowered_title.contains("icon")
        {
            matches.extend_from_slice(IMAGE_TOPICS);
        }

        // Color
        if lowered_title.contains("color")
            || lowered_title.contains("tint")
            || lowered_title.contains("foreground")
            || lowered_title.contains("background")
        {
            matches.extend_from_slice(COLOR_TOPICS);
        }

        // Gestures
        if lowered_title.contains("gesture")
            || lowered_title.contains("tap")
            || lowered_title.contains("drag")
            || lowered_title.contains("swipe")
        {
            matches.extend_from_slice(GESTURE_TOPICS);
        }

        // Animation
        if lowered_title.contains("animation")
            || lowered_title.contains("transition")
            || lowered_title.contains("motion")
        {
            matches.extend_from_slice(ANIMATION_TOPICS);
        }

        // Accessibility
        if lowered_title.contains("accessibility")
            || lowered_title.contains("voiceover")
            || lowered_title.contains("a11y")
        {
            matches.extend_from_slice(ACCESSIBILITY_TOPICS);
        }
    }

    // General fallback for SwiftUI paths with no specific match
    if matches.is_empty() && normalized_path.contains("/swiftui/") {
        matches.extend_from_slice(GENERAL_FOUNDATION_TOPICS);
    }

    matches.sort_unstable();
    matches.dedup();
    matches
}

fn primer_topics_for_technology(technology: &Technology) -> Vec<&'static str> {
    let identifier = technology.identifier.to_ascii_lowercase();
    let title = technology.title.to_ascii_lowercase();
    let mut matches = Vec::new();

    for mapping in PRIMER_MAPPINGS {
        let id_match = mapping
            .identifier_prefix
            .map(|prefix| identifier.starts_with(&prefix.to_ascii_lowercase()))
            .unwrap_or(false);
        let title_match = mapping
            .title_keyword
            .map(|keyword| title.contains(keyword))
            .unwrap_or(false);
        if id_match || title_match {
            matches.extend_from_slice(mapping.topics);
        }
    }

    matches.sort_unstable();
    matches.dedup();
    matches
}

async fn fetch_or_load(context: &AppContext, slug: &'static str) -> Result<Option<DesignSection>> {
    // Check ServerState cache first (O(1) lookup for pre-cached data)
    if let Some(cached) = context.state.design_guidance_cache.read().await.get(slug) {
        return Ok(Some((**cached).clone()));
    }

    // Check global CACHE second
    if let Some(cached) = CACHE.read().await.get(slug).cloned() {
        return Ok(Some((*cached).clone()));
    }

    // Fetch from network if not cached
    let value = match context.client.load_document(slug).await {
        Ok(value) => value,
        Err(error) => {
            tracing::warn!(%slug, "failed to load design guidance: {error:?}");
            return Ok(None);
        }
    };

    let parsed = match parse_guidance(slug, &value)? {
        Some(section) => section,
        None => return Ok(None),
    };

    let arc = Arc::new(parsed);
    // Populate both caches
    CACHE.write().await.insert(slug.to_string(), arc.clone());
    context
        .state
        .design_guidance_cache
        .write()
        .await
        .insert(slug.to_string(), arc.clone());
    Ok(Some((*arc).clone()))
}

fn parse_guidance(slug: &str, value: &Value) -> Result<Option<DesignSection>> {
    let metadata = value
        .get("metadata")
        .and_then(Value::as_object)
        .context("missing metadata in design document")?;
    let title = metadata
        .get("title")
        .and_then(Value::as_str)
        .unwrap_or("Design guidance")
        .to_string();

    let abstract_summary = value
        .get("abstract")
        .and_then(Value::as_array)
        .map(|segments| flatten_rich_text(segments));
    let normalized_summary = abstract_summary
        .as_ref()
        .filter(|summary| !summary.trim().is_empty())
        .map(|summary| abbreviate(summary));

    let mut bullets = Vec::new();
    let mut seen = HashSet::new();

    if let Some(summary) = normalized_summary.as_ref() {
        bullets.push(DesignBullet {
            category: "Overview",
            text: summary.clone(),
        });
        if let Some(original) = abstract_summary.as_ref() {
            seen.insert(original.trim().to_string());
        }
    }

    if let Some(sections) = value
        .get("primaryContentSections")
        .and_then(Value::as_array)
    {
        for section in sections {
            if let Some(content) = section.get("content").and_then(Value::as_array) {
                for item in content {
                    if bullets.len() >= 8 {
                        break;
                    }
                    if let Some(bullet) = extract_bullet(item) {
                        if seen.insert(bullet.text.clone()) {
                            bullets.push(bullet);
                        }
                    }
                }
            }
        }
    }

    if bullets.is_empty() {
        return Ok(None);
    }

    Ok(Some(DesignSection {
        slug: slug.to_string(),
        url: format!("/{}", slug),
        title,
        summary: normalized_summary,
        bullets,
    }))
}

fn extract_bullet(item: &Value) -> Option<DesignBullet> {
    let r#type = item.get("type").and_then(Value::as_str)?;
    if r#type != "paragraph" {
        return None;
    }
    let inline = item.get("inlineContent").and_then(Value::as_array)?;
    if inline.is_empty() {
        return None;
    }

    let mut headline = String::new();
    let mut detail_segments = Vec::new();

    for node in inline {
        match node.get("type").and_then(Value::as_str).unwrap_or_default() {
            "strong" => {
                if headline.is_empty() {
                    headline = flatten_inline(node.get("inlineContent"));
                } else {
                    detail_segments.push(flatten_inline(node.get("inlineContent")));
                }
            }
            "text" => {
                if let Some(text) = node.get("text").and_then(Value::as_str) {
                    detail_segments.push(text.to_string());
                }
            }
            "reference" => {
                if let Some(text) = reference_label(node) {
                    detail_segments.push(text);
                }
            }
            "inlineGroup" | "inlineContainer" => {
                detail_segments.push(flatten_inline(node.get("inlineContent")));
            }
            _ => {}
        }
    }

    if headline.is_empty() {
        return None;
    }

    let detail = detail_segments.join("").replace("  ", " ");
    let detail_trimmed = detail.trim();
    let text = if detail_trimmed.is_empty() {
        headline.clone()
    } else if headline.ends_with('.') || headline.ends_with(':') {
        format!("{headline} {detail_trimmed}")
    } else {
        format!("{headline} — {detail_trimmed}")
    };

    let normalized = text.trim();
    if normalized.is_empty() {
        return None;
    }

    Some(DesignBullet {
        category: categorize(normalized),
        text: abbreviate(normalized),
    })
}

fn flatten_rich_text(segments: &[Value]) -> String {
    let mut parts = Vec::new();
    for segment in segments {
        if let Some(text) = segment.get("text").and_then(Value::as_str) {
            parts.push(text.to_string());
        }
    }
    parts.join(" ")
}

fn flatten_inline(content: Option<&Value>) -> String {
    let mut parts = Vec::new();
    match content {
        Some(Value::Array(items)) => {
            for item in items {
                if let Some(kind) = item.get("type").and_then(Value::as_str) {
                    match kind {
                        "text" => {
                            if let Some(text) = item.get("text").and_then(Value::as_str) {
                                parts.push(text.to_string());
                            }
                        }
                        "reference" => {
                            if let Some(label) = reference_label(item) {
                                parts.push(label);
                            }
                        }
                        _ => parts.push(flatten_inline(item.get("inlineContent"))),
                    }
                }
            }
        }
        Some(Value::Object(map)) => {
            if let Some(kind) = map.get("type").and_then(Value::as_str) {
                if kind == "text" {
                    if let Some(text) = map.get("text").and_then(Value::as_str) {
                        parts.push(text.to_string());
                    }
                } else {
                    parts.push(flatten_inline(map.get("inlineContent")));
                }
            }
        }
        Some(Value::String(text)) => parts.push(text.clone()),
        _ => {}
    }
    parts.join("")
}

fn reference_label(node: &Value) -> Option<String> {
    if let Some(text) = node.get("text").and_then(Value::as_str) {
        if !text.trim().is_empty() {
            return Some(text.to_string());
        }
    }
    if let Some(inline) = node.get("inlineContent") {
        let flattened = flatten_inline(Some(inline));
        if !flattened.trim().is_empty() {
            return Some(flattened);
        }
    }
    if let Some(identifier) = node.get("identifier").and_then(Value::as_str) {
        if let Some(last) = identifier.split('/').next_back() {
            if !last.is_empty() {
                let label = last
                    .replace(['-', '_'], " ")
                    .replace(".html", "")
                    .trim()
                    .to_string();
                if !label.is_empty() {
                    return Some(label);
                }
            }
        }
    }
    None
}

fn categorize(text: &str) -> &'static str {
    let lower = text.to_ascii_lowercase();
    if lower.contains("color") || lower.contains("contrast") || lower.contains("palette") {
        "Color"
    } else if lower.contains("type") || lower.contains("font") || lower.contains("typography") {
        "Typography"
    } else if lower.contains("focus")
        || lower.contains("voiceover")
        || lower.contains("accessibility")
        || lower.contains("assistive")
    {
        "Accessibility"
    } else if lower.contains("layout")
        || lower.contains("spacing")
        || lower.contains("margin")
        || lower.contains("alignment")
    {
        "Layout"
    } else if lower.contains("interaction")
        || lower.contains("tap")
        || lower.contains("gesture")
        || lower.contains("feedback")
    {
        "Interaction"
    } else {
        "Design"
    }
}

fn abbreviate(text: &str) -> String {
    const MAX_LEN: usize = 220;
    let trimmed = text.trim();
    if trimmed.len() <= MAX_LEN {
        return trimmed.to_string();
    }
    let mut truncated = trimmed[..MAX_LEN].to_string();
    if let Some(last_space) = truncated.rfind(' ') {
        truncated.truncate(last_space);
    }
    truncated.push('…');
    truncated
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::AppContext;
    use docs_mcp_client::{AppleDocsClient, ClientConfig};
    use time::Duration;

    #[tokio::test]
    async fn typography_guidance_is_available() {
        let cache_dir = tempfile::tempdir().expect("tempdir");
        let client = AppleDocsClient::with_config(ClientConfig {
            cache_dir: cache_dir.path().to_path_buf(),
            memory_cache_ttl: Duration::minutes(5),
        });
        let context = AppContext::new(client);
        let sections = guidance_for(&context, "Text", "/documentation/swiftui/text")
            .await
            .expect("guidance lookup");
        assert!(
            !sections.is_empty(),
            "expected typography guidance for Text symbol"
        );
    }
}
