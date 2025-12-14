use serde::{Deserialize, Serialize};

/// Represents a Rust crate (either std library or docs.rs crate)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RustCrate {
    pub name: String,
    pub version: String,
    pub description: String,
    pub documentation_url: String,
    pub repository_url: Option<String>,
    /// True for std, core, alloc
    pub is_std: bool,
}

/// Type of Rust item (maps to rustdoc types)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RustItemKind {
    Module,
    Struct,
    Enum,
    Trait,
    Function,
    Method,
    Type,
    Constant,
    Static,
    Macro,
    Derive,
    Primitive,
    ExternCrate,
    Import,
    Union,
    Typedef,
    AssocType,
    AssocConst,
    TraitAlias,
}

impl RustItemKind {
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Module => "module",
            Self::Struct => "struct",
            Self::Enum => "enum",
            Self::Trait => "trait",
            Self::Function => "fn",
            Self::Method => "method",
            Self::Type => "type",
            Self::Constant => "constant",
            Self::Static => "static",
            Self::Macro => "macro",
            Self::Derive => "derive",
            Self::Primitive => "primitive",
            Self::ExternCrate => "externcrate",
            Self::Import => "import",
            Self::Union => "union",
            Self::Typedef => "typedef",
            Self::AssocType => "associatedtype",
            Self::AssocConst => "associatedconstant",
            Self::TraitAlias => "traitalias",
        }
    }

    /// Parse from rustdoc search index type number
    #[must_use]
    pub fn from_type_id(id: u8) -> Option<Self> {
        // Based on rustdoc search index format
        match id {
            0 => Some(Self::Module),
            1 => Some(Self::ExternCrate),
            2 => Some(Self::Import),
            3 => Some(Self::Struct),
            4 => Some(Self::Enum),
            5 => Some(Self::Function),
            6 => Some(Self::Type),
            7 => Some(Self::Static),
            8 => Some(Self::Trait),
            9 => Some(Self::Typedef),
            10 => Some(Self::Method),
            11 => Some(Self::Macro),
            12 => Some(Self::Primitive),
            13 => Some(Self::AssocType),
            14 => Some(Self::Constant),
            15 => Some(Self::AssocConst),
            16 => Some(Self::Union),
            17 => Some(Self::TraitAlias),
            18 => Some(Self::Derive),
            _ => None,
        }
    }
}

#[must_use]
pub fn rustdoc_item_url(crate_name: &str, crate_version: &str, path: &str, kind: RustItemKind) -> String {
    let path_parts: Vec<&str> = path.split("::").collect();
    let segments = if path_parts.len() > 1 {
        &path_parts[1..]
    } else {
        &[][..]
    };

    let is_std = STD_CRATES.iter().any(|(name, _)| *name == crate_name);
    let base = if is_std {
        format!("https://doc.rust-lang.org/{crate_name}")
    } else {
        format!("https://docs.rs/{crate_name}/{crate_version}/{crate_name}")
    };

    if segments.is_empty() {
        return format!("{base}/index.html");
    }

    if kind == RustItemKind::Module {
        let module_path = segments.join("/");
        if module_path.is_empty() {
            format!("{base}/index.html")
        } else {
            format!("{base}/{module_path}/index.html")
        }
    } else {
        let item_name = segments.last().unwrap_or(&"");
        let module_path = if segments.len() > 1 {
            segments[..segments.len() - 1].join("/")
        } else {
            String::new()
        };

        let prefix = match kind {
            RustItemKind::Struct => Some("struct."),
            RustItemKind::Enum => Some("enum."),
            RustItemKind::Trait => Some("trait."),
            RustItemKind::Function => Some("fn."),
            RustItemKind::Type | RustItemKind::Typedef => Some("type."),
            RustItemKind::Constant | RustItemKind::AssocConst => Some("constant."),
            RustItemKind::Static => Some("static."),
            RustItemKind::Macro => Some("macro."),
            RustItemKind::Derive => Some("derive."),
            RustItemKind::Primitive => Some("primitive."),
            RustItemKind::Union => Some("union."),
            RustItemKind::TraitAlias => Some("traitalias."),
            RustItemKind::Module
            | RustItemKind::Method
            | RustItemKind::ExternCrate
            | RustItemKind::Import
            | RustItemKind::AssocType => None,
        };

        let file_name = match prefix {
            Some(prefix) => format!("{prefix}{item_name}.html"),
            None => "index.html".to_string(),
        };

        if module_path.is_empty() {
            format!("{base}/{file_name}")
        } else {
            format!("{base}/{module_path}/{file_name}")
        }
    }
}

impl std::fmt::Display for RustItemKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// A searchable item within a crate
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RustItem {
    pub name: String,
    /// Full path (e.g., "std::collections::HashMap")
    pub path: String,
    pub kind: RustItemKind,
    /// Brief summary (from search index)
    pub summary: String,
    pub crate_name: String,
    pub crate_version: String,
    pub url: String,
    /// Full type declaration/signature (e.g., `pub struct HashMap<K, V, S = RandomState>`)
    pub declaration: Option<String>,
    /// Full documentation text (parsed from HTML)
    pub documentation: Option<String>,
    /// Code examples extracted from documentation
    pub examples: Vec<RustExample>,
    /// Methods and associated functions (for structs, enums, traits)
    pub methods: Vec<RustMethodInfo>,
    /// Trait implementations (for structs, enums)
    pub impl_traits: Vec<String>,
    /// Associated types (for traits)
    pub associated_types: Vec<RustAssociatedType>,
    /// Link to source code
    pub source_url: Option<String>,
    /// Whether rich documentation has been fetched
    pub is_detailed: bool,
}

/// A code example from Rust documentation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RustExample {
    pub code: String,
    pub description: Option<String>,
}

/// Method or associated function information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RustMethodInfo {
    pub name: String,
    pub signature: String,
    pub summary: String,
    pub is_unsafe: bool,
    pub is_const: bool,
    pub is_async: bool,
}

/// Associated type in a trait
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RustAssociatedType {
    pub name: String,
    pub bounds: Option<String>,
    pub default: Option<String>,
}

/// Technology representation for tool integration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RustTechnology {
    /// Identifier like "rust:std", "rust:serde"
    pub identifier: String,
    pub title: String,
    pub description: String,
    pub url: String,
    /// Number of items in the crate
    pub item_count: usize,
    /// Crate metadata
    pub crate_info: RustCrate,
}

/// Category/module listing within a crate
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RustCategory {
    pub identifier: String,
    pub title: String,
    pub description: String,
    pub items: Vec<RustCategoryItem>,
}

/// Item in a category listing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RustCategoryItem {
    pub name: String,
    pub description: String,
    pub kind: RustItemKind,
    pub path: String,
    pub url: String,
}

/// Search index for a crate (parsed from search-index.js)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RustSearchIndex {
    pub crate_name: String,
    pub crate_version: String,
    pub items: Vec<RustSearchIndexEntry>,
}

/// Entry in the search index
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RustSearchIndexEntry {
    pub name: String,
    pub path: String,
    pub kind: RustItemKind,
    pub desc: String,
    /// Parent type for methods/associated items
    pub parent: Option<String>,
}

/// docs.rs crate search result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocsRsSearchResult {
    pub name: String,
    pub version: String,
    pub description: Option<String>,
    pub downloads: u64,
}

/// docs.rs releases search response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocsRsReleasesResponse {
    pub results: Vec<DocsRsRelease>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocsRsRelease {
    pub name: String,
    pub version: String,
    pub description: Option<String>,
    #[serde(default)]
    pub target_name: Option<String>,
    #[serde(default)]
    pub rustdoc_status: bool,
}

/// docs.rs crate data response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocsRsCrateData {
    pub name: String,
    pub version: String,
    pub description: Option<String>,
    pub repository: Option<String>,
    pub documentation: Option<String>,
    #[serde(default)]
    pub keywords: Vec<String>,
    #[serde(default)]
    pub releases: Vec<DocsRsCrateRelease>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocsRsCrateRelease {
    pub version: String,
    #[serde(default)]
    pub build_status: bool,
    pub yanked: bool,
}

impl RustItem {
    /// Create a RustItem from a search index entry (minimal data, not detailed)
    pub fn from_search_entry(
        entry: &RustSearchIndexEntry,
        crate_name: &str,
        crate_version: &str,
    ) -> Self {
        let full_path = if entry.path.is_empty() {
            format!("{}::{}", crate_name, entry.name)
        } else {
            format!("{}::{}::{}", crate_name, entry.path, entry.name)
        };

        let url = rustdoc_item_url(crate_name, crate_version, &full_path, entry.kind);

        Self {
            name: entry.name.clone(),
            path: full_path,
            kind: entry.kind,
            summary: entry.desc.clone(),
            crate_name: crate_name.to_string(),
            crate_version: crate_version.to_string(),
            url,
            declaration: None,
            documentation: None,
            examples: Vec::new(),
            methods: Vec::new(),
            impl_traits: Vec::new(),
            associated_types: Vec::new(),
            source_url: None,
            is_detailed: false,
        }
    }

    /// Create an empty RustItem for error cases
    pub fn empty(name: &str, crate_name: &str) -> Self {
        Self {
            name: name.to_string(),
            path: format!("{}::{}", crate_name, name),
            kind: RustItemKind::Struct,
            summary: String::new(),
            crate_name: crate_name.to_string(),
            crate_version: "latest".to_string(),
            url: String::new(),
            declaration: None,
            documentation: None,
            examples: Vec::new(),
            methods: Vec::new(),
            impl_traits: Vec::new(),
            associated_types: Vec::new(),
            source_url: None,
            is_detailed: false,
        }
    }
}

impl RustTechnology {
    /// Create a RustTechnology from crate info
    pub fn from_crate(crate_info: RustCrate, item_count: usize) -> Self {
        Self {
            identifier: format!("rust:{}", crate_info.name),
            title: if crate_info.is_std {
                format!("Rust {} Library", crate_info.name)
            } else {
                format!("{} (Rust crate)", crate_info.name)
            },
            description: crate_info.description.clone(),
            url: crate_info.documentation_url.clone(),
            item_count,
            crate_info,
        }
    }
}

/// Standard library crates that are always available
pub const STD_CRATES: &[(&str, &str)] = &[
    ("std", "The Rust Standard Library - fundamental abstractions for programs"),
    ("core", "The Rust Core Library - dependency-free foundational types"),
    ("alloc", "The Rust Allocation Library - heap allocation abstractions"),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rustdoc_item_url_docs_rs_function_in_module() {
        let url = rustdoc_item_url("tokio", "1.47.2", "tokio::task::spawn", RustItemKind::Function);
        assert_eq!(url, "https://docs.rs/tokio/1.47.2/tokio/task/fn.spawn.html");
    }

    #[test]
    fn test_rustdoc_item_url_docs_rs_module() {
        let url = rustdoc_item_url("tokio", "1.47.2", "tokio::task", RustItemKind::Module);
        assert_eq!(url, "https://docs.rs/tokio/1.47.2/tokio/task/index.html");
    }

    #[test]
    fn test_rustdoc_item_url_std_function() {
        let url = rustdoc_item_url("std", "latest", "std::thread::spawn", RustItemKind::Function);
        assert_eq!(url, "https://doc.rust-lang.org/std/thread/fn.spawn.html");
    }

    #[test]
    fn test_rustdoc_item_url_derive_macro() {
        let url = rustdoc_item_url("serde", "1.0.197", "serde::Serialize", RustItemKind::Derive);
        assert_eq!(url, "https://docs.rs/serde/1.0.197/serde/derive.Serialize.html");
    }
}
