# Ollama LLM proxy: what Mara records today

This page summarizes **what the HTTP reverse proxy + Ollama normalizer** put into canonical `Event` JSON, why some fields stay `null`, and how to **verify behavior locally** using the checked-in test output.

## What gets populated (high level)

| Area | Filled when… |
|------|----------------|
| **`gen_ai.request`*** | The **client** POST body is JSON (native `/api/generate` / `/api/chat` with `options`, or OpenAI-compat with top-level `temperature`, `max_tokens`, etc.). Truncated bodies are skipped. |
| **`gen_ai.response.model`**, **usage** | The **upstream** JSON includes Ollama counters (`prompt_eval_count`, `eval_count`) or OpenAI-style `usage` / `choices`. |
| **`gen_ai.response.finish_reasons`** | Upstream includes `done_reason` (native) or `choices[0].finish_reason` (compat). |
| **`gen_ai.usage.total_tokens`** | `usage.total_tokens` in the response, or **input + output** when both are known. |
| **`attributes` (`mara.ollama.*`)** | Durations in nanoseconds are present in the upstream JSON (`total_duration`, `load_duration`, …). |
| **`mara.session_id`** | One UUID per proxied HTTP exchange (correlation id for that call). |
| **`resource.host_name` / `resource.process_pid`** | Set on every Ollama-normalized event (host + Mara's PID). Optional `resource.service_name` via `MARA_SERVICE_NAME`. |

\*Request-side fields that are **not** in the client JSON (or are under different keys we do not parse yet) remain unset.

## Fields that often stay `null` (by design or not wired yet)

- **`resource.service_name`**: unless `MARA_SERVICE_NAME` is set (config wiring planned).
- **`resource.service_version`**: not wired yet.
- **`trace_id` / `span_id`**: no W3C `traceparent` propagation from the proxy yet.
- **`body`**, **`body_hashes`**: raw prompt/completion capture is off by default (privacy / volume).
- **`mcp`, `tool`, `agent`, `conversation_id`**: unused for plain Ollama HTTP.
- **`reasoning_tokens`**: not mapped unless we add a stable field from Ollama/OpenAI payloads.
- **`cost_usd`**: placeholder `0` with `mara_estimated` until pricing hooks exist.

## How to re-run the tests

```bash
# Ollama normalizer unit tests (request + response parsing, errors, OpenAI-compat)
cargo test -p mara-runtime-ollama

# Workspace integration tests (config + JSONL pipeline)
cargo test
```

Captured outputs from a clean run are stored under **`docs/captured/`** (see below).

## Captured test results (checked in)

The following transcripts were produced with **`cargo test -p mara-runtime-ollama`** and **`cargo test`** respectively. They are meant as a **snapshot** so readers can see exactly which tests passed without running Cargo.

### `mara-runtime-ollama` (10 tests)

```
running 10 tests
test tests::linux_reconfig_uses_systemd_override ... ok
test tests::default_ports_split_conventional_and_upstream ... ok
test normalizer::tests::upstream_503_records_http_status ... ok
test tests::macos_reconfig_uses_launchctl ... ok
test normalizer::tests::proxy_synthetic_502_records_failure_kind ... ok
test tests::runtime_id_is_stable ... ok
test normalizer::tests::fills_generate_request_from_client_json_and_response_meta ... ok
test normalizer::tests::parses_native_chat_counters ... ok
test normalizer::tests::fills_openai_compat_request_top_level_tuning ... ok
test normalizer::tests::parses_openai_compat_chat_usage ... ok

test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

Full copy: [`docs/captured/mara-runtime-ollama-tests.txt`](captured/mara-runtime-ollama-tests.txt)

Normalizer-focused tests demonstrate, among other things:

- **`fills_generate_request_from_client_json_and_response_meta`** — client `model` / `options` / `stream` plus response `done_reason`, token counts, and derived `total_tokens`.
- **`fills_openai_compat_request_top_level_tuning`** — OpenAI-style request fields and `choices[0].finish_reason`.
- **`parses_native_chat_counters`** / **`parses_openai_compat_chat_usage`** — usage and `event_kind` for chat-style completions.

### Workspace `e2e_pipeline` (3 tests)

```
running 3 tests
test config_loads_minimal_pipeline ... ok
test config_loads_llm_proxy_pipeline ... ok
test e2e_jsonl_to_file_with_pii_redaction ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.04s
```

Full copy: [`docs/captured/workspace-tests.txt`](captured/workspace-tests.txt)

## CI smoke (mock upstream)

Ubuntu CI runs `scripts/benchmarks/ollama_proxy_smoke.sh` (mock Ollama + `mara` + JSONL checks). No real Ollama daemon required.

## Operator null-field guide

See [`ollama-null-fields.md`](ollama-null-fields.md).

## Related code

- Normalizer: `crates/mara-runtime-ollama/src/normalizer.rs`
- Proxy capture: `crates/mara-adapter-llm-proxy/src/http_proxy.rs`, `exchange.rs`
- Smoke harness: `scripts/benchmarks/ollama_proxy_smoke.sh`, `scripts/benchmarks/mock_ollama_upstream.py`, `scripts/benchmarks/check_ollama_proxy_events.py`
- Long-run harness: `scripts/realworld/run-30min-site-research.sh`
