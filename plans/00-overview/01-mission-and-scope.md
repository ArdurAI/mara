# Mission and Scope

## Executive summary

Mara is an AI-native log shipper and telemetry agent, written in Rust, designed to collect, normalize, govern, and route the unique signals that AI agents and LLM workloads produce. Mara is to AI workloads what Fluent Bit is to container logs and what the OpenTelemetry Collector is to traces and metrics — a small, fast, edge-deployable binary with a strict separation between collection adapters, a typed canonical core, policy enforcement, and a fan-out export layer. Mara aligns with the OpenTelemetry `gen_ai.*` semantic conventions so its output is portable, and treats AI-runtime-specific quirks (hooks APIs, JSONL transcript files, ZDR toggles, MCP tool calls) as first-class concerns rather than workarounds bolted onto a generic pipeline.

## Mission

Make AI agent and LLM observability portable, governable, and operationally boring. Operators should not need to choose between vendor lock-in and rolling their own pipeline.

## Vision

In 12 months, Mara is the default "telemetry side of the equation" for any team running AI agents in production or in dev. In 24 months, Mara is in CNCF Sandbox with a multi-vendor governance model.

## In scope (v1)

- A single, statically-linked Rust binary (`mara`) that runs on macOS, Linux, and Windows.
- Adapter contracts for four collection patterns: OTLP receiver, JSONL/log tail, hooks (subprocess JSON-over-stdio), and analytics REST polling.
- First-party runtime presets for six AI runtimes: Claude Code (CLI + desktop), Codex (CLI + desktop), Cursor Agents, Kimi (CLI/app), Augment Code, Gemini (CLI + API).
- Canonical event schema aligned to OpenTelemetry `gen_ai.*` semantic conventions, with `mara.*` extensions for fields not yet covered upstream.
- A WASM-sandboxed policy stage with built-in primitives: redact, allow/deny, sample, rate-limit, transform, classify, route.
- A bounded WAL with backpressure and graceful + ungraceful shutdown durability guarantees.
- Sinks for OTLP (HTTP + gRPC), Loki, Splunk HEC, Elasticsearch, S3/GCS/Azure Blob (JSONL + Parquet), Kafka, Prometheus Remote Write, file rotation, stdout, and generic webhook.
- File-based TOML configuration with hot reload via SIGHUP/inotify.
- Deployment blueprints for launchd, systemd, Windows Service, Kubernetes DaemonSet, Kubernetes sidecar, Lambda Extension, Docker Compose, and ephemeral CI runners.

## In scope (v2)

- Self-hostable gateway tier (`mara-gateway`) for aggregation, buffering, and policy fan-out at the cluster or fleet boundary.
- Gateway-pushed configuration.
- Multi-tenant policy bundles with tenant-scoped redaction profiles.

## In scope (v3)

- ArdurAI-hosted control plane (commercial product over the Apache 2.0 OSS).
- Managed policy bundle distribution, signed.
- Cross-tenant observability for ArdurAI fleet operators.

## Explicitly out of scope (v1)

See [`02-non-goals.md`](02-non-goals.md).

## Audience priority

1. **AI engineering teams** building or operating agents and LLM-powered features in dev and production.
2. **Platform and SRE teams** responsible for AI workload reliability and cost.
3. **Security, compliance, and audit teams** that need provable agent-action trails.
4. **Individual developers** who want their local Claude Code / Codex / Cursor / Kimi / Augment / Gemini sessions captured for review, replay, or sharing.

## Operating principles

1. **Edge-first.** The agent runs where the AI runtime runs. The gateway is optional. The control plane is optional.
2. **Portable schema.** OpenTelemetry `gen_ai.*` is the canonical reference. Mara extensions live under `mara.*` and graduate upstream when accepted by the semconv working group.
3. **Zero phone-home.** No telemetry leaves the agent unless the operator configured a sink. Defaults are conservative.
4. **ZDR-respecting.** Each runtime's prompt-logging toggle is honored. Prompt and raw-API-body capture is opt-in everywhere.
5. **Vendor-neutral.** Any sink is a plugin. Any inspiration we draw from Fluent Bit, OTel Collector, Loki, or Splunk is at the pattern level — no vendored code, no protocol clones.
6. **Boring core, sharp edges.** The core pipeline is deliberately simple. The interesting work happens in adapters, policies, and sinks.
7. **Reproducible builds, signed releases.** SLSA Level 2 in v1, Level 3 in v2.

## Success in 6 months (post-v1)

- v1.0.0 cut, all six runtime presets passing tier-appropriate acceptance.
- ≥1 named external production user (non-ArdurAI).
- SOC 2 Type I audit underway.
- CNCF Sandbox application submitted.
- At least one upstream contribution to OTel `gen_ai.*` semconv coming from Mara learnings.

## Success in 12 months

- v1.x with the v2 gateway shipped.
- ≥10 named external production users.
- SOC 2 Type II complete.
- CNCF Sandbox accepted.
- Three to five sink plugins maintained by external contributors.

## Relationship to other documents

- For canonical engineering milestones see [`../04-implementation/07-phased-milestones.md`](../04-implementation/07-phased-milestones.md).
- For what we deliberately are not building see [`02-non-goals.md`](02-non-goals.md).
- For positioning vs. competitors see [`../03-value-proposition/01-positioning-statement.md`](../03-value-proposition/01-positioning-statement.md).
