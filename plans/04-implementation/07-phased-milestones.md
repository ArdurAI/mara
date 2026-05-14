# Phased Milestones

## Executive summary

The Mara MOS plan is M0 through M5, approximately 20 weeks to v1.0.0-rc.1. Each milestone has scoped deliverables, exit criteria, and acceptance tests. v2 (gateway) and v3 (hosted control plane) follow as separate MOS cycles after v1.0 ships.

This document is the canonical engineering reference. The high-level plan summary lives in `plans/mara_mos_plan_d0da16c1.plan.md`; this file is the engineering-facing detail.

## M0 — Discovery and scope lock

**Duration:** 2 weeks.

**Goals:** Lock scope, license, and toolchain decisions. Bootstrap the workspace. Get CI green on an empty workspace.

**Deliverables:**

- Repo bootstrap: `LICENSE` (Apache 2.0), `NOTICE`, `CONTRIBUTING.md`, `SECURITY.md`, `CODE_OF_CONDUCT.md`, `.gitignore`, `rust-toolchain.toml`, `rustfmt.toml`, `clippy.toml`.
- Cargo workspace with empty crates per [`03-architecture-blocks.md`](03-architecture-blocks.md).
- Initial GitHub Actions CI: format check, clippy, build, test, dependency review, license scan.
- Scope register document under [`../00-overview/`](../00-overview/) covering this milestone's confirmed defaults.
- ADR-0001: license decision (Apache 2.0).

**Exit criteria:**

- All workspace crates `cargo check --workspace` succeed on linux + macos + windows in CI.
- `cargo fmt --all --check` passes.
- `cargo clippy --workspace --all-targets -- -D warnings` passes (with empty crates this is trivial but proves wiring).
- Scope register signed by stakeholders.

**Risks:** license relicense friction; cargo workspace cross-platform path quirks.

## M1 — Architecture decision and contracts

**Duration:** 3 weeks.

**Goals:** Freeze adapter/sink/policy trait surfaces. Generate canonical schema from pinned semconv. Document architecture decisions.

**Deliverables:**

- `crates/mara-core/src/traits/` with `Adapter`, `Sink`, `Policy`, `PolicyContext`, `PolicyOutcome`, `Event`, `EventBatch`, `Health` — all behind `#[non_exhaustive]`.
- `crates/mara-schema/` with codegen pipeline:
  - `xtask codegen-semconv` reads `semconv.lock` and a pinned commit of `open-telemetry/semantic-conventions`,
  - emits Rust types into `crates/mara-schema/src/generated/`,
  - CI fails if generated output diverges from committed.
- ADRs in `docs/adr/`:
  - 0002 WASM policy host choice (wasmtime).
  - 0003 WAL format (segmented append-only with per-sink offsets).
  - 0004 Hot reload mechanism (SIGHUP + inotify with debounce).
  - 0005 Async runtime (tokio multi-thread).
  - 0006 Error model (`thiserror`-backed enums, no `anyhow` on public API).
  - 0007 Config format (TOML primary, YAML alternate, JSON Schema validation).
- Risk register expanded with mitigations and owners.

**Exit criteria:**

- All public traits documented with `rustdoc`.
- Schema codegen runs in CI; drift check passes.
- ADRs reviewed and merged.
- Public API surface auditable via `cargo-public-api`.

**Risks:** semconv drift; WASM ABI choices.

## M2 — Core Rust prototype

**Duration:** 5 weeks.

**Goals:** Implement the core pipeline end-to-end. Validate performance SLOs in CI. Ship the first working adapters and sinks.

**Deliverables:**

- `mara-core`: pipeline scheduler, bounded channels, WAL, backpressure, hot reload, config loader + validator.
- `mara-adapters/otlp/`: OTLP receiver over gRPC, HTTP/protobuf, HTTP/JSON. mTLS optional. Health endpoint.
- `mara-adapters/jsonl/`: file tail with per-file offset checkpoint, rotation handling, rate limiting.
- `mara-sinks/otlp/`: OTLP sender; round-trips events with the receiver.
- `mara-policy`: regex redaction primitive, sampling primitive, rate limiting primitive. WASM host scaffold (no third-party plugins yet).
- `mara-cli`: `run`, `validate`, `test pipeline`, `diag`, `version`.
- One built-in policy pack: `builtin.pii` (regex-based) covering: email, phone, US SSN, US/EU credit cards, GitHub tokens, AWS keys, GCP SA keys, Anthropic/OpenAI keys, Slack tokens, JWTs.
- Benchmark harness (`benches/`) with `criterion` and `iai-callgrind` for memory/CPU.
- Integration tests under `tests/`:
  - OTLP round-trip preserves `gen_ai.*` attributes.
  - JSONL tail of a synthetic Claude Code transcript produces a golden canonical event stream.
  - Sustained 50k EPS for 1h on the CI runner (with reduced runtime in PR CI; full 1h on the nightly bench job).
  - SIGKILL mid-load test: ≤1s of events lost from WAL.

**Exit criteria:**

- All FRs in the M2 scope are implemented per [`01-functional-requirements.md`](01-functional-requirements.md).
- Performance gates per [`02-non-functional-requirements.md`](02-non-functional-requirements.md) NFR-1 are green.
- Documented WAL recovery test passes.
- Self-telemetry shows pipeline events flowing.

**Risks:** WAL durability bugs; tokio channel sizing; redaction false positives.

### Engineering review backlog (tracked on milestone board)

Cross-cutting items from the 2026 performance / backend / observability / cost / security / platform / systems-design review are **not** a separate MOS phase; they are ticketed on [`docs/milestones/mara-m0-m2-board.md`](../../docs/milestones/mara-m0-m2-board.md) as **M1-08–M1-11** (M1 hardening) and **M2-08–M2-15** (M2 reliability and ops). **M2-16–M2-22** add the market-gap telemetry backlog (correlation ID, semconv pin, OpenInference bridge, Presidio path, eval OTLP profile, upstream semantics doc, vector/GPU spike). Summary:

- **Policy honesty:** `deny` stage must be implemented or rejected at config parse (no silent skip).
- **Proxy security:** Threat model and controls when `llm_proxy` binds beyond loopback; upstream timeouts and connection limits.
- **CI:** Semgrep (or equivalent) must not hide failures without a mandatory alternate signal.
- **Observability docs:** Clarify self-metrics latency histogram vs in-process p95 sample semantics for PromQL users.
- **Pipeline:** Parallel sink fan-out; optional `Arc<Event>` to cut clone cost; readiness endpoint beyond liveness.
- **Performance:** Bound `/metrics` scrape cost; optional dual latency (gateway vs engine); cap concurrent scrape connections when metrics are exposed.
- **Cost honesty:** Explicit cost-confidence metadata when truncation or missing usage breaks estimates.
- **Platform:** Minimal Helm/K8s probe wiring; USE-style saturation metrics for bounded channels.
- **Market parity (thin proxy):** Stable **gateway correlation ID** when `traceparent` is absent; **pinned OTel GenAI semconv** with CI drift; **OpenInference / Phoenix** mapping doc; **optional Presidio**-class PII; **eval-backend OTLP** checklist (export-only); **explicit single-upstream** semantics; **vector/GPU** scope spike.

Pull work from this backlog into each sprint according to the board priorities (P0 first: M1-08, M1-09).

## M3 — Runtime integration slice

**Duration:** 4 weeks.

**Goals:** Ship all six runtime presets at their tier-appropriate acceptance bar. Publish the compatibility matrix.

**Deliverables:**

- `crates/mara-runtimes/claude_code/`: Tier A preset (OTLP receive) + Tier B redundant signal (JSONL tail). ZDR toggle honored. Golden tests against synthetic transcripts.
- `crates/mara-runtimes/codex/`: Tier A preset (OTLP receive) + Tier B redundant signal (JSONL tail + `notify` hook). ZDR toggle honored.
- `crates/mara-runtimes/gemini/`: Tier A preset (OTLP receive). `logPrompts` toggle honored.
- `crates/mara-runtimes/cursor/`: Tier B preset (hooks adapter). Coverage ≥80% of documented hook event types. Documented known gaps.
- `crates/mara-runtimes/kimi/`: Tier B preset (JSONL tail). Optional `stream-json` parser. ZIP export ingest.
- `crates/mara-runtimes/augment/`: Tier C preset (analytics REST). Documented latency floor and coverage report.
- `crates/mara-adapters/hooks/`: full implementation supporting stdio and HTTP modes.
- `crates/mara-adapters/analytics/`: full implementation with pagination, dedup, cursor persistence.
- `crates/mara-sinks/loki/` and `crates/mara-sinks/file/`: first non-OTLP sinks shipped.
- `docs/compat-matrix.md` published with per-runtime per-signal pass/fail/partial.
- Smoke tests for all six runtimes wired into CI nightly.

**Exit criteria:**

- All six runtimes pass tier-appropriate acceptance.
- Compatibility matrix published.
- `mara setup <runtime>` works end-to-end for all six.

**Risks:** Augment Code has no public hooks — gap documented but not closeable in v1; Kimi OTel status is in flux; Cursor hook payload schema is undocumented in spots.

## M4 — Security and governance baseline

**Duration:** 3 weeks.

**Goals:** Wire all the supply-chain, security scanning, audit, and signing primitives. Write the threat model and the SOC 2 control mapping.

**Deliverables:**

- STRIDE threat model document (`docs/threat-model.md`) covering: edge agent surface (file reads, network egress, signal handling, IPC), gateway-future surface (auth, tenant isolation).
- Security CI matrix:
  - `cargo-audit` against RustSec advisory db on every PR.
  - `cargo-deny` license + ban + advisories + sources checks.
  - `cargo-vet` baseline (audit-by-imports for first-party crates).
  - OSV scanner action.
  - Trivy filesystem and image scans.
  - Hadolint for Dockerfile if present.
- SBOM generation:
  - CycloneDX via `cargo-cyclonedx` on every release.
  - SPDX via `cargo-sbom` on every release.
  - SBOM published as release artifact.
- SLSA Level 2 build provenance via `slsa-github-generator`.
- `cosign` keyless signatures via Fulcio + Rekor on all release artifacts.
- Reproducible build investigation (Linux first); document findings.
- Signed policy bundles: `cosign sign-blob` on bundle TAR; verifier in `mara-policy`.
- Tamper-evident audit log: append-only with periodic Merkle root export.
- ZDR matrix per runtime documented in [`../01-landscape/08-ai-runtime-telemetry-surfaces.md`](../01-landscape/08-ai-runtime-telemetry-surfaces.md).
- SOC 2 Type I control mapping draft (CC1–CC9) at [`../05-evaluation/03-soc2-control-mapping.md`](../05-evaluation/03-soc2-control-mapping.md). Controls written, evidence pipeline scaffolded.

**Exit criteria:**

- Every CI security check is mandatory on `main`.
- Every release tag produces SBOM + provenance + signatures.
- Policy bundle signing tested end-to-end.
- Audit log integrity proof verified by an external Merkle verifier.

**Risks:** SOC 2 evidence pipeline lag; SLSA L2 setup complexity; cosign keyless signing rate limits.

## M5 — Packaging and v1 readiness

**Duration:** 3 weeks.

**Goals:** Cut v1.0.0-rc.1. Ship installable artifacts for every supported platform. Write the operator-facing docs.

**Deliverables:**

- Release workflow that produces:
  - Static linux/amd64 + linux/arm64 (glibc + musl) binaries via `cargo-zigbuild`.
  - macOS universal2 binary, code-signed and notarized.
  - Windows amd64 binary, signed with EV cert (or self-signed for rc, signed for GA).
  - OCI image (distroless base) published to ghcr.io.
  - DEB + RPM packages.
  - Homebrew tap formula.
  - Helm chart published to OCI registry.
  - systemd unit + launchd plist + Windows service installer.
- Quickstart docs at `plans/07-quickstarts/` for each of the six runtimes with copy-paste configs.
- Operational runbook at `docs/runbook.md`: install, configure, validate, monitor, troubleshoot, upgrade, uninstall.
- CHANGELOG starting at v1.0.0-rc.1.
- Licensing and certification plan: confirm Apache 2.0 final, draft CNCF Sandbox application, schedule SOC 2 Type I audit.
- v2 (gateway) design RFC opened.

**Exit criteria:**

- v1.0.0-rc.1 cut from `main` with all artifacts.
- Known gaps published in CHANGELOG.
- Upgrade path to v2 documented at a high level.
- One named external user has tested the rc.

**Risks:** Apple notarization first-time setup; Windows signing cert procurement; Helm chart edge cases.

## Cross-milestone tracking

- **Milestone board sync:** [`docs/milestones/mara-m0-m2-board.md`](../../docs/milestones/mara-m0-m2-board.md) carries execution tickets (M0–M2 plus M1-08+ / M2-08+ engineering-review rows and **M2-16–M2-22** market-gap telemetry rows); keep MOS narrative here and ticket detail there aligned when dates slip.
- **Performance regression budget**: 5% from rolling 30-day baseline; breaches require an issue and a fix or accepted variance.
- **Documentation budget**: every new public type or trait method requires `rustdoc` with at least one example before merge.
- **Coverage budget**: line coverage ≥75% on `mara-core`, ≥60% on adapters/sinks, ≥80% on policy primitives. Tracked via `cargo-llvm-cov`.
- **Issue triage cadence**: weekly during M0–M2, twice weekly during M3–M5.

## After v1.0: v1.x and beyond

- v1.1: Plugin ABI stabilization (`mara_plugin_v1` C ABI for out-of-tree adapter/sink crates). External sink contributions.
- v1.2: Gateway tier RFC implementation start.
- v1.3: Policy marketplace prototype.
- v2.0: Gateway GA, multi-tenant policy distribution, gateway-pushed config.
- v3.0: Hosted control plane (commercial track), SSO/SCIM, fleet management.
