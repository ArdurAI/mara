# Hermes / OpenClaw integration (M2-04)

Use Mara’s LLM HTTP proxy in front of your upstream model server. Point Hermes or OpenClaw at the **Mara listen address** instead of Ollama directly, and set the upstream in `mara.toml` to the real Ollama (or OpenAI-compatible) base URL.

## Sample `mara.toml` fragment

```toml
[server]
metrics_addr = "127.0.0.1:9099"

[[adapters.llm_proxy]]
name = "ollama-via-mara"
http_listen = "127.0.0.1:11435"
upstream = "http://127.0.0.1:11434"
normalizer = "ollama"

[[sinks.file]]
name = "events"
path = "events.jsonl"

[[pipelines.default]]
adapters = ["ollama-via-mara"]
policy_chain = "default"
sinks = ["events"]
```

## Client hints for agent telemetry

Send JSON fields `agent_id`, `step_id`, `tool_name`, `tool_outcome` (or under `metadata.*`) on chat requests; Mara maps them to `mara.*` on canonical events (M2-03). Use `x-mara-request-id` to correlate without W3C `traceparent` (M2-16).

## Reference workflow

1. Start Ollama on `11434`.
2. Run `mara run --config mara.toml`.
3. Configure Hermes/OpenClaw `OLLAMA_HOST` (or equivalent) to `http://127.0.0.1:11435`.
4. Inspect `events.jsonl` for `mara.request_id`, `mara.agent_id`, and `gen_ai.*` fields.

## Metrics vs logs (M3-15)

This path captures **LLM-shaped events** (prompts, completions, tool traffic, latency, token usage when present) through the `llm_proxy` normalizer. It does **not** automatically scrape Hermes/OpenClaw internal metrics (queue depth, scheduler gauges, etc.). If you need those, run a separate OTLP/metrics scrape to another collector or export them from your process alongside Mara.

Experimental **GenAI agent span** attributes (`gen_ai.agent.*`) may appear when exporters opt in; see `docs/observability/gen-ai-agent-spans-experimental.md` and `OTEL_SEMCONV_STABILITY_OPT_IN` in your OTel SDK.
