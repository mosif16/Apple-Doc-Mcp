use serde::{Deserialize, Serialize};

/// GitHub contents API response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubContent {
    pub name: String,
    pub path: String,
    #[serde(rename = "type")]
    pub content_type: String, // "file" or "dir"
    pub sha: String,
    #[serde(default)]
    pub size: usize,
    pub url: String,
    pub html_url: String,
    #[serde(default)]
    pub download_url: Option<String>,
}

/// Normalized technology representation for Cocoon
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CocoonTechnology {
    pub identifier: String,
    pub title: String,
    pub description: String,
    pub url: Option<String>,
    pub doc_count: usize,
}

/// Section of Cocoon documentation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CocoonSection {
    pub identifier: String,
    pub title: String,
    pub description: String,
    pub documents: Vec<CocoonDocumentSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CocoonDocumentSummary {
    pub path: String,
    pub title: String,
    pub summary: String,
    pub url: String,
}

/// Full document content
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CocoonDocument {
    pub path: String,
    pub title: String,
    pub summary: String,
    pub content: String,
    pub url: String,
}

/// Known Cocoon documentation sections
pub const COCOON_SECTIONS: &[(&str, &str, &str)] = &[
    (
        "architecture",
        "Architecture",
        "System design and architecture documentation",
    ),
    (
        "tdx",
        "TDX & Images",
        "Intel TDX fundamentals and image generation",
    ),
    (
        "ra-tls",
        "RA-TLS",
        "Remote attestation over TLS and certificate processes",
    ),
    (
        "smart-contracts",
        "Smart Contracts",
        "Payment mechanisms and TON blockchain integration",
    ),
    (
        "seal-keys",
        "Seal Keys",
        "Persistent key derivation via SGX/TDX",
    ),
    ("gpu", "GPU", "GPU passthrough and validation"),
    (
        "deployment",
        "Deployment",
        "Testing and debugging procedures",
    ),
];

impl CocoonTechnology {
    pub fn from_section(id: &str, title: &str, description: &str, doc_count: usize) -> Self {
        Self {
            identifier: format!("cocoon:{id}"),
            title: format!("Cocoon {title}"),
            description: description.to_string(),
            url: Some(format!(
                "https://github.com/TelegramMessenger/cocoon/tree/master/docs/{id}"
            )),
            doc_count,
        }
    }
}

/// Extract title from markdown content
pub fn extract_markdown_title(content: &str) -> String {
    for line in content.lines() {
        let trimmed = line.trim();
        if let Some(stripped) = trimmed.strip_prefix("# ") {
            return stripped.trim().to_string();
        }
    }
    String::new()
}

/// Extract first paragraph as summary
pub fn extract_markdown_summary(content: &str) -> String {
    let mut in_header = true;
    let mut summary_lines = Vec::new();

    for line in content.lines() {
        let trimmed = line.trim();

        // Skip headers
        if trimmed.starts_with('#') {
            in_header = true;
            continue;
        }

        // Skip empty lines after headers
        if in_header && trimmed.is_empty() {
            in_header = false;
            continue;
        }

        // Skip code blocks
        if trimmed.starts_with("```") {
            continue;
        }

        // Collect first paragraph
        if !in_header {
            if trimmed.is_empty() {
                break;
            }
            summary_lines.push(trimmed);
        }
    }

    let summary = summary_lines.join(" ");
    if summary.len() > 200 {
        format!("{}...", &summary[..200])
    } else {
        summary
    }
}
