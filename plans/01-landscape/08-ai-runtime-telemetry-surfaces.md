# AI Runtime Telemetry Surfaces

## Executive summary

This document catalogs what telemetry each of the six target AI runtimes actually exposes to a shipper as of May 2026, with citations. It is the empirical foundation for Mara's adapter design and the per-runtime tier classification in [`../00-overview/03-glossary.md`](../00-overview/03-glossary.md). The findings here drive [`../05-evaluation/02-compatibility-matrix-spec.md`](../05-evaluation/02-compatibility-matrix-spec.md).

For each runtime we cover: (1) local artifacts, (2) hooks/event APIs, (3) IPC/MCP/HTTP surfaces, (4) OpenTelemetry support, (5) privacy/ZDR posture, (6) authoritative docs.

## Claude Code — Anthropic CLI + Claude desktop remote-control

**Tier:** A (native OTLP) + B (JSONL/hooks redundancy).

- **Local artifacts.** Conversation transcripts under `~/.claude/projects/<uuid>.jsonl`. The `claude project purge` command enumerates persisted artifacts (transcripts, task lists, edit history). Background sessions: `claude logs <id>`. Debug: `--debug`, `--debug-file`, `CLAUDE_CODE_DEBUG_LOGS_DIR`.
- **Hooks / events.** Rich lifecycle hooks: `SessionStart`, `PreToolUse`, `PostToolUse`, and others. Hooks receive JSON on stdin and can be configured per-project. Schemas documented in the Anthropic hooks reference.
- **IPC / MCP / HTTP.** First-class MCP client (`claude mcp ...`). Remote-control server mode (`claude remote-control`) used by the Claude web/app to drive a local CLI. Not a generic OTLP proxy.
- **OpenTelemetry.** First-party OTLP export from the CLI when `CLAUDE_CODE_ENABLE_TELEMETRY=1`. Standard `OTEL_*` exporter variables. `CLAUDE_CODE_ENHANCED_TELEMETRY_BETA` for traces beta.
- **Privacy / ZDR.** Prompt, tool detail, and raw-API-body logging are explicitly opt-in via `OTEL_LOG_USER_PROMPTS`, `OTEL_LOG_TOOL_DETAILS`, `OTEL_LOG_RAW_API_BODIES`.
- **Docs.** <https://docs.anthropic.com/en/docs/claude-code/cli-reference>, <https://docs.anthropic.com/en/docs/claude-code/hooks>, <https://code.claude.com/docs/en/agent-sdk/observability>, <https://code.claude.com/en/monitoring-usage>.

**Mara approach.** Run an OTLP receiver on `127.0.0.1:4317` and document setting `OTEL_EXPORTER_OTLP_ENDPOINT=http://localhost:4317`. Run a JSONL tail on `~/.claude/projects/**/*.jsonl` as redundant signal for ZDR-strict environments where OTel toggles are off. Hooks adapter optional for richer per-tool event capture.

## Codex — OpenAI CLI + Codex desktop / remote

**Tier:** A (native OTLP) + B (JSONL/hooks redundancy).

- **Local artifacts.** Default history at `~/.codex/history.jsonl` when persistence is enabled. Config and state under `CODEX_HOME` (`~/.codex`) including `config.toml` and `auth.json`. Non-interactive mode `codex exec --json` emits a JSONL event stream (`thread.*`, `turn.*`, `item.*`).
- **Hooks / events.** Experimental hooks via `hooks.json` or `[hooks]` in TOML, feature-flagged `codex_hooks`. The `notify` hook receives JSON for `agent-turn-complete`. Event schemas documented at <https://developers.openai.com/codex/hooks>.
- **IPC / MCP / HTTP.** `codex mcp-server` runs Codex as MCP over stdio. `codex app-server` exposes stdio or WebSocket (`--listen ws://...`) for experimental local bridging. Marked dev/debug; not a stable public OTLP surface.
- **OpenTelemetry.** Official `[otel]` block in `config.toml` for OTLP HTTP/gRPC log exporters + metrics catalog. `log_user_prompt` defaults to false (prompt redaction). `[analytics]` block is separate from OTel and is a phone-home metrics feature; Mara users typically disable it.
- **Privacy / ZDR.** OTel prompt logging gated. Analytics opt-in/opt-out separate. Codex security documentation covers ZDR posture.
- **Docs.** <https://developers.openai.com/codex/noninteractive>, <https://developers.openai.com/codex/config-advanced>, <https://developers.openai.com/codex/cli/reference>, <https://developers.openai.com/codex/hooks>.

**Mara approach.** Same OTLP receiver pattern as Claude Code. JSONL tail on `~/.codex/history.jsonl` as redundant signal. Optional `notify` hook script that POSTs to Mara's hooks adapter when a turn completes (useful for richer per-turn correlation).

## Cursor Agents — IDE Agent + Cmd+K + Cursor CLI

**Tier:** B (hooks only — no transcript path).

- **Local artifacts.** **No documented first-party agent transcript path** for the IDE Agent comparable to Claude/Codex. MCP troubleshooting via the Output panel only. Enterprise audit logs explicitly exclude agent responses and code.
- **Hooks / events.** Official Hooks that run subprocesses with **JSON over stdio** for Cmd+K and Agent Chat events. This is the primary programmatic observability extension point Cursor documents.
- **IPC / MCP / HTTP.** Cursor is an MCP client; ships/logs MCP server interactions — not an MCP telemetry broker by default.
- **OpenTelemetry.** **No public shipper-style OTLP export** documented for the Agent loop itself. Third-party MCP-OTel bridges exist but are not Cursor-first-party.
- **Privacy / ZDR.** Enterprise Privacy/ZDR flows documented. Audit logs explicitly omit agent responses and code, with hooks recommended for prompt/code-adjacent logging.
- **Docs.** <https://cursor.com/docs/hooks.md>, <https://cursor.com/docs/context/model-context-protocol>, <https://cursor.com/docs/enterprise/privacy-and-data-governance>, <https://cursor.com/docs/enterprise/compliance-and-monitoring>.

**Mara approach.** Hooks adapter is the only first-class path. Provide a `mara cursor-hook` subcommand that users wire into their Cursor hook configuration; the subcommand emits to a local Unix socket the Mara agent listens on. Mara normalizes the Cursor hook JSON into canonical events. Document the gap (no transcript path) prominently.

## Kimi — Moonshot CLI / Web UI

**Tier:** B (JSONL tail).

- **Local artifacts.** Debug log at `~/.kimi/logs/kimi.log` when `--debug` is enabled. Session dump via `kimi export` produces a ZIP containing `context.jsonl`, `wire.jsonl`, `state.json`. Print mode `stream-json` emits JSONL on stdout.
- **Hooks / events.** Slash commands, MCP, and plugins are documented; no single "hooks reference" page comparable to Claude/Codex for arbitrary lifecycle events.
- **IPC / MCP / HTTP.** `kimi web` (local HTTP UI), `kimi acp` (ACP server), `kimi wire` (experimental Wire server) — integration transports, not OTLP.
- **OpenTelemetry.** Not documented on official Kimi CLI pages as of the docs available. GitHub PR history shows OTel/telemetry work in flight; treat as implementation-dependent until the pinned release's changelog confirms.
- **Privacy / ZDR.** Moonshot-hosted traffic applies; no public ZDR statement surfaced in CLI docs for telemetry export defaults.
- **Docs.** <https://moonshotai.github.io/kimi-cli/en/reference/kimi-command.html>, <https://moonshotai.github.io/kimi-cli/>.

**Mara approach.** JSONL tail on `~/.kimi/logs/kimi.log` plus opportunistic ingest of `kimi export` ZIPs (file watcher on the user's export directory). Optionally parse `stream-json` from a wrapped invocation. Track upstream OTel work and graduate to Tier A when stable.

## Augment Code — IDE extension + remote agents

**Tier:** C (analytics REST best-effort).

- **Local artifacts.** **No documented local JSONL/SQLite transcript path** for shipper ingestion in public docs reviewed.
- **Hooks / events.** **No public hook API** documented analogous to Claude/Codex/Cursor hooks.
- **IPC / MCP / HTTP.** Enterprise Analytics REST (preview) plus dashboard. Network docs cite telemetry host `evs.grdt.augmentcode.com` for allow-listing.
- **OpenTelemetry.** **No public OTel exporter** documented for the IDE agent loop.
- **Privacy / ZDR.** Usage flows to Augment cloud endpoints for analytics/telemetry; enterprise governance applies via Augment's own controls.
- **Docs.** <https://docs.augmentcode.com/analytics/overview>, <https://docs.augmentcode.com/analytics/analytics-api>, <https://docs.augmentcode.com/setup-augment/network-configuration>.

**Mara approach.** Analytics REST adapter polls the Augment Analytics API on a configurable cadence where the operator opts in. v1 compatibility matrix marks Augment as **best-effort**. Track upstream; design the adapter contract so a future hook adapter slots in without breaking changes.

## Gemini — `gemini-cli` + Gemini API

**Tier:** A (native OTLP).

- **`gemini-cli`.** Telemetry off by default; configurable via `.gemini/settings.json` and environment variables. Supports an OTLP endpoint or an `outfile` (e.g., `.gemini/telemetry.log`). `logPrompts` is opt-in. `GEMINI_TELEMETRY_TRACES_ENABLED` enables richer traces. Extensive `gemini_cli.*` logs/metrics plus `gen_ai.*` attributes documented.
- **Gemini HTTP API (consumer).** No public "local agent transcript file" or official CLI shipper beyond what the consumer instruments server-side. No separate Google-documented edge-agent log format for generic API key consumers.
- **Docs.** <https://geminicli.com/docs/cli/telemetry/>, <https://github.com/google-gemini/gemini-cli/blob/main/docs/cli/telemetry.md>.

**Mara approach.** OTLP receiver. Document the `OTLP endpoint` config and the `logPrompts` toggle. Easiest of the six runtimes.

## Ollama — local LLM runtime (added as 7th target for MVP)

**Tier:** Proxy (a new tier specific to runtimes that expose no native telemetry but a well-defined HTTP API).

- **Local artifacts.** Server log at `~/.ollama/logs/server.log` (macOS, also `~/.ollama/logs/app.log`), `journalctl -u ollama` (Linux), `%LOCALAPPDATA%\Ollama\server.log` (Windows). Format is unstructured text per the official troubleshooting page. **Logs do not carry token counts** structurally.
- **Hooks / events.** No hooks API; no Prometheus `/metrics` endpoint; no OTel exporter as of release v0.21.0 (April 2026). PRs <https://github.com/ollama/ollama/pull/6537> and <https://github.com/ollama/ollama/pull/11159> are open but unmerged.
- **HTTP API.** Both native (`/api/generate`, `/api/chat`, `/api/embed`, `/api/ps`, `/api/tags`, `/api/show`, `/api/version`, `/api/pull`, `/api/push`, `/api/copy`, `/api/delete`, `/api/create`) and OpenAI-compatible (`/v1/chat/completions`, `/v1/completions`, `/v1/embeddings`, `/v1/models`, plus `/v1/responses` added in v0.13.3 and experimental `/v1/images/generations`). Default bind `127.0.0.1:11434`; configurable via `OLLAMA_HOST`.
- **Response telemetry fields (native).** Per <https://docs.ollama.com/api/usage>: `total_duration`, `load_duration`, `prompt_eval_count`, `prompt_eval_duration`, `eval_count`, `eval_duration`, `model`, `created_at`, `done`, `done_reason`. Durations are nanoseconds. Streaming: same fields on the final SSE chunk where `done=true`.
- **OpenTelemetry.** Not first-party as of May 2026; do not design assuming `/metrics` or built-in OTLP without runtime probing.
- **Privacy / ZDR.** All inference is local by design; ZDR question is moot for the workload, but Mara's redaction still applies to events emitted to operator-chosen sinks.
- **License.** MIT (<https://github.com/ollama/ollama/blob/main/LICENSE>).
- **Docs.** <https://docs.ollama.com>, <https://docs.ollama.com/api>, <https://docs.ollama.com/api/usage>, <https://docs.ollama.com/api/openai-compatibility>, <https://docs.ollama.com/faq>, <https://docs.ollama.com/troubleshooting>.

**Mara approach.** HTTP proxy adapter (`mara-adapter-llm-proxy`) binds the conventional `127.0.0.1:11434`; Ollama is reconfigured to listen on `127.0.0.1:11435` via `OLLAMA_HOST`. Mara forwards every request and captures both request and response bodies (unary and streaming) without payload mutation. Token counts pulled from `prompt_eval_count` / `eval_count`; latency from `*_duration` fields divided by 1_000_000 to milliseconds. Local inference is cost-zero (`mara.cost.usd = 0`, `mara.cost.source = "local_inference"`, `mara.compute.is_local = true`). Full design in [`../08-mvp/12-ollama-integration-design.md`](../08-mvp/12-ollama-integration-design.md). Proxy is the only viable path because logs don't structurally carry tokens and no first-party telemetry exporter exists in stock Ollama.

## OpenTelemetry `gen_ai.*` semantic conventions (May 2026 posture)

- Status: `Development` — not yet stable. Migration guidance via `OTEL_SEMCONV_STABILITY_OPT_IN=gen_ai_latest_experimental` vs `v1.36.0`-era defaults.
- Namespace standardizes `gen_ai.*` for events, metrics, model spans, agent spans, plus the MCP conventions page.
- Sources: <https://opentelemetry.io/docs/specs/semconv/gen-ai/>, <https://github.com/open-telemetry/semantic-conventions/tree/main/docs/gen-ai>.

**Mara design takeaway.** Pin to a specific semconv commit. Feature-flag the `latest_experimental` opt-in. Keep `mara.*` extensions clearly namespaced. Avoid inventing parallel top-level schemas where semconv already defines signal shapes.

## Summary tier table

- **Tier A — native OTLP receive.** Claude Code, Codex, Gemini CLI. Mara's job is mostly "be a good OTLP receiver and normalize gen_ai.* attributes faithfully." Rich event content available with opt-in.
- **Tier B — hooks / JSONL tail.** Cursor Agents (hooks only), Kimi (JSONL only), plus Claude Code and Codex as redundant signal. Mara's job is the file-tail bookkeeping (offsets, rotation, decoding) and the per-runtime parsing rules.
- **Tier C — analytics REST best-effort.** Augment Code. Mara's job is graceful polling and clear gap documentation.
- **Proxy tier — HTTP proxy capture.** Ollama (and post-MVP: OpenAI-direct, Anthropic-direct, Bedrock via OpenAI-compat shim). Mara stands in front of the runtime's HTTP server on the conventional port, forwards to a relocated upstream, and captures every request/response pair. The only viable telemetry path for runtimes with rich HTTP APIs but no first-party exporter.

## Cross-runtime concerns

- **Time correlation.** Each runtime emits its own clock. Mara normalizes to UTC nanoseconds at the adapter boundary and preserves the original timestamp under `mara.source.timestamp_raw` when available.
- **Session identification.** Claude Code, Codex, Cursor, Kimi all have their own session/turn identifier formats. Mara stamps `mara.session.id` and `mara.turn.id` from the runtime's identifier; falls back to a Mara-generated UUID when absent.
- **Cost computation.** Tier A runtimes emit `gen_ai.usage.*` and (sometimes) `gen_ai.cost.*`. Tier B/C may not. Mara computes `mara.cost.usd` from token counts × model price when the cost is absent and a price table is configured.
- **Tool-call correlation.** All six runtimes have some notion of tool calls (MCP or proprietary). Mara aligns to `gen_ai.tool.*` and `mcp.*` semconv fields.
- **ZDR enforcement.** Each runtime's ZDR toggle is honored agent-side: prompt and raw-body capture defaults to off; opt-in is per pipeline.

## Ongoing tracking

When any runtime ships a new telemetry surface (hooks API change, new OTLP support, transcript path change), the change is recorded as a PR against this document with a date stamp, and the relevant runtime preset in `crates/mara-runtimes/<runtime>/` is updated.
