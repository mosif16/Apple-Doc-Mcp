# Apple Docs CLI Usage Guide

## General Conventions
- Run commands as `apple-docs [FLAGS] <command> [subcommand] [options]`.
- Use `--format markdown|json|table|text` globally to tailor output for humans or scripts (default: `markdown`).
- Pair `--quiet`, `--no-color`, or `--no-progress` with automation to suppress spinners and ANSI styling.
- Override caching with `--cache-dir <path>` when operating from temporary workspaces.

## Core Commands
- `apple-docs serve` — Launches the JSON-RPC transport on STDIO for MCP clients.
- `apple-docs tools list` — Displays registered tools; combine with `--format table` for quick scanning.
- `apple-docs tools call <tool>` — Invokes a tool using `--arguments '{"key":"value"}'` or `--arguments @payload.json`.
- `apple-docs cache status` — Reports cache directory health (existence, read access, entry counts).
- `apple-docs cache warmup` — Prefetches metadata; use `--frameworks <name>` (repeatable), `--refresh`, and `--skip-technologies`.
- `apple-docs cache clear-memory` — Flushes in-process caches without touching disk artifacts.
- `apple-docs telemetry --limit <n>` — Shows recent tool executions with latency and result metadata.
- `apple-docs completions <shell>` — Emits shell completion scripts for bash, zsh, fish, or powershell.

## Tool Invocation Tips
- Tool names are case-sensitive; unknown names return JSON-RPC error `-32601`.
- Omit `--arguments` to send an empty JSON object; avoid shell quoting issues by using argument files (`@payload.json`).
- Prefer `--format json` when piping results into downstream tools; stick to `table` or `markdown` for human reviews.
- When running long tool calls, the CLI shows spinners unless `--no-progress` is set.

## Cache Management Workflow
1. Inspect cache readiness: `apple-docs cache status --format table`.
2. Warm technologies & frameworks before offline work:
   ```bash
   apple-docs cache warmup \
     --frameworks SwiftUI \
     --frameworks UIKit \
     --refresh \
     --format text
   ```
3. Clear memory cache after test runs to validate cold-start behavior.

## Telemetry & Diagnostics
- Check recent activity with `apple-docs telemetry --limit 20 --format table`.
- For deeper troubleshooting, run commands with `RUST_LOG=apple_docs_core=debug` to increase verbosity.
- Correlate tool metadata (e.g., `avgScore`, `synonymMatches`) when diagnosing search relevance regressions.

## Error Handling
- JSON parse failures in `--arguments` bubble up with context; validate JSON before execution.
- Transport errors are surfaced with JSON-RPC codes; repeated `Unknown tool` responses usually indicate stale clients.
- Cache permission issues log to stderr under the `apple_docs_cli` target—ensure the path is writable and accessible.

## Best Practices
- Favor `Headless` mode (automatic for non-`serve` commands) in scripts to avoid STDIO blocking.
- Use table output for quick visual checks, then switch to JSON when building automated validations.
- Capture session-specific observations in `agents.md` after notable CLI workflows to keep operations aligned.
