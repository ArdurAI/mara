# Changelog

All notable changes to Mara are documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html). Mara is pre-1.0, so breaking changes can happen in any 0.x release; they are nonetheless called out.

## [Unreleased]

### Added

- **M0**: Workspace bootstrap, Apache 2.0 licensing, governance documents, planning encyclopedia under `plans/` (48 documents covering landscape, gaps, value proposition, implementation, evaluation, deployment blueprints, and per-runtime quickstarts).
- **M1**: Canonical event schema aligned with OpenTelemetry `gen_ai.*` semconv in `mara-schema`. Adapter/Sink/Policy trait surfaces in `mara-core`. Seven Architecture Decision Records covering license, WASM policy host, WAL format, hot reload, async runtime, error model, and configuration format.
- **M2**: Pipeline scheduler (adapters → policy chain → sinks fan-out) in `mara-core::pipeline`. TOML configuration loader with JSON-schema-style validation in `mara-core::config`. JSONL tail adapter with per-file offset checkpointing. File rotation sink and stdout sink. Built-in PII redaction policy (regex-based, covering email, US phone, SSN, AWS / GCP / GitHub / OpenAI / Anthropic / Slack tokens, JWT). Head sampling policy. CLI `mara run`, `mara validate`, `mara version`, `mara setup <preset>` (stub) wired against config files. End-to-end integration test: JSONL → PII redaction → file sink.
- **M3**: Runtime presets for Claude Code, Codex, Cursor Agents, Kimi, Augment Code, Gemini CLI under `crates/mara-runtime-*`. Compatibility matrix published at `docs/compat-matrix.md`.
- **M4**: STRIDE threat model at `docs/threat-model.md`. Security CI workflow (`cargo-audit`, `cargo-deny`, OSV scanner, Trivy filesystem scan, Semgrep). Release workflow with cross-platform builds (Linux glibc/musl amd64+arm64, macOS universal2, Windows x64), CycloneDX + SPDX SBOMs, SLSA Level 2 provenance attestations, `cosign` keyless signing, container image (distroless) push to ghcr.io. Security advisories process at `.github/SECURITY-ADVISORIES.md`.
- **M5**: Helm chart at `packaging/helm/mara/`. systemd unit at `packaging/systemd/mara.service`. launchd plist at `packaging/launchd/dev.ardurai.mara.plist`. Homebrew formula at `packaging/homebrew/mara.rb`. Operational runbook at `docs/runbook.md`. This CHANGELOG.

### Known gaps (pre-1.0)

- OTLP gRPC and HTTP receivers and senders are scaffolded as stubs; full implementation pending M2 follow-up.
- WAL is in-memory bounded queues with disk-spill-as-stub; segmented append-only WAL per ADR-0003 pending M2 follow-up.
- WASM policy host (Wasmtime) is scaffolded; built-in primitives ship; third-party WASM bundle loading and signature verification pending M4 follow-up.
- Signed policy bundle distribution via OCI registry pending M4 follow-up.
- Tamper-evident audit log with Merkle root export pending M4 follow-up.
- 1-hour sustained 50k EPS bench is wired into `cargo bench` scaffolding but full perf harness with regression tracking dashboard pending M2 follow-up.
- Per-runtime adapters at M3 are presets + tier classification; production-grade collectors for each runtime's specific surface (Codex `notify` hook subprocess wiring, Cursor hook IPC socket, Augment Analytics REST poller) ship as scaffolds plus documentation.
- SOC 2 Type I audit kickoff is scheduled but not yet underway.
- CNCF Sandbox application is drafted but not submitted.

## [0.1.0-rc.1] - 2026-XX-XX

First release candidate. To be tagged once the gaps above are closed or explicitly accepted as v1.0 release gaps.
