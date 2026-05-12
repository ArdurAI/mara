# Positioning Statement

## Executive summary

Mara is positioned as the **open-source, AI-native, edge-first telemetry agent** for AI agents and LLM workloads — the missing piece between the AI runtime (Claude Code, Codex, Cursor, Kimi, Augment, Gemini) and whatever observability backend the operator already runs. Mara competes on portability, vendor neutrality, governance, and being designed for AI signals from day one rather than retrofitted onto a generic log pipeline.

## One-line positioning

> Mara is to AI agents what Fluent Bit is to container logs: a small, fast, edge-deployable Rust binary that collects, normalizes, governs, and ships AI workload telemetry to wherever you already store observability data.

## Positioning against adjacent categories

### vs. classic log shippers (Fluent Bit, Fluentd, Vector, Logstash, Filebeat, Promtail)

These tools are excellent at generic log collection and routing. They are not designed for the shapes of data that AI agents produce (multi-turn traces with tool-call fan-out, token-accounting semantics, prompt/completion redaction, MCP attribution, ZDR opt-in semantics, eval-result feedback loops). Operators can and do bend Fluent Bit or Vector to ship AI logs, but they hand-write the parsing, the canonical schema, and the policy stage.

**Mara's claim:** the bending stops. Mara provides the canonical AI schema, the redaction primitives, and the runtime presets out of the box; the rest of the pipeline is conventional.

### vs. OpenTelemetry Collector

OTel Collector is general-purpose and excellent. It can absolutely receive `gen_ai.*` OTLP and route it. What it lacks is the file-tail, hooks, and analytics-REST adapters that AI runtimes actually emit through; the runtime-aware presets; the AI-specific redaction packs; and the ZDR-respecting defaults. The OTel project itself documents that the `gen_ai.*` semconv is still Development as of May 2026.

**Mara's claim:** Mara is the AI-specialist OTel Collector sibling. We receive OTLP gladly, we emit OTLP gladly, but we cover the non-OTLP collection surfaces that AI runtimes use today — and we ship runtime presets nobody else does.

### vs. commercial observability platforms (Datadog, Splunk, New Relic, Honeycomb, Sumo Logic)

These platforms have launched LLM observability features (Datadog LLM Observability, New Relic AI Monitoring, Honeycomb AI features, Splunk's gen-AI integrations). They are good products in their own right, but each:

- couples ingestion to their proprietary backend,
- prices on data volume that is high for verbose LLM traffic,
- ships an SDK or a generic agent, not an AI-runtime-aware shipper.

**Mara's claim:** Mara is the neutral input layer. If you already use Datadog, Splunk, or Honeycomb, Mara collects from the AI runtimes that those platforms don't have first-class connectors for, normalizes to `gen_ai.*`, and forwards into your existing ingestion. We don't compete with your backend; we feed it better.

### vs. AI-native observability SaaS (Langfuse, Arize Phoenix, LangSmith, Helicone, Braintrust, W&B Weave, Logfire, Galileo, Comet Opik)

These are excellent at the "I'm building an LLM app and I want a hosted dashboard" use case. Most assume your code calls their SDK or your traffic flows through their proxy. They are application-instrumentation tools, not runtime-agent shippers.

**Mara's claim:** Mara captures from runtimes whose code you don't own — Claude Code on a developer's laptop, Codex in a CI pipeline, Cursor Agents in an IDE. None of these are places where you can drop an SDK. Mara can also forward into Langfuse, Phoenix, LangSmith, or Helicone as sinks, making Mara complementary rather than competitive for SaaS-on-our-own-code customers.

### vs. inference proxies (LiteLLM, Portkey, OpenRouter, Helicone proxy mode)

Proxies sit synchronously in the inference path and can capture request/response payloads. They are great for first-party application code but cannot capture telemetry from AI tools whose inference does not flow through the proxy (Claude Code, Cursor, Augment, Kimi all call vendor APIs directly).

**Mara's claim:** Mara reads from runtime emissions, not from the request path. We are out-of-band and zero-latency-cost.

### vs. governance / guardrail tools (Lakera Guard, Pillar Security, NeMo Guardrails, Llama Guard)

These are runtime guardrails — they evaluate prompts and outputs for harmful content, prompt injection, or policy violations. Mara is observational, not enforcing. Mara can ingest guardrail decisions as canonical events and ship them, but it does not block inference.

**Mara's claim:** Mara is the telemetry side of the governance equation. Guardrails decide; Mara records and routes the audit trail.

## The unique seat at the table

Mara occupies a specific gap that nobody else owns end-to-end:

1. **Edge-first, single-binary, Rust** — every AI-obs SaaS is hosted; every classic shipper is AI-agnostic.
2. **Runtime-aware presets for six AI runtimes** — including ones where you don't own the code (Claude Code, Codex, Cursor).
3. **OTel `gen_ai.*` semconv as the canonical model** — neutral, portable, future-proof.
4. **WASM-sandboxed policy** with day-one redaction primitives — guardrails ingest, redact, sample, route as code.
5. **Apache 2.0, no proprietary core** — operators can self-host fully; ArdurAI's commercial path is gateway/control-plane, not the shipper.

## Anti-positioning

Mara is **not**:

- a query backend,
- a UI,
- a guardrail,
- an inference proxy,
- a general-purpose log shipper,
- a host-metrics agent,
- a FinOps cost-allocation product,
- a SIEM.

This narrow seat is the point. Every category above will be either a sink we feed or a complementary tool we cite, not a competitor we displace.

## Tagline candidates

- "AI telemetry that ships where you already ship."
- "Open, edge-first observability for AI agents."
- "Your AI signals, your sinks, your schema."
- "Fluent Bit for AI workloads."
