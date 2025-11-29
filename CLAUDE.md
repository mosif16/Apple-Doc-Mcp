# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Apple Doc MCP is a Model Context Protocol (MCP) server written in Rust that provides access to Apple's Developer Documentation. It enables AI coding assistants to search, browse, and retrieve official Apple documentation for frameworks like SwiftUI, UIKit, and more.

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
│   └── apple-docs-mcp/          # MCP protocol bootstrap and config resolution
```

### Crate Responsibilities

- **apple-docs-client**: Fetches and caches documentation from `developer.apple.com/tutorials/data`. Uses two-tier caching (memory TTL + disk persistence). Key types: `AppleDocsClient`, `Technology`, `FrameworkData`, `SymbolData`.

- **apple-docs-core**: Contains all MCP tool implementations, application state (`AppContext`, `ServerState`), and the stdio transport layer. Tools are registered via `tools::register_tools()`.

- **apple-docs-mcp**: Thin wrapper that resolves environment config (`APPLEDOC_CACHE_DIR`, `APPLEDOC_HEADLESS`) and launches the core server.

### MCP Tools

Six tools exposed via MCP (`crates/apple-docs-core/src/tools/`):

| Tool | Purpose |
|------|---------|
| `discover_technologies` | Browse/filter available Apple frameworks |
| `choose_technology` | Select active framework for subsequent searches |
| `current_technology` | Show currently selected framework |
| `search_symbols` | Fuzzy keyword search within active framework or globally |
| `get_documentation` | Retrieve symbol documentation by path |
| `how_do_i` | Get guided recipes for common tasks |

### State Flow

1. `AppContext` holds the `AppleDocsClient`, `ServerState`, and `ToolRegistry`
2. `ServerState` tracks: active technology, framework cache, search indexes, telemetry
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

## Maintenance Protocol

**IMPORTANT**: Review and update `agents.md` before finishing any session. It contains the retrieval enhancement roadmap and phase completion status.
