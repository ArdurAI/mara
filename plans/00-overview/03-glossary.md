# Glossary

Words mean specific things in Mara. Authors and reviewers should default to these definitions.

## A

**Adapter** — a Mara collection component that turns one specific input pattern into canonical events. The four v1 adapters are OTLP receiver, JSONL tail, hooks, and analytics REST. Adapters are not per-runtime; runtimes compose adapters into presets.

**Analytics REST adapter** — polls a vendor analytics API (e.g., Augment Code analytics REST) on a schedule and normalizes responses into canonical events. Tier C in the integration tier model.

**ArdurAI** — the legal entity that owns the Mara trademark and ships the eventual hosted control plane.

**Audit log** — a tamper-evident, append-only record of policy decisions and material agent events. Distinct from the regular telemetry stream.

## B

**Backpressure** — the mechanism by which Mara signals upstream producers to slow down when sinks cannot keep up. v1 uses bounded async channels plus a token-bucket producer throttle.

## C

**Canonical event** — a Mara event that has been normalized into the `gen_ai.*` + `mara.*` schema. Adapters produce canonical events; sinks consume them.

**Canonical schema** — the typed model defined in `crates/mara-schema/`, aligned with OpenTelemetry `gen_ai.*` semantic conventions and the OTel MCP attribute conventions, plus `mara.*` extensions for fields not yet covered upstream.

**Compatibility matrix** — the per-runtime per-signal table of pass/fail/partial results published at `docs/compat-matrix.md`. Updated every release.

**Control plane (v3)** — the optional ArdurAI-hosted management surface for fleets of Mara agents. Not in v1.

## E

**Edge agent** — the single Rust binary `mara` running on the same host as an AI runtime. The v1 deployment unit.

**Event kind** — the high-level classification of a canonical event: `prompt`, `completion`, `tool_call`, `tool_result`, `cost`, `error`, `system`, `eval`, `feedback`.

## G

**`gen_ai.*` semconv** — the OpenTelemetry Semantic Conventions for Generative AI, currently in Development status (not yet stable) as of May 2026. Mara aligns with this namespace and feature-flags the experimental stability bucket via `OTEL_SEMCONV_STABILITY_OPT_IN`.

**Gateway (v2)** — an optional self-hostable aggregator that sits between edge agents and sinks. Same `mara-core` codebase, different binary. Not in v1.

## H

**Hooks adapter** — receives JSON-over-stdio (or sometimes JSON-over-HTTP) lifecycle events from an AI runtime that supports a hooks API. Cursor, Claude Code, and Codex all have a hooks surface. Tier B in the integration tier model.

## I

**Integration tier** — a runtime's classification based on the maturity of its telemetry surface. **Tier A** = native OTLP exporter (Claude Code, Codex, Gemini). **Tier B** = hooks or JSONL tail (Cursor, Kimi, plus Claude Code/Codex as redundant signal). **Tier C** = analytics REST only (Augment).

## J

**JSONL** — JSON Lines, a newline-delimited JSON format. Many AI runtimes write session transcripts as JSONL.

**JSONL tail adapter** — reads a JSONL file, advancing a checkpoint offset on disk so it survives restarts. Used for Claude Code session transcripts, Codex history, Kimi logs.

## M

**MCP** — the Model Context Protocol, an open spec for AI runtimes to discover and invoke tools. Mara observes MCP traffic but is not an MCP server or client itself. The OTel MCP attribute conventions are part of our canonical model.

**MOS** — Minimum Viable Plan with Milestones. The M0–M5 plan that the engineering team executes against.

## N

**Normalization** — the step where an adapter's output is mapped into canonical events. Includes attribute renaming, type coercion, severity mapping, and resource enrichment.

## O

**OPA** — Open Policy Agent. A policy engine that uses the Rego language. Mara ships an OPA-backed policy plugin as a WASM module.

**OTel** — OpenTelemetry. The CNCF observability project. Mara is a consumer (via OTLP receiver) and an emitter (via OTLP sink), and aligns with its semantic conventions.

**OTLP** — OpenTelemetry Protocol. The wire format for OTel signals, available over HTTP/protobuf, HTTP/JSON, and gRPC. v1 sinks and adapters support all three.

## P

**Pipeline** — a configured graph of adapters → policy chain → sinks. A Mara process can run multiple pipelines in parallel.

**Policy bundle** — a signed, versioned set of policies. Distributed as a single file (TAR + manifest + signature). Verified by `cosign` before load.

**Policy chain** — the ordered sequence of policy stages applied to a canonical event before it reaches sinks. Each stage can mutate, drop, or fan out the event.

**PII** — personally identifiable information. Mara provides built-in redaction primitives; raw prompt and API-body capture is opt-in everywhere.

**Preset** — a per-runtime configuration template that composes adapters, normalization rules, default policies, and recommended sinks. Lives in `crates/mara-runtimes/<runtime>/`.

## R

**Runtime** — an AI agent or LLM execution environment. The six v1 runtimes are Claude Code, Codex, Cursor Agents, Kimi, Augment Code, Gemini.

## S

**Sink** — a Mara export component that sends canonical events to a downstream destination. v1 sinks: OTLP, Loki, Splunk HEC, Elasticsearch, S3/GCS/Azure Blob, Kafka, Prometheus Remote Write, file, stdout, webhook.

**SLO** — Service Level Objective. Mara's edge agent SLOs are in [`../04-implementation/02-non-functional-requirements.md`](../04-implementation/02-non-functional-requirements.md).

## T

**Tier A / B / C** — see Integration tier.

**Token bucket** — the throttling algorithm Mara uses on the producer side of each pipeline to enforce backpressure without dropping silently.

## W

**WAL** — Write-Ahead Log. Mara's on-disk buffer that guarantees durability across graceful and ungraceful shutdowns. Bounded by time and size, configurable per pipeline.

**WASM policy plugin** — a WebAssembly module loaded into Mara's policy stage. Polyglot (any language that compiles to WASM), sandboxed by `wasmtime`, signed by `cosign`.

## Z

**ZDR** — Zero Data Retention. A commitment by an AI vendor not to retain customer prompts or completions. Mara respects each runtime's ZDR toggle by defaulting prompt and raw-API-body capture to off.
