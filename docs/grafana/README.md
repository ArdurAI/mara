# Grafana — Mara starter dashboards

## `mara-gen-ai-starter.json`

Import into Grafana (**Dashboards → New → Import → Upload JSON file**). Pick your Prometheus datasource when prompted.

Assumes Mara’s self-telemetry endpoint is scraped (default bind from `[server].metrics_addr`, usually `http://127.0.0.1:9099/metrics`). See [M1-05 self-metrics](../observability/mara-self-metrics-m1-05.md).

Panels use metrics emitted by `mara run` (M1-05 + M2 dual-latency / fan-out):

- Request rate — `rate(mara_gen_ai_requests_completed_total[5m])`
- Error rate — `rate(mara_gen_ai_requests_failed_total[5m])`
- Token throughput — `rate(mara_gen_ai_input_tokens_total[5m])`, `rate(mara_gen_ai_output_tokens_total[5m])`
- Latency — gauge `mara_gen_ai_request_duration_ms_p95` or histogram `mara_gen_ai_request_duration_ms_bucket` (engine path)
- Gateway vs dispatcher — gauges `mara_gen_ai_gateway_duration_ms_p95`, `mara_pipeline_fanout_duration_ms_p95` (proxy / fan-out wall time)
- Sink pressure — `rate(mara_pipeline_sink_channel_send_errors_total[5m])` (non-zero implies closed sink channels during fan-out)
- Cost — `rate(mara_gen_ai_cost_micro_usd_total[5m]) / 1e6` (USD per second, approximate); low-confidence completions — `mara_gen_ai_cost_low_confidence_completions_total`

`/readyz` behavior is documented in [Mara `/readyz` semantics](../observability/mara-readyz-semantics.md).
