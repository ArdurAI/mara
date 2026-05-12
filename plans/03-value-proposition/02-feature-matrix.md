# Feature Matrix

## Executive summary

A side-by-side comparison of Mara v1 (target) against the closest tools in each category. The matrix is authored as bulleted prose rather than a Markdown table because the categories don't compare cleanly on a single axis and tabular rendering loses nuance.

This document is updated when a competitor ships a major feature relevant to Mara, or when Mara ships a feature relevant to a competitor.

## Comparison axes

Each comparison covers seven dimensions:

1. **Form factor** — what runs where.
2. **AI runtime knowledge** — does it know about Claude Code / Codex / Cursor / Kimi / Augment / Gemini as first-class concepts?
3. **Canonical schema** — what's the data model? OTel `gen_ai.*`? Proprietary? OpenInference?
4. **Collection adapters** — OTLP receive, JSONL tail, hooks, analytics REST, SDK only?
5. **Policy / governance** — redaction, signing, audit log?
6. **License + governance** — Apache? AGPL? Proprietary?
7. **Deployment** — edge agent? sidecar? SaaS only?

## Mara v1 (target)

1. **Form factor**: single statically-linked Rust binary; v2 adds optional gateway; v3 adds optional hosted control plane.
2. **AI runtime knowledge**: first-party presets for Claude Code, Codex, Cursor, Kimi, Augment, Gemini.
3. **Canonical schema**: OTel `gen_ai.*` + MCP semconv + `mara.*` extensions.
4. **Collection adapters**: OTLP receiver, JSONL tail, hooks (subprocess JSON-over-stdio), analytics REST.
5. **Policy / governance**: WASM-sandboxed policy chain, built-in PII/PHI/PCI redaction packs, signed policy bundles, tamper-evident audit log.
6. **License + governance**: Apache 2.0; CNCF Sandbox track in v1.x.
7. **Deployment**: macOS launchd, Linux systemd, Windows Service, k8s DaemonSet + sidecar, Lambda Extension, Docker Compose, CI runners.

## vs. Fluent Bit

- **Form factor**: small C agent; identical edge-first DNA. ✓ comparable.
- **AI runtime knowledge**: none beyond generic file tail. ✗ Mara distinct.
- **Canonical schema**: untyped JSON or msgpack; user-defined. ✗ Mara distinct.
- **Collection adapters**: many; AI-runtime-specific not included. ~ partial.
- **Policy / governance**: filters (Lua, WASM in development), no signed bundles, no built-in PII packs. ~ partial.
- **License + governance**: Apache 2.0, CNCF. ✓ comparable.
- **Deployment**: identical OS support. ✓ comparable.

**Verdict:** Fluent Bit is the closest spiritual sibling. Mara is the AI-specialist version of the same idea.

## vs. OpenTelemetry Collector

- **Form factor**: Go agent + collector contrib, edge or gateway. ✓ comparable.
- **AI runtime knowledge**: none. ✗ Mara distinct.
- **Canonical schema**: OTLP, with `gen_ai.*` semconv received as attributes. ~ Mara aligns and adds runtime-specific normalization.
- **Collection adapters**: rich receiver ecosystem; no file-tail-for-Claude-Code, no hooks-from-Cursor receiver. ~ partial.
- **Policy / governance**: processors (filter, redaction, tail-sampling), but no signed bundles, no AI-specific redaction packs. ~ partial.
- **License + governance**: Apache 2.0, CNCF Graduated. ✓ comparable.
- **Deployment**: rich. ✓ comparable.

**Verdict:** OTel Collector is the upstream sibling. Mara emits and receives OTLP gladly, but adds the runtime presets and AI-specific policy primitives that OTel Collector deliberately keeps generic.

## vs. Vector (by Datadog/Datadog acquired Timber)

- **Form factor**: Rust agent, edge or aggregator. ✓ comparable language.
- **AI runtime knowledge**: none. ✗ Mara distinct.
- **Canonical schema**: untyped events, VRL transforms. ✗ Mara distinct.
- **Collection adapters**: many; no AI-runtime adapters. ~ partial.
- **Policy / governance**: VRL transforms, no signed policy bundles, no built-in AI redaction. ~ partial.
- **License + governance**: MPL 2.0 (Vector core), MIT (some components); Datadog-owned. ~ acceptable but vendor-led.
- **Deployment**: rich. ✓ comparable.

**Verdict:** Vector is a credible base for "build your own AI shipper." Mara skips that work and ships AI-aware out of the box.

## vs. Splunk Universal Forwarder + Heavy Forwarder

- **Form factor**: closed-source agent. ~ different model.
- **AI runtime knowledge**: none. ✗ Mara distinct.
- **Canonical schema**: Splunk events. ✗ Mara distinct.
- **Collection adapters**: file, syslog, scripts; no AI-runtime adapters. ✗ Mara distinct.
- **Policy / governance**: index-time filters; Splunk Cloud governance features; closed. ~ partial.
- **License + governance**: proprietary. ✗ Mara distinct.
- **Deployment**: rich, enterprise-grade. ✓ comparable in coverage.

**Verdict:** UF is the legacy enterprise shipper. Mara can be a Splunk-input via the HEC sink while running its own canonical core.

## vs. Datadog Agent + Datadog LLM Observability

- **Form factor**: Go agent + SaaS. ~ different model.
- **AI runtime knowledge**: some — Datadog LLM Obs has integrations for OpenAI, Anthropic, Bedrock via SDK auto-instrumentation. ~ partial; doesn't cover AI runtimes whose code you don't own.
- **Canonical schema**: Datadog proprietary, mappable to `gen_ai.*`. ~ partial.
- **Collection adapters**: SDK-based; agent collects host signals. ✗ different.
- **Policy / governance**: Datadog-side scrubbing rules. ~ partial.
- **License + governance**: proprietary. ✗ Mara distinct.
- **Deployment**: rich. ✓ comparable in coverage.

**Verdict:** Datadog is a sink. Mara collects from places Datadog's SDKs don't reach, normalizes to neutral schema, and ships into Datadog.

## vs. Langfuse

- **Form factor**: self-hosted or SaaS web app; client SDKs in Python/JS/Go. ~ different model.
- **AI runtime knowledge**: SDK-instrumented frameworks (LangChain, LlamaIndex). ~ partial; not the runtimes Mara targets.
- **Canonical schema**: Langfuse proprietary, with OTel export. ~ partial.
- **Collection adapters**: SDK only. ✗ Mara distinct.
- **Policy / governance**: server-side. ~ partial.
- **License + governance**: MIT (core); Langfuse Inc. ~ generous.
- **Deployment**: self-hosted possible. ~ comparable in self-host.

**Verdict:** Langfuse is a great sink for application traces. Mara forwards to Langfuse via OTLP. Complementary.

## vs. Arize Phoenix + OpenInference

- **Form factor**: open-source notebook/web UI; OpenInference SDK semconv. ~ different model.
- **AI runtime knowledge**: SDK-instrumented frameworks. ~ partial.
- **Canonical schema**: OpenInference (Arize-led semconv, OTel-compatible). ~ partial; differs from OTel `gen_ai.*`.
- **Collection adapters**: SDK + OTLP receive. ~ partial.
- **Policy / governance**: minimal in core; Arize AX adds enterprise governance. ~ partial.
- **License + governance**: Elastic License 2.0 (Phoenix); Arize-owned. ~ source-available, not OSI-approved.
- **Deployment**: self-hosted possible. ~ comparable.

**Verdict:** Phoenix is a sink and an alternative semconv. Mara maps `gen_ai.*` ↔ `openinference.*` for interop.

## vs. Helicone

- **Form factor**: proxy + SaaS dashboard. ✗ different model.
- **AI runtime knowledge**: proxied-traffic visibility. ~ partial.
- **Canonical schema**: Helicone proprietary. ✗ different.
- **Collection adapters**: proxy interception. ✗ different.
- **Policy / governance**: proxy-side. ~ partial.
- **License + governance**: Apache 2.0 (open-source self-host available). ~ generous.
- **Deployment**: self-host possible. ~ comparable.

**Verdict:** Helicone proxies the request path; Mara observes the agent path. Complementary.

## vs. LangSmith

- **Form factor**: SaaS dashboard with SDK. ✗ different model.
- **AI runtime knowledge**: LangChain-first; broader SDK coverage. ~ partial.
- **Canonical schema**: LangSmith proprietary, OTel export available. ~ partial.
- **Collection adapters**: SDK + OTel ingestion. ~ partial.
- **Policy / governance**: workspace-level redaction rules. ~ partial.
- **License + governance**: proprietary (LangChain Inc.). ✗ Mara distinct.
- **Deployment**: SaaS primary. ✗ Mara distinct.

**Verdict:** LangSmith is a sink target. Complementary.

## vs. Honeycomb AI features

- **Form factor**: SaaS with OTel ingestion. ~ different model.
- **AI runtime knowledge**: receives `gen_ai.*` OTLP, has AI-themed dashboards. ~ partial.
- **Canonical schema**: OTel `gen_ai.*` aligned. ✓ aligned.
- **Collection adapters**: OTel SDKs. ~ partial.
- **Policy / governance**: server-side. ~ partial.
- **License + governance**: proprietary. ✗ Mara distinct.
- **Deployment**: SaaS. ✗ Mara distinct.

**Verdict:** Honeycomb is the most schema-aligned sink. Mara → Honeycomb via OTLP is high-fidelity.

## Conclusion

The matrix repeats a pattern: most adjacent tools meet some of Mara's seven unique-value claims but never all seven. Mara's seat is the conjunction. See [`03-unique-value-claims.md`](03-unique-value-claims.md) for the testable formulation.
