# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Apple Doc MCP is a Model Context Protocol (MCP) server written in Rust that provides access to developer documentation from multiple providers. It enables AI coding assistants to search, browse, and retrieve official documentation for:
- **Apple**: SwiftUI, UIKit, Foundation, CoreML, Vision, and 60+ frameworks (including ML/AI)
- **Telegram**: Bot API methods and types
- **TON**: Blockchain API endpoints
- **Cocoon**: Confidential computing documentation
- **Rust**: Standard library (std, core, alloc) and any crate from docs.rs
- **MDN**: JavaScript, TypeScript, Web APIs, DOM documentation
- **Web Frameworks**: React, Next.js, Node.js, and Bun documentation
- **Bun**: Fast all-in-one JavaScript runtime with bundler, transpiler, test runner, and package manager
- **MLX**: Apple's machine learning framework for Apple Silicon (Swift and Python)
- **Hugging Face**: Transformers library and swift-transformers for LLM/AI development
- **QuickNode**: Solana blockchain RPC documentation (HTTP methods, WebSocket, Marketplace add-ons)
- **Claude Agent SDK**: TypeScript and Python SDKs for building AI agents with Claude Code capabilities

## Build Commands

```bash
# Build (requires Rust 1.76+)
cargo build --release

# Run tests
cargo test

# Run tests for a specific crate
cargo test -p docs-mcp-core

# Run the server (for local development)
cargo run -p docs-mcp-cli

# Lint
cargo clippy --all-targets
```

## Architecture

### Workspace Structure

```
├── apps/cli/                    # CLI entry point (docs-mcp-cli binary)
├── crates/
│   ├── docs-mcp-client/       # HTTP client for Apple's documentation API
│   ├── docs-mcp-core/         # Core logic: tools, state, services, transport
│   ├── docs-mcp/          # MCP protocol bootstrap and config resolution
│   └── multi-provider-client/   # Clients for Telegram, TON, Cocoon, Rust, MDN, Web Frameworks, MLX, Hugging Face, QuickNode, and Claude Agent SDK APIs
│       ├── src/
│       │   ├── telegram/        # Telegram Bot API client
│       │   ├── ton/             # TON blockchain API client
│       │   ├── cocoon/          # Cocoon confidential computing client
│       │   ├── rust/            # Rust documentation client (std + docs.rs)
│       │   ├── mdn/             # MDN Web Docs client (JavaScript, Web APIs)
│       │   ├── web_frameworks/  # React, Next.js, Node.js, Bun documentation client
│       │   ├── mlx/             # MLX ML framework client (Swift + Python)
│       │   ├── huggingface/     # Hugging Face transformers client
│       │   ├── quicknode/       # QuickNode Solana RPC documentation client
│       │   ├── claude_agent_sdk/# Claude Agent SDK client (TypeScript + Python)
│       │   ├── types.rs         # Unified types across all providers
│       │   └── lib.rs           # ProviderClients aggregation
```

### Crate Responsibilities

- **docs-mcp-client**: Fetches and caches documentation from `developer.apple.com/tutorials/data`. Uses two-tier caching (memory TTL + disk persistence). Key types: `AppleDocsClient`, `Technology`, `FrameworkData`, `SymbolData`.

- **docs-mcp-core**: Contains all MCP tool implementations, application state (`AppContext`, `ServerState`), and the stdio transport layer. Tools are registered via `tools::register_tools()`.

- **docs-mcp**: Thin wrapper that resolves environment config (`DOCSMCP_CACHE_DIR`, `DOCSMCP_HEADLESS`) and launches the core server.

- **multi-provider-client**: HTTP clients for non-Apple documentation providers:
  - `TelegramClient`: Telegram Bot API methods and types from `core.telegram.org`
  - `TonClient`: TON blockchain endpoints from `tonapi.io` OpenAPI spec
  - `CocoonClient`: Cocoon documentation from `cocoon.org`
  - `RustClient`: Rust std library + any crate from `docs.rs`
  - `MdnClient`: JavaScript, TypeScript, Web APIs from `developer.mozilla.org`
  - `WebFrameworksClient`: React, Next.js, Node.js, Bun documentation with example extraction
  - `MlxClient`: MLX ML framework documentation (Swift DocC + Python Sphinx) from `ml-explore.github.io`
  - `HuggingFaceClient`: Transformers and swift-transformers documentation from `huggingface.co`
  - `QuickNodeClient`: Solana RPC documentation from `quicknode.com/docs/solana`
  - `ClaudeAgentSdkClient`: Claude Agent SDK documentation for TypeScript and Python from `docs.anthropic.com`

### Provider Architecture

All providers implement a consistent interface through unified types:

```rust
pub enum ProviderType {
    Apple,
    Telegram,
    TON,
    Cocoon,
    Rust,
    Mdn,
    WebFrameworks,
    Mlx,
    HuggingFace,
    QuickNode,
    ClaudeAgentSdk,
}

pub struct ProviderClients {
    pub apple: AppleDocsClient,
    pub telegram: TelegramClient,
    pub ton: TonClient,
    pub cocoon: CocoonClient,
    pub rust: RustClient,
    pub mdn: MdnClient,
    pub web_frameworks: WebFrameworksClient,
    pub mlx: MlxClient,
    pub huggingface: HuggingFaceClient,
    pub quicknode: QuickNodeClient,
    pub claude_agent_sdk: ClaudeAgentSdkClient,
}
```

Each tool dispatches to the appropriate provider based on `active_provider` state.

### MCP Tools

**Single unified tool** exposed via MCP (`crates/docs-mcp-core/src/tools/query.rs`):

| Tool | Purpose | Key Features |
|------|---------|--------------|
| `query` | Unified documentation search engine | • Natural language query parsing<br>• Automatic provider/technology detection<br>• Intelligent search with synonym expansion<br>• Returns structured context with code samples<br>• Combines search + documentation fetching |

The `query` tool acts as an intelligent entry point that:
1. Parses natural language queries to extract intent (how-to, reference, search)
2. Auto-detects the appropriate provider (Apple, Telegram, TON, Rust, Cocoon, MDN, Web Frameworks, MLX, Hugging Face, QuickNode, Claude Agent SDK)
3. Auto-selects the relevant technology/framework
4. Executes optimized search across the detected provider
5. Fetches detailed documentation for top results
6. Returns structured, AI-ready context with usage examples

**Legacy tools** (`discover_technologies`, `choose_technology`, `search_symbols`, etc.) remain in the codebase for reference but are not exposed via MCP.

### Provider-Specific Features

#### Apple
- 50+ frameworks (SwiftUI, UIKit, Foundation, etc.)
- Platform availability info (iOS, macOS, watchOS, tvOS)
- Design guidance from Human Interface Guidelines
- Knowledge base overlays with tips and related APIs

#### Telegram
- Bot API methods (sendMessage, getUpdates, etc.)
- Type definitions (Update, Message, User, etc.)
- Parameter documentation with required/optional flags

#### TON
- Blockchain API endpoints organized by category
- OpenAPI-based documentation
- Request/response schema information

#### Cocoon
- Confidential computing documentation
- Architecture and TDX sections
- Smart contract documentation

#### Rust
- Standard library: std, core, alloc
- Dynamic crate loading from docs.rs
- Search index parsing from rustdoc
- Module and symbol documentation

#### MDN Web Docs
- JavaScript core language features (Array, Object, Promise, etc.)
- Web APIs (DOM, Fetch, WebSocket, Canvas, etc.)
- TypeScript type documentation
- Code examples with quality scoring

#### Web Frameworks
- **React**: Hooks (useState, useEffect, etc.), components, patterns
- **Next.js**: App Router, Server Components, API routes
- **Node.js**: Core modules (fs, path, http, crypto, stream)
- **Bun**: Runtime APIs, HTTP server, WebSocket, SQLite, bundler, test runner
- Usage examples prioritized by completeness and runnability

#### Bun Runtime
- **Runtime APIs**: Bun.serve, Bun.file, Bun.write, Bun.spawn, Bun.sleep, Bun.env
- **HTTP/WebSocket**: Fast HTTP server with WebSocket pub/sub support
- **File I/O**: BunFile with streaming, lazy loading, and efficient writes
- **Database**: Built-in SQLite driver with prepared statements and transactions
- **Bundler**: Bun.build with code splitting, minification, and plugin support
- **Test Runner**: bun test with Jest-compatible API, mocking, and snapshots
- **Package Manager**: Fast npm-compatible package manager with bun.lockb
- **FFI**: Foreign Function Interface for calling native libraries
- **Crypto**: Password hashing (bcrypt/argon2), cryptographic hashers
- **Node.js Compatibility**: Compatible with most Node.js APIs

#### MLX (Apple Silicon ML)
- **MLX Swift**: MLXArray, MLXRandom, MLXLinalg, MLXNN, MLXFFT, MLXOptimizers
- **MLX Python**: mlx.core, mlx.nn, mlx.optimizers, operations
- Apple Silicon optimized machine learning primitives
- Documentation from ml-explore.github.io (DocC for Swift, Sphinx for Python)

#### Hugging Face
- **Transformers**: AutoModel, AutoTokenizer, pipeline, training utilities
- **swift-transformers**: Hub, Models, Tokenizers for iOS/macOS
- LLM model families: GPT, LLaMA, BERT, T5, Whisper, CLIP, etc.
- Model card documentation and usage patterns

#### QuickNode (Solana)
- **HTTP Methods**: 50+ JSON-RPC methods (getAccountInfo, getBalance, sendTransaction, etc.)
- **WebSocket Methods**: Real-time subscriptions (accountSubscribe, logsSubscribe, etc.)
- **Marketplace Add-ons**: JITO bundles, Metaplex DAS API, Yellowstone gRPC
- Solana-specific documentation with code examples

#### Claude Agent SDK
- **TypeScript SDK**: ClaudeClient, query function, ClaudeAgentOptions, hooks, MCP servers
- **Python SDK**: ClaudeSDKClient, @tool decorator, async context manager, hooks
- Core concepts: query, hooks (PreToolUse, PostToolUse), MCP server integration
- Configuration: systemPrompt, maxTurns, allowedTools, permissionMode
- Message types: AssistantMessage, UserMessage, SystemMessage, ResultMessage
- Content blocks: TextBlock, ToolUseBlock, ToolResultBlock
- Authentication: ANTHROPIC_API_KEY, Bedrock, Vertex AI

### Unified Query Tool Features

The `query` tool implements advanced natural language processing:

#### Natural Language Query Parsing
Automatically extracts intent, provider, technology, and keywords from queries:
```rust
// Example: "how to use SwiftUI NavigationStack"
QueryIntent {
    query_type: HowTo,
    provider: Some(Apple),
    technology: Some("swiftui"),
    keywords: ["navigationstack"]
}
```

#### Provider Auto-Detection
Intelligently detects the target provider from query context:
- **Apple**: SwiftUI, UIKit, iOS, macOS keywords + 60+ framework names (including CoreML, Vision, NaturalLanguage)
- **Rust**: std, tokio, serde, and other popular crate names
- **Telegram**: bot, sendmessage, telegram, webhook keywords
- **TON**: blockchain, wallet, jetton, tonapi keywords
- **Cocoon**: confidential computing, TDX keywords
- **MDN**: javascript, js, dom, fetch, promise, array, web, browser keywords
- **React**: react, jsx, hook, usestate, useeffect, component keywords
- **Next.js**: nextjs, next, approuter, servercomponent keywords
- **Bun**: bun, bunjs, bun.serve, bun.file, bunx, bun.spawn, bun:sqlite, bun:test, bunfig keywords
- **Node.js**: nodejs, node, fs, path, http, stream keywords
- **MLX**: mlx, mlxarray, mlxnn, apple silicon, ml-explore keywords
- **Hugging Face**: huggingface, transformers, automodel, autotokenizer, swift-transformers keywords
- **QuickNode**: quicknode, solana, getaccountinfo, getbalance, lamports, pubkey keywords
- **Claude Agent SDK**: claude, agent sdk, claudeagentsdk, claudesdkclient, query, mcp, hooks keywords

#### Query Type Classification
Three query types with specialized handling:
- **HowTo**: Recipes and guided steps with knowledge base tips
- **Reference**: Detailed documentation with code samples
- **Search**: General symbol search with synonym expansion

#### Tool Use Examples
The query tool includes diverse usage examples for natural language understanding:
```rust
input_examples: Some(vec![
    json!({"query": "how to use SwiftUI NavigationStack"}),
    json!({"query": "Rust tokio async task spawning"}),
    json!({"query": "Telegram Bot API sendMessage parameters"}),
    json!({"query": "CoreData fetch request predicates", "maxResults": 5}),
    json!({"query": "JavaScript Array map filter"}),
    json!({"query": "React useState hook"}),
    json!({"query": "Next.js server components"}),
    json!({"query": "Node.js fs readFile"}),
    json!({"query": "Bun serve HTTP server"}),
    json!({"query": "Bun.file read write"}),
    json!({"query": "Bun SQLite database"}),
    json!({"query": "MLX array operations Swift"}),
    json!({"query": "Hugging Face AutoModel from_pretrained"}),
    json!({"query": "Solana getAccountInfo"}),
    json!({"query": "QuickNode getBalance"}),
    json!({"query": "Claude Agent SDK query function typescript"}),
    json!({"query": "agent sdk python ClaudeSDKClient"}),
])
```

### ToolDefinition Structure

```rust
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
    pub input_examples: Option<Vec<serde_json::Value>>,  // Usage demonstrations
    pub allowed_callers: Option<Vec<String>>,            // Programmatic calling
}
```

### State Flow

1. `AppContext` holds the `AppleDocsClient`, `ServerState`, `ToolRegistry`, and `ProviderClients`
2. `ServerState` tracks: active technology, active provider, framework cache, search indexes, telemetry
3. Tool handlers receive `Arc<AppContext>` and return `ToolResponse` with optional metadata

### Search System

The unified `query` tool uses sophisticated ranking (in `tools/query.rs`):
- **Token matching**: camelCase splitting and multi-token queries
- **Synonym expansion**: e.g., "list" → "table", "collection", "tableview"
- **Natural language parsing**: extracts intent (how-to, reference, search)
- **Provider auto-detection**: routes to appropriate search backend
- **Smart scoring**: exact title match (15pts), abstract match (5pts), token match (2pts)
- **Knowledge base overlays**: tips and design guidance for Apple symbols
- **Code sample extraction**: automatically fetches and includes example code
- **Related APIs**: surfaces 5 related symbols for context

### Caching Strategy

All providers use two-tier caching:

| Data Type | Memory TTL | Disk TTL |
|-----------|------------|----------|
| Apple frameworks | 30min | 24h |
| Telegram spec | 1h | 24h |
| TON OpenAPI | 1h | 24h |
| Cocoon docs | 1h | 24h |
| Rust std index | 24h | 7d |
| Rust crate metadata | 30min | 24h |
| MDN search index | 1h | 24h |
| MDN article content | 30min | 24h |
| React/Next.js docs | 1h | 24h |
| Node.js API index | 24h | 7d |
| Bun docs | 1h | 24h |
| MLX Swift docs | 1h | 24h |
| MLX Python docs | 1h | 24h |
| Hugging Face docs | 1h | 24h |
| QuickNode methods | 30min | 24h |
| Claude Agent SDK docs | 24h | 24h |

## Environment Variables

| Variable | Purpose |
|----------|---------|
| `DOCSMCP_CACHE_DIR` | Override disk cache location |
| `DOCSMCP_HEADLESS` | Set to `1` or `true` to skip stdio transport (testing) |
| `RUST_LOG` | Control tracing output (e.g., `info`, `debug`) |

## Testing the MCP Server

```bash
# Test MCP handshake and tools/list
printf '{"jsonrpc":"2.0","method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"1.0"}},"id":1}\n{"jsonrpc":"2.0","method":"notifications/initialized","params":{}}\n{"jsonrpc":"2.0","method":"tools/list","params":{},"id":2}\n' | ./target/release/docs-mcp-cli

# Test unified query tool with Apple/SwiftUI
printf '{"jsonrpc":"2.0","method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"1.0"}},"id":1}\n{"jsonrpc":"2.0","method":"notifications/initialized","params":{}}\n{"jsonrpc":"2.0","method":"tools/call","params":{"name":"query","arguments":{"query":"SwiftUI Button styling"}},"id":3}\n' | ./target/release/docs-mcp-cli

# Test query with Rust
printf '{"jsonrpc":"2.0","method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"1.0"}},"id":1}\n{"jsonrpc":"2.0","method":"notifications/initialized","params":{}}\n{"jsonrpc":"2.0","method":"tools/call","params":{"name":"query","arguments":{"query":"tokio spawn async task"}},"id":3}\n' | ./target/release/docs-mcp-cli

# Test query with Telegram
printf '{"jsonrpc":"2.0","method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"1.0"}},"id":1}\n{"jsonrpc":"2.0","method":"notifications/initialized","params":{}}\n{"jsonrpc":"2.0","method":"tools/call","params":{"name":"query","arguments":{"query":"telegram bot sendMessage"}},"id":3}\n' | ./target/release/docs-mcp-cli

# Test how-to query
printf '{"jsonrpc":"2.0","method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"1.0"}},"id":1}\n{"jsonrpc":"2.0","method":"notifications/initialized","params":{}}\n{"jsonrpc":"2.0","method":"tools/call","params":{"name":"query","arguments":{"query":"how to use SwiftUI NavigationStack"}},"id":3}\n' | ./target/release/docs-mcp-cli

# Test query with MDN (JavaScript)
printf '{"jsonrpc":"2.0","method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"1.0"}},"id":1}\n{"jsonrpc":"2.0","method":"notifications/initialized","params":{}}\n{"jsonrpc":"2.0","method":"tools/call","params":{"name":"query","arguments":{"query":"JavaScript Array map"}},"id":3}\n' | ./target/release/docs-mcp-cli

# Test query with React
printf '{"jsonrpc":"2.0","method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"1.0"}},"id":1}\n{"jsonrpc":"2.0","method":"notifications/initialized","params":{}}\n{"jsonrpc":"2.0","method":"tools/call","params":{"name":"query","arguments":{"query":"React useState hook"}},"id":3}\n' | ./target/release/docs-mcp-cli

# Test query with Next.js
printf '{"jsonrpc":"2.0","method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"1.0"}},"id":1}\n{"jsonrpc":"2.0","method":"notifications/initialized","params":{}}\n{"jsonrpc":"2.0","method":"tools/call","params":{"name":"query","arguments":{"query":"Next.js server components"}},"id":3}\n' | ./target/release/docs-mcp-cli

# Test query with Node.js
printf '{"jsonrpc":"2.0","method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"1.0"}},"id":1}\n{"jsonrpc":"2.0","method":"notifications/initialized","params":{}}\n{"jsonrpc":"2.0","method":"tools/call","params":{"name":"query","arguments":{"query":"Node.js fs readFile"}},"id":3}\n' | ./target/release/docs-mcp-cli

# Test query with Bun
printf '{"jsonrpc":"2.0","method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"1.0"}},"id":1}\n{"jsonrpc":"2.0","method":"notifications/initialized","params":{}}\n{"jsonrpc":"2.0","method":"tools/call","params":{"name":"query","arguments":{"query":"Bun serve HTTP server"}},"id":3}\n' | ./target/release/docs-mcp-cli

# Test query with Bun SQLite
printf '{"jsonrpc":"2.0","method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"1.0"}},"id":1}\n{"jsonrpc":"2.0","method":"notifications/initialized","params":{}}\n{"jsonrpc":"2.0","method":"tools/call","params":{"name":"query","arguments":{"query":"Bun SQLite database"}},"id":3}\n' | ./target/release/docs-mcp-cli

# Test query with MLX (Apple Silicon ML)
printf '{"jsonrpc":"2.0","method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"1.0"}},"id":1}\n{"jsonrpc":"2.0","method":"notifications/initialized","params":{}}\n{"jsonrpc":"2.0","method":"tools/call","params":{"name":"query","arguments":{"query":"MLX array operations Swift"}},"id":3}\n' | ./target/release/docs-mcp-cli

# Test query with Hugging Face
printf '{"jsonrpc":"2.0","method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"1.0"}},"id":1}\n{"jsonrpc":"2.0","method":"notifications/initialized","params":{}}\n{"jsonrpc":"2.0","method":"tools/call","params":{"name":"query","arguments":{"query":"Hugging Face AutoModel from_pretrained"}},"id":3}\n' | ./target/release/docs-mcp-cli

# Test query with QuickNode (Solana)
printf '{"jsonrpc":"2.0","method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"1.0"}},"id":1}\n{"jsonrpc":"2.0","method":"notifications/initialized","params":{}}\n{"jsonrpc":"2.0","method":"tools/call","params":{"name":"query","arguments":{"query":"Solana getAccountInfo"}},"id":3}\n' | ./target/release/docs-mcp-cli

# Test query with Claude Agent SDK (TypeScript)
printf '{"jsonrpc":"2.0","method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"1.0"}},"id":1}\n{"jsonrpc":"2.0","method":"notifications/initialized","params":{}}\n{"jsonrpc":"2.0","method":"tools/call","params":{"name":"query","arguments":{"query":"Claude Agent SDK query function"}},"id":3}\n' | ./target/release/docs-mcp-cli

# Test query with Claude Agent SDK (Python)
printf '{"jsonrpc":"2.0","method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"1.0"}},"id":1}\n{"jsonrpc":"2.0","method":"notifications/initialized","params":{}}\n{"jsonrpc":"2.0","method":"tools/call","params":{"name":"query","arguments":{"query":"agent sdk python ClaudeSDKClient"}},"id":3}\n' | ./target/release/docs-mcp-cli
```

## Adding a New Provider

1. Create a new module in `crates/multi-provider-client/src/<provider>/`
   - `mod.rs` - exports
   - `types.rs` - provider-specific data models
   - `client.rs` - HTTP client with caching implementing `search()` and `get_documentation()` methods

2. Update `crates/multi-provider-client/src/types.rs`:
   - Add variant to `ProviderType` enum
   - Add variant to `TechnologyKind` enum
   - Add variant to `SymbolContent` enum (if needed)
   - Add `from_<provider>()` conversion methods

3. Update `crates/multi-provider-client/src/lib.rs`:
   - Add client to `ProviderClients` struct
   - Initialize client in `ProviderClients::new()`

4. Update the unified query tool in `crates/docs-mcp-core/src/tools/query.rs`:
   - Add provider keywords to detection lists (`<PROVIDER>_KEYWORDS`)
   - Update `detect_provider_and_technology()` to detect your provider
   - Add `search_<provider>()` function implementing search logic
   - Add match arm in `execute_search_query()` to route to your search function
   - Update `resolve_technology()` to handle technology selection for your provider

## Maintenance Protocol

**IMPORTANT**: Review and update `agents.md` before finishing any session. It contains the retrieval enhancement roadmap and phase completion status.
