# Multi-Provider Documentation MCP Server

A Model Context Protocol (MCP) server written in Rust that provides seamless access to developer documentation from multiple sources directly within your AI coding assistant.

## Supported Documentation Providers

| Provider | Description | Technologies |
|----------|-------------|--------------|
| **Apple** | iOS/macOS development | SwiftUI, UIKit, Foundation, and 50+ frameworks |
| **Telegram** | Bot API documentation | Methods, types, and parameters |
| **TON** | Blockchain API | REST endpoints and schemas |
| **Cocoon** | Confidential computing | Architecture and smart contracts |
| **Rust** | Rust documentation | std, core, alloc + any crate from docs.rs |

## Quick Start

Build the Rust binary (requires Rust 1.76+):

```bash
cargo build --release
```

Configure your MCP client:

```json
{
  "mcpServers": {
    "apple-docs": {
      "command": "/absolute/path/to/target/release/apple-docs-cli"
    }
  }
}
```

For local development:

```bash
cargo run -p apple-docs-cli
```

## Typical Workflow

### 1. Discover Available Technologies

```
discover_technologies { "provider": "all" }
discover_technologies { "provider": "apple", "query": "swift" }
discover_technologies { "provider": "rust" }
discover_technologies { "provider": "telegram" }
```

### 2. Select a Technology

```
choose_technology { "identifier": "doc://com.apple.documentation/documentation/swiftui", "name": "SwiftUI" }
choose_technology { "identifier": "rust:std", "name": "Rust std Library" }
choose_technology { "identifier": "telegram:methods", "name": "Telegram Bot API Methods" }
```

### 3. Search Within the Active Technology

```
search_symbols { "query": "button" }
search_symbols { "query": "HashMap", "maxResults": 10 }
search_symbols { "query": "sendMessage" }
```

### 4. Get Documentation

```
get_documentation { "path": "Button" }
get_documentation { "path": "std::collections::HashMap" }
get_documentation { "path": "sendMessage" }
```

### 5. Ask for Guided Recipes (Apple only)

```
how_do_i { "task": "add search suggestions" }
how_do_i { "task": "implement tab navigation" }
```

## Available Tools

| Tool | Description |
|------|-------------|
| `discover_technologies` | Browse/filter frameworks from all providers |
| `choose_technology` | Set the active framework for subsequent searches |
| `current_technology` | Show current selection and quick next steps |
| `search_symbols` | Fuzzy keyword search within the active framework |
| `get_documentation` | View symbol/API documentation |
| `how_do_i` | Fetch guided multi-step recipes (Apple) |
| `batch_documentation` | Fetch docs for multiple symbols in one call |

## Provider-Specific Examples

### Apple (SwiftUI, UIKit, etc.)

```
discover_technologies { "provider": "apple", "category": "ui" }
choose_technology { "identifier": "doc://com.apple.documentation/documentation/swiftui" }
search_symbols { "query": "NavigationStack" }
get_documentation { "path": "TabView" }
```

### Rust

```
discover_technologies { "provider": "rust" }
choose_technology { "identifier": "rust:std", "name": "Rust std Library" }
search_symbols { "query": "HashMap" }
get_documentation { "path": "Vec" }
```

You can also load any crate from docs.rs dynamically:

```
choose_technology { "identifier": "rust:serde", "name": "serde" }
choose_technology { "identifier": "rust:tokio", "name": "tokio" }
```

### Telegram Bot API

```
discover_technologies { "provider": "telegram" }
choose_technology { "identifier": "telegram:methods" }
search_symbols { "query": "send" }
get_documentation { "path": "sendMessage" }
```

### TON Blockchain

```
discover_technologies { "provider": "ton" }
choose_technology { "identifier": "ton:accounts" }
search_symbols { "query": "account" }
get_documentation { "path": "getAccountInfo" }
```

### Cocoon

```
discover_technologies { "provider": "cocoon" }
choose_technology { "identifier": "cocoon:architecture" }
search_symbols { "query": "tdx" }
```

## Search Tips

- Start broad (e.g., `"button"`, `"animation"`, `"hash"`)
- Try synonyms (`"sheet"` vs `"modal"`, `"map"` vs `"hashmap"`)
- Use multiple keywords (`"tab view layout"`) to narrow results
- Use `"scope": "global"` to search across all cached frameworks
- If nothing turns up, try `discover_technologies` with a different provider

## Enriched Output

### Apple Provider
- Quick summaries with inline Swift snippets
- Platform availability badges (iOS, macOS, watchOS, tvOS)
- Human Interface Guidelines (HIG) integration
- SwiftUI/UIKit/AppKit bridge hints
- Related APIs from knowledge base

### Rust Provider
- Module paths (e.g., `std::collections::HashMap`)
- Item kinds (Struct, Enum, Trait, Function, etc.)
- Links to docs.rs documentation
- Crate version information

### Telegram Provider
- Method parameters with required/optional flags
- Return type information
- Field documentation for types

### TON Provider
- HTTP method and path
- Parameter locations (path, query, body)
- Response schema information

## Environment Variables

| Variable | Purpose |
|----------|---------|
| `APPLEDOC_CACHE_DIR` | Override disk cache location |
| `APPLEDOC_HEADLESS` | Set to `1` to skip stdio transport (testing) |
| `RUST_LOG` | Control logging (`info`, `debug`, `trace`) |

## Architecture

```
├── apps/cli/                    # CLI entry point
├── crates/
│   ├── apple-docs-client/       # Apple documentation API client
│   ├── apple-docs-core/         # MCP tools, state, services
│   ├── apple-docs-mcp/          # MCP protocol bootstrap
│   └── multi-provider-client/   # Telegram, TON, Cocoon, Rust clients
```

## Development

```bash
# Build
cargo build --release

# Run tests
cargo test

# Lint
cargo clippy --all-targets

# Test MCP handshake
printf '{"jsonrpc":"2.0","method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"1.0"}},"id":1}\n' | ./target/release/apple-docs-cli
```

## License

See LICENSE file for details.
