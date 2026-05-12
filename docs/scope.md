# Mara M0 Scope Register

This document is the canonical record of M0 scope decisions for the Mara project. It captures the resolved defaults, owners, and dependencies at the close of M0 so that subsequent milestones have a stable foundation.

For canonical engineering detail see [`../plans/04-implementation/07-phased-milestones.md`](../plans/04-implementation/07-phased-milestones.md). For high-level positioning see [`../plans/00-overview/01-mission-and-scope.md`](../plans/00-overview/01-mission-and-scope.md).

## M0 deliverables — status

| Deliverable | Status | Reference |
|---|---|---|
| Apache 2.0 licensing | Done | [`../LICENSE`](../LICENSE), [`../NOTICE`](../NOTICE) |
| Governance docs | Done | [`../CONTRIBUTING.md`](../CONTRIBUTING.md), [`../SECURITY.md`](../SECURITY.md), [`../CODE_OF_CONDUCT.md`](../CODE_OF_CONDUCT.md) |
| Cargo workspace | Done | [`../Cargo.toml`](../Cargo.toml), `crates/`, `xtask/` |
| Rust toolchain pin | Done | [`../rust-toolchain.toml`](../rust-toolchain.toml) |
| Lint configuration | Done | [`../rustfmt.toml`](../rustfmt.toml), [`../clippy.toml`](../clippy.toml), [`../deny.toml`](../deny.toml) |
| Initial CI | Done | [`../.github/workflows/ci.yml`](../.github/workflows/ci.yml), [`../.github/workflows/security.yml`](../.github/workflows/security.yml) |
| Dependabot config | Done | [`../.github/dependabot.yml`](../.github/dependabot.yml) |
| Planning encyclopedia | Done | [`../plans/`](../plans/) (48 documents) |
| Architecture & scope docs | Done | [`../plans/00-overview/`](../plans/00-overview/) |

## Confirmed decisions (from M0 scope-lock conversation)

- **Inspirations:** Fluentd, Fluent Bit, OpenTelemetry Collector, Grafana Loki, Splunk Universal Forwarder / HEC (pattern-level only; no vendored code).
- **Target runtimes:** Claude Code (CLI + desktop), Codex (CLI + desktop), Cursor Agents, Kimi (CLI/app), Augment Code, Gemini (CLI + API).
- **Deployment topology:** Tiered. v1 edge agent → v2 self-hostable gateway → v3 ArdurAI-hosted control plane. SDK route open from day 1.

## Resolved defaults

These were proposed during M0 planning and remain unchallenged at scope-lock; subsequent ADRs supersede individual entries as needed.

- **License:** Apache 2.0 (relicensed from MIT on day 1; rationale in `docs/adr/0001-license.md` when written in M1).
- **Non-goals (v1):** no long-term storage backend, no UI/dashboard, no host-metrics agent, no eval harness, no inference-time guardrail.
- **Target audience priority:** AI eng teams (1) → platform/SRE (2) → security/compliance (3) → individual devs (4).
- **Data retention:** Mara is transient (in-memory + 4h / 1 GiB WAL default). Long-term retention is delegated to user-chosen sinks.
- **Compliance boundary (v1):** SOC 2 Type I targeted in M5; GDPR/CCPA day-1 design alignment; HIPAA/PCI/FedRAMP not in v1 but architecture must not preclude.
- **Performance SLOs (v1):** ≥50k EPS sustained on single x86_64 core, p99 ≤ 2 s ingest-to-export, ≤128 MiB idle RSS, ≤512 MiB under load, ≤1 s data loss on SIGKILL with WAL.
- **Canonical schema:** OpenTelemetry `gen_ai.*` + MCP semconv + `mara.*` extensions.
- **Policy model:** WASM-sandboxed plugins + built-in primitives, signed bundles via `cosign`.
- **Sinks (v1):** OTLP, Loki, Splunk HEC, Elasticsearch, object store, Kafka, Prom RW, file, stdout, webhook.
- **Config:** TOML primary, YAML alternate, JSON Schema validated, hot reload on SIGHUP/inotify.
- **Backpressure:** bounded async channels + token-bucket producer throttle; per-sink retry + dead-letter on disk WAL.

## Open items deferred from M0

These were called out in the MOS plan as "M0 open items" and remain pending stakeholder input.

- **Named owners** for: Core Lead, Schema Lead, Integrations Lead, Security Lead, PM/Legal. Tracked in `MAINTAINERS.md` (to be authored in M5).
- **SOC 2 automation vendor pick** (Vanta / Drata / Lacework / TrustCloud). Driven by ArdurAI Legal/PM. Decision needed before M4 starts.
- **CNCF Sandbox application timing** (M5 vs v1.1). Decision needed before M5 packaging.
- **Weighted-decision criteria weights** in the MOS plan are sane defaults; override at any time and add a note here.

## Crate inventory (25 crates)

Foundation:
- `mara-core` (pipeline scheduler, traits, WAL, backpressure, hot reload)
- `mara-schema` (canonical event types aligned with OTel gen_ai semconv)
- `mara-policy` (WASM host + built-in primitives + signed bundles)

Adapters (collection):
- `mara-adapter-otlp` (Tier A)
- `mara-adapter-jsonl` (Tier B)
- `mara-adapter-hooks` (Tier B)
- `mara-adapter-analytics` (Tier C)

Runtime presets:
- `mara-runtime-claude-code` (Tier A + B)
- `mara-runtime-codex` (Tier A + B)
- `mara-runtime-cursor` (Tier B)
- `mara-runtime-kimi` (Tier B)
- `mara-runtime-augment` (Tier C)
- `mara-runtime-gemini` (Tier A)

Sinks (export):
- `mara-sink-otlp`
- `mara-sink-loki`
- `mara-sink-splunk-hec`
- `mara-sink-elasticsearch`
- `mara-sink-object-store`
- `mara-sink-kafka`
- `mara-sink-prom-rw`
- `mara-sink-file`
- `mara-sink-webhook`

Binaries:
- `mara-cli` (the `mara` binary)
- `mara-gateway` (v2 placeholder binary in v1)

Tooling:
- `xtask` (internal codegen + release runner; not published)

## CI gates active at end of M0

- `cargo fmt --all --check` (rustfmt 2024 edition profile).
- `cargo clippy --workspace --all-targets -- -D warnings` (with workspace-level lints: pedantic, nursery, all, plus `unsafe_code = "forbid"`).
- `cargo test --workspace --all-targets` on Ubuntu, macOS, Windows.
- `cargo build --workspace --release` matrix for `x86_64-unknown-linux-gnu`, `aarch64-apple-darwin`, `x86_64-pc-windows-msvc`.
- `cargo doc --workspace --no-deps` with `RUSTDOCFLAGS=-D warnings`.
- MSRV pin check at 1.94.1.
- License presence check.
- Conventional commit lint on PR titles.
- `cargo-audit`, `cargo-deny`, OSV scanner, Trivy fs, semgrep (scheduled + on PR).

## Exit verification

- `cargo fmt --all --check` → clean.
- `cargo clippy --workspace --all-targets -- -D warnings` → clean.
- `cargo test --workspace --all-targets` → 1 test (mara-core version sanity), passes.
- `cargo doc --workspace --no-deps` → clean.

M0 is closed at commit-time of this file plus the workspace bootstrap. M1 begins with trait surface design in `mara-core`, ADR drafting under `docs/adr/`, and `mara-schema` codegen from a pinned OTel semconv commit.
