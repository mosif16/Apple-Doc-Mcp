use std::path::PathBuf;
use std::time::Duration as StdDuration;

use anyhow::{Context, Result};
use directories::ProjectDirs;
use reqwest::Client;
use tokio::sync::Mutex;
use tracing::{debug, instrument};

use super::types::{
    extract_markdown_summary, extract_markdown_title, CocoonDocument, CocoonDocumentSummary,
    CocoonSection, CocoonTechnology, GitHubContent, COCOON_SECTIONS,
};
use docs_mcp_client::cache::{DiskCache, MemoryCache};

const GITHUB_API_BASE: &str = "https://api.github.com/repos/TelegramMessenger/cocoon/contents";
const RAW_CONTENT_BASE: &str =
    "https://raw.githubusercontent.com/TelegramMessenger/cocoon/master";

#[derive(Debug)]
pub struct CocoonClient {
    http: Client,
    disk_cache: DiskCache,
    #[allow(dead_code)]
    memory_cache: MemoryCache<Vec<u8>>,
    #[allow(dead_code)]
    contents_lock: Mutex<()>,
    cache_dir: PathBuf,
}

impl Default for CocoonClient {
    fn default() -> Self {
        Self::new()
    }
}

impl CocoonClient {
    #[must_use]
    pub fn new() -> Self {
        let project_dirs = ProjectDirs::from("com", "RecordAndLearn", "multi-docs-mcp")
            .expect("unable to resolve project directories");

        let cache_dir = project_dirs.cache_dir().join("cocoon");
        if let Err(e) = std::fs::create_dir_all(&cache_dir) {
            tracing::warn!(error = %e, "Failed to create Cocoon cache directory");
        }

        let http = Client::builder()
            .user_agent("MultiDocsMCP/1.0")
            .timeout(StdDuration::from_secs(30))
            .gzip(true)
            .build()
            .expect("failed to build reqwest client");

        Self {
            http,
            disk_cache: DiskCache::new(&cache_dir),
            memory_cache: MemoryCache::new(time::Duration::minutes(30)),
            contents_lock: Mutex::new(()),
            cache_dir,
        }
    }

    /// List contents of a directory in the Cocoon repo
    #[instrument(name = "cocoon_client.list_contents", skip(self))]
    async fn list_contents(&self, path: &str) -> Result<Vec<GitHubContent>> {
        let cache_key = format!("contents_{}.json", path.replace('/', "_"));

        // Check disk cache
        if let Ok(Some(entry)) = self.disk_cache.load::<Vec<GitHubContent>>(&cache_key).await {
            debug!(path, "Cocoon contents served from disk cache");
            return Ok(entry.value);
        }

        // Fetch from GitHub API
        let url = format!("{GITHUB_API_BASE}/{path}");
        debug!(url = url, "Fetching Cocoon contents");

        let response = self
            .http
            .get(&url)
            .header("Accept", "application/vnd.github.v3+json")
            .send()
            .await
            .context("Failed to fetch Cocoon contents")?;

        if !response.status().is_success() {
            anyhow::bail!("GitHub API request failed: {}", response.status());
        }

        let contents: Vec<GitHubContent> = response
            .json()
            .await
            .context("Failed to parse GitHub contents")?;

        // Store in cache
        self.disk_cache.store(&cache_key, contents.clone()).await?;

        Ok(contents)
    }

    /// Fetch raw file content
    #[instrument(name = "cocoon_client.fetch_file", skip(self))]
    async fn fetch_file(&self, path: &str) -> Result<String> {
        let cache_key = format!("file_{}.txt", path.replace('/', "_"));

        // Check disk cache
        if let Ok(Some(entry)) = self.disk_cache.load::<String>(&cache_key).await {
            debug!(path, "Cocoon file served from disk cache");
            return Ok(entry.value);
        }

        let url = format!("{RAW_CONTENT_BASE}/{path}");
        debug!(url = url, "Fetching Cocoon file");

        let response = self
            .http
            .get(&url)
            .send()
            .await
            .context("Failed to fetch Cocoon file")?;

        if !response.status().is_success() {
            anyhow::bail!("File fetch failed: {}", response.status());
        }

        let content = response
            .text()
            .await
            .context("Failed to read file content")?;

        // Store in cache
        self.disk_cache.store(&cache_key, content.clone()).await?;

        Ok(content)
    }

    /// Get available technologies (documentation sections)
    #[instrument(name = "cocoon_client.get_technologies", skip(self))]
    pub async fn get_technologies(&self) -> Result<Vec<CocoonTechnology>> {
        // Try to list the docs directory to get actual counts
        let doc_counts = match self.list_contents("docs").await {
            Ok(contents) => {
                let mut counts = std::collections::HashMap::new();
                for item in contents {
                    if item.content_type == "dir" {
                        counts.insert(item.name.clone(), 1); // Placeholder count
                    }
                }
                counts
            }
            Err(_) => std::collections::HashMap::new(),
        };

        let technologies: Vec<CocoonTechnology> = COCOON_SECTIONS
            .iter()
            .map(|(id, title, desc)| {
                let count = doc_counts.get(*id).copied().unwrap_or(0);
                CocoonTechnology::from_section(id, title, desc, count)
            })
            .collect();

        Ok(technologies)
    }

    /// Get documents in a section
    #[instrument(name = "cocoon_client.get_section", skip(self))]
    pub async fn get_section(&self, identifier: &str) -> Result<CocoonSection> {
        // Extract section ID from identifier (e.g., "cocoon:architecture" -> "architecture")
        let section_id = identifier
            .strip_prefix("cocoon:")
            .unwrap_or(identifier)
            .to_lowercase();

        // Find section metadata
        let (_, title, description) = COCOON_SECTIONS
            .iter()
            .find(|(id, _, _)| *id == section_id)
            .ok_or_else(|| anyhow::anyhow!("Cocoon section not found: {identifier}"))?;

        // The Cocoon repo has flat files at docs/*.md, not subdirectories
        // Look for docs/{section_id}.md as the main document for this section
        let file_path = format!("docs/{section_id}.md");

        let mut documents = Vec::new();

        // Try to fetch the section's main document
        if let Ok(content) = self.fetch_file(&file_path).await {
            let doc_title = extract_markdown_title(&content);
            let summary = extract_markdown_summary(&content);

            documents.push(CocoonDocumentSummary {
                path: file_path.clone(),
                title: if doc_title.is_empty() {
                    title.to_string()
                } else {
                    doc_title
                },
                summary,
                url: format!(
                    "https://github.com/TelegramMessenger/cocoon/blob/main/{file_path}"
                ),
            });
        }

        Ok(CocoonSection {
            identifier: format!("cocoon:{section_id}"),
            title: format!("Cocoon {title}"),
            description: description.to_string(),
            documents,
        })
    }

    /// Get a specific document
    #[instrument(name = "cocoon_client.get_document", skip(self))]
    pub async fn get_document(&self, path: &str) -> Result<CocoonDocument> {
        let content = self.fetch_file(path).await?;

        let title = extract_markdown_title(&content);
        let summary = extract_markdown_summary(&content);

        Ok(CocoonDocument {
            path: path.to_string(),
            title: if title.is_empty() {
                path.split('/')
                    .last()
                    .unwrap_or(path)
                    .strip_suffix(".md")
                    .unwrap_or(path)
                    .to_string()
            } else {
                title
            },
            summary,
            content,
            url: format!(
                "https://github.com/TelegramMessenger/cocoon/blob/master/{path}"
            ),
        })
    }

    /// Search for documents matching a query
    #[instrument(name = "cocoon_client.search", skip(self))]
    pub async fn search(&self, query: &str) -> Result<Vec<CocoonDocumentSummary>> {
        let query_lower = query.to_lowercase();
        let query_terms: Vec<&str> = query_lower.split_whitespace().collect();
        let mut results = Vec::new();

        // List all files in the docs directory
        let contents = self.list_contents("docs").await.unwrap_or_default();

        for item in contents {
            // Only process markdown files
            if item.content_type != "file"
                || !std::path::Path::new(&item.name)
                    .extension()
                    .is_some_and(|ext| ext.eq_ignore_ascii_case("md"))
            {
                continue;
            }

            // Fetch content to get title and summary for searching
            if let Ok(content) = self.fetch_file(&item.path).await {
                let title = extract_markdown_title(&content);
                let summary = extract_markdown_summary(&content);
                let content_lower = content.to_lowercase();
                let title_lower = title.to_lowercase();
                let summary_lower = summary.to_lowercase();

                // Check if any query term matches
                let matches = query_terms.iter().any(|term| {
                    title_lower.contains(term)
                        || summary_lower.contains(term)
                        || content_lower.contains(term)
                });

                if matches {
                    results.push(CocoonDocumentSummary {
                        path: item.path.clone(),
                        title: if title.is_empty() {
                            item.name
                                .strip_suffix(".md")
                                .unwrap_or(&item.name)
                                .replace('-', " ")
                                .replace('_', " ")
                        } else {
                            title
                        },
                        summary,
                        url: item.html_url.clone(),
                    });
                }
            }
        }

        Ok(results)
    }

    pub fn cache_dir(&self) -> &PathBuf {
        &self.cache_dir
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_creation() {
        let _client = CocoonClient::new();
    }

    #[test]
    fn test_markdown_extraction() {
        let content = "# Test Title\n\nThis is the first paragraph.\n\nThis is the second.";
        assert_eq!(extract_markdown_title(content), "Test Title");
        assert_eq!(extract_markdown_summary(content), "This is the first paragraph.");
    }
}
