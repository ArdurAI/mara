# Runtime parity matrix (M2-01)

This matrix compares **how fully** Mara can populate a shared set of **GenAI / resource fields** for each supported runtime, given the **primary integration path** in this repository today. It is an operator-facing snapshot for M2; it complements the deeper surface-area survey in [`../../plans/01-landscape/08-ai-runtime-telemetry-surfaces.md`](../../plans/01-landscape/08-ai-runtime-telemetry-surfaces.md).

## Required field set (M0-03 + Ollama CI guardrail)

These are the fields Mara treats as **production-critical** for completion-style GenAI work (aligned with `scripts/benchmarks/check_ollama_proxy_events.py` and OTLP normalizer tests):

| # | Field | Notes |
|---|--------|--------|
| R1 | `gen_ai.request.model` | |
| R2 | `gen_ai.response.model` | |
| R3 | `gen_ai.operation.name` | e.g. `chat`, `text_completion` |
| R4 | `gen_ai.usage.input_tokens` | May be absent when upstream only reports totals |
| R5 | `gen_ai.usage.output_tokens` | |
| R6 | `resource.host_name` | |
| R7 | `resource.process_pid` | |

**Extended fields** (tracked for parity but not in the M0-03 minimum set):

| # | Field | Notes |
|---|--------|--------|
| E1 | `trace_id` / `span_id` | W3C context when the client or runtime supplies it |
| E2 | `gen_ai.conversation_id` / `mara.turn_id` | Client JSON or correlation headers (M1-02) |
| E3 | `mara.cost_usd` (estimate) | Needs usage + `[server.gen_ai_pricing]` (M1-04) |

## Integration path (per runtime)

| Runtime | `mara.source.runtime` | Primary Mara path | Secondary / redundant |
|---------|----------------------|-------------------|------------------------|
| **Claude Code** | `claude_code` | OTLP → `mara-adapter-otlp` (`gen_ai.*` from Claude’s exporter) | JSONL tail `~/.claude/projects/**/*.jsonl` |
| **Codex** | `codex` | OTLP → `mara-adapter-otlp` | JSONL `~/.codex/history.jsonl`, optional hooks |
| **Cursor** | `cursor` | Hooks → `mara-adapter-hooks` (normalized to canonical events) | *No* first-party OTLP for the agent loop |
| **Kimi** | `kimi` | JSONL tail (`~/.kimi/logs/kimi.log`, export ZIPs) | OTLP not stable in upstream docs at matrix time |
| **Ollama** | `ollama` | HTTP reverse proxy → `mara-adapter-llm-proxy` + `mara-runtime-ollama` normalizer | Server logs are not used for structured tokens |

Preset crates: `crates/mara-runtime-claude-code`, `mara-runtime-codex`, `mara-runtime-cursor`, `mara-runtime-kimi`, `mara-runtime-ollama`.

## Completeness score (primary path)

Scores are **fractions of R1–R7 populated under “happy path” configuration** (telemetry enabled, JSON where expected, successful completion). They are **engineering estimates** for planning; **measured** fixture scores are in [Measured completeness](#measured-completeness-ci-fixtures-m2-02) below.

| Runtime | Primary path | R1–R7 score | Notes |
|---------|----------------|------------|--------|
| **Ollama** | LLM proxy + normalizer | **7 / 7** | Validated continuously by CI smoke (`check_ollama_proxy_events.py`) for `chat` + `text_completion` rows. |
| **Claude Code** | OTLP | **7 / 7** | When `CLAUDE_CODE_ENABLE_TELEMETRY=1` and semconv-shaped `gen_ai.*` + resource attrs are present (see `mara-adapter-otlp` tests). JSONL tail shape varies; not scored here. |
| **Codex** | OTLP | **6–7 / 7** | Same adapter as Claude; completeness depends on Codex `[otel]` payload. Token fields may be sparse on some operations. |
| **Cursor** | Hooks | **3–5 / 7** | Hook JSON focuses on IDE/agent actions; model and usage lines may be partial vs OTLP-first runtimes. |
| **Kimi** | JSONL | **2–4 / 7** | Debug / export formats are verbose but not aligned 1:1 with `gen_ai.*` yet; expect gaps until Kimi ships stable OTLP. |

### Extended-field expectation (informal)

| Runtime | E1 trace | E2 conversation / turn | E3 cost estimate |
|---------|----------|-------------------------|------------------|
| Claude Code (OTLP) | Often when trace exporter on | When runtime encodes them | When usage present + pricing enabled |
| Codex (OTLP) | Same | Same | Same |
| Cursor (hooks) | Rare | Hook-dependent | Usually needs manual correlation |
| Kimi (JSONL) | Rare | File-dependent | Often missing usage |
| Ollama (proxy) | If client sends `traceparent` | If client JSON / headers (M1-02) | Usage + optional pricing map |

## Measured completeness (CI fixtures, M2-02)

Ubuntu CI runs `scripts/benchmarks/schema_completeness_gate.py` on checked-in canonical **Event** JSONL under `docs/captured/fixtures/`. Each runtime fixture must reach **≥85%** mean per-row fill on the same seven required fields as the Ollama smoke checker; **at least three** runtimes must pass (Cursor remains intentionally sparse until hooks normalization improves).

| Runtime fixture | Qualifying rows | Mean fill % | Pass (≥85%) |
|-----------------|----------------:|------------:|:-------------:|
| `ollama` | 2 | 100.0 | yes |
| `claude_code` | 2 | 100.0 | yes |
| `codex` | 2 | 100.0 | yes |
| `kimi` | 2 | ~92.9 | yes |
| `cursor` | 2 | ~71.4 | no |

Re-run locally:

```bash
python3 scripts/benchmarks/schema_completeness_gate.py
```

## Follow-up: deeper measurement

1. **Broaden fixtures** — Add live-capture samples per runtime under `docs/captured/fixtures/` and extend `schema_completeness_gate.py` when new normalizers land.
2. **Normalize** — Extend runtime-specific parsers (hooks, JSONL) to map into the same `gen_ai.*` slots OTLP already uses (prioritize Cursor hooks to clear the 85% bar).
3. **Report** — Run `scripts/benchmarks/telemetry_quality_report.py` on larger JSONL exports and commit fill-rates under `docs/captured/`.

## Related documentation

- Ollama proxy field behavior: [`../ollama-llm-proxy-capabilities.md`](../ollama-llm-proxy-capabilities.md)
- Null-field operator guide: [`../ollama-null-fields.md`](../ollama-null-fields.md)
- Telemetry quality snapshot: [`../captured/telemetry-quality-from-fixture.md`](../captured/telemetry-quality-from-fixture.md)
