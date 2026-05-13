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
| `mara.session_id` | One UUID per proxied HTTP exchange |
| `attributes.mara.ollama.*` | Timing breakdown when upstream includes duration fields |
| `resource.host_name` | Best-effort host name from the OS (`hostname` crate) |
| `resource.process_pid` | Mara's process id at event emission time |
| `resource.service_name` | Set when the Mara process has `MARA_SERVICE_NAME` in its environment |

## Common nulls (and why)

| Field | Typical reason |
|-------|----------------|
| `resource.service_name` | `MARA_SERVICE_NAME` not set in the Mara process environment. |
| `resource.service_version` | Not populated yet (binary version wiring planned). |
| `trace_id`, `span_id` | No `traceparent` propagation from the HTTP proxy yet. |
| `gen_ai.request.temperature`, `top_p`, … | Client did not send them (or they live only under `options` / OpenAI-compat top-level keys we did not receive). |
| `gen_ai.tool`, `gen_ai.agent`, `mcp` | Not used for plain Ollama HTTP. |
| `gen_ai.conversation_id`, `mara.turn_id` | Not inferred unless the client sends explicit correlation headers/fields. |
| `body` | Raw prompt/completion capture is **off by default** (privacy / volume). |
| `body_hashes` | Not computed until an explicit capture/redaction policy fills them. |
| `reasoning_tokens` | Not mapped from vendor-specific “thinking” payloads yet. |
| `cost_usd` | Placeholder until pricing tables / policy-backed estimates land. |

## When values appear

- **Request-side tuning** appears when the client JSON includes `model`, `stream`, and either native `options.{...}` or OpenAI-compat top-level fields we parse.
- **Response-side** fields appear when Ollama returns the relevant JSON keys (counters, durations, `done_reason`, etc.).
- **Resource** defaults: `host_name` / `process_pid` are filled by the Ollama normalizer; optional `service_name` via `MARA_SERVICE_NAME`.
- **Resource and trace** extras (version, trace ids) appear once additional enrichment lands (see milestone board).

## How to verify locally

- **Unit guardrails:** `cargo test -p mara-runtime-ollama`
- **CI-style smoke (mock upstream, no real Ollama):** `bash scripts/benchmarks/ollama_proxy_smoke.sh`
- **Long real-world run (needs Ollama + cloud or local model):** `bash scripts/realworld/run-30min-site-research.sh`

## Related docs

- [`ollama-llm-proxy-capabilities.md`](ollama-llm-proxy-capabilities.md) — field matrix and captured test transcripts
- [`milestones/mara-m0-m2-board.md`](milestones/mara-m0-m2-board.md) — roadmap for enrichment and metrics
