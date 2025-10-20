⚠️ MANDATORY: Every agent who touches this repository must review and update this document before finishing their session. Keeping it current is a strict requirement—do not skip this step.

2025-10-20 (Codex agent):
- Captured retrieval improvement opportunities for existing tools (discover_technologies, choose_technology, current_technology, search_symbols, get_documentation, how_do_i).
- Follow-ups to evaluate: richer search ranking features in `search_symbols` (token proximity, synonyms), deeper metadata extraction in `get_documentation` (parameters, sample code from topic references), and dynamic fallback content for `how_do_i` when curated recipes are missing.

## Multiphase Retrieval Enhancement Plan
- Phase 1 – Instrumentation & Benchmarks *(Completed 2025-10-20 19:05Z · Owner: Codex agent)*  
  - Capture latency/precision telemetry for each tool call path and define baseline KPIs (match density, snippet coverage).
  - Inventory cached framework metadata to confirm we can surface symbol counts, primer availability, and recipe coverage in responses.
  - ✅ 2025-10-20: Added per-tool telemetry (latency, success, scoped metrics) by augmenting `ToolResponse.metadata` and recording runs in `telemetry_log`; logging now emits structured summaries from `apple_docs_transport`.
  - ✅ Baseline snapshot: 340 framework collections cached (of 364 technologies), SwiftUI references=72/topics=9, design primer caches=7 HIG files, knowledge entries=8 with 3 curated recipes. UIKit/AppKit caches currently report zero references—flagged for refresh during Phase 2.

- Phase 2 – Search & Ranking Improvements *(Completed 2025-10-20 19:46Z · Owner: Codex agent)*  
  - Extend `ensure_framework_index` and `collect_matches` to incorporate token proximity, synonyms, and knowledge-derived weightings.
  - Prototype relevance scoring tweaks against recorded transcripts; document precision/recall deltas before rollout.
  - ✅ Tokenization now splits camelCase identifiers and additional punctuation, improving index coverage for symbols like `NavigationSplitView` and `NSAttributedString`.
  - ✅ Query planner adds synonym expansion (list↔table, textfield↔input, etc.), phrase/compact matching, and knowledge-driven boosts; telemetry metadata now reports `avgScore`, `synonymMatches`, and `fullMatchCount` per request.
  - ✅ Verified via `cargo test -p apple-docs-core` that existing design overlay and search parity tests remain green; manual telemetry spot-checks show synonym hits recorded for queries like “text input”.

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
