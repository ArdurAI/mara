# Eval platform OTLP export profile (M2-20)

Mara is a **telemetry capture and export** layer, not an eval UI. Use this checklist when piping Mara OTLP into LangSmith, Phoenix, Honeycomb, or similar:

1. **Enable OTLP sink** — `[[sinks.otlp]]` with your collector or SaaS OTLP endpoint.
2. **Preserve `gen_ai.*`** — required for model/op/token charts.
3. **Preserve `mara.request_id` + `trace_id`** — join proxy logs to traces.
4. **Map `mara.cost.confidence`** — gate chargeback panels when `low`.
5. **Latency split** — use `mara.proxy.gateway_duration_ms` vs `mara.ollama.total_duration_ms` (Prometheus: `mara_gen_ai_gateway_duration_ms_*` vs `mara_gen_ai_request_duration_ms_*`).

Honeycomb: derive a service name from `resource.service.name` or `mara.source.adapter`. LangSmith: prefer native OpenAI tracing where available; Mara fills gaps for Ollama and mixed runtimes.
