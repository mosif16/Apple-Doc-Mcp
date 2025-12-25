# Feedback Intake (docs-mcp)

This folder is the drop-box for **agent + developer feedback** about this MCP server.

Use the `submit_feedback` MCP tool to write a structured JSON record here. The server writes files named like `feedback_<unix>_<nanos>_pid<pid>.json`.

## Best Practices (Ingestion)

- **Prefer concrete examples**: include the exact query, symbol name, provider, and what you expected vs. what happened.
- **One issue per bullet**: keep `improvements`, `missingDocs`, and `painPoints` as short actionable bullets (not essays).
- **Include reproducibility hints**: agent name/version/model, OS, and any relevant env/config (cache dir, headless/stdio).
- **Codex CLI automation**: call `submit_feedback` via `codex exec` and set `client.model` to `gpt-5.2-codex` and `client.reasoning` to `xhigh` (separate from the model name).
- **Avoid secrets**: never include API keys, tokens, user PII, or proprietary source code. Summarize privately instead.
- **Keep feedback stable**: reference docs by symbol/URL/technology rather than “the third result”.
- **Use severity**: if something is a blocker, say so (and why); otherwise mark it as “nice-to-have”.

## Suggested Triage Workflow

1. **Normalize**: dedupe similar feedback by grouping on `missingDocs` and repeated queries.
2. **Classify**: tag as `ranking`, `coverage`, `performance`, `formatting`, `tooling`, or `protocol`.
3. **Reproduce**: rerun the exact query; compare output with expectations; capture logs if needed.
4. **Fix**: implement smallest change that improves the class of issues (avoid one-off hacks).
5. **Measure**: validate improvements with recorded transcripts/telemetry and tests.
