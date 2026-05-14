# Ollama proxy telemetry: null fields and how to interpret them

This note is for **operators** running Mara’s `llm_proxy` in front of Ollama (local or cloud models). It explains why many JSON fields are `null`, what is expected today, and how to get richer data later.

See also: [`ollama-llm-proxy-capabilities.md`](ollama-llm-proxy-capabilities.md).

## What you should expect to be non-null (happy path)

For successful **`/api/generate`** and **`/api/chat`** calls with JSON bodies and JSON responses, Mara typically emits:

| Field | Meaning |
|-------|---------|
| `gen_ai.system` | `"ollama"` |
| `gen_ai.operation_name` | `"text_completion"` or `"chat"` |
| `gen_ai.request.model` | From the **client** JSON (`model`), when the body is parsed |
| `gen_ai.response.model` | From the **upstream** JSON (`model`) |
| `gen_ai.usage.input_tokens` / `output_tokens` | From Ollama counters or OpenAI-style `usage` |
| `gen_ai.usage.total_tokens` | From upstream `usage.total_tokens` or **input + output** |
| `gen_ai.response.finish_reasons` | From `done_reason` or `choices[0].finish_reason` |
| `gen_ai.conversation_id` | When the client supplies correlation (see **Correlation** below). |
| `mara.turn_id` | When the client supplies a turn id (see **Correlation** below). |
| `resource.host_name` | Best-effort host name from the OS (`hostname` crate) |
| `resource.process_pid` | Mara's process id at event emission time |
| `resource.service_name` | From **`[server].telemetry_service_name`** in TOML if set (non-empty), else `MARA_SERVICE_NAME` |
| `resource.service_version` | From **`[server].telemetry_service_version`** if set, else `MARA_SERVICE_VERSION` |

## Common nulls (and why)

| Field | Typical reason |
|-------|----------------|
| `resource.service_name` | Neither `[server].telemetry_service_name` nor `MARA_SERVICE_NAME` set. |
| `resource.service_version` | Neither `[server].telemetry_service_version` nor `MARA_SERVICE_VERSION` set. |
| `trace_id`, `span_id` | No valid W3C **`traceparent`** header on the inbound proxied request (malformed values and all-zero trace ids are rejected). |
| `gen_ai.request.temperature`, `top_p`, … | Client did not send them (or they live only under `options` / OpenAI-compat top-level keys we did not receive). |
| `gen_ai.tool`, `gen_ai.agent`, `mcp` | Not used for plain Ollama HTTP. |
| `gen_ai.conversation_id` | Client did not send `conversation_id` / `metadata.conversation_id` or `X-Mara-Conversation-Id` / `X-Conversation-Id`. |
| `mara.turn_id` | Client did not send `turn_id` / `metadata.turn_id` or `X-Mara-Turn-Id` / `X-Turn-Id`. |
| `body` | Raw prompt/completion capture is **off by default** (privacy / volume). |
| `body_hashes` | Not computed until an explicit capture/redaction policy fills them. |
| `reasoning_tokens` | Not mapped from vendor-specific “thinking” payloads yet. |
| `cost_usd` | `0.0` until **`[server.gen_ai_pricing].estimate_enabled = true`** and token usage is present; see [`ollama-gen-ai-pricing.md`](ollama-gen-ai-pricing.md). |

## When values appear

- **Request-side tuning** appears when the client JSON includes `model`, `stream`, and either native `options.{...}` or OpenAI-compat top-level fields we parse.
- **Response-side** fields appear when Ollama returns the relevant JSON keys (counters, durations, `done_reason`, etc.).
- **Correlation (M1-02):** `gen_ai.conversation_id` and `mara.turn_id` are filled from JSON (`conversation_id`, `turn_id`, or `metadata.*`) when present; otherwise from the correlation headers above.
- **W3C trace context (M1-03):** when the client sends **`traceparent`**, Mara copies the trace and span identifiers onto emitted events for that exchange.
- **Resource** extras: `service_name` / `service_version` from `[server].telemetry_*` or `MARA_SERVICE_*` when set.

## How to verify locally

- **Unit guardrails:** `cargo test -p mara-runtime-ollama`
- **CI-style smoke (mock upstream, no real Ollama):** `bash scripts/benchmarks/ollama_proxy_smoke.sh`
- **Short Ollama Cloud smoke (local daemon + `*-cloud` model, real network):** `bash scripts/realworld/ollama_cloud_smoke.sh`
- **Varied ≥15 min workload (generate/chat/OpenAI-compat, HTTP fetch + summarize, sequential “subagent” calls, optional local dashboard):** `python3 scripts/realworld/ollama_cloud_varied_workload.py` (see `--help`; default `--duration 900`)
- **Long real-world run (needs Ollama + cloud or local model):** `bash scripts/realworld/run-30min-site-research.sh`

## Related docs

- [`ollama-llm-proxy-capabilities.md`](ollama-llm-proxy-capabilities.md) — field matrix and captured test transcripts
- [`ollama-proxy-error-taxonomy.md`](ollama-proxy-error-taxonomy.md) — stable `mara.proxy.failure_kind` codes from the proxy
- [`milestones/mara-m0-m2-board.md`](milestones/mara-m0-m2-board.md) — roadmap for enrichment and metrics
