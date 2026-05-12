# Classic Log Shippers and Collectors

## Executive summary

The classic log-shipping and collector category — Fluentd, Fluent Bit, OpenTelemetry Collector, Vector, Logstash, Filebeat, rsyslog, syslog-ng, Promtail / Grafana Alloy, NXLog, Splunk Universal Forwarder, AWS CloudWatch Agent, Google Ops Agent, Cribl Stream — is the pattern library Mara draws from. None of these tools target AI-runtime telemetry specifically, but their architectural choices, deployment patterns, plugin ABIs, and operational lessons inform Mara's design at every level. This document is a structured tour, organized by tool, with notes on what Mara emulates and what Mara deliberately departs from.

## Tool-by-tool

### Fluentd

- **What it is:** Ruby-based unified logging layer. Pioneer of the "plugin-rich generic log shipper" pattern.
- **Runtime:** Ruby (CRuby). Single agent process. Can also run as a forwarder + aggregator pair.
- **Footprint:** larger than C/Rust agents; tens of MB RSS typical.
- **Inputs:** ~1,000 plugins via RubyGems — tail, syslog, HTTP, TCP, forward (its own protocol), and many more.
- **Outputs:** comparable plugin breadth.
- **Deployment:** systemd, init.d, Docker, Kubernetes DaemonSet historically; less common in K8s now (Fluent Bit took over).
- **License:** Apache 2.0. CNCF graduated.
- **Notable limitations:** Ruby footprint; slower than C/Rust; plugin compatibility matrix drift over years.
- **Recent news:** stable; Fluent Bit is the strategic direction within the Fluent family.
- **Docs:** <https://www.fluentd.org>, <https://github.com/fluent/fluentd>.

**Mara takeaway:** plugin-rich design is correct; choose Rust over Ruby for footprint; emulate the input → filter → output topology with strict types.

### Fluent Bit

- **What it is:** C-based light-weight log + metrics + traces processor and forwarder. The de facto K8s DaemonSet for log shipping.
- **Runtime:** C. Single binary, tiny footprint (~10 MB RSS baseline).
- **Footprint:** very small.
- **Inputs:** ~100 plugins; tail, syslog, systemd, OpenTelemetry, statsd, Prometheus, Kubernetes events.
- **Outputs:** rich — OpenSearch, Loki, S3, Splunk, OTLP, Kafka, many vendors.
- **Filters / processors:** Lua, Wasm (recent), record-modifier, parser.
- **Deployment:** K8s DaemonSet, systemd, init.d, Docker, Windows Service, Lambda Extension, ECS.
- **License:** Apache 2.0. CNCF graduated.
- **Notable limitations:** C-language ergonomics for contributors; plugin authoring requires C or Wasm; complex regex semantics.
- **Recent news:** Wasm filter support; OpenTelemetry receiver/exporter support; ongoing performance work.
- **Docs:** <https://docs.fluentbit.io>, <https://github.com/fluent/fluent-bit>.

**Mara takeaway:** Fluent Bit is Mara's nearest spiritual sibling. Choose Rust for memory safety + ergonomics; emulate the small-footprint and K8s-first deployment posture; differentiate by being AI-runtime-aware and OTel `gen_ai.*`-aligned.

### OpenTelemetry Collector (core + contrib)

- **What it is:** vendor-neutral collector for logs, metrics, traces. Operating in agent and gateway modes.
- **Runtime:** Go. Single binary, modular via build-time configuration.
- **Footprint:** ~30–80 MB RSS depending on receivers/exporters enabled.
- **Inputs (receivers):** OTLP, Jaeger, Zipkin, Prometheus, host metrics, K8s events, syslog, many specific apps via contrib.
- **Outputs (exporters):** OTLP, Loki, Prometheus, OpenSearch, Splunk, vendor-specific.
- **Processors:** batching, attribute manipulation, sampling, redaction, transformations via OTTL.
- **Deployment:** K8s DaemonSet/Deployment/StatefulSet, systemd, container, Lambda, ECS. OpenTelemetry Operator manages K8s deployments.
- **License:** Apache 2.0. CNCF graduated.
- **Notable limitations:** Go memory footprint vs C/Rust; configuration verbosity; complex "receiver/processor/exporter" wiring.
- **Recent news:** OTTL (Transformations Language) maturing; gen_ai semconv now first-class; Operator improvements.
- **Docs:** <https://opentelemetry.io/docs/collector/>, <https://github.com/open-telemetry/opentelemetry-collector>, <https://github.com/open-telemetry/opentelemetry-collector-contrib>.

**Mara takeaway:** OTel Collector is the broader, vendor-neutral platform Mara complements. Mara talks to it over OTLP; Mara does NOT replace it. Mara's smaller scope = narrower seat.

### Vector

- **What it is:** Rust-based collector by Datadog (via Timber acquisition). High-performance pipeline tool.
- **Runtime:** Rust. Single binary.
- **Footprint:** small (~30 MB RSS); high throughput.
- **Inputs:** ~50 sources — file, syslog, statsd, Kafka, OTLP, vector itself, journald, AWS Kinesis, many more.
- **Outputs:** ~80 sinks — Datadog, Elasticsearch, Loki, OTLP, Kafka, S3, Splunk, many more.
- **Transforms:** VRL (Vector Remap Language) — a strict typed DSL for transformations.
- **Deployment:** K8s, systemd, Docker, Lambda, Windows.
- **License:** MPL 2.0.
- **Notable limitations:** VRL learning curve; vendor-led (Datadog); roadmap can shift with corporate priorities.
- **Recent news:** ongoing performance work; continued sink expansion.
- **Docs:** <https://vector.dev>, <https://github.com/vectordotdev/vector>.

**Mara takeaway:** Vector proves Rust at this scale works. Mara's smaller scope means it can be more opinionated about AI-runtime presets while leaning on Vector's lessons for performance.

### Logstash

- **What it is:** JRuby-based log processor; classic ELK stack member.
- **Runtime:** JVM (JRuby).
- **Footprint:** hundreds of MB RSS baseline.
- **Inputs / outputs:** plugin ecosystem, primarily Elasticsearch-oriented.
- **Deployment:** systemd, container; less common in K8s now (Filebeat / Fluent Bit replace it for ingest, Logstash sometimes used as aggregator).
- **License:** Elastic License v2 / SSPL (was Apache 2.0).
- **Notable limitations:** JVM footprint; license shift in 2021; Beats family superseded it for ingest.
- **Docs:** <https://www.elastic.co/logstash>.

**Mara takeaway:** Logstash's footprint is a cautionary tale for "agent on every node"; informs Mara's choice to be a small Rust binary.

### Filebeat (Elastic Beats family)

- **What it is:** Go-based shipper for files (Beats family includes Metricbeat, Auditbeat, Heartbeat, etc.).
- **Runtime:** Go. Single binary.
- **Footprint:** small.
- **Inputs:** files, modules for common services (nginx, MySQL, etc.).
- **Outputs:** Elasticsearch, Logstash, Kafka, Redis.
- **Deployment:** systemd, container, Windows Service.
- **License:** Elastic License v2 / SSPL.
- **Notable limitations:** license; tight coupling to Elasticsearch ecosystem.
- **Docs:** <https://www.elastic.co/beats/filebeat>.

**Mara takeaway:** Filebeat's "small dedicated shipper" pattern is right; Mara generalizes to a broader sink ecosystem.

### rsyslog

- **What it is:** GPL'd syslog daemon, common on Linux.
- **Runtime:** C.
- **Footprint:** very small.
- **Inputs:** syslog (RFC 3164/5424), files, journal, plugins.
- **Outputs:** files, syslog forwarding, Elasticsearch, Kafka, ZMQ, MongoDB.
- **License:** GPL 3.0 + commercial.
- **Docs:** <https://www.rsyslog.com>.

**Mara takeaway:** syslog interop is a v1.x sink; rsyslog inspires the modest-resource-use philosophy.

### syslog-ng

- **What it is:** another open-source syslog daemon.
- **Runtime:** C.
- **License:** GPL 2.0 + commercial.
- **Docs:** <https://www.syslog-ng.com>.

### Promtail / Grafana Alloy

- **What it is:** Promtail historically shipped logs to Loki. Grafana Alloy is the successor that combines Promtail + Grafana Agent into one binary.
- **Runtime:** Go. Single binary.
- **License:** Promtail under AGPL 3.0 (2024 relicense); Grafana Alloy under Apache 2.0.
- **Docs:** <https://grafana.com/docs/alloy/>.

**Mara takeaway:** Grafana Alloy's Apache 2.0 status post-Promtail-relicense is the model for distinguishing what is freely licensed vs vendor-protected; informs Mara's clean Apache 2.0 stance.

### Splunk Universal Forwarder + Heavy Forwarder

- **What it is:** lightweight (UF) and full-featured (HF) closed-source agents for Splunk.
- **Runtime:** C++.
- **Footprint:** UF is small; HF heavier.
- **Inputs:** files, syslog, scripts, HTTP Event Collector forwarding.
- **Outputs:** Splunk-receive endpoints (TCP/9997 or HEC).
- **License:** proprietary.
- **Docs:** <https://docs.splunk.com/Documentation/Forwarder>.

**Mara takeaway:** UF is the canonical "enterprise edge shipper"; Splunk HEC remains a v1 Mara sink target.

### NXLog (Community + Enterprise)

- **What it is:** cross-platform log collection tool, strong on Windows event logs.
- **Runtime:** C. Single binary.
- **License:** Community = Apache 2.0; Enterprise = proprietary.
- **Docs:** <https://nxlog.co>.

### AWS CloudWatch Agent

- **What it is:** AWS-published agent that ships logs and metrics to CloudWatch.
- **Runtime:** Go.
- **License:** MIT.
- **Docs:** <https://docs.aws.amazon.com/AmazonCloudWatch/latest/monitoring/Install-CloudWatch-Agent.html>.

### Google Ops Agent

- **What it is:** Google Cloud's agent for sending logs and metrics to Cloud Logging and Cloud Monitoring.
- **Runtime:** Fluent Bit (logs) + OpenTelemetry Collector (metrics) under the hood.
- **License:** Apache 2.0.
- **Docs:** <https://cloud.google.com/logging/docs/agent/ops-agent>.

**Mara takeaway:** Google Ops Agent shows the trend of cloud-provider agents being thin wrappers around Fluent Bit + OTel Collector; reaffirms Mara's strategic relevance as an AI-specialist alongside these generalists.

### Datadog Agent

- **What it is:** Datadog's Go agent for shipping host metrics, logs, APM, and many integrations to Datadog Cloud.
- **License:** Apache 2.0 (open core; many integrations open, some proprietary).
- **Docs:** <https://docs.datadoghq.com/agent/>.

### Cribl Stream / Edge

- **What it is:** observability data pipeline; routes/transforms before fan-out.
- **License:** proprietary; free tier with limits.
- **Docs:** <https://cribl.io>.

**Mara takeaway:** Cribl's "routing/transform between sources and destinations" is similar to what OTel Collector + Vector do open-source; Mara stays on the open side.

## What an AI-native shipper can learn from this category

1. **Single statically-linked binary** is the right form factor for edge agents.
2. **Bounded queues + WAL** is the right durability model.
3. **Plugin or modular receivers / exporters** scale the project sustainably.
4. **Configuration-as-data (TOML / YAML / HCL)** beats configuration-as-code for operators.
5. **DSL for transforms** (VRL, OTTL) is powerful but adds learning curve; Mara picks WASM (polyglot) + typed primitives.

## What Mara explicitly diverges on

1. **AI-runtime-specific presets** as core, not as an integration.
2. **Canonical schema aligned to OTel `gen_ai.*`** rather than untyped events.
3. **Policy as code with signed bundles** as core, not as an add-on.
4. **Smaller surface area** — narrower than Fluent Bit / OTel Collector / Vector; we don't aim to be all things.

## Comparative footprint targets

- Fluent Bit baseline: ≈ 10–15 MB RSS.
- OTel Collector: ≈ 30–80 MB RSS.
- Vector: ≈ 30 MB RSS.
- Logstash: ≈ 200–500 MB RSS.
- Splunk UF: ≈ 30–50 MB RSS.
- **Mara v1 target:** ≤ 128 MB RSS idle, ≤ 512 MB at SLO load. We're allowing more headroom than Fluent Bit because we're carrying WASM hosting and a more typed model; we beat Logstash easily.

## References

- CNCF landscape: <https://landscape.cncf.io>.
- "Observability Whitepaper" (CNCF): <https://github.com/cncf/tag-observability/blob/main/whitepaper.md>.
- Fluentd / Fluent Bit comparison: <https://docs.fluentbit.io/manual/about/fluentd-and-fluent-bit>.
- OTel Collector design: <https://opentelemetry.io/docs/collector/architecture/>.
- Vector design: <https://vector.dev/docs/about/concepts/>.
