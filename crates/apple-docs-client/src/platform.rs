use serde::{Deserialize, Serialize};
use std::fmt;

/// Represents the documentation platform/ecosystem
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DocsPlatform {
    /// Apple Developer Documentation (iOS, macOS, watchOS, tvOS, visionOS)
    Apple,
    /// Android Developer Documentation (Jetpack, Compose, AndroidX)
    Android,
    /// Flutter/Dart Documentation
    Flutter,
}

impl DocsPlatform {
    /// Returns all available platforms
    pub fn all() -> &'static [DocsPlatform] {
        &[DocsPlatform::Apple, DocsPlatform::Android, DocsPlatform::Flutter]
    }

    /// Returns the display name for the platform
    pub fn display_name(&self) -> &'static str {
        match self {
            DocsPlatform::Apple => "Apple",
            DocsPlatform::Android => "Android",
            DocsPlatform::Flutter => "Flutter",
        }
    }

    /// Returns the base URL for the platform's documentation
    pub fn base_url(&self) -> &'static str {
        match self {
            DocsPlatform::Apple => "https://developer.apple.com/documentation",
            DocsPlatform::Android => "https://developer.android.com/reference",
            DocsPlatform::Flutter => "https://api.flutter.dev/flutter",
        }
    }

    /// Returns a brief description of the platform
    pub fn description(&self) -> &'static str {
        match self {
            DocsPlatform::Apple => "iOS, macOS, watchOS, tvOS, and visionOS development with Swift and SwiftUI",
            DocsPlatform::Android => "Android development with Kotlin, Jetpack Compose, and AndroidX libraries",
            DocsPlatform::Flutter => "Cross-platform development with Flutter and Dart",
        }
    }

    /// Returns the primary programming languages for the platform
    pub fn languages(&self) -> &'static [&'static str] {
        match self {
            DocsPlatform::Apple => &["Swift", "Objective-C"],
            DocsPlatform::Android => &["Kotlin", "Java"],
            DocsPlatform::Flutter => &["Dart"],
        }
    }

    /// Parse platform from string (case-insensitive)
    pub fn from_str_loose(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "apple" | "ios" | "macos" | "swiftui" | "swift" | "uikit" | "appkit" => Some(DocsPlatform::Apple),
            "android" | "kotlin" | "jetpack" | "compose" | "androidx" => Some(DocsPlatform::Android),
            "flutter" | "dart" => Some(DocsPlatform::Flutter),
            _ => None,
        }
    }
}

impl fmt::Display for DocsPlatform {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.display_name())
    }
}

impl Default for DocsPlatform {
    fn default() -> Self {
        DocsPlatform::Apple
    }
}

/// Unified search result that works across all platforms
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedSearchResult {
    /// The platform this result is from
    pub platform: DocsPlatform,
    /// Display name of the item
    pub name: String,
    /// Fully qualified name or path
    pub qualified_name: String,
    /// URL or path to the documentation
    pub href: String,
    /// Type of item (class, function, library, etc.)
    pub kind: Option<String>,
    /// Brief description
    pub description: Option<String>,
    /// Parent/enclosing element if applicable
    pub parent: Option<String>,
    /// Relevance score for search ranking
    pub score: i32,
}

/// Unified technology/library representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedTechnology {
    /// The platform this technology is from
    pub platform: DocsPlatform,
    /// Identifier for the technology
    pub identifier: String,
    /// Display name
    pub name: String,
    /// Brief description
    pub description: Option<String>,
    /// URL to documentation
    pub url: String,
    /// Category or group
    pub category: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn platform_from_str_loose_works() {
        assert_eq!(DocsPlatform::from_str_loose("iOS"), Some(DocsPlatform::Apple));
        assert_eq!(DocsPlatform::from_str_loose("kotlin"), Some(DocsPlatform::Android));
        assert_eq!(DocsPlatform::from_str_loose("DART"), Some(DocsPlatform::Flutter));
        assert_eq!(DocsPlatform::from_str_loose("unknown"), None);
    }

    #[test]
    fn all_platforms_returns_three() {
        assert_eq!(DocsPlatform::all().len(), 3);
    }
}
