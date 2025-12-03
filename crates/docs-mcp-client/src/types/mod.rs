pub mod models;

pub use models::{
    CacheEntry, FrameworkData, FrameworkMetadata, PlatformInfo, ReferenceData, RichText,
    SearchResult, SymbolData, SymbolMetadata, Technology, TopicData, TopicMetadata, TopicSection,
};

pub fn extract_text(segments: &[RichText]) -> String {
    segments
        .iter()
        .filter_map(|item| item.text.as_deref())
        .collect()
}

pub fn format_platforms(platforms: &[PlatformInfo]) -> String {
    if platforms.is_empty() {
        return "All platforms".to_string();
    }

    platforms
        .iter()
        .map(|platform| {
            let mut text = platform.name.clone();
            if let Some(introduced) = &platform.introduced_at {
                text.push(' ');
                text.push_str(introduced);
            }
            if platform.beta {
                text.push_str(" (Beta)");
            }
            text
        })
        .collect::<Vec<_>>()
        .join(", ")
}
