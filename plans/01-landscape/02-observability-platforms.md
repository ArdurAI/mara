# Observability Platforms — Sink Targets for Mara

## Executive summary

This document surveys the commercial and open-source observability platforms Mara must interoperate with as sinks. Mara does not compete with any of these — they are downstream destinations for Mara's canonical events. The relevant questions per platform are: what protocol does it accept (OTLP, HEC, Loki HTTP, Elasticsearch bulk, proprietary), what AI/LLM observability features it has, what its license and pricing posture is, and where Mara's normalization adds value over native instrumentation.

## Commercial SaaS (proprietary backends)

### Splunk

- **Description:** the original enterprise log analytics platform; now also a metrics + traces backend.
- **Ingestion:** HEC (HTTP Event Collector), syslog forwarder, Universal Forwarder via TCP/9997, OTLP via the OTel collector contrib's Splunk exporter, OpenTelemetry Splunk Distribution.
- **Signals:** logs, metrics, traces, RUM. AI/LLM: integrations for OpenAI, Anthropic, Bedrock via Splunk's AI/ML toolkit.
- **License + hosting:** proprietary. Splunk Enterprise (self-host), Splunk Cloud (SaaS).
- **Pricing:** historically per-GB ingested; "workload pricing" introduced to address volume cost. Expensive at scale for verbose LLM data.
- **AI/LLM features:** dashboards in the AI/ML toolkit; not a first-class gen-ai semconv consumer.
- **Mara sink:** `splunk_hec` is a v1 sink.
- **Docs:** <https://docs.splunk.com>.

### Datadog

- **Description:** unified observability SaaS; logs, metrics, traces, APM, RUM, security, LLM Obs.
- **Ingestion:** Datadog Agent, OTLP (via Datadog Agent and direct), Datadog API.
- **Signals:** all of the above plus first-party LLM Observability product.
- **License + hosting:** proprietary SaaS.
- **Pricing:** per-host + per-GB + per-feature. Expensive at scale for LLM traffic.
- **AI/LLM features:** LLM Observability product with auto-instrumentation for OpenAI, Anthropic, Bedrock, LangChain. Tracks cost, latency, error rates.
- **Mara sink:** OTLP via Datadog Agent's OTLP receiver (production path).
- **Docs:** <https://docs.datadoghq.com>.

### New Relic

- **Description:** application performance management + observability SaaS.
- **Ingestion:** New Relic agents per language, OTLP, API.
- **Signals:** APM, browser, mobile, infrastructure, logs, AI Monitoring.
- **License + hosting:** proprietary SaaS.
- **Pricing:** user-based + per-GB.
- **AI/LLM features:** AI Monitoring includes prompts, completions, costs, evals.
- **Mara sink:** OTLP.
- **Docs:** <https://docs.newrelic.com>.

### Sumo Logic

- **Description:** SaaS log analytics + SIEM-adjacent.
- **Ingestion:** collectors + HTTP source + OTLP.
- **Signals:** logs primarily; metrics, traces support.
- **License + hosting:** proprietary SaaS; self-host enterprise option.
- **Pricing:** per-GB.
- **AI/LLM features:** Cloud SIEM with detection rules; gen-AI dashboards in beta.
- **Mara sink:** HTTP source / OTLP.
- **Docs:** <https://help.sumologic.com>.

### Honeycomb

- **Description:** events-and-traces-centric observability SaaS.
- **Ingestion:** OTLP (HTTP + gRPC).
- **Signals:** events, traces; some metrics (via OTLP). High-cardinality friendly.
- **License + hosting:** proprietary SaaS.
- **Pricing:** per-event.
- **AI/LLM features:** dedicated AI views; native consumer of `gen_ai.*` semconv.
- **Mara sink:** OTLP-HTTP highly recommended.
- **Docs:** <https://docs.honeycomb.io>.
- **Why Mara → Honeycomb is high fidelity:** Honeycomb's schema-on-read model means Mara's full canonical events are queryable without information loss.

### Dynatrace

- **Description:** enterprise observability + AIOps SaaS.
- **Ingestion:** OneAgent + OTLP.
- **License + hosting:** proprietary SaaS.
- **Pricing:** consumption-based.
- **AI/LLM features:** dashboards; some gen-ai semconv awareness.
- **Mara sink:** OTLP.
- **Docs:** <https://docs.dynatrace.com>.

### Logz.io

- **Description:** managed ELK / OpenSearch + tracing on Jaeger + Prometheus metrics.
- **Ingestion:** standard agents + OTLP.
- **License + hosting:** proprietary SaaS.
- **Pricing:** per-GB + tiered.
- **Mara sink:** OTLP / Elasticsearch bulk.

### Better Stack (formerly Logtail)

- **Description:** logs + uptime monitoring SaaS.
- **Ingestion:** HTTP, syslog, OTLP.
- **Pricing:** per-GB.
- **Mara sink:** OTLP / webhook.

### Axiom

- **Description:** large-scale log storage with high-cardinality query.
- **Ingestion:** OTLP, HTTP.
- **Pricing:** per-GB ingested, generous query allowance.
- **AI/LLM features:** AI-friendly schema; query Engine handles high-cardinality.
- **Mara sink:** OTLP / native HTTP.

### Coralogix

- **Description:** logs, metrics, traces with streaming analytics.
- **Mara sink:** OTLP.

### Mezmo (formerly LogDNA)

- **Description:** log management SaaS.
- **Mara sink:** native HTTP / OTLP.

## Open-source / self-hostable

### Grafana stack: Loki + Tempo + Mimir + Pyroscope + Grafana

- **License:** AGPL 3.0 (post-2024 relicense from Apache 2.0).
- **Components:**
  - Loki — logs.
  - Tempo — traces.
  - Mimir — metrics (Prometheus-compatible).
  - Pyroscope — continuous profiling.
  - Grafana — UI.
- **Ingestion:** OTLP (all components), Loki HTTP push, Prometheus remote write, Grafana Alloy as collector.
- **Self-host:** yes; Grafana Cloud is the SaaS version.
- **AI/LLM features:** generic; with `gen_ai.*` attributes the dashboards are first-class.
- **Mara sinks:** `loki`, `otlp` (Tempo), `prom_rw` (Mimir).

### Elasticsearch + Kibana

- **License:** Elastic License v2 / SSPL / AGPL (since 2024 a triple license).
- **Ingestion:** bulk API, Logstash, Beats, OTLP via APM/OTel.
- **Self-host:** yes; Elastic Cloud is SaaS.
- **Mara sink:** `elasticsearch` (Bulk API).

### OpenSearch + OpenSearch Dashboards

- **License:** Apache 2.0 (AWS-led fork of Elasticsearch).
- **Ingestion:** identical to Elasticsearch (compatible APIs).
- **Mara sink:** the same `elasticsearch` sink works.

### Signoz

- **Description:** open-source Datadog alternative; OTel-native.
- **License:** MIT.
- **Ingestion:** OTLP.
- **Self-host:** primary mode.
- **Mara sink:** OTLP.

### Hyperdx

- **Description:** open-source observability platform; OTel-aligned.
- **License:** MIT.
- **Ingestion:** OTLP.
- **Mara sink:** OTLP.

### Highlight.io

- **Description:** open-source observability with session replay.
- **License:** Apache 2.0.
- **Ingestion:** OTLP + SDK.
- **Mara sink:** OTLP.

### ClickHouse-based stacks

- **Description:** raw ClickHouse + Vector / OTel + Grafana as a custom stack.
- **License:** Apache 2.0 (ClickHouse).
- **Ingestion:** SQL inserts, Vector, OTel.
- **Mara sink:** could be added via a ClickHouse sink (v1.x candidate).

### Logfire (Pydantic)

- **Description:** OpenTelemetry-aligned observability product for Python apps.
- **License:** proprietary SaaS + open-source SDK (MIT).
- **Ingestion:** OTLP.
- **AI/LLM features:** native `gen_ai.*` support.
- **Mara sink:** OTLP.

## SIEM-adjacent platforms (relevant for security/compliance use cases)

### Microsoft Sentinel

- **Ingestion:** Azure Monitor, syslog, Common Event Format, custom.
- **Mara sink:** webhook → Sentinel data collector API, or via Azure Monitor agent / OTLP collector.

### Google Chronicle (now Google SecOps)

- **Ingestion:** Forwarder agent, Pub/Sub, direct ingestion API.
- **Mara sink:** webhook / Pub/Sub via custom sink (v1.x).

### Elastic Security

- **Same Elasticsearch base.**

### Splunk Enterprise Security

- **Same Splunk base.**

## Pricing-aware sink prioritization for AI workloads

Verbose LLM telemetry (full prompts, completions, raw API bodies) can blow up per-GB budgets fast. Recommendations:

- For high-volume capture, ship to **object storage (Parquet)** for raw retention and to a smaller-volume **searchable backend** for the curated subset.
- Use Mara's policy chain to sample and route: heavy events to S3, summary events to Datadog/Splunk/Honeycomb.
- Loki + S3-backed object retention is a cost-effective pattern.

## Sink prioritization for an AI-native shipper (Mara v1 ten)

1. **OTLP** (HTTP + gRPC) — covers Honeycomb, Datadog, New Relic, Tempo, Signoz, Hyperdx, Phoenix, Logfire, and any future OTel-aligned backend in one sink.
2. **Loki HTTP** — for Grafana stack.
3. **Splunk HEC** — for Splunk Enterprise / Cloud.
4. **Elasticsearch Bulk** — for Elastic / OpenSearch.
5. **Object store (S3/GCS/Azure Blob)** — for archive and Parquet analytics.
6. **Kafka** — for enterprise data spines.
7. **Prometheus Remote Write** — for derived metrics.
8. **File** — for local debugging.
9. **Stdout** — for debug.
10. **Webhook** — generic catch-all.

This set covers ≈ 95% of operator destinations. The remaining 5% (Sentinel, Chronicle, vendor-specific APIs) is post-v1 via community sinks or webhooks with templates.

## Schema fidelity tier by sink

- **High fidelity (no information loss):** OTLP-shaped sinks (any OTel-aligned backend), Object Store (Parquet preserves all fields), Kafka with JSON or Protobuf.
- **Medium fidelity (label/index constraints):** Loki (labels are a stable small set; rest is structured metadata), Elasticsearch (mapped index template).
- **Lower fidelity (schema-shaped):** Splunk HEC (event JSON + small fields list), Prometheus Remote Write (metrics-only).

## Mara as the neutralizer

Mara's value as a neutral input layer: operators can switch sinks without changing their telemetry source. Move from Datadog to Honeycomb? Reconfigure the sink, schema doesn't change. Adding a parallel S3 archive? Add a sink, existing path unchanged. This is exactly what bare OTel SDKs in app code don't give you — you'd have to rewire each app's SDK exporter.

## References

- CNCF landscape (observability category): <https://landscape.cncf.io/category=observability>.
- OTel collector exporter documentation per backend: <https://github.com/open-telemetry/opentelemetry-collector-contrib/tree/main/exporter>.
- Honeycomb on `gen_ai.*` adoption: <https://honeycomb.io/blog>.
- Grafana Labs licensing: <https://grafana.com/licensing>.
- Datadog LLM Observability: <https://docs.datadoghq.com/llm_observability/>.
