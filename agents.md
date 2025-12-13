⚠️ MANDATORY: Every agent who touches this repository must review and update this document before finishing their session. Keeping it current is a strict requirement—do not skip this step.

2025-12-13 (Codex agent):
- Added an automated feedback loop: MCP `submit_feedback` tool writes structured JSON into `Feedback/`, and the server prompts for feedback via initialization instructions + a `notifications/message` nudge after `notifications/initialized`.
- Added `Feedback/README.md` with best practices for feedback ingestion and triage; `Feedback/.gitignore` keeps artifacts out of git.
- Follow-up: fixed the workspace so `cargo clippy --all-targets -- -D warnings` and `cargo test --all` both pass (includes reducing dead code in `docs-mcp-core` tool module wiring, and consolidating clippy allowances + small refactors in `multi-provider-client`).
- Fix: MCP stdio transport now supports both newline-delimited JSON and LSP-style `Content-Length` framing (and replies in the same framing) to avoid client startup failures where `tools/list` closed the transport.

2025-10-20 (Codex agent):
- Captured retrieval improvement opportunities for existing tools (discover_technologies, choose_technology, current_technology, search_symbols, get_documentation, how_do_i).
- Follow-ups to evaluate: richer search ranking features in `search_symbols` (token proximity, synonyms), deeper metadata extraction in `get_documentation` (parameters, sample code from topic references), and dynamic fallback content for `how_do_i` when curated recipes are missing.

## Multiphase Retrieval Enhancement Plan
- Phase 1 – Instrumentation & Benchmarks *(Completed 2025-10-20 19:05Z · Owner: Codex agent)*  
  - Capture latency/precision telemetry for each tool call path and define baseline KPIs (match density, snippet coverage).
  - Inventory cached framework metadata to confirm we can surface symbol counts, primer availability, and recipe coverage in responses.
  - ✅ 2025-10-20: Added per-tool telemetry (latency, success, scoped metrics) by augmenting `ToolResponse.metadata` and recording runs in `telemetry_log`; logging now emits structured summaries from `docs_mcp_transport`.
  - ✅ Baseline snapshot: 340 framework collections cached (of 364 technologies), SwiftUI references=72/topics=9, design primer caches=7 HIG files, knowledge entries=8 with 3 curated recipes. UIKit/AppKit caches currently report zero references—flagged for refresh during Phase 2.

- Phase 2 – Search & Ranking Improvements *(Completed 2025-10-20 19:46Z · Owner: Codex agent)*  
  - Extend `ensure_framework_index` and `collect_matches` to incorporate token proximity, synonyms, and knowledge-derived weightings.
  - Prototype relevance scoring tweaks against recorded transcripts; document precision/recall deltas before rollout.
  - ✅ Tokenization now splits camelCase identifiers and additional punctuation, improving index coverage for symbols like `NavigationSplitView` and `NSAttributedString`.
  - ✅ Query planner adds synonym expansion (list↔table, textfield↔input, etc.), phrase/compact matching, and knowledge-driven boosts; telemetry metadata now reports `avgScore`, `synonymMatches`, and `fullMatchCount` per request.
  - ✅ Verified via `cargo test -p docs-mcp-core` that existing design overlay and search parity tests remain green; manual telemetry spot-checks show synonym hits recorded for queries like “text input”.

- Phase 3 – Documentation Summaries & Snippet Harvesting *(Completed 2025-10-20 20:05Z · Owner: Codex agent)*  
  - Enhance `get_documentation` to extract parameter tables, relationships, and referenced sample code snippets into the quick summary.
  - Cache extracted snippets and metadata alongside `last_symbol` to reduce repeated parsing cost.
  - ✅ Quick Summary now lists related types and parameter names when available; response body adds dedicated “Relationships” and “Parameters” sections fed by reference metadata or inline JSON.
  - ✅ Telemetry metadata includes `parameterCount`, `relationshipCount`, `summaryCount`, and `hasSampleSummary` so clients can track documentation richness. `last_symbol` continues to retain full payloads; follow-up to persist parsed caches remains open.

- Phase 4 – Guided Workflow & Fallback Recipes *(Completed 2025-10-20 20:20Z · Owner: Codex agent)*
  - Teach `how_do_i` to synthesize fallback recipes from recent `search_symbols` results plus design guidance when curated content is missing.
  - Update `current_technology` and `discover_technologies` to surface next-step actions driven by newly enriched metadata and recipes.
  - ✅ `how_do_i` now logs technology-scoped search queries and, when no curated recipe exists, returns a suggested plan (re-run top search, inspect docs, review related APIs). Telemetry embeds fallback metadata (`suggestedQuery`, `matchesObserved`, `relatedKnowledge`).
  - ✅ `discover_technologies` lists available recipe counts per framework; `current_technology` highlights curated recipes and the most recent search query for the active technology.

- Phase 5 – Performance, Caching, and Quality *(Completed 2025-11-29 · Owner: Claude Code)*
  - Implement pagination, cache metrics, LRU eviction, and expand recipe library to production-ready scale.
  - Add comprehensive test coverage and ensure code quality standards with clippy compliance.
  - ✅ **Search Pagination**: Added `page` and `pageSize` parameters to `discover_technologies` (max 100 per page, returns total_pages metadata).
  - ✅ **Token Proximity Scoring**: Enhanced search ranking with proximity bonuses (adjacent tokens +5, within 2 positions +3, within 4 positions +1) to prioritize "SwiftUI Layout" over scattered matches. Proximity bonus is cumulative across all matched token pairs.
  - ✅ **Cache Metrics System**: Implemented `CacheStats` with atomic counters tracking hits, misses, bytes_served, entry_count, and evictions. Added `CombinedCacheStats` for unified disk+memory reporting. Exposed via `AppleDocsClient::cache_stats()` method.
  - ✅ **LRU Disk Cache Eviction**: Implemented automatic eviction when cache exceeds 500MB limit (configurable). Uses file modification time as LRU proxy, evicts oldest entries first. Properly updates stats and entry counts.
  - ✅ **Design Guidance Pre-caching**: Modified framework loading to eagerly fetch design guidance during `load_active_framework` and `ensure_global_framework_index`, reducing latency for design-related queries.
  - ✅ **Expanded Recipe Library**: Grew from 8 to 30 curated recipes across SwiftUI (18), Foundation Models (7), and other frameworks. Covers navigation, state management, data flow, animations, accessibility, and AI integration patterns.
  - ✅ **Comprehensive Testing**: Added 91 total tests across all crates (36 cache tests, 44 search/tools tests, 6 design overlay tests, 3 fallback tests, 2 smoke tests). All tests passing with 100% success rate.
  - ✅ **Code Quality**: Fixed invalid clippy.toml configuration. Resolved all clippy warnings including iterator efficiency, option handling, struct initialization patterns. Full compliance with `cargo clippy --all-targets -- -D warnings`.
  - ✅ **UIKit/AppKit Verification**: Confirmed zero references for UIKit/AppKit is expected behavior (Apple's documentation API structure), not a cache refresh issue. These frameworks use different documentation patterns.
  - ✅ **Build Verification**: Release binary builds successfully in 4.89s. All tests pass in ~2.5s total runtime.

- Phase 6 – Web Documentation Providers *(Completed 2025-12-02 · Owner: Claude Code)*
  - **MDN Web Docs Provider**: Full integration for JavaScript, TypeScript, Web APIs, and DOM documentation.
  - **Web Frameworks Provider**: React, Next.js, and Node.js documentation with example extraction and quality scoring.
  - ✅ **MDN Client Implementation**: Created `MdnClient` with search API integration, HTML parsing for examples, disk caching, and article fetching.
  - ✅ **Web Frameworks Client Implementation**: Created `WebFrameworksClient` supporting React (hooks, components), Next.js (App Router, Server Components), and Node.js (core modules).
  - ✅ **Type System Extensions**: Added `ProviderType::Mdn` and `ProviderType::WebFrameworks` to the unified type system. Added `TechnologyKind::MdnCategory` and `TechnologyKind::WebFramework`.
  - ✅ **Query Tool Integration**: Added comprehensive keyword detection for MDN (javascript, dom, fetch, promise, etc.), React (hook, useState, useEffect, etc.), Next.js (serverComponent, appRouter, etc.), and Node.js (fs, path, http, etc.).
  - ✅ **Search Functions**: Implemented `search_mdn()` and `search_web_frameworks()` with full documentation fetching and code example extraction.
  - ✅ **Example Quality Scoring**: Implemented `quality_score()` for code examples based on completeness (imports, exports), descriptions, and length.
  - ✅ **Documentation Updated**: CLAUDE.md updated with new providers, detection keywords, caching strategy, and test commands.

- Phase 7 – ML/AI Documentation Providers *(Completed 2025-12-02 · Owner: Claude Code)*
  - **MLX Provider**: Apple Silicon ML framework documentation for Swift and Python.
  - **Hugging Face Provider**: Transformers and swift-transformers documentation for LLM development.
  - ✅ **MLX Client Implementation**: Created `MlxClient` with search, get_article for Swift (DocC) and Python (Sphinx) documentation from ml-explore.github.io.
  - ✅ **MLX Types**: Added `MlxTechnology`, `MlxCategory`, `MlxArticle`, `MlxLanguage` enum with 60+ predefined topics for Swift and Python.
  - ✅ **Hugging Face Client Implementation**: Created `HuggingFaceClient` with search, get_article, search_models for Transformers and swift-transformers.
  - ✅ **Hugging Face Types**: Added `HfTechnology`, `HfCategory`, `HfArticle`, `HfTechnologyKind` with TRANSFORMERS_TOPICS, SWIFT_TRANSFORMERS_TOPICS, and LLM_MODEL_FAMILIES.
  - ✅ **Type System Extensions**: Added `ProviderType::Mlx` and `ProviderType::HuggingFace` to unified types. Added `TechnologyKind::MlxFramework` and `TechnologyKind::HfLibrary`.
  - ✅ **Query Tool Integration**: Added MLX_KEYWORDS and HUGGINGFACE_KEYWORDS detection. Implemented `search_mlx()` and `search_huggingface()` functions.
  - ✅ **Enhanced Apple ML Keywords**: Added 11 ML/AI frameworks to Apple detection (CoreML, CreateML, Vision, NaturalLanguage, Speech, SoundAnalysis, VisionKit, Accelerate, MLCompute, MetalPerformanceShaders, MetalPerformanceShadersGraph).
  - ✅ **Documentation Updated**: CLAUDE.md, README.md, and MCP server instructions updated with new providers.

- Phase 8 – Future Enhancements *(Pending)*
  - **Performance Optimization**: Profile hot paths in search and caching. Consider implementing parallel framework loading for global searches.
  - **Enhanced Metrics**: Add cache hit rate histograms, query latency p99 tracking, and per-framework usage analytics.
  - **Intelligent Caching**: Implement TTL-based cache invalidation for frequently updated frameworks. Consider predictive pre-fetching based on usage patterns.
  - **Advanced Search Features**: Fuzzy matching improvements, platform-specific filtering (iOS vs macOS), and semantic similarity ranking.
  - **Recipe Expansion**: Add more curated recipes for UIKit, AppKit, Core Data, and Combine. Implement dynamic recipe generation from documentation patterns.
  - **MCP Protocol Enhancements**: Explore streaming responses for large result sets, progressive loading indicators, and cancellation support.
  - **Web Framework Enhancements**: Add Vue.js, Angular, Svelte, and Deno documentation providers. Implement live documentation fetching from react.dev and nextjs.org.
  - **MLX/Hugging Face Enhancements**: Add model card parsing, fine-tuning documentation, and MLX-LM integration guides.

## Code Quality Notes (2025-11-29)
- **Strengths**: Excellent test coverage, proper use of Rust atomics for thread-safe cache stats, clean separation of concerns across crates.
- **Architecture**: Three-tier caching (memory TTL + disk persistence + LRU eviction) is well-designed and performant.
- **Testing**: 91 tests with diverse coverage (unit, integration, concurrency). All passing consistently.
- **Recommendations**:
  - Consider adding benchmark suite for regression detection on search performance.
  - Document cache eviction behavior in user-facing docs (CLAUDE.md or README).
  - Monitor memory usage under heavy load; current unbounded memory cache could grow large.
  - Add telemetry for cache eviction frequency to tune the 500MB limit.

## Next Agent Checklist
- [ ] Review this document and update with your session's changes
- [ ] Run full test suite: `cargo test --all`
- [ ] Verify clippy compliance: `cargo clippy --all-targets -- -D warnings`
- [ ] Update phase completion dates and add new roadmap items if applicable
- [ ] Document any breaking changes or API modifications
