# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Apple Doc MCP is a Model Context Protocol (MCP) server written in Rust that provides access to developer documentation from multiple providers. It enables AI coding assistants to search, browse, and retrieve official documentation for:
- **Apple**: SwiftUI, UIKit, Foundation, and 50+ frameworks
- **Telegram**: Bot API methods and types
- **TON**: Blockchain API endpoints
- **Cocoon**: Confidential computing documentation
- **Rust**: Standard library (std, core, alloc) and any crate from docs.rs

## Build Commands

```bash
# Build (requires Rust 1.76+)
cargo build --release

# Run tests
cargo test

# Run tests for a specific crate
cargo test -p apple-docs-core

# Run the server (for local development)
cargo run -p apple-docs-cli

# Lint
cargo clippy --all-targets
```

## Architecture

### Workspace Structure

```
├── apps/cli/                    # CLI entry point (apple-docs-cli binary)
├── crates/
│   ├── apple-docs-client/       # HTTP client for Apple's documentation API
│   ├── apple-docs-core/         # Core logic: tools, state, services, transport
│   ├── apple-docs-mcp/          # MCP protocol bootstrap and config resolution
│   └── multi-provider-client/   # Clients for Telegram, TON, Cocoon, and Rust APIs
│       ├── src/
│       │   ├── telegram/        # Telegram Bot API client
│       │   ├── ton/             # TON blockchain API client
│       │   ├── cocoon/          # Cocoon confidential computing client
│       │   ├── rust/            # Rust documentation client (std + docs.rs)
│       │   ├── types.rs         # Unified types across all providers
│       │   └── lib.rs           # ProviderClients aggregation
```

### Crate Responsibilities

- **apple-docs-client**: Fetches and caches documentation from `developer.apple.com/tutorials/data`. Uses two-tier caching (memory TTL + disk persistence). Key types: `AppleDocsClient`, `Technology`, `FrameworkData`, `SymbolData`.

- **apple-docs-core**: Contains all MCP tool implementations, application state (`AppContext`, `ServerState`), and the stdio transport layer. Tools are registered via `tools::register_tools()`.

- **apple-docs-mcp**: Thin wrapper that resolves environment config (`APPLEDOC_CACHE_DIR`, `APPLEDOC_HEADLESS`) and launches the core server.

- **multi-provider-client**: HTTP clients for non-Apple documentation providers:
  - `TelegramClient`: Telegram Bot API methods and types from `core.telegram.org`
  - `TonClient`: TON blockchain endpoints from `tonapi.io` OpenAPI spec
  - `CocoonClient`: Cocoon documentation from `cocoon.org`
  - `RustClient`: Rust std library + any crate from `docs.rs`

### Provider Architecture

All providers implement a consistent interface through unified types:

```rust
pub enum ProviderType {
    Apple,
    Telegram,
    TON,
    Cocoon,
    Rust,
}

pub struct ProviderClients {
    pub apple: AppleDocsClient,
    pub telegram: TelegramClient,
    pub ton: TonClient,
    pub cocoon: CocoonClient,
    pub rust: RustClient,
}
```

Each tool dispatches to the appropriate provider based on `active_provider` state.

### MCP Tools

Seven tools exposed via MCP (`crates/apple-docs-core/src/tools/`):

| Tool | Purpose | Programmatic Calling |
|------|---------|---------------------|
| `discover_technologies` | Browse/filter frameworks from all providers | Enabled |
| `choose_technology` | Select active framework for subsequent searches | - |
| `current_technology` | Show currently selected framework | - |
| `search_symbols` | Fuzzy keyword search within active framework or globally | Enabled |
| `get_documentation` | Retrieve symbol documentation by path | Enabled |
| `how_do_i` | Get guided recipes for common tasks | - |
| `batch_documentation` | Fetch docs for multiple symbols in one call | Enabled |

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

### Advanced Tool Use Features

This server implements Anthropic's Advanced Tool Use patterns for improved AI agent performance:

#### Tool Use Examples (`inputExamples`)
Each tool includes usage examples that help Claude understand parameter combinations:
```rust
input_examples: Some(vec![
    json!({"query": "Button"}),
    json!({"query": "NavigationStack", "maxResults": 10}),
    json!({"query": "animation", "symbolType": "struct", "scope": "global"}),
])
```
**Impact**: +18 points accuracy on complex parameter formatting.

#### Programmatic Tool Calling (`allowedCallers`)
Tools marked with `allowed_callers: ["code_execution_20250825"]` enable Claude to orchestrate via code:
```rust
allowed_callers: Some(vec!["code_execution_20250825".to_string()])
```
**Impact**: 37% token reduction, 95% fewer inference passes for batch operations.

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

`search_symbols` uses sophisticated ranking (in `tools/search_symbols.rs`):
- Token matching with camelCase splitting
- Synonym expansion (e.g., "list" -> "table", "collection")
- Abbreviation expansion (e.g., "nav" -> "navigation")
- Typo tolerance via edit distance
- Symbol kind boosting (structs/classes rank higher)
- Knowledge base and design guidance overlays
- Provider-specific search dispatch

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

## Environment Variables

| Variable | Purpose |
|----------|---------|
| `APPLEDOC_CACHE_DIR` | Override disk cache location |
| `APPLEDOC_HEADLESS` | Set to `1` or `true` to skip stdio transport (testing) |
| `RUST_LOG` | Control tracing output (e.g., `info`, `debug`) |

## Testing the MCP Server

```bash
# Test MCP handshake and tools/list
printf '{"jsonrpc":"2.0","method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"1.0"}},"id":1}\n{"jsonrpc":"2.0","method":"notifications/initialized","params":{}}\n{"jsonrpc":"2.0","method":"tools/list","params":{},"id":2}\n' | ./target/release/apple-docs-cli

# Test Apple provider
printf '...\n{"jsonrpc":"2.0","method":"tools/call","params":{"name":"discover_technologies","arguments":{"provider":"apple"}},"id":3}\n' | ./target/release/apple-docs-cli

# Test Rust provider
printf '...\n{"jsonrpc":"2.0","method":"tools/call","params":{"name":"discover_technologies","arguments":{"provider":"rust"}},"id":3}\n' | ./target/release/apple-docs-cli

# Test all providers
printf '...\n{"jsonrpc":"2.0","method":"tools/call","params":{"name":"discover_technologies","arguments":{"provider":"all"}},"id":3}\n' | ./target/release/apple-docs-cli
```

## Adding a New Provider

1. Create a new module in `crates/multi-provider-client/src/<provider>/`
   - `mod.rs` - exports
   - `types.rs` - provider-specific data models
   - `client.rs` - HTTP client with caching

2. Update `crates/multi-provider-client/src/types.rs`:
   - Add variant to `ProviderType` enum
   - Add variant to `TechnologyKind` enum
   - Add variant to `SymbolContent` enum
   - Add `from_<provider>()` conversion methods

3. Update `crates/multi-provider-client/src/lib.rs`:
   - Add client to `ProviderClients` struct

4. Update tools in `crates/apple-docs-core/src/tools/`:
   - `discover.rs` - add provider filtering
   - `choose_technology.rs` - add provider handler
   - `search_symbols.rs` - add provider search
   - `get_documentation.rs` - add provider docs
   - `batch_documentation.rs` - add provider batch

## Maintenance Protocol

**IMPORTANT**: Review and update `agents.md` before finishing any session. It contains the retrieval enhancement roadmap and phase completion status.
