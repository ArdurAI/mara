# Functional Requirements

## Executive summary

This document enumerates what Mara v1 must do, in numbered, testable form. Each `FR-N` is referenced from milestones, ADRs, and acceptance tests. Non-functional requirements (performance, security, etc.) are in [`02-non-functional-requirements.md`](02-non-functional-requirements.md).

## FR-1 — Configuration

- **FR-1.1** Mara MUST read configuration from a single TOML file passed via `--config` or the `MARA_CONFIG` env var.
- **FR-1.2** Mara MUST support optional YAML configuration (same schema, alternate serialization).
- **FR-1.3** Mara MUST hot-reload configuration on SIGHUP without dropping in-flight events.
- **FR-1.4** Mara MUST validate configuration against a JSON Schema before applying it. Validation errors MUST be reported with file path and line/column.
- **FR-1.5** Mara MUST support environment variable interpolation in configuration values (`${VAR}` and `${VAR:-default}`).
- **FR-1.6** Mara MUST support file-based secrets references (`@file:/path/to/secret`) and Vault references (`@vault:path#field`) in configuration.
- **FR-1.7** Mara MUST allow per-pipeline configuration overrides via `pipelines.<name>` sections.

## FR-2 — Collection adapters

- **FR-2.1** Mara MUST provide an **OTLP receiver** adapter supporting:
  - gRPC (default port 4317),
  - HTTP/protobuf (default port 4318),
  - HTTP/JSON (default port 4318 with `/v1/logs` etc.),
  - mTLS optional on all transports.
- **FR-2.2** Mara MUST provide a **JSONL tail** adapter that:
  - tails one or more file globs,
  - persists per-file offset to a durable checkpoint store,
  - handles file rotation (size-based and time-based),
  - resumes correctly from checkpoint after restart,
  - bounds per-file read rate via per-file token bucket.
- **FR-2.3** Mara MUST provide a **hooks** adapter that:
  - reads JSON over stdio from subprocess invocations,
  - reads JSON over POST when invoked as an HTTP hook,
  - associates each hook event with a runtime, session, and turn identifier.
- **FR-2.4** Mara MUST provide an **analytics REST** adapter that:
  - polls a configured URL on a schedule,
  - handles pagination and rate limits with exponential backoff,
  - dedupes events via configurable key fields,
  - persists last-seen cursor durably.
- **FR-2.5** Mara MUST allow multiple instances of each adapter type per pipeline.
- **FR-2.6** Mara MUST expose adapter health (running, lagging, failed) via `mara diag`.

## FR-3 — Runtime presets

- **FR-3.1** Mara MUST ship a `claude-code` preset that uses JSONL tail of `~/.claude/projects/*.jsonl` and OTLP receive on the standard ports, respecting Anthropic ZDR toggles.
- **FR-3.2** Mara MUST ship a `codex` preset that uses OTLP receive (Codex `[otel]` config block) and JSONL tail of `~/.codex/history.jsonl` and an optional Codex `notify` hook.
- **FR-3.3** Mara MUST ship a `cursor` preset that uses the hooks adapter to receive Cursor agent hook events over stdio.
- **FR-3.4** Mara MUST ship a `kimi` preset that uses JSONL tail of `~/.kimi/logs/kimi.log` and the `stream-json` print mode parser.
- **FR-3.5** Mara MUST ship an `augment` preset that uses the analytics REST adapter against the Augment analytics API where the operator opts in.
- **FR-3.6** Mara MUST ship a `gemini` preset that uses OTLP receive from `gemini-cli` configured for OTLP export.
- **FR-3.7** Each preset MUST be activatable via `mara setup <preset>` with sensible defaults and no manual config editing.

## FR-4 — Normalization

- **FR-4.1** Mara MUST normalize all incoming events into the canonical schema defined in [`04-data-model.md`](04-data-model.md).
- **FR-4.2** Normalization MUST preserve a `mara.source.adapter` attribute identifying the originating adapter.
- **FR-4.3** Normalization MUST preserve a `mara.source.runtime` attribute identifying the originating runtime when known.
- **FR-4.4** Normalization MUST gracefully degrade: unknown fields are preserved under `attributes.*`, never dropped.
- **FR-4.5** Normalization MUST honor `OTEL_SEMCONV_STABILITY_OPT_IN=gen_ai_latest_experimental` toggle.

## FR-5 — Policy stage

- **FR-5.1** Mara MUST execute an ordered policy chain on every canonical event before it reaches any sink.
- **FR-5.2** Policy stages MUST support: `redact`, `allow`, `deny`, `sample`, `rate_limit`, `transform`, `classify`, `route`.
- **FR-5.3** Mara MUST load policy plugins as WASM modules sandboxed by `wasmtime`.
- **FR-5.4** Policy bundles MUST be loadable from local file, HTTP(S) URL, OCI registry, or S3-compatible store.
- **FR-5.5** Policy bundles MUST be verifiable with `cosign` keyless or key-based signatures before load.
- **FR-5.6** Mara MUST ship built-in policy primitives without requiring WASM: regex redaction, sampling, rate limiting.
- **FR-5.7** Mara MUST ship a built-in PII redaction pack covering: email, phone, US SSN, EU IBAN, US/EU credit card numbers, GitHub tokens, AWS access keys, GCP service account keys, Anthropic/OpenAI API keys, Slack tokens, JWT, generic API tokens.
- **FR-5.8** Mara MUST ship a built-in PHI redaction pack (HIPAA-aware) and a built-in PCI redaction pack (cardholder data).
- **FR-5.9** Mara MUST emit a `policy_decision` event to the audit log for every redact/deny/sample decision when audit logging is enabled.

## FR-6 — Buffering and durability

- **FR-6.1** Mara MUST buffer canonical events to a bounded in-memory queue.
- **FR-6.2** When in-memory queue fills, Mara MUST spill to a write-ahead log (WAL) on disk.
- **FR-6.3** WAL MUST be bounded by time (default 4h) and size (default 1 GiB), whichever is reached first.
- **FR-6.4** WAL MUST be crash-safe: ungraceful shutdown (SIGKILL) MUST lose ≤1 second of events when WAL is enabled.
- **FR-6.5** On graceful shutdown (SIGTERM), Mara MUST drain in-memory queue and flush WAL before exit.
- **FR-6.6** WAL MUST support per-sink offsets so individual sinks can replay independently.

## FR-7 — Export sinks

Mara MUST ship the following sinks in v1:

- **FR-7.1** `otlp_grpc` and `otlp_http` (the inverse of FR-2.1).
- **FR-7.2** `loki` — HTTP push API, labels-as-stream model, structured metadata as needed.
- **FR-7.3** `splunk_hec` — HTTPS POST with HEC token, ack mode optional.
- **FR-7.4** `elasticsearch` — bulk API, optional index template auto-create.
- **FR-7.5** `object_store` — S3, GCS, Azure Blob via `object_store` crate; JSONL and Parquet formats; configurable batching and partitioning.
- **FR-7.6** `kafka` — librdkafka via `rdkafka`; idempotent producer; configurable serialization (JSON, Avro, Protobuf).
- **FR-7.7** `prometheus_remote_write` — for metrics derived from `gen_ai.usage.*` aggregations.
- **FR-7.8** `file` — local file rotation (size + time based).
- **FR-7.9** `stdout` and `stderr` — for debug.
- **FR-7.10** `webhook` — generic HTTPS POST with configurable headers, template body, mTLS.

## FR-8 — Retry, backpressure, dead-letter

- **FR-8.1** Each sink MUST have configurable retry policy (max attempts, backoff base, jitter).
- **FR-8.2** When all retries exhaust, events MUST go to a per-sink dead-letter queue on disk.
- **FR-8.3** Dead-letter queue MUST be inspectable via `mara dlq list / show / replay / drop`.
- **FR-8.4** Mara MUST apply token-bucket backpressure to producers when downstream sinks lag beyond a configurable threshold.

## FR-9 — CLI

The `mara` binary MUST support the following subcommands:

- **FR-9.1** `mara run [--config PATH]` — start the agent.
- **FR-9.2** `mara validate [--config PATH]` — validate configuration without starting.
- **FR-9.3** `mara test pipeline [--name PIPELINE] [--input PATH]` — feed a fixture file through a configured pipeline, print resulting canonical events and sink dispatch.
- **FR-9.4** `mara diag` — print health of all adapters, policy stages, sinks, WAL, and dead-letter queues.
- **FR-9.5** `mara setup <preset>` — apply a runtime preset.
- **FR-9.6** `mara dlq <list|show|replay|drop>` — inspect and manage dead-letter queues.
- **FR-9.7** `mara version` — print version, build commit, Rust version, OTel semconv version pinned.
- **FR-9.8** `mara completions <shell>` — print shell completion script.

## FR-10 — Observability of the agent itself

- **FR-10.1** Mara MUST expose its own metrics (events ingested, normalized, policy-allowed, policy-denied, sink-success, sink-error, WAL depth, queue depth) on a Prometheus-compatible endpoint by default at `127.0.0.1:9099/metrics`.
- **FR-10.2** Mara MUST emit its own structured logs to stderr by default, with `--log-level` and `--log-format <json|text>` flags.
- **FR-10.3** Mara MUST emit its own OTel traces when configured with `MARA_SELF_OTLP_ENDPOINT`.
- **FR-10.4** Mara MUST expose a health endpoint at `127.0.0.1:9099/healthz` returning 200 when all adapters and sinks are healthy.

## FR-11 — Audit log

- **FR-11.1** Mara MUST support an optional tamper-evident audit log that records every policy decision and every material configuration change.
- **FR-11.2** Audit log MUST be append-only with periodic Merkle-root commits to a configurable external store.
- **FR-11.3** Audit log MUST be configurable to route to a separate sink from regular telemetry.

## FR-12 — Multi-tenant support (forward-compatible)

- **FR-12.1** Canonical events MUST carry optional `tenant.id` and `mara.policy.profile` attributes.
- **FR-12.2** Policy chains MUST be selectable per tenant via configuration in v1; tenant-aware policy distribution is v2.
