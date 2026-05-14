# Mara Compatibility Matrix

Generated at: 2026-05-12 (M3 baseline). Target schema: see [`plans/05-evaluation/02-compatibility-matrix-spec.md`](../plans/05-evaluation/02-compatibility-matrix-spec.md).

For **required `gen_ai.*` / `resource.*` field parity** scores (M2-01), see [`integrations/runtime-parity-matrix.md`](integrations/runtime-parity-matrix.md).

Legend: ✅ PASS · 🟡 PARTIAL · ❌ GAP · 🚫 N/A · ⚠️ EXPERIMENTAL

## Tier classification

- **Tier A — native OTLP**: Claude Code, Codex, Gemini CLI.
- **Tier B — hooks / JSONL tail**: Cursor Agents, Kimi (plus Claude Code & Codex as redundant).
- **Tier C — analytics REST best-effort**: Augment Code.
- **Proxy tier — HTTP proxy capture**: Ollama. The runtime has no native telemetry but a well-defined HTTP API on `127.0.0.1:11434`. Mara sits in front of it and captures every request/response.

## Signal coverage by runtime

### Claude Code (Tier A + B redundant)

- session_lifecycle: ✅
- prompt: ✅ (when `OTEL_LOG_USER_PROMPTS=true` + `mara.policy.capture_optin=true`)
- completion: ✅ (same gating)
- tool_call: ✅
- tool_result: ✅
- token_usage: ✅
- cost: ✅ (Mara-computed when not vendor-emitted)
- latency: ✅
- error: ✅
- eval: 🟡 (when Claude Code exposes eval results)
- feedback: 🟡 (user thumbs etc.)
- mcp_traffic: ✅
- agent_loop: ✅

### Codex (Tier A + B redundant)

- session_lifecycle: ✅
- prompt: ✅ (opt-in)
- completion: ✅ (opt-in)
- tool_call: ✅
- tool_result: ✅
- token_usage: ✅
- cost: 🟡 (computed from price table)
- latency: ✅
- error: ✅
- eval: 🟡
- feedback: 🟡
- mcp_traffic: ✅
- agent_loop: ✅

### Cursor Agents (Tier B — hooks only)

- session_lifecycle: ✅ (via hook lifecycle)
- prompt: 🟡 (payload-dependent; truncation possible)
- completion: 🟡 (same)
- tool_call: ✅
- tool_result: ✅
- token_usage: 🟡 (`gen_ai.usage.*` when hooks JSON includes token fields; otherwise gap)
- cost: 🟡 (`mara.cost_usd` estimated when usage + `[server.gen_ai_pricing]` allow; not vendor-native)
- latency: ✅ (hook timing)
- error: ✅
- eval: 🚫 (no surface)
- feedback: 🚫
- mcp_traffic: ✅ (when Cursor uses MCP tools and the hook payload includes it)
- agent_loop: 🟡 (no canonical session id in some hook events)

### Kimi (Tier B — JSONL tail)

- session_lifecycle: 🟡 (requires `--debug`)
- prompt: 🟡
- completion: 🟡
- tool_call: 🟡
- tool_result: 🟡
- token_usage: 🟡 (implementation-dependent)
- cost: 🟡 (Mara-computed from token counts)
- latency: 🟡
- error: 🟡
- eval: 🚫
- feedback: 🚫
- mcp_traffic: 🟡
- agent_loop: 🟡

### Augment Code (Tier C — best-effort analytics)

- session_lifecycle: 🟡 (analytics-derived)
- prompt: ❌ (not exposed)
- completion: ❌
- tool_call: ❌
- tool_result: ❌
- token_usage: 🟡 (aggregate-only when exposed)
- cost: 🟡 (aggregate-only)
- latency: 🟡 (aggregate)
- error: 🟡 (counts)
- eval: ❌
- feedback: ❌
- mcp_traffic: ❌
- agent_loop: ❌

### Gemini CLI (Tier A)

- session_lifecycle: ✅
- prompt: ✅ (when `logPrompts=true` + `mara.policy.capture_optin=true`)
- completion: ✅ (same)
- tool_call: ✅
- tool_result: ✅
- token_usage: ✅
- cost: ✅ (Mara-computed from price table)
- latency: ✅
- error: ✅
- eval: 🚫
- feedback: 🚫
- mcp_traffic: 🟡
- agent_loop: ✅

### Ollama (Proxy tier — added for MVP)

- session_lifecycle: 🟡 (synthetic per-request session id; multi-turn correlation requires client cooperation, deferred to MVP+1)
- prompt: ✅ (when `mara.policy.capture_optin=true`)
- completion: ✅ (same; SSE-streamed responses captured in full on `done=true` chunk)
- tool_call: 🚫 (Ollama doesn't natively emit tool calls; user code does, and that path is observed via Tier A OTLP receive)
- tool_result: 🚫 (same)
- token_usage: ✅ (`prompt_eval_count` → `input_tokens`, `eval_count` → `output_tokens`)
- cost: ✅ (`mara.cost.usd = 0`, `mara.cost.source = "local_inference"`, `mara.compute.is_local = true`)
- latency: ✅ (`mara.ollama.total_duration_ms`, `load_duration_ms`, `prompt_eval_duration_ms`, `eval_duration_ms`, `tokens_per_sec`)
- error: ✅ (HTTP status forwarded; error events emitted with `mara.upstream.error` reason)
- eval: 🚫
- feedback: 🚫
- mcp_traffic: 🚫 (Ollama does not emit MCP traffic; MCP-using apps that call Ollama are observed via the app's own instrumentation)
- agent_loop: 🚫 (no native concept; multi-turn correlation in MVP+1)

## Per-runtime version range tested

This baseline reflects the M3 design + intel. M3 acceptance smoke tests
run against synthetic fixtures derived from public docs.

- Claude Code: aligned with docs for the 0.4x series.
- Codex CLI: aligned with the current `[otel]` block surface.
- Cursor Agents: aligned with the public Hooks surface.
- Kimi CLI: aligned with the public reference docs at the time of the M3 baseline.
- Augment Code: aligned with the Analytics REST preview.
- Gemini CLI: aligned with the public telemetry docs.

Each runtime preset crate (`crates/mara-runtime-<runtime>/`) reads
authoritative URLs in its `lib.rs`-level module docs.

## Update workflow

1. Smoke fixtures in `tests/external/<runtime>/` represent the expected payload shape.
2. Adapter / preset code is updated when a runtime ships a payload change.
3. Golden tests regenerate; PR review checks the diff.
4. This matrix updates per release.

## Caveats

- The matrix is implementation-defined; PARTIAL cells include a footnote describing what is and isn't captured.
- ZDR-aware capture is gated on the operator's opt-in plus the runtime's own toggle; PASS for prompt/completion assumes both are enabled.
- Augment as Tier C is best-effort by design; new Augment surfaces will move it up.
