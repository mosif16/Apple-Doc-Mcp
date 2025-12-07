//! Claude Agent SDK documentation client.
//!
//! Provides access to Claude Agent SDK documentation for TypeScript and Python,
//! enabling AI agents to search and retrieve SDK reference information.

use std::path::PathBuf;
use std::time::Duration as StdDuration;

use anyhow::{Context, Result};
use directories::ProjectDirs;
use reqwest::Client;
use scraper::{Html, Selector};
use tracing::{debug, instrument, warn};

use super::types::{
    AgentSdkArticle, AgentSdkCategory, AgentSdkCategoryItem, AgentSdkExample,
    AgentSdkItemKind, AgentSdkLanguage, AgentSdkParameter, AgentSdkSearchResult,
    AgentSdkTechnology, COMMON_SDK_CONCEPTS, PYTHON_SDK_TOPICS, TYPESCRIPT_SDK_TOPICS,
};
use docs_mcp_client::cache::{DiskCache, MemoryCache};

const DOCS_BASE_URL: &str = "https://docs.anthropic.com/en/docs/agents-and-tools/claude-agent-sdk";
const TYPESCRIPT_GITHUB: &str = "https://github.com/anthropics/claude-agent-sdk-typescript";
const PYTHON_GITHUB: &str = "https://github.com/anthropics/claude-agent-sdk-python";

#[derive(Debug)]
pub struct ClaudeAgentSdkClient {
    http: Client,
    disk_cache: DiskCache,
    memory_cache: MemoryCache<Vec<u8>>,
    cache_dir: PathBuf,
}

impl Default for ClaudeAgentSdkClient {
    fn default() -> Self {
        Self::new()
    }
}

impl ClaudeAgentSdkClient {
    #[must_use]
    pub fn new() -> Self {
        let project_dirs = ProjectDirs::from("com", "RecordAndLearn", "multi-docs-mcp")
            .expect("unable to resolve project directories");

        let cache_dir = project_dirs.cache_dir().join("claude_agent_sdk");
        if let Err(e) = std::fs::create_dir_all(&cache_dir) {
            warn!(error = %e, "Failed to create Claude Agent SDK cache directory");
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
            memory_cache: MemoryCache::new(time::Duration::hours(24)),
            cache_dir,
        }
    }

    /// Get available Agent SDK technologies (TypeScript and Python)
    #[instrument(name = "agent_sdk_client.get_technologies", skip(self))]
    pub async fn get_technologies(&self) -> Result<Vec<AgentSdkTechnology>> {
        Ok(vec![
            AgentSdkTechnology {
                identifier: "agent-sdk:typescript".to_string(),
                title: "Claude Agent SDK (TypeScript)".to_string(),
                description: "Build AI agents with Claude Code capabilities in TypeScript/Node.js"
                    .to_string(),
                url: TYPESCRIPT_GITHUB.to_string(),
                language: AgentSdkLanguage::TypeScript,
            },
            AgentSdkTechnology {
                identifier: "agent-sdk:python".to_string(),
                title: "Claude Agent SDK (Python)".to_string(),
                description: "Build AI agents with Claude Code capabilities in Python".to_string(),
                url: PYTHON_GITHUB.to_string(),
                language: AgentSdkLanguage::Python,
            },
        ])
    }

    /// Get category listing for a specific language
    #[instrument(name = "agent_sdk_client.get_category", skip(self))]
    pub async fn get_category(&self, identifier: &str) -> Result<AgentSdkCategory> {
        let (topics, language, title, description) = if identifier.contains("python") {
            (
                PYTHON_SDK_TOPICS,
                AgentSdkLanguage::Python,
                "Claude Agent SDK (Python)",
                "Python SDK for building AI agents with Claude Code capabilities",
            )
        } else {
            (
                TYPESCRIPT_SDK_TOPICS,
                AgentSdkLanguage::TypeScript,
                "Claude Agent SDK (TypeScript)",
                "TypeScript/Node.js SDK for building AI agents",
            )
        };

        let items: Vec<AgentSdkCategoryItem> = topics
            .iter()
            .map(|(name, path, desc, item_kind)| AgentSdkCategoryItem {
                name: (*name).to_string(),
                description: (*desc).to_string(),
                kind: *item_kind,
                path: (*path).to_string(),
                url: format!("{}/{}", DOCS_BASE_URL, path),
            })
            .collect();

        Ok(AgentSdkCategory {
            identifier: identifier.to_string(),
            title: title.to_string(),
            description: description.to_string(),
            items,
            language,
        })
    }

    /// Search Agent SDK documentation
    #[instrument(name = "agent_sdk_client.search", skip(self))]
    pub async fn search(
        &self,
        query: &str,
        language: Option<AgentSdkLanguage>,
    ) -> Result<Vec<AgentSdkSearchResult>> {
        let query_lower = query.to_lowercase();
        let query_terms: Vec<&str> = query_lower.split_whitespace().collect();

        let mut results = Vec::new();

        // Search TypeScript topics
        if language.is_none() || language == Some(AgentSdkLanguage::TypeScript) {
            for (name, path, desc, item_kind) in TYPESCRIPT_SDK_TOPICS {
                let score = calculate_score(name, desc, &query_terms);
                if score > 0 {
                    results.push(AgentSdkSearchResult {
                        name: (*name).to_string(),
                        path: (*path).to_string(),
                        url: format!("{}/{}", DOCS_BASE_URL, path),
                        kind: *item_kind,
                        language: AgentSdkLanguage::TypeScript,
                        description: (*desc).to_string(),
                        score,
                    });
                }
            }
        }

        // Search Python topics
        if language.is_none() || language == Some(AgentSdkLanguage::Python) {
            for (name, path, desc, item_kind) in PYTHON_SDK_TOPICS {
                let score = calculate_score(name, desc, &query_terms);
                if score > 0 {
                    results.push(AgentSdkSearchResult {
                        name: (*name).to_string(),
                        path: (*path).to_string(),
                        url: format!("{}/{}", DOCS_BASE_URL, path),
                        kind: *item_kind,
                        language: AgentSdkLanguage::Python,
                        description: (*desc).to_string(),
                        score,
                    });
                }
            }
        }

        // Search common concepts
        for (concept, desc) in COMMON_SDK_CONCEPTS {
            if query_terms.iter().any(|t| concept.contains(t) || t.contains(concept)) {
                // Add as a guide for both languages if not already found
                let base_score = 60;
                if language.is_none() || language == Some(AgentSdkLanguage::TypeScript) {
                    if !results.iter().any(|r| r.name.to_lowercase() == *concept) {
                        results.push(AgentSdkSearchResult {
                            name: concept.to_string(),
                            path: format!("concepts/{}", concept.replace(' ', "-")),
                            url: format!("{}/overview", DOCS_BASE_URL),
                            kind: AgentSdkItemKind::Guide,
                            language: AgentSdkLanguage::TypeScript,
                            description: (*desc).to_string(),
                            score: base_score,
                        });
                    }
                }
            }
        }

        // Sort by score descending
        results.sort_by(|a, b| b.score.cmp(&a.score));
        results.truncate(20);

        Ok(results)
    }

    /// Get detailed article documentation
    #[instrument(name = "agent_sdk_client.get_article", skip(self))]
    pub async fn get_article(
        &self,
        path: &str,
        language: AgentSdkLanguage,
    ) -> Result<AgentSdkArticle> {
        let topics: &[(&str, &str, &str, AgentSdkItemKind)] = match language {
            AgentSdkLanguage::TypeScript => TYPESCRIPT_SDK_TOPICS,
            AgentSdkLanguage::Python => PYTHON_SDK_TOPICS,
        };

        // Find in predefined topics
        let topic = topics
            .iter()
            .find(|(_, p, _, _)| *p == path || path.ends_with(p))
            .or_else(|| topics.iter().find(|(n, _, _, _)| path.contains(n)));

        let (name, url, desc, kind) = match topic {
            Some((n, p, d, k)) => (
                (*n).to_string(),
                format!("{}/{}", DOCS_BASE_URL, p),
                (*d).to_string(),
                *k,
            ),
            None => {
                let clean_path = path
                    .strip_prefix("agent-sdk:")
                    .unwrap_or(path);
                (
                    clean_path
                        .split('/')
                        .last()
                        .unwrap_or(clean_path)
                        .to_string(),
                    format!("{}/{}", DOCS_BASE_URL, clean_path),
                    String::new(),
                    AgentSdkItemKind::Class,
                )
            }
        };

        // Check cache
        let cache_key = format!("article_{}_{}.json", language, path.replace('/', "_"));

        if let Ok(Some(entry)) = self.disk_cache.load::<AgentSdkArticle>(&cache_key).await {
            return Ok(entry.value);
        }

        // Build article from predefined data and try to fetch live content
        let article = self
            .build_article(&name, &url, &desc, kind, language, path)
            .await;

        // Cache result
        let _ = self.disk_cache.store(&cache_key, article.clone()).await;

        Ok(article)
    }

    /// Build article with predefined content and optional live fetch
    async fn build_article(
        &self,
        name: &str,
        url: &str,
        default_desc: &str,
        kind: AgentSdkItemKind,
        language: AgentSdkLanguage,
        path: &str,
    ) -> AgentSdkArticle {
        // Try to fetch live documentation
        let live_content = self.fetch_docs_page(url).await.ok();

        // Get predefined examples and parameters
        let (examples, parameters, declaration, content) = self.get_predefined_content(name, language, path);

        // Use live content if available, otherwise use predefined
        let final_content = live_content
            .map(|c| if c.is_empty() { content.clone() } else { c })
            .unwrap_or(content);

        AgentSdkArticle {
            title: name.to_string(),
            description: default_desc.to_string(),
            path: path.to_string(),
            url: url.to_string(),
            kind,
            language,
            declaration,
            content: final_content,
            examples,
            parameters,
            return_value: self.get_return_value(name, language),
            related: self.get_related_items(name, language),
        }
    }

    /// Fetch documentation page content
    async fn fetch_docs_page(&self, url: &str) -> Result<String> {
        debug!(url = %url, "Fetching Claude Agent SDK documentation");

        let response = self.http.get(url).send().await;

        match response {
            Ok(resp) if resp.status().is_success() => {
                let html = resp.text().await?;
                Ok(self.parse_docs_html(&html))
            }
            Ok(resp) => {
                anyhow::bail!("Failed to fetch docs: HTTP {}", resp.status())
            }
            Err(e) => {
                anyhow::bail!("Failed to fetch docs: {}", e)
            }
        }
    }

    /// Parse HTML documentation
    fn parse_docs_html(&self, html: &str) -> String {
        let document = Html::parse_document(html);

        // Try to extract main content
        let selectors = [
            "article",
            ".prose",
            ".doc-content",
            "main",
            ".content",
        ];

        for selector_str in selectors {
            if let Ok(selector) = Selector::parse(selector_str) {
                if let Some(element) = document.select(&selector).next() {
                    let text: String = element
                        .text()
                        .collect::<Vec<_>>()
                        .join(" ")
                        .split_whitespace()
                        .collect::<Vec<_>>()
                        .join(" ");
                    if !text.is_empty() && text.len() > 100 {
                        return text;
                    }
                }
            }
        }

        String::new()
    }

    /// Get predefined content for known items
    fn get_predefined_content(
        &self,
        name: &str,
        language: AgentSdkLanguage,
        _path: &str,
    ) -> (Vec<AgentSdkExample>, Vec<AgentSdkParameter>, Option<String>, String) {
        match (name, language) {
            // TypeScript query function
            ("query", AgentSdkLanguage::TypeScript) => (
                vec![AgentSdkExample {
                    code: r#"import { query } from '@anthropic-ai/claude-agent-sdk';

const response = await query({
  prompt: "Write a function that calculates fibonacci numbers",
  options: {
    systemPrompt: "You are a helpful coding assistant",
    maxTurns: 10,
    allowedTools: ["Read", "Write", "Bash"]
  }
});

for await (const message of response) {
  console.log(message);
}"#.to_string(),
                    language: "typescript".to_string(),
                    description: Some("Basic query example".to_string()),
                }],
                vec![
                    AgentSdkParameter {
                        name: "prompt".to_string(),
                        description: "The prompt to send to the agent".to_string(),
                        param_type: Some("string".to_string()),
                        default_value: None,
                        required: true,
                    },
                    AgentSdkParameter {
                        name: "options".to_string(),
                        description: "Optional configuration for the agent".to_string(),
                        param_type: Some("ClaudeAgentOptions".to_string()),
                        default_value: None,
                        required: false,
                    },
                ],
                Some("query(prompt: string, options?: ClaudeAgentOptions): AsyncIterator<Message>".to_string()),
                "Async function that sends a prompt to the Claude agent and returns an AsyncIterator of response messages. Use this for stateless, single-turn interactions.".to_string(),
            ),

            // Python query function
            ("query", AgentSdkLanguage::Python) => (
                vec![AgentSdkExample {
                    code: r#"from claude_agent_sdk import query, ClaudeAgentOptions

options = ClaudeAgentOptions(
    system_prompt="You are a helpful coding assistant",
    max_turns=10,
    allowed_tools=["Read", "Write", "Bash"]
)

async for message in query("Write a fibonacci function", options=options):
    print(message)"#.to_string(),
                    language: "python".to_string(),
                    description: Some("Basic query example".to_string()),
                }],
                vec![
                    AgentSdkParameter {
                        name: "prompt".to_string(),
                        description: "The prompt to send to the agent".to_string(),
                        param_type: Some("str".to_string()),
                        default_value: None,
                        required: true,
                    },
                    AgentSdkParameter {
                        name: "options".to_string(),
                        description: "Optional configuration for the agent".to_string(),
                        param_type: Some("ClaudeAgentOptions".to_string()),
                        default_value: Some("None".to_string()),
                        required: false,
                    },
                ],
                Some("async def query(prompt: str, options: ClaudeAgentOptions = None) -> AsyncIterator[Message]".to_string()),
                "Async function that sends a prompt to the Claude agent and returns an AsyncIterator of response messages. Use this for stateless, single-turn interactions.".to_string(),
            ),

            // ClaudeAgentOptions TypeScript
            ("ClaudeAgentOptions", AgentSdkLanguage::TypeScript) => (
                vec![AgentSdkExample {
                    code: r#"const options: ClaudeAgentOptions = {
  systemPrompt: "You are a code review expert",
  maxTurns: 20,
  allowedTools: ["Read", "Grep", "Glob"],
  permissionMode: "acceptEdits",
  cwd: "/path/to/project",
  mcpServers: {
    custom: {
      command: "node",
      args: ["./my-mcp-server.js"]
    }
  }
};"#.to_string(),
                    language: "typescript".to_string(),
                    description: Some("Full configuration example".to_string()),
                }],
                vec![
                    AgentSdkParameter {
                        name: "systemPrompt".to_string(),
                        description: "Custom system instructions for the agent".to_string(),
                        param_type: Some("string".to_string()),
                        default_value: None,
                        required: false,
                    },
                    AgentSdkParameter {
                        name: "maxTurns".to_string(),
                        description: "Maximum number of conversation turns".to_string(),
                        param_type: Some("number".to_string()),
                        default_value: None,
                        required: false,
                    },
                    AgentSdkParameter {
                        name: "allowedTools".to_string(),
                        description: "List of tools the agent can use".to_string(),
                        param_type: Some("string[]".to_string()),
                        default_value: None,
                        required: false,
                    },
                    AgentSdkParameter {
                        name: "permissionMode".to_string(),
                        description: "Auto-acceptance mode for agent actions".to_string(),
                        param_type: Some("'acceptEdits' | 'default'".to_string()),
                        default_value: Some("'default'".to_string()),
                        required: false,
                    },
                    AgentSdkParameter {
                        name: "cwd".to_string(),
                        description: "Working directory for the agent".to_string(),
                        param_type: Some("string".to_string()),
                        default_value: None,
                        required: false,
                    },
                ],
                Some("interface ClaudeAgentOptions".to_string()),
                "Configuration object for the Claude Agent SDK. Controls system prompt, tool access, permissions, and MCP server integration.".to_string(),
            ),

            // ClaudeAgentOptions Python
            ("ClaudeAgentOptions", AgentSdkLanguage::Python) => (
                vec![AgentSdkExample {
                    code: r#"from claude_agent_sdk import ClaudeAgentOptions
from pathlib import Path

options = ClaudeAgentOptions(
    system_prompt="You are a code review expert",
    max_turns=20,
    allowed_tools=["Read", "Grep", "Glob"],
    permission_mode="acceptEdits",
    cwd=Path("/path/to/project"),
    cli_path="/custom/path/to/claude"
)"#.to_string(),
                    language: "python".to_string(),
                    description: Some("Full configuration example".to_string()),
                }],
                vec![
                    AgentSdkParameter {
                        name: "system_prompt".to_string(),
                        description: "Custom system instructions for the agent".to_string(),
                        param_type: Some("str".to_string()),
                        default_value: Some("None".to_string()),
                        required: false,
                    },
                    AgentSdkParameter {
                        name: "max_turns".to_string(),
                        description: "Maximum number of conversation turns".to_string(),
                        param_type: Some("int".to_string()),
                        default_value: Some("None".to_string()),
                        required: false,
                    },
                    AgentSdkParameter {
                        name: "allowed_tools".to_string(),
                        description: "List of tools the agent can use".to_string(),
                        param_type: Some("List[str]".to_string()),
                        default_value: Some("None".to_string()),
                        required: false,
                    },
                    AgentSdkParameter {
                        name: "permission_mode".to_string(),
                        description: "Auto-acceptance mode for agent actions".to_string(),
                        param_type: Some("str".to_string()),
                        default_value: Some("'default'".to_string()),
                        required: false,
                    },
                    AgentSdkParameter {
                        name: "cwd".to_string(),
                        description: "Working directory (string or Path)".to_string(),
                        param_type: Some("Union[str, Path]".to_string()),
                        default_value: Some("None".to_string()),
                        required: false,
                    },
                    AgentSdkParameter {
                        name: "cli_path".to_string(),
                        description: "Custom path to Claude CLI".to_string(),
                        param_type: Some("str".to_string()),
                        default_value: Some("None".to_string()),
                        required: false,
                    },
                ],
                Some("class ClaudeAgentOptions".to_string()),
                "Configuration object for the Claude Agent SDK. Renamed from ClaudeCodeOptions. Controls system prompt, tool access, permissions, and CLI configuration.".to_string(),
            ),

            // Python @tool decorator
            ("@tool", AgentSdkLanguage::Python) => (
                vec![AgentSdkExample {
                    code: r#"from claude_agent_sdk import tool, create_sdk_mcp_server

@tool("get_weather", "Get weather for a location", {"location": str})
async def get_weather(args):
    location = args["location"]
    # Call your weather API
    return f"Weather in {location}: Sunny, 72°F"

# Create MCP server from decorated functions
mcp_server = create_sdk_mcp_server([get_weather])

# Use with ClaudeSDKClient
async with ClaudeSDKClient(mcp_servers=[mcp_server]) as client:
    response = await client.query("What's the weather in San Francisco?")"#.to_string(),
                    language: "python".to_string(),
                    description: Some("Custom tool with @tool decorator".to_string()),
                }],
                vec![
                    AgentSdkParameter {
                        name: "name".to_string(),
                        description: "Tool name exposed to the agent".to_string(),
                        param_type: Some("str".to_string()),
                        default_value: None,
                        required: true,
                    },
                    AgentSdkParameter {
                        name: "description".to_string(),
                        description: "Tool description for the agent".to_string(),
                        param_type: Some("str".to_string()),
                        default_value: None,
                        required: true,
                    },
                    AgentSdkParameter {
                        name: "parameters".to_string(),
                        description: "Parameter schema as dict".to_string(),
                        param_type: Some("Dict[str, type]".to_string()),
                        default_value: None,
                        required: true,
                    },
                ],
                Some("@tool(name: str, description: str, parameters: Dict[str, type])".to_string()),
                "Decorator for defining Python functions as in-process MCP tools. Provides no subprocess overhead, faster performance, and easier debugging compared to external MCP servers.".to_string(),
            ),

            // ClaudeSDKClient Python
            ("ClaudeSDKClient", AgentSdkLanguage::Python) => (
                vec![AgentSdkExample {
                    code: r#"from claude_agent_sdk import ClaudeSDKClient, ClaudeAgentOptions

options = ClaudeAgentOptions(
    system_prompt="You are a helpful assistant",
    max_turns=10
)

async with ClaudeSDKClient(options=options) as client:
    # Send initial query
    response = await client.query("Hello, can you help me?")

    # Continue conversation
    response = await client.query("Now do something else")

    # Receive streaming response
    async for message in client.receive_response():
        print(message)"#.to_string(),
                    language: "python".to_string(),
                    description: Some("Bidirectional conversation example".to_string()),
                }],
                vec![
                    AgentSdkParameter {
                        name: "options".to_string(),
                        description: "Configuration options for the client".to_string(),
                        param_type: Some("ClaudeAgentOptions".to_string()),
                        default_value: Some("None".to_string()),
                        required: false,
                    },
                    AgentSdkParameter {
                        name: "mcp_servers".to_string(),
                        description: "Custom MCP servers to register".to_string(),
                        param_type: Some("List[MCPServer]".to_string()),
                        default_value: Some("None".to_string()),
                        required: false,
                    },
                    AgentSdkParameter {
                        name: "hooks".to_string(),
                        description: "Event hooks for the agent lifecycle".to_string(),
                        param_type: Some("Dict[str, Callable]".to_string()),
                        default_value: Some("None".to_string()),
                        required: false,
                    },
                ],
                Some("class ClaudeSDKClient(AsyncContextManager)".to_string()),
                "Async context manager for bidirectional conversations with custom tools and hooks. Use this for multi-turn conversations that require session state.".to_string(),
            ),

            // Hooks
            ("hooks", _) => (
                vec![AgentSdkExample {
                    code: r#"# Python hook example
def pre_tool_use_hook(tool_name: str, tool_input: dict) -> dict | None:
    """Hook called before each tool execution."""
    forbidden_patterns = ["/etc/passwd", "rm -rf"]

    if tool_name == "Bash":
        command = tool_input.get("command", "")
        for pattern in forbidden_patterns:
            if pattern in command:
                return {"denied": True, "reason": f"Forbidden: {pattern}"}

    return None  # Allow the tool to execute

options = ClaudeAgentOptions(
    hooks={"PreToolUse": pre_tool_use_hook}
)"#.to_string(),
                    language: "python".to_string(),
                    description: Some("PreToolUse hook for security".to_string()),
                }],
                vec![
                    AgentSdkParameter {
                        name: "PreToolUse".to_string(),
                        description: "Called before tool execution, can deny or modify".to_string(),
                        param_type: Some("Callable".to_string()),
                        default_value: None,
                        required: false,
                    },
                    AgentSdkParameter {
                        name: "PostToolUse".to_string(),
                        description: "Called after tool execution with results".to_string(),
                        param_type: Some("Callable".to_string()),
                        default_value: None,
                        required: false,
                    },
                    AgentSdkParameter {
                        name: "OnMessage".to_string(),
                        description: "Called when a message is received".to_string(),
                        param_type: Some("Callable".to_string()),
                        default_value: None,
                        required: false,
                    },
                ],
                None,
                "Hooks are functions invoked at specific points in the agent execution cycle. Use them for permission control, logging, or custom processing.".to_string(),
            ),

            // Default
            _ => (
                vec![],
                vec![],
                None,
                format!("Documentation for {} in the Claude Agent SDK.", name),
            ),
        }
    }

    /// Get return value documentation
    fn get_return_value(&self, name: &str, language: AgentSdkLanguage) -> Option<String> {
        match (name, language) {
            ("query", AgentSdkLanguage::TypeScript) => {
                Some("AsyncIterator<Message> - Yields AssistantMessage, UserMessage, SystemMessage, or ResultMessage objects".to_string())
            }
            ("query", AgentSdkLanguage::Python) => {
                Some("AsyncIterator[Message] - Yields message objects as they arrive from the agent".to_string())
            }
            _ => None,
        }
    }

    /// Get related items
    fn get_related_items(&self, name: &str, _language: AgentSdkLanguage) -> Vec<String> {
        match name {
            "query" => vec![
                "ClaudeAgentOptions".to_string(),
                "AssistantMessage".to_string(),
                "UserMessage".to_string(),
                "ResultMessage".to_string(),
            ],
            "ClaudeAgentOptions" => vec![
                "query".to_string(),
                "ClaudeSDKClient".to_string(),
                "hooks".to_string(),
                "mcpServers".to_string(),
            ],
            "ClaudeSDKClient" => vec![
                "ClaudeAgentOptions".to_string(),
                "query".to_string(),
                "receive_response".to_string(),
            ],
            "@tool" => vec![
                "create_sdk_mcp_server".to_string(),
                "ClaudeSDKClient".to_string(),
                "mcp_servers".to_string(),
            ],
            "hooks" => vec![
                "PreToolUse".to_string(),
                "PostToolUse".to_string(),
                "OnMessage".to_string(),
                "ClaudeAgentOptions".to_string(),
            ],
            _ => vec![],
        }
    }

    pub fn cache_dir(&self) -> &PathBuf {
        &self.cache_dir
    }
}

/// Calculate search score
fn calculate_score(name: &str, desc: &str, query_terms: &[&str]) -> i32 {
    let name_lower = name.to_lowercase();
    let desc_lower = desc.to_lowercase();

    let mut score = 0;

    for term in query_terms {
        // Exact name match
        if name_lower == *term {
            score += 100;
        } else if name_lower.starts_with(term) {
            score += 50;
        } else if name_lower.contains(term) {
            score += 30;
        } else if desc_lower.contains(term) {
            score += 10;
        }
    }

    // Boost important SDK terms
    let boost_terms = [
        "query", "client", "options", "hook", "tool", "mcp",
        "message", "stream", "async", "agent",
    ];
    for boost in boost_terms {
        if name_lower.contains(boost) && query_terms.iter().any(|t| t.contains(boost)) {
            score += 25;
        }
    }

    score
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_creation() {
        let _client = ClaudeAgentSdkClient::new();
    }

    #[test]
    fn test_calculate_score() {
        let terms = vec!["query", "async"];
        assert!(calculate_score("query", "Async function for queries", &terms) > 0);
        assert!(calculate_score("random", "unrelated", &terms) == 0);
    }
}
