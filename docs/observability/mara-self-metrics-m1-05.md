# M1-05 — Self-metrics (`/metrics`) and derived GenAI counters

When you run `mara run`, Mara binds **`[server].metrics_addr`** (default `127.0.0.1:9099`) and serves:

- **`GET /metrics`** — Prometheus text exposition (derived from events **after** the policy chain is applied).
- **`GET /healthz`** — plain `ok` when the process is up.
- **`GET /readyz`** — aggregate adapter/sink readiness (200 vs 503). Semantics: [Mara `/readyz` semantics](mara-readyz-semantics.md).

## Metric families (per `pipeline` label)

| Metric | Type | Meaning |
| --- | --- | --- |
| `mara_pipeline_events_delivered_total` | counter | All events delivered to sinks after policy. |
| `mara_gen_ai_requests_completed_total` | counter | `Completion` events with `gen_ai.operation_name` in `chat` / `text_completion` / `embeddings` and Ollama/proxy signals (`source_runtime=ollama`, `gen_ai.system=ollama`, or `llm-proxy` scope). |
| `mara_gen_ai_requests_failed_total` | counter | Proxy-style `Error` events with `http.status_code` or `mara.proxy.failure_kind`. |
| `mara_gen_ai_input_tokens_total` / `mara_gen_ai_output_tokens_total` | counter | Sums of `gen_ai.usage.*_tokens` on counted completions. |
| `mara_gen_ai_cost_micro_usd_total` | counter | Sum of `mara.cost_usd × 1e6` (integer micro-dollars). |
| `mara_gen_ai_request_duration_ms_p95` | gauge | In-process p95 over recent `mara.ollama.total_duration_ms` samples (bounded window). |
| `mara_gen_ai_request_duration_ms_*` | histogram | Classic cumulative buckets on the same latency field. |
| `mara_gen_ai_gateway_duration_ms_p95` | gauge | Approximate p95 from buckets on `mara.proxy.gateway_duration_ms` (LLM proxy wall time). |
| `mara_gen_ai_gateway_duration_ms_*` | histogram | Cumulative buckets for gateway latency. |
| `mara_pipeline_fanout_duration_ms_p95` | gauge | Approximate p95 dispatcher sink fan-out wall time (ms). |
| `mara_pipeline_fanout_duration_ms_*` | histogram | Cumulative buckets for fan-out wall time. |
| `mara_pipeline_sink_channel_send_errors_total` | counter | Sink `mpsc` closed during fan-out (`send` errors). |
| `mara_gen_ai_cost_low_confidence_completions_total` | counter | Completions tagged with low cost confidence. |

Latency semantics (`_sum`, `_count`, `histogram_quantile`, p95 gauge, and the 4096-sample ring vs cumulative buckets) are documented in [`mara-self-metrics-latency-histogram.md`](mara-self-metrics-latency-histogram.md) (M1-11).

## PromQL snippets

```promql
sum by (pipeline) (rate(mara_gen_ai_requests_completed_total[5m]))
sum by (pipeline) (rate(mara_gen_ai_requests_failed_total[5m]))
sum by (pipeline) (rate(mara_gen_ai_input_tokens_total[5m]))
histogram_quantile(0.95, sum by (pipeline, le) (rate(mara_gen_ai_request_duration_ms_bucket[5m])))
sum by (pipeline) (rate(mara_gen_ai_cost_micro_usd_total[5m])) / 1e6
max by (pipeline) (mara_gen_ai_gateway_duration_ms_p95)
max by (pipeline) (mara_pipeline_fanout_duration_ms_p95)
sum(rate(mara_pipeline_sink_channel_send_errors_total[5m]))
sum(rate(mara_gen_ai_cost_low_confidence_completions_total[5m]))
```

## Starter dashboard

See [`docs/grafana/mara-gen-ai-starter.json`](../grafana/mara-gen-ai-starter.json) and [`docs/grafana/README.md`](../grafana/README.md).
