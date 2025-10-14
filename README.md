# Apple Doc MCP

A Model Context Protocol (MCP) server written in Rust that provides seamless access to Apple's Developer Documentation directly within your AI coding assistant.



## Quick Start

```"Use apple mcp select swiftui search tabbar"```

Configure your MCP client (example):

Build the Rust binary (requires Rust 1.76+):

```bash
cargo build --release
```

Then point your MCP client at the compiled CLI:

```json
{
  "mcpServers": {
    "apple-docs": {
      "command": "/absolute/path/to/apple-doc-mcp-main/target/release/apple-docs-cli"
    }
  }
}
```

For local development you can run the server directly with:

```bash
cargo run -p apple-docs-cli
```

## 🔄 Typical Workflow

1. Explore the catalogue:
   - `discover_technologies { "query": "swift" }`
   - `discover_technologies { "page": 2, "pageSize": 10 }`
2. Lock in a framework:
   - `choose_technology "SwiftUI"`
   - `current_technology`
3. Search within the active framework:
   - `search_symbols { "query": "tab view layout" }`
   - `search_symbols { "query": "toolbar", "maxResults": 5 }`
4. Open documentation:
   - `get_documentation { "path": "TabView" }`
   - `get_documentation { "path": "documentation/SwiftUI/TabViewStyle" }`

### Search Tips
- Start broad (e.g. `"tab"`, `"animation"`, `"gesture"`).
- Try synonyms (`"sheet"` vs `"modal"`, `"toolbar"` vs `"tabbar"`).
- Use multiple keywords (`"tab view layout"`) to narrow results.
- If nothing turns up, re-run `discover_technologies` with a different keyword or pick another framework.

## 🧰 Available Tools
- `discover_technologies` – browse/filter frameworks before selecting one.
- `choose_technology` – set the active framework; required before searching docs.
- `current_technology` – show the current selection and quick next steps.
- `search_symbols` – fuzzy keyword search within the active framework.
- `get_documentation` – view symbol docs (relative names allowed).
