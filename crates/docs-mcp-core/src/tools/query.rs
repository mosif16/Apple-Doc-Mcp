//! Unified query tool - a single entry point for documentation search.
//!
//! This tool acts as an intelligent search engine that:
//! 1. Parses natural language queries to extract intent
//! 2. Auto-detects the appropriate provider and technology
//! 3. Searches for relevant symbols and documentation
//! 4. Fetches detailed documentation for top matches
//! 5. Returns structured context ready for AI consumption

use std::sync::Arc;

use anyhow::{Context, Result};
use multi_provider_client::types::{ProviderType, UnifiedTechnology};
use once_cell::sync::Lazy;
use regex::Regex;
use serde::Deserialize;
use serde_json::json;

use crate::{
    markdown,
    services::{ensure_framework_index, knowledge},
    state::{AppContext, ToolDefinition, ToolHandler, ToolResponse},
    tools::{parse_args, text_response, wrap_handler},
};

/// Maximum number of search results to include in the response
const MAX_SEARCH_RESULTS: usize = 10;
/// Maximum number of detailed documentation entries to fetch (with full content)
const MAX_DETAILED_DOCS: usize = 5;
/// Maximum length for summaries in non-detailed results
const MAX_SUMMARY_LENGTH: usize = 300;
/// Maximum length for code samples
const MAX_CODE_LENGTH: usize = 2000;
/// Maximum length for full documentation content
const MAX_CONTENT_LENGTH: usize = 4000;

#[derive(Debug, Deserialize)]
struct Args {
    query: String,
    #[serde(rename = "maxResults")]
    max_results: Option<usize>,
}

/// Parsed intent from the user's query
#[derive(Debug, Clone)]
struct QueryIntent {
    /// The original query
    raw_query: String,
    /// Detected provider (Apple, Telegram, TON, etc.)
    provider: Option<ProviderType>,
    /// Detected technology/framework name
    technology: Option<String>,
    /// Extracted search keywords
    keywords: Vec<String>,
    /// Type of query (how-to, reference, search)
    query_type: QueryType,
}

#[derive(Debug, Clone, PartialEq)]
enum QueryType {
    /// User wants to know how to do something
    HowTo,
    /// User wants reference documentation
    Reference,
    /// User wants to search for symbols
    Search,
}

/// Structured documentation result
#[derive(Debug, Clone)]
struct DocResult {
    title: String,
    kind: String,
    path: String,
    summary: String,
    platforms: Option<String>,
    code_sample: Option<String>,
    related_apis: Vec<String>,
    /// Full documentation content (for detailed results)
    full_content: Option<String>,
    /// Declaration/signature
    declaration: Option<String>,
    /// Parameters or properties
    parameters: Vec<(String, String)>,
}

/// Technology detection patterns
static APPLE_FRAMEWORKS: Lazy<Vec<(&'static str, &'static str)>> = Lazy::new(|| {
    vec![
        ("swiftui", "doc://com.apple.documentation/documentation/swiftui"),
        ("uikit", "doc://com.apple.documentation/documentation/uikit"),
        ("foundation", "doc://com.apple.documentation/documentation/foundation"),
        ("combine", "doc://com.apple.documentation/documentation/combine"),
        ("coredata", "doc://com.apple.documentation/documentation/coredata"),
        ("cloudkit", "doc://com.apple.documentation/documentation/cloudkit"),
        ("mapkit", "doc://com.apple.documentation/documentation/mapkit"),
        ("avfoundation", "doc://com.apple.documentation/documentation/avfoundation"),
        ("webkit", "doc://com.apple.documentation/documentation/webkit"),
        ("corelocation", "doc://com.apple.documentation/documentation/corelocation"),
        ("usernotifications", "doc://com.apple.documentation/documentation/usernotifications"),
        ("swift", "doc://com.apple.documentation/documentation/swift"),
        ("appkit", "doc://com.apple.documentation/documentation/appkit"),
        ("realitykit", "doc://com.apple.documentation/documentation/realitykit"),
        ("arkit", "doc://com.apple.documentation/documentation/arkit"),
        ("metal", "doc://com.apple.documentation/documentation/metal"),
        ("spritekit", "doc://com.apple.documentation/documentation/spritekit"),
        ("scenekit", "doc://com.apple.documentation/documentation/scenekit"),
        ("healthkit", "doc://com.apple.documentation/documentation/healthkit"),
        ("storekit", "doc://com.apple.documentation/documentation/storekit"),
        ("gamekit", "doc://com.apple.documentation/documentation/gamekit"),
        ("passkit", "doc://com.apple.documentation/documentation/passkit"),
        ("photokit", "doc://com.apple.documentation/documentation/photokit"),
        ("musickit", "doc://com.apple.documentation/documentation/musickit"),
        ("carplay", "doc://com.apple.documentation/documentation/carplay"),
        ("widgetkit", "doc://com.apple.documentation/documentation/widgetkit"),
        ("activitykit", "doc://com.apple.documentation/documentation/activitykit"),
        ("appintents", "doc://com.apple.documentation/documentation/appintents"),
        ("charts", "doc://com.apple.documentation/documentation/charts"),
        ("observation", "doc://com.apple.documentation/documentation/observation"),
        ("swiftdata", "doc://com.apple.documentation/documentation/swiftdata"),
        // ML/AI frameworks
        ("coreml", "doc://com.apple.documentation/documentation/coreml"),
        ("createml", "doc://com.apple.documentation/documentation/createml"),
        ("vision", "doc://com.apple.documentation/documentation/vision"),
        ("naturallanguage", "doc://com.apple.documentation/documentation/naturallanguage"),
        ("speech", "doc://com.apple.documentation/documentation/speech"),
        ("soundanalysis", "doc://com.apple.documentation/documentation/soundanalysis"),
        ("visionkit", "doc://com.apple.documentation/documentation/visionkit"),
        ("accelerate", "doc://com.apple.documentation/documentation/accelerate"),
        ("mlcompute", "doc://com.apple.documentation/documentation/mlcompute"),
        ("metalperformanceshaders", "doc://com.apple.documentation/documentation/metalperformanceshaders"),
        ("metalperformanceshadersgraph", "doc://com.apple.documentation/documentation/metalperformanceshadersgraph"),
    ]
});

/// Rust crate detection patterns
static RUST_CRATES: Lazy<Vec<&'static str>> = Lazy::new(|| {
    vec![
        "std", "core", "alloc", "tokio", "serde", "reqwest", "axum", "actix",
        "diesel", "sqlx", "rocket", "clap", "tracing", "anyhow", "thiserror",
        "async-std", "hyper", "warp", "tonic", "prost", "futures", "rayon",
    ]
});

/// Telegram-related keywords
static TELEGRAM_KEYWORDS: Lazy<Vec<&'static str>> = Lazy::new(|| {
    vec![
        "telegram", "bot", "sendmessage", "getme", "getupdates", "webhook",
        "inline", "callback", "chat", "chatmember", "botcommand",
    ]
});

/// TON-related keywords
static TON_KEYWORDS: Lazy<Vec<&'static str>> = Lazy::new(|| {
    vec![
        "ton", "blockchain", "wallet", "jetton", "nft", "toncoin", "cell",
        "boc", "getaccount", "gettransactions", "tonapi",
    ]
});

/// MDN Web Docs keywords (JavaScript, Web APIs, TypeScript)
static MDN_KEYWORDS: Lazy<Vec<&'static str>> = Lazy::new(|| {
    vec![
        "javascript", "js", "ecmascript", "typescript", "ts", "dom", "fetch",
        "promise", "async", "await", "array", "object", "function", "class",
        "map", "set", "weakmap", "weakset", "proxy", "reflect", "symbol",
        "iterator", "generator", "module", "import", "export", "json",
        "localstorage", "sessionstorage", "indexeddb", "webworker", "serviceworker",
        "websocket", "xmlhttprequest", "formdata", "url", "urlsearchparams",
        "blob", "file", "filereader", "canvas", "webgl", "audio", "video",
        "geolocation", "notification", "clipboard", "intersectionobserver",
        "mutationobserver", "resizeobserver", "customelement", "shadowdom",
        "template", "slot", "eventlistener", "addeventlistener", "queryselector",
        "mdn", "web", "browser", "html", "css",
    ]
});

/// React keywords
static REACT_KEYWORDS: Lazy<Vec<&'static str>> = Lazy::new(|| {
    vec![
        "react", "jsx", "tsx", "hook", "usestate", "useeffect", "usecontext",
        "usereducer", "usecallback", "usememo", "useref", "uselayouteffect",
        "useimperativehandle", "usedebugvalue", "usetransition", "usedeferredvalue",
        "useid", "usesyncexternalstore", "useinsertioneffect", "component",
        "props", "children", "fragment", "suspense", "lazy", "memo", "forwardref",
        "createcontext", "createref", "strictmode", "profiler", "reactdom",
        "createroot", "hydrateroot", "flushsync", "createportal",
    ]
});

/// Next.js keywords
static NEXTJS_KEYWORDS: Lazy<Vec<&'static str>> = Lazy::new(|| {
    vec![
        "nextjs", "next", "approuter", "pagesrouter", "servercomponent",
        "clientcomponent", "serveraction", "getserversideprops", "getstaticprops",
        "getstaticpaths", "incrementalstaticregeneration", "isr", "middleware",
        "nextimage", "nextlink", "nextscript", "nexthead", "userouter",
        "usepathname", "usesearchparams", "useparams", "notfound", "redirect",
        "generatemetadata", "generatestaticparams", "routehandler", "apiRoute",
        "layout", "page", "loading", "error", "notfound", "template",
    ]
});

/// Node.js keywords
static NODEJS_KEYWORDS: Lazy<Vec<&'static str>> = Lazy::new(|| {
    vec![
        "nodejs", "node", "fs", "path", "http", "https", "crypto", "stream",
        "buffer", "events", "util", "os", "child_process", "cluster", "worker_threads",
        "readline", "repl", "net", "dgram", "dns", "tls", "zlib", "assert",
        "querystring", "string_decoder", "timers", "tty", "v8", "vm", "process",
        "console", "require", "module", "exports", "global", "dirname", "filename",
    ]
});

/// MLX (Apple Silicon ML) keywords
static MLX_KEYWORDS: Lazy<Vec<&'static str>> = Lazy::new(|| {
    vec![
        "mlx", "mlxarray", "mlxswift", "mlx-swift", "apple silicon", "unified memory",
        "mlxnn", "mlx.nn", "mlx.core", "mlx.optimizers", "mlx_lm",
        // Core operations
        "matmul", "conv2d", "softmax", "relu", "gelu", "layernorm", "rmsnorm",
        // Optimizers
        "adamw",
        // Compilation
        "jit", "compile", "eval", "valueandgrad",
        // LLM specific
        "kvcache", "rope", "rotary",
    ]
});

/// Hugging Face keywords
static HUGGINGFACE_KEYWORDS: Lazy<Vec<&'static str>> = Lazy::new(|| {
    vec![
        "huggingface", "hugging face", "hf", "transformers", "automodel", "autotokenizer",
        "pipeline", "trainer", "from_pretrained", "push_to_hub",
        // Model families
        "llama", "mistral", "gemma", "phi", "qwen", "falcon", "codellama", "starcoder",
        "bert", "gpt2", "t5", "whisper", "clip", "stable diffusion",
        // Swift transformers
        "swift-transformers", "swifttransformers",
        // Libraries
        "tokenizers", "datasets", "diffusers", "peft", "accelerate", "trl",
        // Tasks
        "text-generation", "text-classification", "token-classification", "question-answering",
        "summarization", "translation", "conversational", "fill-mask",
    ]
});

/// QuickNode / Solana keywords
static QUICKNODE_KEYWORDS: Lazy<Vec<&'static str>> = Lazy::new(|| {
    vec![
        "quicknode", "solana", "spl", "lamports", "pubkey",
        // HTTP methods
        "getaccountinfo", "getbalance", "getblock", "getblockheight", "gettransaction",
        "sendtransaction", "simulatetransaction", "getlatestblockhash", "getslot",
        "getsignaturestatuses", "getsignaturesforaddress", "gettokenaccountbalance",
        "gettokenaccountsbyowner", "getprogramaccounts", "getmultipleaccounts",
        "requestairdrop", "getepochinfo", "getvoteaccounts", "getclusterNodes",
        // WebSocket methods
        "accountsubscribe", "programsubscribe", "logssubscribe", "slotsubscribe",
        "blocksubscribe", "signaturesubscribe", "rootsubscribe",
        // Marketplace add-ons
        "jito", "metaplex", "das", "yellowstone", "geyser", "grpc",
        // General Solana terms
        "devnet", "mainnet", "testnet", "anchor", "serum", "raydium", "jupiter",
    ]
});

/// Claude Agent SDK keywords
static CLAUDE_AGENT_SDK_KEYWORDS: Lazy<Vec<&'static str>> = Lazy::new(|| {
    vec![
        // SDK names
        "claude agent sdk", "claude-agent-sdk", "agent sdk", "claudeagentsdk",
        "claude code sdk", "claude sdk",
        // Core API (TypeScript: ClaudeClient, Python: ClaudeSDKClient)
        "claudeclient", "claudesdkclient", "claudeagentoptions", "claudecodeoptions",
        // Key functions
        "query", "mcp", "mcpservers",
        // Hooks
        "pretooluse", "posttooluse", "onmessage",
        // Configuration
        "systemprompt", "system_prompt", "maxturns", "max_turns",
        "allowedtools", "allowed_tools", "permissionmode", "permission_mode",
        // Python specific
        "@tool", "create_sdk_mcp_server", "cli_path",
        // Messages
        "assistantmessage", "usermessage", "systemmessage", "resultmessage",
        // Content blocks
        "textblock", "tooluseblock", "toolresultblock",
    ]
});

/// How-to query patterns
static HOWTO_PATTERNS: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)^(how\s+(do\s+i|to|can\s+i)|what'?s?\s+the\s+(best\s+)?way\s+to|implement|create|make|build|add|show\s+me\s+how)").unwrap()
});

/// Reference query patterns
static REFERENCE_PATTERNS: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)^(what\s+is|explain|describe|tell\s+me\s+about|documentation\s+for|docs\s+for|api\s+for)").unwrap()
});

pub fn definition() -> (ToolDefinition, ToolHandler) {
    (
        ToolDefinition {
            name: "query".to_string(),
            description:
                "Complete documentation retrieval in a single call. Returns full documentation \
                 content, code examples, declarations, and parameters—no follow-up calls needed. \
                 Auto-detects provider (Apple, Rust, Telegram, TON, Cocoon, MDN, React, Next.js, \
                 Node.js, MLX, Hugging Face, QuickNode, Claude Agent SDK) from your query. \
                 Top 5 results include complete documentation; remaining results include summaries. \
                 Use natural language: 'SwiftUI NavigationStack', 'Rust tokio spawn', 'Claude Agent SDK query function'."
                    .to_string(),
            input_schema: json!({
                "type": "object",
                "required": ["query"],
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "Natural language query. Include technology name for best results (e.g., 'SwiftUI List selection', 'Rust HashMap', 'Telegram Bot API webhooks')"
                    },
                    "maxResults": {
                        "type": "number",
                        "description": "Maximum results to return (default: 10, max: 20). Top 5 get full documentation."
                    }
                }
            }),
            input_examples: Some(vec![
                json!({"query": "SwiftUI NavigationStack path-based navigation"}),
                json!({"query": "UIKit UITableView delegate methods"}),
                json!({"query": "Rust tokio spawn async task"}),
                json!({"query": "Rust std HashMap insert"}),
                json!({"query": "Telegram Bot API sendMessage"}),
                json!({"query": "how to implement CoreData fetch requests"}),
                json!({"query": "Solana getAccountInfo"}),
                json!({"query": "QuickNode getBalance"}),
                json!({"query": "Claude Agent SDK query function typescript"}),
                json!({"query": "agent sdk python ClaudeSDKClient"}),
                json!({"query": "Claude Agent SDK hooks PreToolUse"}),
            ]),
            allowed_callers: None,
        },
        wrap_handler(|context, value| async move {
            let args: Args = parse_args(value)?;
            handle(context, args).await
        }),
    )
}

async fn handle(context: Arc<AppContext>, args: Args) -> Result<ToolResponse> {
    let max_results = args.max_results.unwrap_or(MAX_SEARCH_RESULTS).min(20);

    // Step 1: Parse the query to extract intent
    let intent = parse_query_intent(&args.query);

    // Step 2: Ensure we have the right technology selected
    let (provider, technology) = resolve_technology(&context, &intent).await?;

    // Step 3: Execute the appropriate search strategy based on intent
    let results = match intent.query_type {
        QueryType::HowTo => execute_howto_query(&context, &intent, max_results).await?,
        QueryType::Reference => execute_reference_query(&context, &intent, max_results).await?,
        QueryType::Search => execute_search_query(&context, &intent, max_results).await?,
    };

    // Step 4: Build structured response
    build_response(&intent, &provider, &technology, &results)
}

/// Parse the user's query to extract intent, provider, technology, and keywords
fn parse_query_intent(query: &str) -> QueryIntent {
    let query_lower = query.to_lowercase();
    let query_trimmed = query.trim();

    // Detect query type
    let query_type = if HOWTO_PATTERNS.is_match(query_trimmed) {
        QueryType::HowTo
    } else if REFERENCE_PATTERNS.is_match(query_trimmed) {
        QueryType::Reference
    } else {
        QueryType::Search
    };

    // Detect provider and technology
    let (provider, technology) = detect_provider_and_technology(&query_lower);

    // Extract keywords (remove common stop words and query prefixes)
    let keywords = extract_keywords(&query_lower);

    QueryIntent {
        raw_query: query.to_string(),
        provider,
        technology,
        keywords,
        query_type,
    }
}

/// Check if a word exists as a whole word in the query (not as a substring of another word)
fn contains_word(query: &str, word: &str) -> bool {
    let query_words: Vec<&str> = query
        .split(|c: char| c.is_whitespace() || c == '-' || c == '_' || c == '/' || c == '.')
        .filter(|s| !s.is_empty())
        .collect();
    query_words.contains(&word)
}

/// Detect the provider and technology from the query
fn detect_provider_and_technology(query: &str) -> (Option<ProviderType>, Option<String>) {
    // Check for Apple frameworks first (most common case)
    for (name, identifier) in APPLE_FRAMEWORKS.iter() {
        if contains_word(query, name) {
            return (Some(ProviderType::Apple), Some(identifier.to_string()));
        }
    }

    // Check for iOS/macOS/Swift-related keywords that imply Apple
    if contains_word(query, "ios") || contains_word(query, "macos") || contains_word(query, "swift")
        || contains_word(query, "xcode") || contains_word(query, "apple")
    {
        // Default to SwiftUI if no specific framework detected
        return (
            Some(ProviderType::Apple),
            Some("doc://com.apple.documentation/documentation/swiftui".to_string()),
        );
    }

    // Check for ML/AI-related keywords that imply Apple CoreML
    if query.contains("machine learning") || query.contains("neural network")
        || query.contains("ml model") || query.contains("model inference")
        || query.contains("bnns") || query.contains("image classification")
        || query.contains("object detection") || query.contains("text recognition")
        || query.contains("face detection") || query.contains("pose estimation")
        || query.contains("sentiment analysis") || query.contains("language model")
    {
        // Default to CoreML for general ML queries
        return (
            Some(ProviderType::Apple),
            Some("doc://com.apple.documentation/documentation/coreml".to_string()),
        );
    }

    // Check for Rust crates
    for crate_name in RUST_CRATES.iter() {
        if contains_word(query, crate_name) {
            return (Some(ProviderType::Rust), Some(format!("rust:{}", crate_name)));
        }
    }

    // Check for Telegram keywords
    for keyword in TELEGRAM_KEYWORDS.iter() {
        if contains_word(query, keyword) {
            return (Some(ProviderType::Telegram), Some("telegram:methods".to_string()));
        }
    }

    // Check for TON keywords (use word boundary to avoid "button" matching "ton")
    for keyword in TON_KEYWORDS.iter() {
        if contains_word(query, keyword) {
            return (Some(ProviderType::TON), Some("ton:accounts".to_string()));
        }
    }

    // Check for Cocoon keywords
    if contains_word(query, "cocoon") || query.contains("confidential computing") || contains_word(query, "tdx") {
        return (Some(ProviderType::Cocoon), Some("cocoon:architecture".to_string()));
    }

    // Check for React keywords (before general MDN keywords since React uses JS)
    for keyword in REACT_KEYWORDS.iter() {
        if contains_word(query, keyword) {
            return (Some(ProviderType::WebFrameworks), Some("webfw:react".to_string()));
        }
    }

    // Check for Next.js keywords
    for keyword in NEXTJS_KEYWORDS.iter() {
        if contains_word(query, keyword) {
            return (Some(ProviderType::WebFrameworks), Some("webfw:nextjs".to_string()));
        }
    }

    // Check for Node.js keywords
    for keyword in NODEJS_KEYWORDS.iter() {
        if contains_word(query, keyword) {
            return (Some(ProviderType::WebFrameworks), Some("webfw:nodejs".to_string()));
        }
    }

    // Check for MLX keywords (Apple Silicon ML)
    for keyword in MLX_KEYWORDS.iter() {
        if contains_word(query, keyword) || query.contains(keyword) {
            // Determine if Swift or Python based on context
            let tech = if query.contains("swift") || query.contains("ios") || query.contains("macos") {
                "mlx:swift"
            } else {
                "mlx:python"
            };
            return (Some(ProviderType::Mlx), Some(tech.to_string()));
        }
    }

    // Check for Hugging Face keywords
    for keyword in HUGGINGFACE_KEYWORDS.iter() {
        if contains_word(query, keyword) || query.contains(keyword) {
            // Determine if Swift Transformers or Python Transformers
            let tech = if query.contains("swift") {
                "hf:swift-transformers"
            } else {
                "hf:transformers"
            };
            return (Some(ProviderType::HuggingFace), Some(tech.to_string()));
        }
    }

    // Check for QuickNode/Solana keywords
    for keyword in QUICKNODE_KEYWORDS.iter() {
        if contains_word(query, keyword) || query.contains(keyword) {
            // Determine category based on query content
            let tech = if query.contains("websocket") || query.contains("subscribe") {
                "quicknode:solana:websocket"
            } else if query.contains("jito") || query.contains("metaplex") || query.contains("das") || query.contains("yellowstone") {
                "quicknode:solana:marketplace"
            } else {
                "quicknode:solana:http"
            };
            return (Some(ProviderType::QuickNode), Some(tech.to_string()));
        }
    }

    // Check for Claude Agent SDK keywords (before MDN since SDK uses JavaScript/TypeScript)
    for keyword in CLAUDE_AGENT_SDK_KEYWORDS.iter() {
        if contains_word(query, keyword) || query.contains(keyword) {
            // Determine language based on query content
            let tech = if query.contains("python") || query.contains("@tool") || query.contains("cli_path") {
                "agent-sdk:python"
            } else if query.contains("typescript") || query.contains("javascript") || query.contains("node") {
                "agent-sdk:typescript"
            } else {
                // Default to TypeScript
                "agent-sdk:typescript"
            };
            return (Some(ProviderType::ClaudeAgentSdk), Some(tech.to_string()));
        }
    }

    // Check for MDN/JavaScript keywords
    for keyword in MDN_KEYWORDS.iter() {
        if contains_word(query, keyword) {
            return (Some(ProviderType::Mdn), Some("mdn:javascript".to_string()));
        }
    }

    // Default: no specific provider detected, will use current active
    (None, None)
}

/// Extract meaningful keywords from the query
fn extract_keywords(query: &str) -> Vec<String> {
    // Common stop words and query prefixes to remove
    static STOP_WORDS: Lazy<Vec<&'static str>> = Lazy::new(|| {
        vec![
            "how", "do", "i", "to", "the", "a", "an", "in", "on", "for", "with",
            "what", "is", "are", "can", "could", "would", "should", "use", "using",
            "implement", "create", "make", "build", "add", "get", "set", "show",
            "me", "please", "want", "need", "like", "way", "best", "proper",
            "tell", "about", "explain", "describe", "documentation", "docs", "api",
        ]
    });

    query
        .split(|c: char| c.is_whitespace() || c == '-' || c == '_' || c == '/' || c == '.')
        .filter(|word| !word.is_empty() && word.len() > 1)
        .filter(|word| !STOP_WORDS.contains(word))
        .map(String::from)
        .collect()
}

/// Resolve and set the appropriate technology based on intent
async fn resolve_technology(
    context: &Arc<AppContext>,
    intent: &QueryIntent,
) -> Result<(ProviderType, String)> {
    // If we detected a specific provider/technology, set it
    if let (Some(provider), Some(tech_id)) = (&intent.provider, &intent.technology) {
        // Set the active provider
        *context.state.active_provider.write().await = *provider;

        match provider {
            ProviderType::Apple => {
                // Clear cached framework data to force reload for new technology
                *context.state.framework_cache.write().await = None;
                *context.state.framework_index.write().await = None;

                // Find and set the Apple technology
                let technologies = context.client.get_technologies().await?;
                if let Some(tech) = technologies.get(tech_id) {
                    *context.state.active_technology.write().await = Some(tech.clone());
                    return Ok((*provider, tech.title.clone()));
                }
                // Fallback: create a basic technology object
                let title = tech_id
                    .split('/')
                    .next_back()
                    .unwrap_or("Unknown")
                    .to_string();
                let capitalized = title
                    .chars()
                    .next()
                    .map(|c| c.to_uppercase().to_string())
                    .unwrap_or_default()
                    + &title[1..];
                let fallback_tech = docs_mcp_client::types::Technology {
                    identifier: tech_id.clone(),
                    title: capitalized.clone(),
                    r#abstract: vec![],
                    kind: "symbol".to_string(),
                    role: "collection".to_string(),
                    url: format!("https://developer.apple.com/documentation/{}", title),
                };
                *context.state.active_technology.write().await = Some(fallback_tech);
                Ok((*provider, capitalized))
            }
            ProviderType::Rust => {
                let crate_name = tech_id.strip_prefix("rust:").unwrap_or("std");
                let unified = UnifiedTechnology {
                    identifier: tech_id.clone(),
                    title: format!("Rust {}", crate_name),
                    description: format!("Rust {} crate documentation", crate_name),
                    provider: ProviderType::Rust,
                    url: Some(format!("https://docs.rs/{}", crate_name)),
                    kind: multi_provider_client::types::TechnologyKind::RustCrate,
                };
                *context.state.active_unified_technology.write().await = Some(unified);
                Ok((*provider, format!("Rust {}", crate_name)))
            }
            ProviderType::Telegram => {
                let unified = UnifiedTechnology {
                    identifier: tech_id.clone(),
                    title: "Telegram Bot API".to_string(),
                    description: "Telegram Bot API methods and types".to_string(),
                    provider: ProviderType::Telegram,
                    url: Some("https://core.telegram.org/bots/api".to_string()),
                    kind: multi_provider_client::types::TechnologyKind::ApiCategory,
                };
                *context.state.active_unified_technology.write().await = Some(unified);
                Ok((*provider, "Telegram Bot API".to_string()))
            }
            ProviderType::TON => {
                let unified = UnifiedTechnology {
                    identifier: tech_id.clone(),
                    title: "TON API".to_string(),
                    description: "TON blockchain API".to_string(),
                    provider: ProviderType::TON,
                    url: Some("https://tonapi.io/docs".to_string()),
                    kind: multi_provider_client::types::TechnologyKind::BlockchainApi,
                };
                *context.state.active_unified_technology.write().await = Some(unified);
                Ok((*provider, "TON API".to_string()))
            }
            ProviderType::Cocoon => {
                let unified = UnifiedTechnology {
                    identifier: tech_id.clone(),
                    title: "Cocoon".to_string(),
                    description: "Cocoon confidential computing".to_string(),
                    provider: ProviderType::Cocoon,
                    url: Some("https://cocoon.dev/docs".to_string()),
                    kind: multi_provider_client::types::TechnologyKind::DocSection,
                };
                *context.state.active_unified_technology.write().await = Some(unified);
                Ok((*provider, "Cocoon".to_string()))
            }
            ProviderType::Mdn => {
                let unified = UnifiedTechnology {
                    identifier: tech_id.clone(),
                    title: "MDN Web Docs".to_string(),
                    description: "JavaScript, Web APIs, and TypeScript documentation".to_string(),
                    provider: ProviderType::Mdn,
                    url: Some("https://developer.mozilla.org".to_string()),
                    kind: multi_provider_client::types::TechnologyKind::MdnCategory,
                };
                *context.state.active_unified_technology.write().await = Some(unified);
                Ok((*provider, "MDN Web Docs".to_string()))
            }
            ProviderType::WebFrameworks => {
                // Parse framework from tech_id (e.g., "webfw:react" -> "React")
                let framework_name = tech_id
                    .strip_prefix("webfw:")
                    .map(|f| match f {
                        "react" => "React",
                        "nextjs" => "Next.js",
                        "nodejs" => "Node.js",
                        _ => "React",
                    })
                    .unwrap_or("React");
                let unified = UnifiedTechnology {
                    identifier: tech_id.clone(),
                    title: framework_name.to_string(),
                    description: format!("{} documentation", framework_name),
                    provider: ProviderType::WebFrameworks,
                    url: Some(match framework_name {
                        "React" => "https://react.dev".to_string(),
                        "Next.js" => "https://nextjs.org/docs".to_string(),
                        "Node.js" => "https://nodejs.org/api".to_string(),
                        _ => "https://react.dev".to_string(),
                    }),
                    kind: multi_provider_client::types::TechnologyKind::WebFramework,
                };
                *context.state.active_unified_technology.write().await = Some(unified);
                Ok((*provider, framework_name.to_string()))
            }
            ProviderType::Mlx => {
                // Parse language from tech_id (e.g., "mlx:swift" -> "MLX Swift")
                let lang_name = tech_id
                    .strip_prefix("mlx:")
                    .map(|l| match l {
                        "swift" => "MLX Swift",
                        "python" => "MLX Python",
                        _ => "MLX Swift",
                    })
                    .unwrap_or("MLX Swift");
                let unified = UnifiedTechnology {
                    identifier: tech_id.clone(),
                    title: lang_name.to_string(),
                    description: format!("{} - Machine learning on Apple Silicon", lang_name),
                    provider: ProviderType::Mlx,
                    url: Some("https://ml-explore.github.io/mlx-swift/documentation/mlx".to_string()),
                    kind: multi_provider_client::types::TechnologyKind::MlxFramework,
                };
                *context.state.active_unified_technology.write().await = Some(unified);
                Ok((*provider, lang_name.to_string()))
            }
            ProviderType::HuggingFace => {
                // Parse technology from tech_id (e.g., "hf:transformers" -> "Transformers")
                let tech_name = tech_id
                    .strip_prefix("hf:")
                    .map(|t| match t {
                        "transformers" => "Transformers",
                        "swift-transformers" => "Swift Transformers",
                        "models" => "Models",
                        _ => "Transformers",
                    })
                    .unwrap_or("Transformers");
                let unified = UnifiedTechnology {
                    identifier: tech_id.clone(),
                    title: tech_name.to_string(),
                    description: format!("Hugging Face {} documentation", tech_name),
                    provider: ProviderType::HuggingFace,
                    url: Some("https://huggingface.co/docs/transformers".to_string()),
                    kind: multi_provider_client::types::TechnologyKind::HfLibrary,
                };
                *context.state.active_unified_technology.write().await = Some(unified);
                Ok((*provider, tech_name.to_string()))
            }
            ProviderType::QuickNode => {
                // Parse category from tech_id (e.g., "quicknode:solana:http" -> "Solana HTTP Methods")
                let category_name = tech_id
                    .strip_prefix("quicknode:solana:")
                    .map(|c| match c {
                        "http" => "Solana HTTP Methods",
                        "websocket" => "Solana WebSocket Methods",
                        "marketplace" => "Solana Marketplace Add-ons",
                        _ => "Solana HTTP Methods",
                    })
                    .unwrap_or("Solana HTTP Methods");
                let unified = UnifiedTechnology {
                    identifier: tech_id.clone(),
                    title: category_name.to_string(),
                    description: format!("QuickNode {} documentation", category_name),
                    provider: ProviderType::QuickNode,
                    url: Some("https://www.quicknode.com/docs/solana".to_string()),
                    kind: multi_provider_client::types::TechnologyKind::QuickNodeApi,
                };
                *context.state.active_unified_technology.write().await = Some(unified);
                Ok((*provider, category_name.to_string()))
            }
            ProviderType::ClaudeAgentSdk => {
                // Parse language from tech_id (e.g., "agent-sdk:typescript" -> "Claude Agent SDK (TypeScript)")
                let lang_name = tech_id
                    .strip_prefix("agent-sdk:")
                    .map(|l| match l {
                        "typescript" => "Claude Agent SDK (TypeScript)",
                        "python" => "Claude Agent SDK (Python)",
                        _ => "Claude Agent SDK (TypeScript)",
                    })
                    .unwrap_or("Claude Agent SDK (TypeScript)");
                let unified = UnifiedTechnology {
                    identifier: tech_id.clone(),
                    title: lang_name.to_string(),
                    description: "Build AI agents with Claude Code capabilities".to_string(),
                    provider: ProviderType::ClaudeAgentSdk,
                    url: Some("https://docs.anthropic.com/en/docs/agents-and-tools/claude-agent-sdk".to_string()),
                    kind: multi_provider_client::types::TechnologyKind::AgentSdkLibrary,
                };
                *context.state.active_unified_technology.write().await = Some(unified);
                Ok((*provider, lang_name.to_string()))
            }
        }
    } else {
        // No provider detected - check if there's an active technology, otherwise default to Apple/SwiftUI
        let current_provider = *context.state.active_provider.read().await;
        let has_active_tech = match current_provider {
            ProviderType::Apple => context.state.active_technology.read().await.is_some(),
            _ => context.state.active_unified_technology.read().await.is_some(),
        };

        if has_active_tech {
            // Use the currently active provider/technology
            let tech_name = match current_provider {
                ProviderType::Apple => context
                    .state
                    .active_technology
                    .read()
                    .await
                    .as_ref()
                    .map(|t| t.title.clone())
                    .unwrap_or_else(|| "SwiftUI".to_string()),
                _ => context
                    .state
                    .active_unified_technology
                    .read()
                    .await
                    .as_ref()
                    .map(|t| t.title.clone())
                    .unwrap_or_else(|| "Unknown".to_string()),
            };
            Ok((current_provider, tech_name))
        } else {
            // Default to Apple/SwiftUI when nothing is active
            *context.state.active_provider.write().await = ProviderType::Apple;
            // Clear cached framework data
            *context.state.framework_cache.write().await = None;
            *context.state.framework_index.write().await = None;

            let technologies = context.client.get_technologies().await?;
            let swiftui_id = "doc://com.apple.documentation/documentation/swiftui";
            if let Some(tech) = technologies.get(swiftui_id) {
                *context.state.active_technology.write().await = Some(tech.clone());
                Ok((ProviderType::Apple, tech.title.clone()))
            } else {
                // Create a minimal SwiftUI technology
                let fallback = docs_mcp_client::types::Technology {
                    identifier: swiftui_id.to_string(),
                    title: "SwiftUI".to_string(),
                    r#abstract: vec![],
                    kind: "symbol".to_string(),
                    role: "collection".to_string(),
                    url: "https://developer.apple.com/documentation/swiftui".to_string(),
                };
                *context.state.active_technology.write().await = Some(fallback);
                Ok((ProviderType::Apple, "SwiftUI".to_string()))
            }
        }
    }
}

/// Execute a how-to query - focuses on recipes and guided steps
async fn execute_howto_query(
    context: &Arc<AppContext>,
    intent: &QueryIntent,
    max_results: usize,
) -> Result<Vec<DocResult>> {
    // Get the technology name for knowledge base lookups
    let tech_name = intent.technology.as_deref().unwrap_or("SwiftUI");

    // Search for relevant symbols
    let mut results = execute_search_query(context, intent, max_results).await?;

    // Enhance with knowledge base tips if available
    for result in &mut results {
        if let Some(entry) = knowledge::lookup(tech_name, &result.title) {
            if let Some(tip) = entry.quick_tip {
                result.summary = format!("{}\n\n**Tip:** {}", result.summary, tip);
            }
        }
    }

    Ok(results)
}

/// Execute a reference query - focuses on detailed documentation
async fn execute_reference_query(
    context: &Arc<AppContext>,
    intent: &QueryIntent,
    max_results: usize,
) -> Result<Vec<DocResult>> {
    // Similar to search but with more detail emphasis
    execute_search_query(context, intent, max_results).await
}

/// Execute a general search query
async fn execute_search_query(
    context: &Arc<AppContext>,
    intent: &QueryIntent,
    max_results: usize,
) -> Result<Vec<DocResult>> {
    let provider = *context.state.active_provider.read().await;

    // Filter out ONLY provider name keywords - keep actual search terms like "wallet", "bot"
    let provider_keywords: Vec<&str> = vec![
        // Apple framework names (but not concepts like "button", "list")
        "swiftui", "uikit", "foundation", "swift", "ios", "macos", "apple",
        "appkit", "coredata", "cloudkit", "combine", "realitykit", "arkit",
        // Rust but not crate names that might be search terms
        "rust", "crate", "cargo",
        // Telegram but not "bot" as that might be a search term
        "telegram",
        // TON blockchain but not "wallet" as that's a search term
        "ton", "blockchain", "tonapi",
        // Cocoon
        "cocoon",
        // MLX but not ML concepts like "array", "neural"
        "mlx", "mlxswift",
        // Hugging Face but not model names that might be search terms
        "huggingface", "hf", "transformers",
        // Claude Agent SDK provider names only - keep class names like "claudesdkclient", "claudeclient"
        "claude", "agent", "sdk", "claudeagentsdk",
    ];

    let search_keywords: Vec<&str> = intent
        .keywords
        .iter()
        .map(|s| s.as_str())
        .filter(|k| !provider_keywords.contains(k))
        .collect();

    // Use filtered keywords, or fall back to original if all were filtered
    let search_query = if search_keywords.is_empty() {
        intent.keywords.join(" ")
    } else {
        search_keywords.join(" ")
    };

    match provider {
        ProviderType::Apple => search_apple(context, &search_query, max_results).await,
        ProviderType::Rust => search_rust(context, intent, &search_query, max_results).await,
        ProviderType::Telegram => search_telegram(context, &search_query, max_results).await,
        ProviderType::TON => search_ton(context, &search_query, max_results).await,
        ProviderType::Cocoon => search_cocoon(context, &search_query, max_results).await,
        ProviderType::Mdn => search_mdn(context, &search_query, max_results).await,
        ProviderType::WebFrameworks => search_web_frameworks(context, intent, &search_query, max_results).await,
        ProviderType::Mlx => search_mlx(context, intent, &search_query, max_results).await,
        ProviderType::HuggingFace => search_huggingface(context, intent, &search_query, max_results).await,
        ProviderType::QuickNode => search_quicknode(context, &search_query, max_results).await,
        ProviderType::ClaudeAgentSdk => search_claude_agent_sdk(context, intent, &search_query, max_results).await,
    }
}

/// Synonym expansion for Apple documentation search
static SEARCH_SYNONYMS: Lazy<std::collections::HashMap<&'static str, Vec<&'static str>>> = Lazy::new(|| {
    std::collections::HashMap::from([
        ("button", vec!["control", "action", "tap", "press", "click", "controls"]),
        ("list", vec!["table", "collection", "outline", "foreach", "tableview"]),
        ("table", vec!["list", "collection", "tableview", "uitableview", "grid"]),
        ("tableview", vec!["table", "list", "uitableview", "collection", "datasource", "delegate"]),
        ("navigation", vec!["stack", "navigator", "navigationstack", "routing", "navigationcontroller"]),
        ("text", vec!["label", "string", "typography", "uilabel", "textfield"]),
        ("image", vec!["photo", "picture", "icon", "asyncimage", "uiimage", "imageview"]),
        ("stack", vec!["vstack", "hstack", "zstack", "layout", "stackview"]),
        ("form", vec!["settings", "preferences", "input"]),
        ("alert", vec!["dialog", "notification", "popup", "uialert"]),
        ("sheet", vec!["modal", "presentation", "popover"]),
        ("animation", vec!["transition", "animate", "motion", "uiview"]),
        ("gesture", vec!["tap", "drag", "swipe", "touch", "recognizer"]),
        ("state", vec!["binding", "observable", "published"]),
        ("view", vec!["ui", "component", "widget", "uiview", "viewcontroller"]),
        ("menu", vec!["picker", "dropdown", "contextmenu"]),
        ("search", vec!["find", "lookup", "searchable", "filter", "searchbar"]),
        ("toolbar", vec!["navigationbar", "actions", "bar", "uitoolbar"]),
        ("tab", vec!["segmented", "page", "tabview", "tabbar", "uitabbar"]),
        ("controller", vec!["viewcontroller", "uiviewcontroller", "navigation"]),
    ])
});

/// Search Apple documentation
async fn search_apple(
    context: &Arc<AppContext>,
    query: &str,
    max_results: usize,
) -> Result<Vec<DocResult>> {
    use docs_mcp_client::types::extract_text;

    // Ensure a technology is selected
    let _tech = context
        .state
        .active_technology
        .read()
        .await
        .clone()
        .context("No Apple technology selected")?;

    // Load the framework index
    let mut index = ensure_framework_index(context).await?;

    // Build search terms with synonym expansion
    let query_lower = query.to_lowercase();
    let base_terms: Vec<&str> = query_lower.split_whitespace().collect();

    // Expand terms with synonyms
    let mut all_terms: Vec<String> = base_terms.iter().map(|s| s.to_string()).collect();
    for term in &base_terms {
        if let Some(synonyms) = SEARCH_SYNONYMS.get(term) {
            all_terms.extend(synonyms.iter().map(|s| s.to_string()));
        }
    }

    let mut matches: Vec<(i32, &crate::state::FrameworkIndexEntry)> = index
        .iter()
        .filter_map(|entry| {
            let title_lower = entry
                .reference
                .title
                .as_deref()
                .unwrap_or_default()
                .to_lowercase();

            // Also check abstract/description
            let abstract_lower = entry
                .reference
                .r#abstract
                .as_ref()
                .map(|a| docs_mcp_client::types::extract_text(a).to_lowercase())
                .unwrap_or_default();

            let mut score = 0i32;
            for term in &all_terms {
                // Exact title match gets highest score
                if title_lower.contains(term) {
                    score += 15;
                }
                // Abstract match
                if abstract_lower.contains(term) {
                    score += 5;
                }
                // Token match
                for token in &entry.tokens {
                    if token.contains(term) {
                        score += 2;
                    }
                }
            }

            // Boost symbols over articles/collections (symbols have code samples)
            if score > 0 {
                let kind = entry.reference.kind.as_deref().unwrap_or_default();
                if matches!(kind, "struct" | "class" | "protocol" | "enum" | "typealias" | "func" | "var" | "property" | "method") {
                    score += 20; // Significantly boost actual symbols
                } else if matches!(kind, "article" | "collection" | "collectionGroup") {
                    score -= 5; // Slightly penalize article pages
                }
            }

            if score > 0 {
                Some((score, entry))
            } else {
                None
            }
        })
        .collect();

    matches.sort_by(|a, b| b.0.cmp(&a.0));

    // If no good symbol matches found (only articles/collections), expand the index with symbols from topic sections
    let has_symbol_matches = matches.iter().take(5).any(|(_, entry)| {
        let kind = entry.reference.kind.as_deref().unwrap_or_default();
        matches!(kind, "struct" | "class" | "protocol" | "enum" | "typealias" | "func" | "var" | "property" | "method")
    });

    if matches.is_empty() || !has_symbol_matches {
        use crate::services::{expand_identifiers, load_active_framework};
        let framework = load_active_framework(context).await?;
        let identifiers: Vec<String> = framework
            .topic_sections
            .iter()
            .flat_map(|section| section.identifiers.iter().cloned())
            .take(200)
            .collect();
        if !identifiers.is_empty() {
            index = expand_identifiers(context, &identifiers).await?;

            // Re-search with expanded index
            matches = index
                .iter()
                .filter_map(|entry| {
                    let title_lower = entry
                        .reference
                        .title
                        .as_deref()
                        .unwrap_or_default()
                        .to_lowercase();

                    let abstract_lower = entry
                        .reference
                        .r#abstract
                        .as_ref()
                        .map(|a| docs_mcp_client::types::extract_text(a).to_lowercase())
                        .unwrap_or_default();

                    let mut score = 0i32;
                    for term in &all_terms {
                        if title_lower.contains(term) {
                            score += 15;
                        }
                        if abstract_lower.contains(term) {
                            score += 5;
                        }
                        for token in &entry.tokens {
                            if token.contains(term) {
                                score += 2;
                            }
                        }
                    }

                    // Boost symbols over articles/collections
                    if score > 0 {
                        let kind = entry.reference.kind.as_deref().unwrap_or_default();
                        if matches!(kind, "struct" | "class" | "protocol" | "enum" | "typealias" | "func" | "var" | "property" | "method") {
                            score += 20;
                        } else if matches!(kind, "article" | "collection" | "collectionGroup") {
                            score -= 5;
                        }
                    }

                    if score > 0 {
                        Some((score, entry))
                    } else {
                        None
                    }
                })
                .collect();

            matches.sort_by(|a, b| b.0.cmp(&a.0));
        }
    }

    let mut results = Vec::new();
    for (_, entry) in matches.into_iter().take(max_results) {
        let title = entry
            .reference
            .title
            .clone()
            .unwrap_or_else(|| "Symbol".to_string());
        let kind = entry
            .reference
            .kind
            .clone()
            .unwrap_or_else(|| "unknown".to_string());
        let path = entry
            .reference
            .url
            .clone()
            .unwrap_or_else(|| entry.id.clone());
        let summary = entry
            .reference
            .r#abstract
            .as_ref()
            .map(|segments| extract_text(segments))
            .unwrap_or_default();
        let platforms = entry
            .reference
            .platforms
            .as_ref()
            .map(|p| docs_mcp_client::types::format_platforms(p));

        results.push(DocResult {
            title,
            kind,
            path,
            summary,
            platforms,
            code_sample: None,
            related_apis: Vec::new(),
            full_content: None,
            declaration: None,
            parameters: Vec::new(),
        });
    }

    // Fetch detailed docs for top results (with full content)
    for result in results.iter_mut().take(MAX_DETAILED_DOCS) {
        if let Ok(doc) = context.client.load_document(&result.path).await {
            if let Ok(symbol) = serde_json::from_value::<docs_mcp_client::types::SymbolData>(doc.clone()) {
                // Extract code sample if available
                result.code_sample = extract_code_sample(&symbol);

                // Extract declaration/signature
                result.declaration = extract_declaration(&symbol);

                // Extract parameters
                result.parameters = extract_parameters(&symbol);

                // Extract full documentation content
                result.full_content = extract_full_content(&symbol);

                // Extract related APIs
                result.related_apis = symbol
                    .topic_sections
                    .iter()
                    .flat_map(|s| s.identifiers.iter())
                    .take(8)
                    .filter_map(|id| symbol.references.get(id)?.title.clone())
                    .collect();
            }
        }
    }

    Ok(results)
}

/// Search Rust documentation
async fn search_rust(
    context: &Arc<AppContext>,
    intent: &QueryIntent,
    query: &str,
    max_results: usize,
) -> Result<Vec<DocResult>> {
    let crate_name = intent
        .technology
        .as_ref()
        .and_then(|t| t.strip_prefix("rust:"))
        .unwrap_or("std");

    let items = match context.providers.rust.search(crate_name, query).await {
        Ok(items) => items,
        Err(e) => {
            tracing::warn!(error = %e, crate_name = %crate_name, "Rust search failed, returning empty results");
            return Ok(Vec::new());
        }
    };

    let results = items
        .into_iter()
        .take(max_results)
        .map(|item| {
            let full_content = if !item.summary.is_empty() {
                Some(item.summary.clone())
            } else {
                None
            };
            DocResult {
                title: item.name,
                kind: format!("{:?}", item.kind),
                path: item.path.clone(),
                summary: item.summary,
                platforms: Some(format!("{} v{}", item.crate_name, item.crate_version)),
                code_sample: item.examples.first().map(|e| e.code.clone()),
                related_apis: item.methods.iter().take(8).map(|m| m.name.clone()).collect(),
                full_content,
                declaration: Some(item.path),
                parameters: Vec::new(),
            }
        })
        .collect();

    Ok(results)
}

/// Search Telegram Bot API
async fn search_telegram(
    context: &Arc<AppContext>,
    query: &str,
    max_results: usize,
) -> Result<Vec<DocResult>> {
    let items = match context.providers.telegram.search(query).await {
        Ok(items) => items,
        Err(e) => {
            tracing::warn!(error = %e, "Telegram search failed, returning empty results");
            return Ok(Vec::new());
        }
    };

    let results = items
        .into_iter()
        .take(max_results)
        .map(|item| {
            let path = item.name.clone();
            let parameters: Vec<(String, String)> = item
                .fields
                .iter()
                .map(|f| (f.name.clone(), f.description.clone()))
                .collect();
            DocResult {
                title: item.name,
                kind: item.kind,
                path,
                summary: item.description.clone(),
                platforms: Some("Telegram Bot API".to_string()),
                code_sample: None,
                related_apis: item.fields.iter().take(8).map(|f| f.name.clone()).collect(),
                full_content: Some(item.description),
                declaration: None,
                parameters,
            }
        })
        .collect();

    Ok(results)
}

/// Search TON API
async fn search_ton(
    context: &Arc<AppContext>,
    query: &str,
    max_results: usize,
) -> Result<Vec<DocResult>> {
    let items = match context.providers.ton.search(query).await {
        Ok(items) => items,
        Err(e) => {
            tracing::warn!(error = %e, "TON search failed, returning empty results");
            return Ok(Vec::new());
        }
    };

    let results = items
        .into_iter()
        .take(max_results)
        .map(|item| {
            let summary = item.summary.clone().unwrap_or_else(|| item.description.clone().unwrap_or_default());
            let parameters: Vec<(String, String)> = item
                .parameters
                .iter()
                .map(|p| (p.name.clone(), p.description.clone().unwrap_or_default()))
                .collect();
            DocResult {
                title: item.operation_id.clone(),
                kind: item.method.to_uppercase(),
                path: item.operation_id,
                summary: summary.clone(),
                platforms: Some("TON API".to_string()),
                code_sample: None,
                related_apis: item.parameters.iter().take(8).map(|p| p.name.clone()).collect(),
                full_content: Some(summary),
                declaration: None,
                parameters,
            }
        })
        .collect();

    Ok(results)
}

/// Search Cocoon documentation
async fn search_cocoon(
    context: &Arc<AppContext>,
    query: &str,
    max_results: usize,
) -> Result<Vec<DocResult>> {
    // Use the client's search method which searches all docs files
    let docs = context
        .providers
        .cocoon
        .search(query)
        .await
        .unwrap_or_default();

    // Fetch full content for top results
    let mut results = Vec::new();
    for doc in docs.into_iter().take(max_results) {
        let full_content = if results.len() < MAX_DETAILED_DOCS {
            // Fetch full document content for top results
            context
                .providers
                .cocoon
                .get_document(&doc.path)
                .await
                .ok()
                .map(|d| d.content)
        } else {
            None
        };

        results.push(DocResult {
            title: doc.title,
            kind: "Document".to_string(),
            path: doc.path,
            summary: doc.summary,
            platforms: Some("Cocoon".to_string()),
            code_sample: None,
            related_apis: Vec::new(),
            full_content,
            declaration: None,
            parameters: Vec::new(),
        });
    }

    Ok(results)
}

/// Search MDN Web Docs
async fn search_mdn(
    context: &Arc<AppContext>,
    query: &str,
    max_results: usize,
) -> Result<Vec<DocResult>> {
    let items = match context.providers.mdn.search(query).await {
        Ok(items) => items,
        Err(e) => {
            tracing::warn!(error = %e, "MDN search failed, returning empty results");
            return Ok(Vec::new());
        }
    };

    let mut results = Vec::new();
    for item in items.into_iter().take(max_results) {
        // Fetch full article for top results
        let (full_content, code_sample, parameters) = if results.len() < MAX_DETAILED_DOCS {
            match context.providers.mdn.get_article(&item.slug).await {
                Ok(article) => {
                    let code = article.examples.first().map(|e| e.code.clone());
                    let params: Vec<(String, String)> = article
                        .parameters
                        .iter()
                        .map(|p| (p.name.clone(), p.description.clone()))
                        .collect();
                    let content = if !article.summary.is_empty() {
                        Some(article.summary.clone())
                    } else {
                        None
                    };
                    (content, code, params)
                }
                Err(_) => (None, None, Vec::new()),
            }
        } else {
            (None, None, Vec::new())
        };

        results.push(DocResult {
            title: item.title.clone(),
            kind: "Article".to_string(),
            path: item.slug.clone(),
            summary: item.summary.clone(),
            platforms: Some("MDN Web Docs".to_string()),
            code_sample,
            related_apis: Vec::new(),
            full_content,
            declaration: None,
            parameters,
        });
    }

    Ok(results)
}

/// Search Web Frameworks documentation (React, Next.js, Node.js)
async fn search_web_frameworks(
    context: &Arc<AppContext>,
    intent: &QueryIntent,
    query: &str,
    max_results: usize,
) -> Result<Vec<DocResult>> {
    use multi_provider_client::web_frameworks::types::WebFramework;

    // Determine which framework to search based on the technology identifier
    let framework = intent
        .technology
        .as_ref()
        .and_then(|t| t.strip_prefix("webfw:"))
        .map(|f| match f {
            "react" => WebFramework::React,
            "nextjs" => WebFramework::NextJs,
            "nodejs" => WebFramework::NodeJs,
            _ => WebFramework::React,
        })
        .unwrap_or(WebFramework::React);

    let items = match context.providers.web_frameworks.search(framework, query).await {
        Ok(items) => items,
        Err(e) => {
            tracing::warn!(error = %e, "Web Frameworks search failed, returning empty results");
            return Ok(Vec::new());
        }
    };

    let framework_name = match framework {
        WebFramework::React => "React",
        WebFramework::NextJs => "Next.js",
        WebFramework::NodeJs => "Node.js",
    };

    let mut results = Vec::new();
    for item in items.into_iter().take(max_results) {
        // Fetch full article for top results
        let (full_content, code_sample) = if results.len() < MAX_DETAILED_DOCS {
            match context.providers.web_frameworks.get_article(framework, &item.slug).await {
                Ok(article) => {
                    let code = article
                        .examples
                        .iter()
                        .max_by_key(|e| e.quality_score())
                        .map(|e| e.code.clone());
                    let content = if !article.content.is_empty() {
                        Some(trim_text(&article.content, MAX_CONTENT_LENGTH))
                    } else {
                        None
                    };
                    (content, code)
                }
                Err(_) => (None, None),
            }
        } else {
            (None, None)
        };

        results.push(DocResult {
            title: item.title.clone(),
            kind: item.category.clone().unwrap_or_else(|| "Article".to_string()),
            path: item.slug.clone(),
            summary: item.description.clone(),
            platforms: Some(framework_name.to_string()),
            code_sample,
            related_apis: Vec::new(),
            full_content,
            declaration: None,
            parameters: Vec::new(),
        });
    }

    Ok(results)
}

/// Search MLX documentation (Apple Silicon ML framework)
async fn search_mlx(
    context: &Arc<AppContext>,
    intent: &QueryIntent,
    query: &str,
    max_results: usize,
) -> Result<Vec<DocResult>> {
    use multi_provider_client::mlx::types::MlxLanguage;

    // Determine if Swift or Python based on technology identifier
    let language = intent
        .technology
        .as_ref()
        .and_then(|t| t.strip_prefix("mlx:"))
        .map(|l| match l {
            "swift" => Some(MlxLanguage::Swift),
            "python" => Some(MlxLanguage::Python),
            _ => None,
        })
        .flatten();

    let items = match context.providers.mlx.search(query, language).await {
        Ok(items) => items,
        Err(e) => {
            tracing::warn!(error = %e, "MLX search failed, returning empty results");
            return Ok(Vec::new());
        }
    };

    let mut results = Vec::new();
    for item in items.into_iter().take(max_results) {
        // Fetch full article for top results
        let (full_content, code_sample, declaration) = if results.len() < MAX_DETAILED_DOCS {
            match context.providers.mlx.get_article(&item.path, item.language).await {
                Ok(article) => {
                    let code = article.examples.first().map(|e| e.code.clone());
                    let content = if !article.content.is_empty() {
                        Some(trim_text(&article.content, MAX_CONTENT_LENGTH))
                    } else {
                        None
                    };
                    (content, code, article.declaration)
                }
                Err(_) => (None, None, None),
            }
        } else {
            (None, None, None)
        };

        results.push(DocResult {
            title: item.name.clone(),
            kind: item.kind.to_string(),
            path: item.path.clone(),
            summary: item.description.clone(),
            platforms: Some(format!("MLX {}", item.language)),
            code_sample,
            related_apis: Vec::new(),
            full_content,
            declaration,
            parameters: Vec::new(),
        });
    }

    Ok(results)
}

/// Search Hugging Face documentation
async fn search_huggingface(
    context: &Arc<AppContext>,
    intent: &QueryIntent,
    query: &str,
    max_results: usize,
) -> Result<Vec<DocResult>> {
    use multi_provider_client::huggingface::types::HfTechnologyKind;

    // Determine which technology to search
    let technology = intent
        .technology
        .as_ref()
        .and_then(|t| t.strip_prefix("hf:"))
        .map(|tech| match tech {
            "swift-transformers" => Some(HfTechnologyKind::SwiftTransformers),
            "transformers" => Some(HfTechnologyKind::Transformers),
            "models" => Some(HfTechnologyKind::Models),
            _ => None,
        })
        .flatten();

    let items = match context.providers.huggingface.search(query, technology).await {
        Ok(items) => items,
        Err(e) => {
            tracing::warn!(error = %e, "Hugging Face search failed, returning empty results");
            return Ok(Vec::new());
        }
    };

    let mut results = Vec::new();
    for item in items.into_iter().take(max_results) {
        // Fetch full article for top results
        let (full_content, code_sample, declaration, parameters) = if results.len() < MAX_DETAILED_DOCS {
            match context.providers.huggingface.get_article(&item.path, item.technology).await {
                Ok(article) => {
                    let code = article.examples.first().map(|e| e.code.clone());
                    let content = if !article.content.is_empty() {
                        Some(trim_text(&article.content, MAX_CONTENT_LENGTH))
                    } else {
                        None
                    };
                    let params: Vec<(String, String)> = article
                        .parameters
                        .iter()
                        .map(|p| (p.name.clone(), p.description.clone()))
                        .collect();
                    (content, code, article.declaration, params)
                }
                Err(_) => (None, None, None, Vec::new()),
            }
        } else {
            (None, None, None, Vec::new())
        };

        results.push(DocResult {
            title: item.name.clone(),
            kind: item.kind.to_string(),
            path: item.path.clone(),
            summary: item.description.clone(),
            platforms: Some(format!("Hugging Face {}", item.technology)),
            code_sample,
            related_apis: Vec::new(),
            full_content,
            declaration,
            parameters,
        });
    }

    Ok(results)
}

/// Search QuickNode Solana documentation
async fn search_quicknode(
    context: &Arc<AppContext>,
    query: &str,
    max_results: usize,
) -> Result<Vec<DocResult>> {
    let items = match context.providers.quicknode.search(query).await {
        Ok(items) => items,
        Err(e) => {
            tracing::warn!(error = %e, "QuickNode search failed, returning empty results");
            return Ok(Vec::new());
        }
    };

    let mut results = Vec::new();
    for item in items.into_iter().take(max_results) {
        // Fetch full method documentation for top results
        let (full_content, code_sample, parameters) = if results.len() < MAX_DETAILED_DOCS {
            match context.providers.quicknode.get_method(&item.name).await {
                Ok(method) => {
                    let code = method.examples.first().map(|e| e.code.clone());
                    let params: Vec<(String, String)> = method
                        .parameters
                        .iter()
                        .map(|p| (p.name.clone(), p.description.clone()))
                        .collect();
                    let content = if !method.description.is_empty() {
                        Some(method.description.clone())
                    } else {
                        None
                    };
                    (content, code, params)
                }
                Err(_) => (Some(item.description.clone()), None, Vec::new()),
            }
        } else {
            (None, None, Vec::new())
        };

        results.push(DocResult {
            title: item.name.clone(),
            kind: item.kind.to_string(),
            path: item.name,
            summary: item.description.clone(),
            platforms: Some("QuickNode Solana".to_string()),
            code_sample,
            related_apis: Vec::new(),
            full_content,
            declaration: None,
            parameters,
        });
    }

    Ok(results)
}

/// Search Claude Agent SDK documentation
async fn search_claude_agent_sdk(
    context: &Arc<AppContext>,
    intent: &QueryIntent,
    query: &str,
    max_results: usize,
) -> Result<Vec<DocResult>> {
    use multi_provider_client::claude_agent_sdk::types::AgentSdkLanguage;

    // Determine language from technology identifier
    let language = intent
        .technology
        .as_ref()
        .and_then(|t| t.strip_prefix("agent-sdk:"))
        .map(|l| match l {
            "python" => Some(AgentSdkLanguage::Python),
            "typescript" => Some(AgentSdkLanguage::TypeScript),
            _ => None,
        })
        .flatten();

    let items = match context.providers.claude_agent_sdk.search(query, language).await {
        Ok(items) => items,
        Err(e) => {
            tracing::warn!(error = %e, "Claude Agent SDK search failed, returning empty results");
            return Ok(Vec::new());
        }
    };

    let mut results = Vec::new();
    for item in items.into_iter().take(max_results) {
        // Fetch full article for top results
        let (full_content, code_sample, declaration, parameters) = if results.len() < MAX_DETAILED_DOCS {
            match context
                .providers
                .claude_agent_sdk
                .get_article(&item.path, item.language)
                .await
            {
                Ok(article) => {
                    let code = article.examples.first().map(|e| e.code.clone());
                    let content = if !article.content.is_empty() {
                        Some(trim_text(&article.content, MAX_CONTENT_LENGTH))
                    } else {
                        None
                    };
                    let params: Vec<(String, String)> = article
                        .parameters
                        .iter()
                        .map(|p| (p.name.clone(), p.description.clone()))
                        .collect();
                    (content, code, article.declaration, params)
                }
                Err(_) => (Some(item.description.clone()), None, None, Vec::new()),
            }
        } else {
            (None, None, None, Vec::new())
        };

        results.push(DocResult {
            title: item.name.clone(),
            kind: item.kind.to_string(),
            path: item.path.clone(),
            summary: item.description.clone(),
            platforms: Some(format!("Claude Agent SDK ({})", item.language)),
            code_sample,
            related_apis: Vec::new(),
            full_content,
            declaration,
            parameters,
        });
    }

    Ok(results)
}

/// Extract code sample from Apple symbol data
fn extract_code_sample(symbol: &docs_mcp_client::types::SymbolData) -> Option<String> {
    // Look for code listings in primary content sections
    for section in &symbol.primary_content_sections {
        if let Some(code) = extract_code_from_value(section) {
            return Some(code);
        }
    }
    None
}

fn extract_code_from_value(value: &serde_json::Value) -> Option<String> {
    match value {
        serde_json::Value::Object(map) => {
            let kind = map
                .get("type")
                .or_else(|| map.get("kind"))
                .and_then(|v| v.as_str())
                .unwrap_or_default();

            if kind.eq_ignore_ascii_case("codelisting") {
                if let Some(code_value) = map.get("code") {
                    match code_value {
                        serde_json::Value::Array(lines) => {
                            let code = lines
                                .iter()
                                .filter_map(|l| l.as_str())
                                .collect::<Vec<_>>()
                                .join("\n");
                            if !code.trim().is_empty() {
                                return Some(code);
                            }
                        }
                        serde_json::Value::String(s) if !s.trim().is_empty() => {
                            return Some(s.clone());
                        }
                        _ => {}
                    }
                }
            }

            for nested in map.values() {
                if let Some(code) = extract_code_from_value(nested) {
                    return Some(code);
                }
            }
            None
        }
        serde_json::Value::Array(items) => {
            for item in items {
                if let Some(code) = extract_code_from_value(item) {
                    return Some(code);
                }
            }
            None
        }
        _ => None,
    }
}

/// Extract declaration/signature from Apple symbol data
fn extract_declaration(symbol: &docs_mcp_client::types::SymbolData) -> Option<String> {
    // Look for declaration in primary content sections
    for section in &symbol.primary_content_sections {
        if let Some(decl) = extract_declaration_from_value(section) {
            return Some(decl);
        }
    }
    None
}

fn extract_declaration_from_value(value: &serde_json::Value) -> Option<String> {
    match value {
        serde_json::Value::Object(map) => {
            let kind = map
                .get("kind")
                .and_then(|v| v.as_str())
                .unwrap_or_default();

            if kind == "declarations" {
                if let Some(declarations) = map.get("declarations").and_then(|v| v.as_array()) {
                    for decl in declarations {
                        if let Some(tokens) = decl.get("tokens").and_then(|t| t.as_array()) {
                            let text: String = tokens
                                .iter()
                                .filter_map(|t| t.get("text").and_then(|v| v.as_str()))
                                .collect();
                            if !text.trim().is_empty() {
                                return Some(text);
                            }
                        }
                    }
                }
            }

            // Recurse into nested objects
            for nested in map.values() {
                if let Some(decl) = extract_declaration_from_value(nested) {
                    return Some(decl);
                }
            }
            None
        }
        serde_json::Value::Array(items) => {
            for item in items {
                if let Some(decl) = extract_declaration_from_value(item) {
                    return Some(decl);
                }
            }
            None
        }
        _ => None,
    }
}

/// Extract parameters from Apple symbol data
fn extract_parameters(symbol: &docs_mcp_client::types::SymbolData) -> Vec<(String, String)> {
    let mut params = Vec::new();

    // Look in primary content sections for parameters
    for section in &symbol.primary_content_sections {
        if let Some(parameters) = extract_parameters_from_value(section) {
            params.extend(parameters);
        }
    }

    params
}

fn extract_parameters_from_value(value: &serde_json::Value) -> Option<Vec<(String, String)>> {
    match value {
        serde_json::Value::Object(map) => {
            let kind = map
                .get("kind")
                .and_then(|v| v.as_str())
                .unwrap_or_default();

            if kind == "parameters" {
                if let Some(params) = map.get("parameters").and_then(|v| v.as_array()) {
                    let result: Vec<(String, String)> = params
                        .iter()
                        .filter_map(|p| {
                            let name = p.get("name")?.as_str()?.to_string();
                            let content = p
                                .get("content")
                                .and_then(|c| c.as_array())
                                .map(|arr| {
                                    arr.iter()
                                        .filter_map(|item| {
                                            item.get("text").and_then(|t| t.as_str())
                                        })
                                        .collect::<Vec<_>>()
                                        .join("")
                                })
                                .unwrap_or_default();
                            Some((name, content))
                        })
                        .collect();
                    if !result.is_empty() {
                        return Some(result);
                    }
                }
            }

            // Recurse into nested objects
            for nested in map.values() {
                if let Some(params) = extract_parameters_from_value(nested) {
                    return Some(params);
                }
            }
            None
        }
        serde_json::Value::Array(items) => {
            for item in items {
                if let Some(params) = extract_parameters_from_value(item) {
                    return Some(params);
                }
            }
            None
        }
        _ => None,
    }
}

/// Extract full documentation content from Apple symbol data
fn extract_full_content(symbol: &docs_mcp_client::types::SymbolData) -> Option<String> {
    use docs_mcp_client::types::extract_text;

    let mut content_parts = Vec::new();

    // Add abstract (it's a Vec, not Option)
    let abs_text = extract_text(&symbol.r#abstract);
    if !abs_text.is_empty() {
        content_parts.push(abs_text);
    }

    // Add primary content sections (may contain discussion, overview, etc.)
    for section in &symbol.primary_content_sections {
        if let Some(text) = extract_content_from_value(section) {
            content_parts.push(text);
        }
    }

    if content_parts.is_empty() {
        None
    } else {
        Some(content_parts.join("\n\n"))
    }
}

fn extract_content_from_value(value: &serde_json::Value) -> Option<String> {
    match value {
        serde_json::Value::Object(map) => {
            let kind = map
                .get("type")
                .or_else(|| map.get("kind"))
                .and_then(|v| v.as_str())
                .unwrap_or_default();

            // Skip code listings (handled separately)
            if kind.eq_ignore_ascii_case("codelisting") {
                return None;
            }

            // Handle paragraph content
            if kind == "paragraph" || kind == "text" {
                if let Some(inline_content) = map.get("inlineContent").and_then(|c| c.as_array()) {
                    let text: String = inline_content
                        .iter()
                        .filter_map(|item| item.get("text").and_then(|t| t.as_str()))
                        .collect::<Vec<_>>()
                        .join("");
                    if !text.trim().is_empty() {
                        return Some(text);
                    }
                }
            }

            // Handle content arrays
            if let Some(content) = map.get("content").and_then(|c| c.as_array()) {
                let parts: Vec<String> = content
                    .iter()
                    .filter_map(extract_content_from_value)
                    .collect();
                if !parts.is_empty() {
                    return Some(parts.join(" "));
                }
            }

            // Try text field directly
            if let Some(text) = map.get("text").and_then(|t| t.as_str()) {
                if !text.trim().is_empty() {
                    return Some(text.to_string());
                }
            }

            None
        }
        serde_json::Value::Array(items) => {
            let parts: Vec<String> = items
                .iter()
                .filter_map(extract_content_from_value)
                .collect();
            if !parts.is_empty() {
                Some(parts.join(" "))
            } else {
                None
            }
        }
        serde_json::Value::String(s) if !s.trim().is_empty() => Some(s.clone()),
        _ => None,
    }
}

/// Build the final response with full documentation context
fn build_response(
    intent: &QueryIntent,
    provider: &ProviderType,
    technology: &str,
    results: &[DocResult],
) -> Result<ToolResponse> {
    let mut lines = vec![
        markdown::header(1, &format!("📚 Documentation: {}", intent.raw_query)),
        String::new(),
        format!("**Provider:** {} | **Technology:** {} | **Results:** {}",
            provider.name(), technology, results.len()),
    ];

    if results.is_empty() {
        lines.push(String::new());
        lines.push("No results found. Try different keywords or a more specific query.".to_string());
    } else {
        // Detailed documentation for top results
        lines.push(String::new());
        lines.push(markdown::header(2, "Documentation"));

        for (i, result) in results.iter().enumerate() {
            let is_detailed = i < MAX_DETAILED_DOCS && result.full_content.is_some();

            lines.push(String::new());
            lines.push(format!("### {}. {} `{}`", i + 1, result.title, result.kind));

            if let Some(platforms) = &result.platforms {
                lines.push(format!("**Availability:** {}", platforms));
            }

            // Declaration/signature for detailed results
            if is_detailed {
                if let Some(decl) = &result.declaration {
                    lines.push(String::new());
                    lines.push("**Declaration:**".to_string());
                    // Determine code language based on provider/platform
                    let code_lang = detect_code_language(provider, result.platforms.as_deref());
                    lines.push(format!("```{}\n{}\n```", code_lang, decl));
                }
            }

            // Full content or summary
            if let Some(content) = &result.full_content {
                lines.push(String::new());
                lines.push("**Overview:**".to_string());
                lines.push(trim_text(content, MAX_CONTENT_LENGTH));
            } else if !result.summary.is_empty() {
                lines.push(String::new());
                lines.push(trim_text(&result.summary, MAX_SUMMARY_LENGTH));
            }

            // Parameters for detailed results
            if is_detailed && !result.parameters.is_empty() {
                lines.push(String::new());
                lines.push("**Parameters:**".to_string());
                for (name, desc) in &result.parameters {
                    if desc.is_empty() {
                        lines.push(format!("- `{}`", name));
                    } else {
                        lines.push(format!("- `{}`: {}", name, desc));
                    }
                }
            }

            // Code sample
            if let Some(code) = &result.code_sample {
                lines.push(String::new());
                lines.push("**Example:**".to_string());
                // Determine code language based on provider/platform
                let code_lang = detect_code_language(provider, result.platforms.as_deref());
                lines.push(format!("```{}\n{}\n```", code_lang, trim_text(code, MAX_CODE_LENGTH)));
            }

            // Related APIs
            if !result.related_apis.is_empty() {
                lines.push(String::new());
                lines.push(format!("**Related:** {}", result.related_apis.join(" · ")));
            }
        }
    }

    // Helpful tips section (no references to non-existent tools)
    if !results.is_empty() {
        lines.push(String::new());
        lines.push(markdown::header(2, "Tips"));
        lines.push("• Query with different keywords to find related APIs".to_string());
        lines.push("• Include framework name (e.g., 'SwiftUI Button') for better results".to_string());
        lines.push("• Try 'how to...' queries for implementation guidance".to_string());
    }

    let metadata = json!({
        "query": intent.raw_query,
        "provider": provider.name(),
        "technology": technology,
        "queryType": format!("{:?}", intent.query_type),
        "keywords": intent.keywords,
        "resultCount": results.len(),
        "hasCodeSamples": results.iter().any(|r| r.code_sample.is_some()),
        "hasFullContent": results.iter().any(|r| r.full_content.is_some()),
    });

    Ok(text_response(lines).with_metadata(metadata))
}

fn trim_text(text: &str, max: usize) -> String {
    if text.len() <= max {
        text.to_string()
    } else {
        let mut end = max;
        while end > 0 && !text.is_char_boundary(end) {
            end -= 1;
        }
        format!("{}...", &text[..end])
    }
}

/// Detect the appropriate code language for syntax highlighting based on provider and platform
fn detect_code_language(provider: &ProviderType, platforms: Option<&str>) -> &'static str {
    match provider {
        ProviderType::Apple => "swift",
        ProviderType::Rust => "rust",
        ProviderType::Telegram | ProviderType::TON => "json",
        ProviderType::Mdn => "javascript",
        ProviderType::WebFrameworks => {
            // Check platform for hints
            if let Some(p) = platforms {
                let p_lower = p.to_lowercase();
                if p_lower.contains("node") {
                    return "javascript";
                }
            }
            "typescript"
        }
        ProviderType::Mlx => {
            // Check platform for Swift vs Python
            if let Some(p) = platforms {
                let p_lower = p.to_lowercase();
                if p_lower.contains("python") {
                    return "python";
                }
            }
            "swift"
        }
        ProviderType::HuggingFace => {
            // Check platform for Swift vs Python
            if let Some(p) = platforms {
                let p_lower = p.to_lowercase();
                if p_lower.contains("swift") {
                    return "swift";
                }
            }
            "python"
        }
        ProviderType::QuickNode => "javascript",
        ProviderType::ClaudeAgentSdk => {
            // Check platform for TypeScript vs Python
            if let Some(p) = platforms {
                let p_lower = p.to_lowercase();
                if p_lower.contains("python") {
                    return "python";
                }
            }
            "typescript"
        }
        ProviderType::Cocoon => "text",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_howto_intent() {
        let intent = parse_query_intent("how to use SwiftUI NavigationStack");
        assert_eq!(intent.query_type, QueryType::HowTo);
        assert_eq!(intent.provider, Some(ProviderType::Apple));
        assert!(intent.keywords.contains(&"swiftui".to_string()));
        assert!(intent.keywords.contains(&"navigationstack".to_string()));
    }

    #[test]
    fn test_parse_reference_intent() {
        let intent = parse_query_intent("what is UIKit UITableView");
        assert_eq!(intent.query_type, QueryType::Reference);
        assert_eq!(intent.provider, Some(ProviderType::Apple));
    }

    #[test]
    fn test_parse_search_intent() {
        let intent = parse_query_intent("Button styling");
        assert_eq!(intent.query_type, QueryType::Search);
    }

    #[test]
    fn test_detect_rust_provider() {
        let intent = parse_query_intent("tokio spawn async task");
        assert_eq!(intent.provider, Some(ProviderType::Rust));
        assert!(intent.technology.as_ref().unwrap().contains("tokio"));
    }

    #[test]
    fn test_detect_telegram_provider() {
        let intent = parse_query_intent("telegram bot sendMessage");
        assert_eq!(intent.provider, Some(ProviderType::Telegram));
    }

    #[test]
    fn test_extract_keywords() {
        let keywords = extract_keywords("how to use swiftui navigationstack with binding");
        assert!(keywords.contains(&"swiftui".to_string()));
        assert!(keywords.contains(&"navigationstack".to_string()));
        assert!(keywords.contains(&"binding".to_string()));
        // Stop words should be removed
        assert!(!keywords.contains(&"how".to_string()));
        assert!(!keywords.contains(&"to".to_string()));
        assert!(!keywords.contains(&"use".to_string()));
    }
}
