⚠️ MANDATORY: Every agent who touches this repository must review and update this document before finishing their session. Keeping it current is a strict requirement—do not skip this step.

# Agent Operations Manual

## 1. Document Maintenance Protocol
- **Update cadence:** Re-read and amend this file at the start and end of each engagement. Capture new insights, pitfalls, or workflow changes immediately.
- **Change logging:** When you adjust processes or tooling, append the relevant section here and cross-reference the PR/commit that introduced it.
- **Accuracy gate:** Before handing off work, skim the sections that changed during your session to ensure instructions, file paths, and command examples remain correct.
- **Escalation:** If information becomes outdated but you cannot verify the fix (e.g., blocked by missing credentials), highlight the gap in a new `⚠️ Outstanding Questions` subsection with next steps.
- **Progress tracking:** Record your start, in-progress notes, and completion status for major initiatives—especially the Rust rewrite milestones in Section 16—directly in this file before ending your session.

## 2. Project Snapshot
- **Purpose:** Model Context Protocol (MCP) server that surfaces Apple Developer Documentation to AI coding tools. Entry point lives in `src/index.ts` and the server wiring is in `src/server/app.ts`.
- **Primary dependencies:** `@modelcontextprotocol/sdk` for server scaffolding, `axios` for HTTP requests, `typescript` for build output, and `xo` for linting (`package.json` scripts detail usage).
- **Data strategy:** Apple documentation payloads are cached in JSON under `docs/`. The cache is populated lazily via `AppleDevDocsClient` and persists between runs.
- **Output artifacts:** TypeScript compiles to `dist/`. Never edit files in `dist/` directly—regenerate via the build script.
- **Supported tooling:** Node.js (≥18 recommended for native ESM support), npm for package management, and the MCP transport via stdio (default CLI behavior).

## 3. Repository Layout
- `src/index.ts` — CLI entry point launching the MCP server over stdio.
- `src/server/app.ts` — Server factory; instantiates the MCP `Server`, `AppleDevDocsClient`, and `ServerState`, then registers tools.
- `src/server/tools.ts` — Central definition of MCP tool metadata and handlers.
- `src/server/handlers/` — Logical handlers for each tool (discover, choose, current, search, documentation). Subdirectories host search strategies and formatters.
- `src/server/services/` — Shared services; `framework-loader.ts` handles lazy framework loading, indexing, and symbol expansion logic.
- `src/apple-client.ts` & `src/apple-client/*` — Apple documentation client, HTTP wrapper, file cache, and type definitions.
- `docs/` — Persisted documentation cache (framework JSON, technology list, symbol snapshots). Treat as generated content unless deliberately refreshing upstream data.
- `dist/` — Compiled JavaScript (excluded from linting; regenerate through `npm run build`).
- Root config files: `.xo-config.js`, `tsconfig.json`, `.gitignore`, `package.json`, and this `agents.md`.

## 4. Execution Flow Overview
1. **Startup (`src/index.ts`):** Creates an MCP `Server`, configures stdio transport, and connects. Logs run state to stderr for visibility.
2. **Tool registration (`src/server/app.ts` + `src/server/tools.ts`):** Constructs `AppleDevDocsClient` and `ServerState`, then binds tool definitions (`discover_technologies`, `choose_technology`, `current_technology`, `search_symbols`, `get_documentation`) along with JSON schemas.
3. **Requests:** The MCP runtime calls `CallTool`. The matching handler composes a markdown-centric text response (see `src/server/markdown.ts` helpers for consistent formatting).
4. **Stateful interactions:** `ServerState` tracks the active technology, cached framework data, computed token index, expanded identifiers, and the last discovery result set to support guided flows.
5. **Data retrieval:** `AppleDevDocsClient` first checks `FileCache` (reads from `docs/`). On cache miss, it hits the Apple docs endpoint through `HttpClient`, then persists the JSON for future use.

## 5. AppleDevDocsClient & Caching
- **Instantiation:** `AppleDevDocsClient` creates an `HttpClient` plus a `FileCache` rooted at `process.cwd()/docs` (`src/apple-client.ts`).
- **Framework caching:** `getFramework(name)` loads `docs/<framework>.json` when available; otherwise requests `documentation/<framework>` from Apple, writes the file, and returns it.
- **Symbol caching:** Each symbol path is sanitized (`/` → `__`) and stored as `docs/<symbol>.json`. Cached hits avoid redundant network calls.
- **Technology list:** `docs/technologies.json` stores a record keyed by identifier. Loading logic tolerates both raw records and wrapper objects for backward compatibility.
- **Refresh hooks:** `refreshFramework` and `refreshTechnologies` bypass cache and forcibly re-fetch, overwriting local JSON. Invoke these if documentation changes upstream.
- **Storage hygiene:** Keep the `docs/` directory under 50–100 MB to prevent bloated repos; prune stale caches when frameworks are removed or renamed.

## 6. ServerState & Search Mechanics
- **Active context:** `ServerState` keeps the currently selected `Technology` plus its `FrameworkData`. Clearing the technology also clears the per-framework index and expanded identifier set.
- **Global cache:** `global_indexes` memoizes `FrameworkIndexEntry` vectors per technology so `search_symbols` can scan every framework when callers set `scope: "global"`. Entries hydrate lazily from cached `FrameworkData`.
- **Index building:** `ensureFrameworkIndex` tokenizes symbol titles, URLs, and abstracts into a searchable token map (see `crates/apple-docs-core/src/services/mod.rs`). Tokens are lowercased and split on whitespace, punctuation, and `/._-`.
- **Expansion:** When initial matches are sparse, handlers call `expandSymbolReferences` to fetch nested documentation entries based on identifier batches (default 50 per batch) and merge them into the index.
- **Scoring:** `collectMatches` ranks entries by term overlap (exact token = +3, substring match = +1) and respects optional filters (`platform`, `symbolType`).
- **Fallback strategy:** If direct matches fail, `performFallbackSearches` runs hierarchical search, regex search, then finally a simple client-side scan (`src/server/handlers/search/strategies/*`). Each result is annotated with `foundVia` to describe the fallback path.
- **Last discovery snapshot:** The discover handler stores the visible page in `state` so future UX additions (e.g., follow-up questions) can reuse the context.

## 7. MCP Tools Reference
| Tool | Primary Handler | Purpose | Notes |
| ---- | ---------------- | ------- | ----- |
| `discover_technologies` | `src/server/handlers/discover.ts` | Paginate & filter frameworks before selection | Stores results in state; uses abstract text for fuzzy filtering. |
| `choose_technology` | `src/server/handlers/choose-technology.ts` | Sets the active technology | Performs fuzzy string scoring and validates the selection is a framework collection. |
| `current_technology` | `src/server/handlers/current-technology.ts` | Reports current selection and usage tips | Gracefully handles no-selection case. |
| `search_symbols` | `crates/apple-docs-core/src/tools/search_symbols.rs` | Ranked symbol search within active framework or globally | Supports `scope: "global"` plus `maxResults`, `platform`, `symbolType`; falls back to hierarchical/regex heuristics. |
| `get_documentation` | `src/server/handlers/get-documentation.ts` | Fetches documentation for a symbol path | Accepts relative names (prepends technology identifier when needed). |

- **Formatting:** Handlers assemble markdown responses using helpers in `src/server/markdown.ts` to keep headings, bold sections, and lists uniform.
- **Error surfacing:** Use `McpError` with `ErrorCode.InvalidRequest` when user action is required (e.g., missing technology selection).

## 8. Development Workflow
1. **Bootstrap:** `npm install`.
2. **Compile:** `npm run build` (runs `tsc`, writes to `dist/`, and ensures the executable bit on `dist/index.js`).
3. **Linting:** `npm run lint` (TypeScript type-check + XO). Use `npm run lint:fix` for autofixes, but review formatting changes.
4. **Local run:** `npm start` to launch `dist/index.js` directly (assumes build output is current). Alternatively, during development, run `node --loader ts-node/esm src/index.ts` with caution—project defaults to compiled code.
5. **Testing strategy:** No automated tests yet. Rely on manual tool queries via your MCP client. Document smoke-test transcripts here when introducing new functionality.
6. **Pre-commit checklist:** Ensure `dist/` reflects the latest TypeScript output, caches are sane, lint passes, and this manual reflects any workflow change.
7. **Smoke test (Codex agent, 2025-10-13):** Ran `discover_technologies` → `choose_technology` → `get_documentation` for SwiftUI `Text` and its localized initializer; Quick Summary displayed platform availability and overview sections, but sample code suggestions did not surface—worth a follow-up check.

## 9. Coding Standards
- **TypeScript strict mode:** Keep code type-safe; address new compiler warnings promptly.
- **ESM only:** All files use native ES modules (`type: "module"` in `package.json`). Use `.js` extension in imports even for TS sources (tsconfig rewrites paths at build time).
- **Lint rules:** XO defaults apply; `.xo-config.js` points to `tsconfig.json`. Avoid non-ASCII characters unless required.
- **Error messaging:** Prefer human-friendly descriptions. When rejecting user calls, include actionable follow-ups in the message body.
- **Comments:** Only add contextual comments for non-obvious logic (e.g., explanation of tokenization or batch sizes).

## 10. Documentation Cache Management
- **Location:** `docs/` at repo root, created automatically by `FileCache`. Pre-populated JSON files make first-run experiences fast.
- **Structure:** Framework files mirror Apple’s JSON schema (`metadata`, `references`, `topicSections`). Symbol files store the raw response.
- **Refreshing data:** When upstream docs change, run the relevant `refresh*` methods or delete specific JSON files so the client re-fetches them.
- **Versioning:** Large cache updates can bloat commits. If trimming size is critical, gzip large JSON files before commit or move bulky caches to a release artifact instead of source control.
- **Integrity checks:** After refreshing, run `search_symbols` for known queries to confirm tokens and identifiers populate as expected.

## 11. Server Operations & Tool Reliability
- **Startup & env:** Use `cargo build --release` for production binaries and run with `APPLEDOC_CACHE_DIR` pointing to a writable cache plus `RUST_LOG=apple_docs_core=info,apple_docs_transport=info` for visibility; keep `APPLEDOC_HEADLESS=1` reserved for tests.
- **Handshake sanity check:** After each build, pipe a minimal JSON-RPC sequence into `cargo run --release -p apple-docs-cli` to confirm the `initialize` reply and that `tools/list` returns all definitions (e.g., `printf '...tools/list...' | cargo run ...`).
- **Tool registry health:** Ensure new handlers call `tools::register_tools` and expose a `ToolDefinition` with the `inputSchema` key; verify `context.tools.definitions().await` includes the handler before shipping.
- **RPC compatibility:** The transport accepts both `tools/list` and `list_tools` plus `tools/call` and `call_tool`; do not remove the legacy aliases without coordinating client updates, and document any protocol changes here.
- **Cache migration:** When pointing `APPLEDOC_CACHE_DIR` at the repo’s `docs/` folder, the client auto-upgrades legacy JSON files (e.g., `technologies.json`) into the new cache format on first access, avoiding network fetches.
- **Logging & monitoring:** Watch for `Unknown method` or `Unknown tool` in `apple_docs_transport` logs; these indicate client/transport drift or registry misses and must be resolved before release.
- **Release gate:** Block deploys until `cargo test` passes, the manual `tools/list` probe succeeds, and Codex CLI (or another MCP client) confirms the tool list with the release binary.

## 12. Networking & Rate Limits
- **HTTP client:** `src/apple-client/http-client.ts` uses Axios with automatic retries disabled. Handle transient issues at call sites if needed.
- **Endpoints:** All calls hit `https://developer.apple.com/tutorials/data/documentation/` style URLs. Respect Apple’s rate limits by relying on the cache—avoid bulk refreshing without delays.
- **Error handling:** Transient failures propagate as thrown errors; wrap handler logic in `try/catch` where user-friendly messaging is needed.

## 13. Troubleshooting Playbook
- **“No technology selected” errors:** Ensure the workflow called `discover_technologies` + `choose_technology` before `search_symbols` or `get_documentation`.
- **Empty search results:** Confirm tokens exist by dumping `state.getFrameworkIndex()` in a debug build. Use fallback search logs to trace regex/hierarchical attempts.
- **Clients report no tools listed:** Verify the transport responds to the MCP-standard `tools/list` and `tools/call` methods. The Rust handler now supports both the standard names and the legacy `list_tools`/`call_tool` aliases; if tools disappear again, double-check recent transport edits and rerun `cargo test`.
- **Stale results:** Delete affected `docs/*.json` or call `refreshFramework`/`refreshTechnologies` to rebuild caches.
- **Build failures:** Re-run `npm run lint` for diagnostic output; TypeScript errors reference source lines.
- **Binary permission issues:** On non-Unix systems, skip the `chmod` step in `npm run build` or adjust script for platform compatibility.

## 14. Release Process (Draft)
- **Update version:** Bump `package.json` and tag via `npm version <patch|minor|major>`.
- **Rebuild artifacts:** Run `npm run build` to refresh `dist/`.
- **Smoke test:** Execute representative tool calls against popular frameworks (SwiftUI, UIKit).
- **Publish:** `npm publish` (requires maintainership rights). Verify the package entry lists `docs/technologies.json` per `files` array.
- **Change log:** Reflect release notes in `README.md` under “📋 Changelog” and update this manual if workflows changed.

## 15. Outstanding Questions / Future Work
- Document automated integration tests for MCP tool flows.
- Evaluate caching eviction strategy to limit repo bloat.
- Investigate incremental documentation updates (diff-based refresh) instead of full rewrites.
- Consider telemetry or analytics opt-in for common queries to fine-tune defaults (ensure privacy).
- Identify a Rust-native MCP server transport (evaluate existing crates or plan a minimal stdio protocol layer).
- Confirm continued compliance with Apple documentation usage terms when accessing `https://developer.apple.com/tutorials/data`.
- Decide how to share or migrate cached JSON (`docs/`) between TypeScript and Rust implementations without bloating the repo.
- Determine testing strategy for Rust port (integration harness or snapshot parity) before Phase 3 begins.
- Specify CLI configuration model (flags vs config file) before wiring runtime in Phase 4.
- Build HTTP mocking strategy for Rust client (consider `wiremock` or custom trait) to keep tests offline in later phases.
- Define JSON-RPC schema validation for MCP transport once real handlers land (consider schema tests or typed structs).
- ✅ `get_documentation` now normalizes doc:// identifiers, relative paths, and article responses (SwiftUI drag-and-drop, text input/output, Search) so documentation requests succeed after `choose_technology`. (Codex agent, 2025-10-13)
- ✅ Legacy cache compatibility: Added automatic migration for `docs/technologies.json` and other cached assets so tools succeed with MCP-standard `tools/call` flows. (Codex agent, 2025-10-13)
- ✅ `Swift documentation summaries` (Codex agent, 2025-10-13): Completed – `get_documentation` now adds “Quick Summary” sections with availability, highlights, and sample code pointers for symbols and topics.
  - Implementation notes: Introduced summary helpers in `crates/apple-docs-core/src/tools/get_documentation.rs` and accompanying unit coverage to lock formatting.
  - Tests: `cargo test -p apple-docs-core`
- ✅ `Sample code visibility audit` (Codex agent, 2025-10-13): Completed – Quick Summary now falls back to inline code listings when explicit sample references are absent.
  - Implementation notes: Added `has_code_examples` helper in `crates/apple-docs-core/src/tools/get_documentation.rs` plus new unit coverage for the fallback path.
  - Tests: `cargo test -p apple-docs-core`

## 16. Rust Rewrite Master Plan
Use this section as the authoritative roadmap and progress log for the full Rust port. Update each phase’s status, owner, and notable decisions in-place—do not track progress elsewhere.

- **Phase 0 – Discovery**
  - Audit existing TypeScript modules (`src/server`, `src/apple-client`, caching) to capture behaviors, data contracts, and gaps.
  - Output: inventory of tools, error pathways, outstanding questions appended to Section 15.
  - Phase 0 – Completed (Codex agent, 2025-10-12): Catalogued server architecture, added outstanding questions, and aligned licensing with Mohammed Sayf / Record and Learn LLC.
    - Tool map verified: MCP definitions in `src/server/tools.ts` route to handlers in `src/server/handlers/*`.
    - Server state flow documented: `ServerState` stores technology selection, framework cache, index, and discovery snapshot.
    - Data pipeline noted: `HttpClient` (`src/apple-client/http-client.ts`) hits Apple endpoints with in-memory caching; `FileCache` persists JSON under `docs/`.
    - Search stack recorded: Token index (`framework-loader.ts`), scoring (`search/scoring.ts`), and fallback strategies (hierarchical + regex) for parity.
    - Observed gaps: no automated tests, manual cache management, direct axios usage without retry/backoff configuration.
- **Phase 1 – Architectural Design**
  - Define Rust workspace layout (e.g., crates for client, cache, MCP server, handlers).
  - Select async runtime (Tokio), HTTP/serialization stack (reqwest + serde), and caching strategy.
  - Document module responsibilities and error-handling approach in this file.
  - Phase 1 – Completed (Codex agent, 2025-10-12): Locked-in Rust workspace structure and core technical stack.
    - Workspace layout: `Cargo.toml` workspace with members `crates/apple-docs-client`, `crates/apple-docs-core`, `crates/apple-docs-mcp`, and `apps/cli`.
    - `apple-docs-client`: wraps `reqwest` HTTP calls, serde-based models, formatter helpers, and persistent cache API compatible with existing `docs/` JSON.
    - `apple-docs-core`: owns `ServerState`, search index/tokenization, fallback expansion, and shared domain types; leverages `dashmap` + `tokio::sync` for concurrency.
    - `apple-docs-mcp`: implements stdio-based MCP transport, JSON-RPC framing, tool registry, and markdown builders; exposes `run_server(config)` for reuse.
    - `apps/cli`: binary crate providing CLI flags (cache dir, logging level), initializes `tracing` subscriber, and launches `apple_docs_mcp::run_server`.
    - Technical stack: `tokio` (multi-thread + macros + fs), `reqwest` (`json`, `gzip`, `brotli`), `serde`/`serde_json`, `thiserror`, `anyhow`, `tracing`, `tokio-util`, `dashmap`, `time`, `config` (for env overrides).
    - Error handling: domain enums derive `thiserror::Error`, converted to unified `AppleDocsError`; MCP layer maps to protocol errors, CLI wraps with `anyhow` and structured logs via `tracing`.
    - Caching strategy: async fs persistence mirroring `docs/` layout (override via `APPLEDOC_CACHE_DIR`), layered with TTL in-memory cache using `dashmap`; fallback to synchronous writes behind `spawn_blocking` where needed.
    - Testing & observability: `insta` snapshot tests for tool responses, `tokio::test` async unit coverage, integration harness under `tests/` with stubbed HTTP, optional `criterion` benches for search scoring, `tracing` spans for performance diagnostics.
- **Phase 2 – Infrastructure Setup**
  - Initialize Cargo workspace, configure `rustfmt`, `clippy`, and CI targets mirroring TypeScript lint/build.
  - Establish integration test harness and golden-response fixtures.
  - Phase 2 – Completed (Codex agent, 2025-10-12): Bootstrapped Rust workspace scaffolding and tooling.
    - Added workspace `Cargo.toml`, crate manifests (`apple-docs-client`, `apple-docs-core`, `apple-docs-mcp`, `apple-docs-cli`), and stub source files that compile.
    - Introduced `rustfmt.toml` and `clippy.toml` with pedantic defaults covering workspace members.
    - Stubbed async entry points (`ServerConfig`, `run`, `run_server`, CLI `main`) while honoring `APPLEDOC_CACHE_DIR` env override and tracing hooks.
    - Created initial integration harness (`crates/apple-docs-mcp/tests/smoke.rs`) plus unit scaffolds to keep `cargo test` green during development.
    - Captured dependency convergence via `[workspace.dependencies]` to maintain consistent versions across crates.
- **Phase 3 – Client & Cache Port**
  - Implement Apple docs client with disk cache parity (JSON layout, sanitization, refresh APIs).
  - Provide mocks/fakes for offline testing and note rate-limit/backoff rules.
  - Phase 3 – Completed (Codex agent, 2025-10-12): Implemented Rust HTTP client and caching stack.
    - Added `apple-docs-client` modules for types, memory/disk caches, and error handling with TTL in-memory layer plus persistent JSON mirroring TypeScript format.
    - Implemented `get_framework`, `get_symbol`, `get_technologies`, and refresh helpers with concurrency guards and sanitized filenames.
    - Wired `DiskCache` to serialize `CacheEntry<T>` with timestamps, ensured cache directories auto-create, and added unit tests for cache round-trips and TTL expiry.
    - Extended `apple-docs-core` stub to instantiate `AppleDocsClient` honoring optional cache-dir overrides.
    - Flagged need for HTTP stubbing (outstanding items) before enabling networked tests.
- **Phase 4 – MCP Server Core**
  - Build stdio transport, request router, shared state structure, and markdown formatting utilities.
  - Align MCP error propagation semantics with the TypeScript server.
  - Phase 4 – Completed (Codex agent, 2025-10-12): Established Rust MCP runtime scaffolding.
    - Added shared state/context (`apple-docs-core/src/state.rs`) with `AppleDocsClient` and framework caches guarded by async locks.
    - Stubbed tool registry (`apple-docs-core/src/tools/mod.rs`) and markdown helpers mirroring TS formatting utilities.
    - Implemented stdio transport loop (`apple-docs-core/src/transport/mod.rs`) that emits readiness signal and echoes stub responses while wiring in `tokio` async I/O.
    - Extended `ServerConfig` with runtime mode (`Stdio` vs `Headless`) and updated `run` to register tools, instantiate context, and invoke the transport.
    - Updated `apple-docs-mcp` CLI to honour `APPLEDOC_HEADLESS` for tests and log mode/cache selection. Future phases will replace stub responses with real tool dispatch.
- **Phase 5 – Tool Handlers**
  - Port `discover`, `choose`, `current`, `search`, `get_documentation` sequentially.
  - Validate output parity with TS implementation using snapshot tests; note deviations here.
  - Phase 5 – Completed (Codex agent, 2025-10-12): Implemented Rust tool handlers and JSON-RPC routing.
    - Added shared tool registry with async handlers, full stdio JSON-RPC router, and list/call tool support with structured error reporting.
    - Ported discover/choose/current/search/get_documentation logic, including tokenized search index, identifier expansion, markdown formatting, and technology state management.
    - Ensured `cargo check` passes across workspace; `cargo fmt` applied. Pending follow-up: build offline mocks and parity snapshot tests (see Outstanding Questions).
- **Phase 6 – Search Enhancements**
  - Recreate hierarchical and regex fallbacks with async batching and memoization.
  - Benchmark vs. TS implementation; log performance findings and tuning changes.
  - Phase 6 – Completed (Codex agent, 2025-10-12): Added hierarchical/regex fallback search with JSON-RPC surfacing.
    - Implemented fallback search pipeline in Rust, including identifier expansion reuse, hierarchical substring matches, and fuzzy regex suggestions when primary scoring fails.
    - Integrated fallback results into tool output with platform metadata and provenance labels; added `regex` dependency and ensured `cargo check` passes across workspace.
    - Future follow-up: tune regex pattern ranking and add snapshot tests to validate parity with TypeScript responses.
    - (Codex agent, 2025-10-13): Extended `search_symbols` with a `scope` parameter that aggregates indexes from every technology using the new `global_indexes` cache and added regression coverage for the cached global search path.
- **Phase 7 – Quality & Parity**
  - Build end-to-end MCP integration suite, add property tests for scoring, enforce `clippy -D warnings`.
  - Compare response diffs; capture residual gaps and mitigation plans in this section.
  - Phase 7 – Completed (Codex agent, 2025-10-12): Established initial Rust testing coverage.
    - Added `cargo test` suite for search fallbacks (`crates/apple-docs-core/tests/search.rs`), verifying direct matches vs. fallback recommendations.
    - Ensured workspace formatting and `cargo check`/`cargo test -p apple-docs-core` succeed; outstanding follow-up: expand coverage with HTTP mocks + snapshot parity.
    - Implemented MCP `initialize` response (including protocol negotiation), removed the premature `ready` emission, and taught the transport to ignore `notifications/initialized`, restoring Codex handshake compatibility.
- **Phase 8 – Migration & Release**
  - Publish Rust-only release artifacts (crate and standalone binary); no Node/TypeScript build remains.
  - Update documentation (`README.md`, agents manual, changelog) to reflect the Rust CLI, tag release, and verify distribution via `cargo install` or binary uploads.
  - Phase 8 – Pending (Codex agent, 2025-10-13): Confirmed phases 0–7 remain green and TypeScript sources have been removed; release packaging, documentation refresh, and distribution steps still outstanding.
    - (Codex agent, 2025-10-13): Exercised apple_docs MCP workflow end-to-end (discover → choose → current → search → documentation) against SwiftUI; added negative search and invalid documentation path probes to confirm graceful handling.
- **Phase 9 – Post-release Hardening**
  - Exercise release binaries on supported platforms, add regression tests, and monitor for issues reported after distribution.
  - Iterate on tooling (benchmarks, profiling, telemetry opt-in) and prepare the roadmap for future enhancements.
- **Progress Log Template**
  - `Phase X – <status/notes> (owner, date)` ➝ Append under the relevant bullet as you make progress.

## 17. Hand-off Checklist
- ✅ Build & lint scripts succeed.
- ✅ Required caches exist and reflect latest upstream docs.
- ✅ Key workflows verified (discover → choose → search → get docs).
- ✅ `agents.md` updated to capture new insights—including this checklist if it evolves.
- ✅ Open questions recorded in Section 15 with owners or next steps.
