# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Apple Doc MCP is a Model Context Protocol (MCP) server written in Rust that provides access to developer documentation from multiple providers. It enables AI coding assistants to search, browse, and retrieve official documentation for:
- **Apple**: SwiftUI, UIKit, Foundation, and 50+ frameworks
- **Telegram**: Bot API methods and types
- **TON**: Blockchain API endpoints
- **Cocoon**: Confidential computing documentation

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
│   └── multi-provider-client/   # Clients for Telegram, TON, and Cocoon APIs
```

### Crate Responsibilities

- **apple-docs-client**: Fetches and caches documentation from `developer.apple.com/tutorials/data`. Uses two-tier caching (memory TTL + disk persistence). Key types: `AppleDocsClient`, `Technology`, `FrameworkData`, `SymbolData`.

- **apple-docs-core**: Contains all MCP tool implementations, application state (`AppContext`, `ServerState`), and the stdio transport layer. Tools are registered via `tools::register_tools()`.

- **apple-docs-mcp**: Thin wrapper that resolves environment config (`APPLEDOC_CACHE_DIR`, `APPLEDOC_HEADLESS`) and launches the core server.

- **multi-provider-client**: HTTP clients for non-Apple documentation providers (Telegram Bot API, TON blockchain, Cocoon).

### MCP Tools

Seven tools exposed via MCP (`crates/apple-docs-core/src/tools/`):

| Tool | Purpose | Programmatic Calling |
|------|---------|---------------------|
| `discover_technologies` | Browse/filter frameworks from all providers | ✅ Enabled |
| `choose_technology` | Select active framework for subsequent searches | - |
| `current_technology` | Show currently selected framework | - |
| `search_symbols` | Fuzzy keyword search within active framework or globally | ✅ Enabled |
| `get_documentation` | Retrieve symbol documentation by path | ✅ Enabled |
| `how_do_i` | Get guided recipes for common tasks | - |
| `batch_documentation` | Fetch docs for multiple symbols in one call | ✅ Enabled |

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

# Test a tool call
printf '...\n{"jsonrpc":"2.0","method":"tools/call","params":{"name":"discover_technologies","arguments":{"provider":"apple"}},"id":3}\n' | ./target/release/apple-docs-cli
```

## Maintenance Protocol

**IMPORTANT**: Review and update `agents.md` before finishing any session. It contains the retrieval enhancement roadmap and phase completion status.
