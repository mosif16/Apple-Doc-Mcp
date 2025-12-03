# Multi-Provider Documentation MCP Server

A Model Context Protocol (MCP) server written in Rust that provides seamless access to developer documentation from multiple sources directly within your AI coding assistant.

## Supported Documentation Providers

| Provider | Description | Technologies |
|----------|-------------|--------------|
| **Apple** | iOS/macOS development | SwiftUI, UIKit, Foundation, and 50+ frameworks |
| **Rust** | Rust documentation | std, core, alloc + any crate from docs.rs |
| **Telegram** | Bot API documentation | Methods, types, and parameters |
| **TON** | Blockchain API | REST endpoints and schemas |
| **Cocoon** | Confidential computing | Architecture and smart contracts |
| **MDN** | Web development | JavaScript, TypeScript, Web APIs, DOM |
| **Web Frameworks** | Frontend/Backend | React, Next.js, Node.js |

## Quick Start

Build the Rust binary (requires Rust 1.76+):

```bash
cargo build --release
```

Configure your MCP client:

```json
{
  "mcpServers": {
    "docs-mcp": {
      "command": "/absolute/path/to/target/release/docs-mcp-cli"
    }
  }
}
```

For local development:

```bash
cargo run -p docs-mcp-cli
```

## Usage

The server exposes a single unified `query` tool that automatically detects the appropriate provider and returns comprehensive documentation.

### Natural Language Queries

Simply describe what you're looking for:

```
query { "query": "SwiftUI NavigationStack" }
query { "query": "Rust tokio spawn async" }
query { "query": "Telegram sendMessage parameters" }
query { "query": "JavaScript Array map filter" }
query { "query": "React useState hook" }
query { "query": "Next.js server components" }
query { "query": "Node.js fs readFile" }
```

### How-To Queries

Ask implementation questions:

```
query { "query": "how to use SwiftUI NavigationStack" }
query { "query": "how to implement tab navigation in SwiftUI" }
```

### Provider Auto-Detection

The query tool automatically routes to the correct provider based on keywords:

- **Apple**: SwiftUI, UIKit, iOS, macOS, Foundation, CoreData, etc.
- **Rust**: std, tokio, serde, HashMap, Vec, async, etc.
- **Telegram**: bot, sendMessage, getUpdates, webhook, etc.
- **TON**: blockchain, wallet, jetton, tonapi, etc.
- **Cocoon**: confidential computing, TDX, attestation, etc.
- **MDN**: JavaScript, JS, DOM, fetch, promise, array, etc.
- **React**: hook, useState, useEffect, component, JSX, etc.
- **Next.js**: nextjs, App Router, server component, etc.
- **Node.js**: nodejs, fs, path, http, stream, etc.

## What You Get

For each query, the tool returns:

- **Full documentation content** (not truncated summaries)
- **Code examples** ready to use
- **Declarations/signatures** for API reference
- **Parameters** with descriptions
- **Platform availability** information
- **Related APIs** for further exploration

## Provider-Specific Examples

### Apple (SwiftUI, UIKit, etc.)

```
query { "query": "SwiftUI Button styling" }
query { "query": "UIKit TableView delegate" }
query { "query": "CoreData fetch request" }
```

### Rust

```
query { "query": "Rust HashMap insert" }
query { "query": "tokio spawn async task" }
query { "query": "serde serialize struct" }
```

### Telegram Bot API

```
query { "query": "telegram sendMessage" }
query { "query": "telegram bot webhook" }
query { "query": "telegram inline keyboard" }
```

### TON Blockchain

```
query { "query": "TON wallet address" }
query { "query": "TON jetton transfer" }
```

### Cocoon

```
query { "query": "Cocoon TDX attestation" }
query { "query": "Cocoon confidential computing" }
```

### MDN Web Docs

```
query { "query": "JavaScript Array map" }
query { "query": "fetch API POST request" }
query { "query": "Promise async await" }
query { "query": "DOM querySelector" }
```

### React

```
query { "query": "React useState hook" }
query { "query": "React useEffect cleanup" }
query { "query": "React useContext provider" }
query { "query": "React memo performance" }
```

### Next.js

```
query { "query": "Next.js server components" }
query { "query": "Next.js App Router layout" }
query { "query": "Next.js API routes" }
query { "query": "Next.js middleware" }
```

### Node.js

```
query { "query": "Node.js fs readFile" }
query { "query": "Node.js http server" }
query { "query": "Node.js path join" }
query { "query": "Node.js stream pipe" }
```

## Search Tips

- Use natural language queries for best results
- Include the technology name for precise matching (e.g., "SwiftUI Button" not just "Button")
- Try how-to queries for implementation guidance
- Use `maxResults` parameter to control result count

## Environment Variables

| Variable | Purpose |
|----------|---------|
| `DOCSMCP_CACHE_DIR` | Override disk cache location |
| `DOCSMCP_HEADLESS` | Set to `1` to skip stdio transport (testing) |
| `RUST_LOG` | Control logging (`info`, `debug`, `trace`) |

## Architecture

```
├── apps/cli/                    # CLI entry point
├── crates/
│   ├── docs-mcp-client/         # Apple documentation API client
│   ├── docs-mcp-core/           # MCP tools, state, services
│   ├── docs-mcp/                # MCP protocol bootstrap
│   └── multi-provider-client/   # All provider clients
│       ├── telegram/            # Telegram Bot API
│       ├── ton/                 # TON Blockchain
│       ├── cocoon/              # Cocoon confidential computing
│       ├── rust/                # Rust std + docs.rs
│       ├── mdn/                 # MDN Web Docs
│       └── web_frameworks/      # React, Next.js, Node.js
```

## Development

```bash
# Build
cargo build --release

# Run tests
cargo test

# Lint
cargo clippy --all-targets

# Test MCP handshake and query tool
printf '{"jsonrpc":"2.0","method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"1.0"}},"id":1}\n{"jsonrpc":"2.0","method":"notifications/initialized","params":{}}\n{"jsonrpc":"2.0","method":"tools/call","params":{"name":"query","arguments":{"query":"SwiftUI Button"}},"id":2}\n' | ./target/release/docs-mcp-cli
```

## License

See LICENSE file for details.
