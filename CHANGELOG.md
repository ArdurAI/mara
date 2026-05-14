# Changelog

All notable changes to Mara are documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html). Mara is pre-1.0, so breaking changes can happen in any 0.x release; they are nonetheless called out.

## [Unreleased]

### Added

- **[INSTALL.md](INSTALL.md)** at repository root: install-from-source steps and the full verification checklist aligned with CI (format, clippy, tests, schema gate, open-verification SHA256, optional Hugo).

- **M0**: Workspace bootstrap, Apache 2.0 licensing, governance documents, planning encyclopedia under `plans/` (48 documents covering landscape, gaps, value proposition, implementation, evaluation, deployment blueprints, and per-runtime quickstarts).
- **M1**: Canonical event schema aligned with OpenTelemetry `gen_ai.*` semconv in `mara-schema`. Adapter/Sink/Policy trait surfaces in `mara-core`. Seven Architecture Decision Records covering license, WASM policy host, WAL format, hot reload, async runtime, error model, and configuration format.
- **M2**: Pipeline scheduler (adapters → policy chain → sinks fan-out) in `mara-core::pipeline`. TOML configuration loader with JSON-schema-style validation in `mara-core::config`. JSONL tail adapter with per-file offset checkpointing. File rotation sink and stdout sink. Built-in PII redaction policy (regex-based, covering email, US phone, SSN, AWS / GCP / GitHub / OpenAI / Anthropic / Slack tokens, JWT). Head sampling policy. CLI `mara run`, `mara validate`, `mara version`, `mara setup <preset>` (stub) wired against config files. End-to-end integration test: JSONL → PII redaction → file sink.
- **M3**: Runtime presets for Claude Code, Codex, Cursor Agents, Kimi, Augment Code, Gemini CLI under `crates/mara-runtime-*`. Compatibility matrix published at `docs/compat-matrix.md`.
- **M4**: STRIDE threat model at `docs/threat-model.md`. Security CI workflow (`cargo-audit`, `cargo-deny`, OSV scanner, Trivy filesystem scan, Semgrep). Release workflow with cross-platform builds (Linux glibc/musl amd64+arm64, macOS universal2, Windows x64), CycloneDX + SPDX SBOMs, SLSA Level 2 provenance attestations, `cosign` keyless signing, container image (distroless) push to ghcr.io. Security advisories process at `.github/SECURITY-ADVISORIES.md`.
- **M5**: Helm chart at `packaging/helm/mara/`. systemd unit at `packaging/systemd/mara.service`. launchd plist at `packaging/launchd/dev.ardurai.mara.plist`. Homebrew formula at `packaging/homebrew/mara.rb`. Operational runbook at `docs/runbook.md`. This CHANGELOG.
- **CI**: Semgrep in `.github/workflows/security.yml` fails on findings (`--error`) and uploads SARIF for GitHub code scanning (`github/codeql-action/upload-sarif`).
- **Docs**: GenAI field parity matrix for Claude Code, Codex, Cursor, Kimi, and Ollama at `docs/integrations/runtime-parity-matrix.md`.
- **Docs**: Self-metrics latency histogram semantics (`mara_gen_ai_request_duration_ms_*`, PromQL, ring buffer vs cumulative buckets) at `docs/observability/mara-self-metrics-latency-histogram.md`.
- **CI / M2-02**: `scripts/benchmarks/schema_completeness_gate.py` plus job `schema-completeness-gate` — at least three runtime fixtures must average ≥85% fill on seven required `gen_ai`/`resource` fields.
- **M2 (milestone board)**: Agent fields on `MaraExtensions`, gateway correlation id (`x-mara-request-id` / `mara.request_id`), dual latency metrics (gateway vs engine), `CostConfidence` for honest cost, parallel sink fan-out, `/readyz` readiness, bounded `/metrics` rendering, fan-out and sink-send-error counters, optional `server.metrics_max_in_flight_connections`, `docs/semconv.lock` + CI drift script, quickstarts, Kubernetes probe fragment, Phoenix/Presidio/eval OTLP notes, single-upstream proxy doc, vector/GPU spike ADR, Grafana compare-by-pipeline panel, and `scripts/materialize_agent_run_summary.py`.
- **Docs / CI**: Redacted Ollama+Mara live-run bundles in `docs/captured/open-verification/` (SHA256 manifest + `scripts/captured/verify_open_verification.sh`; CI job `open-verification-sha256`).
- Per-pipeline **`audit_policy_drops`** (default `false`): when enabled, a minimal `System` audit event (no body; correlation + `mara.policy_audit.*` + policy decisions) is sent to sinks on every policy drop. See `docs/observability/mara-policy-drop-audit.md`.

### Changed

- **Breaking (policy authors)**: `PolicyOutcome::Drop` now includes the dropped `Event`. Use `PolicyOutcome::drop(event, reason)` instead of `PolicyOutcome::drop(reason)`. `ChainOutcome::Drop` is now `{ reason, event }` instead of a bare `String`.
- **`/readyz` readiness**: `Health::is_aggregate_ready` is true only for `healthy` or `degraded` (not `starting`, `stopping`, `stopped`, or `failed`). `Adapter::health` / `Sink::health` default to `Health::healthy()` so typical pipelines stay ready unless an implementation opts into finer-grained status. See `docs/observability/mara-readyz-semantics.md`.
- **LLM proxy**: Response `x-mara-request-id` is always a header-safe value (ASCII fast path, stripped graphic ASCII fallback, or UUID) while events keep the logical correlation id from the client or minted UUID.

### Known gaps (pre-1.0)

- **OTLP**: HTTP/protobuf receivers for `/v1/logs` and `/v1/traces` are implemented in `mara-adapter-otlp`; optional gRPC (`:4317` style) logs+traces is available when `grpc_listen` is set. OTLP **HTTP exporters** in sinks are implemented; advanced exporter features remain incremental.
- **Pipeline WAL**: optional post-policy JSONL spool per pipeline (`wal_spool_path`) appends delivered events with `fsync` per line batching policy documented in `docs/observability/pipeline-wal-spool.md`; full segmented append-only WAL + replay per ADR-0003 is still future work.
- WASM policy host (Wasmtime) is scaffolded; built-in primitives ship; third-party WASM bundle loading and signature verification pending M4 follow-up.
- Signed policy bundle distribution via OCI registry pending M4 follow-up.
- Tamper-evident audit log with Merkle root export pending M4 follow-up.
- 1-hour sustained 50k EPS bench is wired into `cargo bench` scaffolding but full perf harness with regression tracking dashboard pending M2 follow-up.
- Per-runtime **presets** under `crates/mara-runtime-*` remain the fastest path to a working `mara.toml`; Tier **B** HTTP hooks ingest (`mara-adapter-hooks`) and Tier **C** analytics polling (`mara-adapter-analytics`) are now real adapters wired from `mara run`. Runtime-specific edge transports (Codex `notify` IPC, Cursor hook sockets) may still need local glue outside Mara.
- SOC 2 Type I audit kickoff is scheduled but not yet underway.
- CNCF Sandbox application is drafted but not submitted.

## [0.1.0-rc.1] - 2026-XX-XX

First release candidate. To be tagged once the gaps above are closed or explicitly accepted as v1.0 release gaps.
