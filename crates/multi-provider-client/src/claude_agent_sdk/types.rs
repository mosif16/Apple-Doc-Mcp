//! Claude Agent SDK documentation types for TypeScript and Python.
//!
//! Provides access to Claude Agent SDK documentation for building AI agents
//! with Claude Code capabilities in TypeScript/Node.js and Python.

use serde::{Deserialize, Serialize};

/// Claude Agent SDK technology/language representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentSdkTechnology {
    pub identifier: String,
    pub title: String,
    pub description: String,
    pub url: String,
    pub language: AgentSdkLanguage,
}

/// Supported SDK languages
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AgentSdkLanguage {
    /// TypeScript/JavaScript SDK
    TypeScript,
    /// Python SDK
    Python,
}

impl std::fmt::Display for AgentSdkLanguage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TypeScript => write!(f, "TypeScript"),
            Self::Python => write!(f, "Python"),
        }
    }
}

/// Claude Agent SDK category
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentSdkCategory {
    pub identifier: String,
    pub title: String,
    pub description: String,
    pub items: Vec<AgentSdkCategoryItem>,
    pub language: AgentSdkLanguage,
}

/// Item in an Agent SDK category
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentSdkCategoryItem {
    pub name: String,
    pub description: String,
    pub kind: AgentSdkItemKind,
    pub path: String,
    pub url: String,
}

/// Types of Agent SDK documentation items
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AgentSdkItemKind {
    /// Class (e.g., ClaudeSDKClient, ClaudeAgentOptions)
    Class,
    /// Function (e.g., query)
    Function,
    /// Type/Interface
    Type,
    /// Configuration option
    Config,
    /// Hook
    Hook,
    /// Tool
    Tool,
    /// Message type
    Message,
    /// Error type
    Error,
    /// Guide/Tutorial
    Guide,
}

impl std::fmt::Display for AgentSdkItemKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Class => write!(f, "class"),
            Self::Function => write!(f, "function"),
            Self::Type => write!(f, "type"),
            Self::Config => write!(f, "config"),
            Self::Hook => write!(f, "hook"),
            Self::Tool => write!(f, "tool"),
            Self::Message => write!(f, "message"),
            Self::Error => write!(f, "error"),
            Self::Guide => write!(f, "guide"),
        }
    }
}

/// Full Agent SDK documentation article
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentSdkArticle {
    pub title: String,
    pub description: String,
    pub path: String,
    pub url: String,
    pub kind: AgentSdkItemKind,
    pub language: AgentSdkLanguage,
    /// Declaration/signature
    pub declaration: Option<String>,
    /// Full documentation content
    pub content: String,
    /// Code examples
    pub examples: Vec<AgentSdkExample>,
    /// Parameters (for functions/classes)
    pub parameters: Vec<AgentSdkParameter>,
    /// Return type/value description
    pub return_value: Option<String>,
    /// Related items
    pub related: Vec<String>,
}

/// Code example in Agent SDK documentation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentSdkExample {
    pub code: String,
    pub language: String,
    pub description: Option<String>,
}

/// Parameter documentation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentSdkParameter {
    pub name: String,
    pub description: String,
    pub param_type: Option<String>,
    pub default_value: Option<String>,
    pub required: bool,
}

/// Search result from Agent SDK documentation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentSdkSearchResult {
    pub name: String,
    pub path: String,
    pub url: String,
    pub kind: AgentSdkItemKind,
    pub language: AgentSdkLanguage,
    pub description: String,
    /// Relevance score
    pub score: i32,
}

// TypeScript SDK predefined topics
pub const TYPESCRIPT_SDK_TOPICS: &[(&str, &str, &str, AgentSdkItemKind)] = &[
    // Core Client
    ("ClaudeClient", "client", "Main client class for interacting with Claude Agent SDK", AgentSdkItemKind::Class),
    ("query", "query", "Async function returning an AsyncIterator of response messages for stateless interactions", AgentSdkItemKind::Function),

    // Configuration
    ("ClaudeAgentOptions", "options", "Configuration object for Claude Agent SDK including system prompts, tools, and permissions", AgentSdkItemKind::Config),
    ("systemPrompt", "options/system-prompt", "Custom system instructions for the agent", AgentSdkItemKind::Config),
    ("maxTurns", "options/max-turns", "Maximum number of conversation turns", AgentSdkItemKind::Config),
    ("allowedTools", "options/allowed-tools", "Tool restrictions (e.g., 'Read', 'Write', 'Bash')", AgentSdkItemKind::Config),
    ("permissionMode", "options/permission-mode", "Auto-acceptance modes like 'acceptEdits'", AgentSdkItemKind::Config),
    ("cwd", "options/cwd", "Working directory for the agent", AgentSdkItemKind::Config),

    // MCP Servers
    ("mcpServers", "mcp-servers", "Custom MCP server configurations for extending agent capabilities", AgentSdkItemKind::Config),
    ("createMcpServer", "mcp/create-server", "Create custom MCP tool servers", AgentSdkItemKind::Function),

    // Hooks
    ("hooks", "hooks", "Event-based callback definitions for agent lifecycle", AgentSdkItemKind::Hook),
    ("PreToolUse", "hooks/pre-tool-use", "Hook invoked before a tool is executed", AgentSdkItemKind::Hook),
    ("PostToolUse", "hooks/post-tool-use", "Hook invoked after a tool is executed", AgentSdkItemKind::Hook),
    ("OnMessage", "hooks/on-message", "Hook invoked when a message is received", AgentSdkItemKind::Hook),

    // Messages
    ("AssistantMessage", "messages/assistant", "Message from the assistant", AgentSdkItemKind::Message),
    ("UserMessage", "messages/user", "Message from the user", AgentSdkItemKind::Message),
    ("SystemMessage", "messages/system", "System message for context", AgentSdkItemKind::Message),
    ("ResultMessage", "messages/result", "Result message containing tool outputs", AgentSdkItemKind::Message),

    // Content Blocks
    ("TextBlock", "content/text", "Text content block in messages", AgentSdkItemKind::Type),
    ("ToolUseBlock", "content/tool-use", "Tool use request block", AgentSdkItemKind::Type),
    ("ToolResultBlock", "content/tool-result", "Tool execution result block", AgentSdkItemKind::Type),

    // Streaming
    ("stream", "streaming", "Streaming mode for interactive, low-latency responses", AgentSdkItemKind::Function),
    ("AsyncIterator", "streaming/iterator", "Async iterator for processing streamed responses", AgentSdkItemKind::Type),

    // Session Management
    ("session", "session", "Session management for long-running tasks", AgentSdkItemKind::Class),
    ("fork", "session/fork", "Fork a session for parallel execution", AgentSdkItemKind::Function),
    ("contextCompaction", "session/compaction", "Context memory management for long conversations", AgentSdkItemKind::Function),

    // Authentication
    ("ANTHROPIC_API_KEY", "auth/api-key", "API key authentication via environment variable", AgentSdkItemKind::Config),
    ("CLAUDE_CODE_USE_BEDROCK", "auth/bedrock", "Enable Amazon Bedrock as the API provider", AgentSdkItemKind::Config),
    ("CLAUDE_CODE_USE_VERTEX", "auth/vertex", "Enable Google Vertex AI as the API provider", AgentSdkItemKind::Config),
];

// Python SDK predefined topics
pub const PYTHON_SDK_TOPICS: &[(&str, &str, &str, AgentSdkItemKind)] = &[
    // Core Functions
    ("query", "query", "Async function returning an AsyncIterator of response messages", AgentSdkItemKind::Function),
    ("ClaudeSDKClient", "client", "Async context manager for bidirectional conversations with custom tools and hooks", AgentSdkItemKind::Class),

    // Configuration
    ("ClaudeAgentOptions", "options", "Configuration object for Claude Agent SDK (renamed from ClaudeCodeOptions)", AgentSdkItemKind::Config),
    ("system_prompt", "options/system-prompt", "Custom system instructions for the agent", AgentSdkItemKind::Config),
    ("max_turns", "options/max-turns", "Conversation turn limits", AgentSdkItemKind::Config),
    ("allowed_tools", "options/allowed-tools", "Tool restrictions (e.g., 'Read', 'Write', 'Bash')", AgentSdkItemKind::Config),
    ("permission_mode", "options/permission-mode", "Auto-acceptance modes like 'acceptEdits'", AgentSdkItemKind::Config),
    ("cwd", "options/cwd", "Working directory (string or Path object)", AgentSdkItemKind::Config),
    ("cli_path", "options/cli-path", "Custom CLI path for Claude Code", AgentSdkItemKind::Config),

    // Custom Tools (In-Process MCP)
    ("@tool", "tools/decorator", "Decorator for defining Python functions as in-process MCP tools", AgentSdkItemKind::Tool),
    ("create_sdk_mcp_server", "tools/mcp-server", "Create an in-process MCP server from decorated functions", AgentSdkItemKind::Function),
    ("mcp_servers", "tools/mcp-servers", "Custom MCP server configurations", AgentSdkItemKind::Config),

    // Hooks
    ("hooks", "hooks", "Python functions invoked at specific agent loop points", AgentSdkItemKind::Hook),
    ("PreToolUse", "hooks/pre-tool-use", "Hook for permission-based control before tool execution", AgentSdkItemKind::Hook),
    ("PostToolUse", "hooks/post-tool-use", "Hook invoked after tool execution", AgentSdkItemKind::Hook),

    // Messages
    ("AssistantMessage", "messages/assistant", "Message from the assistant", AgentSdkItemKind::Message),
    ("UserMessage", "messages/user", "Message from the user", AgentSdkItemKind::Message),
    ("SystemMessage", "messages/system", "System message for context", AgentSdkItemKind::Message),
    ("ResultMessage", "messages/result", "Result message containing tool outputs", AgentSdkItemKind::Message),

    // Content Blocks
    ("TextBlock", "content/text", "Text content block in messages", AgentSdkItemKind::Type),
    ("ToolUseBlock", "content/tool-use", "Tool use request block", AgentSdkItemKind::Type),
    ("ToolResultBlock", "content/tool-result", "Tool execution result block", AgentSdkItemKind::Type),

    // Error Handling
    ("ClaudeSDKError", "errors/base", "Base exception class for SDK errors", AgentSdkItemKind::Error),
    ("CLINotFoundError", "errors/cli-not-found", "Raised when Claude CLI is not found", AgentSdkItemKind::Error),
    ("ProcessError", "errors/process", "Raised on process failures with exit codes", AgentSdkItemKind::Error),
    ("CLIConnectionError", "errors/connection", "Raised on connection failures", AgentSdkItemKind::Error),
    ("CLIJSONDecodeError", "errors/json", "Raised on JSON parsing failures", AgentSdkItemKind::Error),

    // Async Support
    ("AsyncIterator", "async/iterator", "Async iterator for processing responses", AgentSdkItemKind::Type),
    ("receive_response", "async/receive", "Receive response in bidirectional mode", AgentSdkItemKind::Function),

    // Authentication
    ("ANTHROPIC_API_KEY", "auth/api-key", "API key authentication via environment variable", AgentSdkItemKind::Config),
    ("CLAUDE_CODE_USE_BEDROCK", "auth/bedrock", "Enable Amazon Bedrock (set to '1')", AgentSdkItemKind::Config),
    ("CLAUDE_CODE_USE_VERTEX", "auth/vertex", "Enable Google Vertex AI (set to '1')", AgentSdkItemKind::Config),
];

/// Common Agent SDK concepts (shared across languages)
pub const COMMON_SDK_CONCEPTS: &[(&str, &str)] = &[
    ("agent", "Autonomous AI agent that can understand codebases, edit files, and run commands"),
    ("mcp", "Model Context Protocol for registering custom tool servers"),
    ("tool", "Capability that the agent can invoke (Read, Write, Bash, etc.)"),
    ("hook", "Deterministic callback invoked during agent execution cycles"),
    ("streaming", "Real-time response streaming for interactive UX"),
    ("session", "Persistent conversation state with context management"),
    ("context compaction", "Memory management for long-running tasks"),
    ("permission mode", "Control over auto-acceptance of agent actions"),
];
