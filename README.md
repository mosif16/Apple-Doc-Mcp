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

## CLI Usage

The CLI now exposes dedicated subcommands so you can script documentation lookups without wiring up an MCP client first:

```bash
# Run the JSON-RPC transport (existing MCP integration)
apple-docs serve

# Inspect available tools with structured table output (default format = markdown)
apple-docs tools list --format table

# Execute a tool directly from the shell
apple-docs tools call search_symbols \
  --arguments '{"query": "tab view layout"}' --format json

# Leverage argument files for more complex payloads
apple-docs tools call get_documentation --arguments @payload.json

# Monitor recent tool activity
apple-docs telemetry --limit 10 --format table

# Review and hydrate the on-disk cache before going offline
apple-docs cache status
apple-docs cache warmup --frameworks SwiftUI --frameworks UIKit --refresh
apple-docs cache clear-memory
```

Global flags help tune accessibility and automation:

- `--format json|markdown|table|text` switches between renderer styles.
- `--quiet` suppresses human-oriented messaging for scripts.
- `--no-color` and `--no-progress` disable ANSI styling and spinners.
- `--cache-dir <path>` overrides `APPLEDOC_CACHE_DIR` for temporary workspaces.

Progress spinners appear for long-running operations (framework warmups, remote tool calls) and automatically collapse into concise status lines when finished.

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
5. Ask for a guided recipe:
   - `how_do_i { "task": "add search suggestions" }`
   - `how_do_i { "task": "limit search with scopes" }`

### Search Tips
- Start broad (e.g. `"tab"`, `"animation"`, `"gesture"`).
- Try synonyms (`"sheet"` vs `"modal"`, `"toolbar"` vs `"tabbar"`).
- Use multiple keywords (`"tab view layout"`) to narrow results.
- If nothing turns up, re-run `discover_technologies` with a different keyword or pick another framework.

### Enriched Output
- Quick Summaries now include inline Swift snippets (when available), platform availability, curated next steps, and Human Interface Guideline (HIG) highlights for layout, typography, color, and accessibility.
- Search results show availability badges, SwiftUI ↔ UIKit/AppKit bridge hints, HIG “Design checklist” bullets, and related symbols pulled from the knowledge base.
- `Integration Notes` in documentation call out migration tips, UIKit/AppKit counterparts, and related APIs you should explore next; the new **Design Guidance** section links directly to relevant HIG articles.
- `current_technology` surfaces HIG primers for the selected framework so you can jump straight into design best practices, and `discover_technologies` labels frameworks with built-in design guidance.

## 🧰 Available Tools
- `discover_technologies` – browse/filter frameworks before selecting one.
- `choose_technology` – set the active framework; required before searching docs.
- `current_technology` – show the current selection and quick next steps.
- `search_symbols` – fuzzy keyword search within the active framework.
- `get_documentation` – view symbol docs (relative names allowed).
- `how_do_i` – fetch a guided multi-step recipe for common SwiftUI tasks.
