# Non-goals

## Executive summary

Mara's leverage comes from doing one thing extremely well: collecting, normalizing, governing, and routing AI-workload telemetry from where the workload runs. Everything else is explicitly out of scope. This document is the canonical list of things we are not building, so the team can say "no" with confidence and the community can route those needs to better-suited tools.

## v1 non-goals

### Not a backend store
Mara does not persist telemetry for query. It buffers (WAL) for durability and routes to user-chosen backends (Loki, Splunk, Elasticsearch, S3, Datadog, Honeycomb, etc.). If you want long-term storage, choose a sink that does that.

### Not a query UI or dashboard
There is no Mara web UI in v1. CLI introspection (`mara diag`, `mara test pipeline`) is the only first-party surface. Visualization is delegated to the sink (Grafana, Splunk, Datadog, Loki Logs UI).

### Not a generic host-metrics agent
Mara does not collect host CPU, memory, network, or disk metrics. We focus on AI-workload signals. If you need host metrics, run `node_exporter`, `vector`, the OTel Collector host metrics receiver, or your cloud provider's agent alongside Mara.

### Not an APM tracer for non-AI code
Mara does not auto-instrument arbitrary application code. The OTel SDKs do that. Mara receives OTLP from your app code and forwards or transforms it, but it is not in the auto-instrumentation business.

### Not an eval or benchmark harness
Mara emits the data that eval tools consume. It does not run evals itself. Inspect AI, DeepEval, Phoenix evals, Braintrust evals, and LangSmith evals fill that gap.

### Not a prompt-injection firewall or runtime guardrail
Mara observes; it does not block prompt-injection attempts at inference time. Lakera Guard, Pillar Security, Prompt Security, NVIDIA NeMo Guardrails, and Llama Guard are guardrail products. Mara can ingest signals from those tools and ship them, but Mara itself is not a guardrail.

### Not an inference proxy
Mara does not sit in the synchronous request path of an LLM call. Tools like LiteLLM, Portkey, OpenRouter, and Helicone do. Mara collects telemetry from those proxies via OTLP or webhook; it does not replace them.

### Not a configuration management agent
Mara does not push configuration to AI runtimes. We do not modify `~/.codex/config.toml` or Claude Code settings. We read what's there and ship what those tools emit. If you want to push config, use Ansible/Chef/Puppet/Salt or each runtime's enterprise management surface.

### Not a feature flag or experimentation system
LaunchDarkly, Statsig, GrowthBook fill that gap.

### Not a SIEM
Mara can ship to a SIEM (Splunk ES, Microsoft Sentinel, Chronicle, Elastic Security) but is not a detection or correlation engine. Detection rules belong in the SIEM, not in Mara's policy stage. Policies in Mara are for shaping (redact, sample, route), not for alerting.

### Not a cost-allocation FinOps tool
Mara emits per-call cost telemetry with `gen_ai.usage.*` attributes. It does not produce monthly invoices, chargebacks, or budget alerts. Vantage, CloudZero, Finout, OpenCost, and Krateo cover that.

## Probable v2/v3 considerations (still v1 non-goals)

- A managed control plane.
- A managed sink (Mara as a hosted observability backend) — unlikely; the value is in being neutral, not in becoming another sink.
- A policy marketplace (likely v2, with signed bundles).
- A query gateway (read path) — unlikely; let the sinks do their job.

## Adjacent ecosystems we explicitly defer to

- **OpenTelemetry Collector** — for non-AI signal routing, host metrics, infra telemetry. Mara complements, does not replace.
- **Vector / Fluent Bit / Promtail** — for generic log shipping. Mara focuses on the AI signal classes.
- **OPA / Cedar** — for organizational authorization. Mara embeds OPA as a policy plugin but does not try to be OPA.
- **Sigstore / SLSA** — for supply chain attestation. Mara is a consumer and producer, not the framework itself.

## When to use Mara vs. another tool

- You're shipping generic container logs to Elasticsearch → use Fluent Bit. Mara is overkill.
- You're instrumenting a Python web app with OTel → use OTel SDKs. Mara is downstream.
- You're running Claude Code locally and want session telemetry into Grafana → use Mara.
- You're running an agent that calls 4 LLM vendors, makes MCP tool calls, and you need normalized cost + latency in your existing SIEM → use Mara.
- You're a regulated industry shipping AI-agent activity to a tamper-evident audit log → use Mara.

## How to propose adding a non-goal back as a goal

1. Open an issue with the use case.
2. Demonstrate that no upstream tool covers it.
3. Show that fitting it into Mara wouldn't compromise the operating principles in [`01-mission-and-scope.md`](01-mission-and-scope.md).
4. Open an ADR under [`../../docs/adr/`](../../docs/adr/) proposing the scope expansion.
