# Test Strategy

## Executive summary

Tests are organized in five layers: unit tests inside each crate, integration tests at the workspace level, golden-file tests per runtime, soak/perf tests in CI nightly, and external-fixture tests against real (or recorded) AI-runtime emissions. The pyramid is wide at the bottom and narrow at the top, but Mara invests deliberately in the middle (integration + golden) because the value of the project lives in the contract between adapters, the canonical schema, and sinks.

## Layer 1 — Unit tests (per crate)

- **Location:** `crates/<crate>/src/**/tests.rs` modules and `crates/<crate>/tests/` integration directories.
- **Coverage target:** ≥ 75% line coverage on `mara-core`; ≥ 60% on adapters/sinks; ≥ 80% on policy primitives.
- **Style:** `#[tokio::test]` for async, `proptest` for property-based testing of normalization and policy primitives, `insta` for snapshot tests of canonical events.
- **CI gate:** `cargo test --workspace --all-targets` on every PR.

## Layer 2 — Integration tests

- **Location:** `tests/` at workspace root.
- **Scope:** wire multiple crates together. Examples:
  - OTLP receiver → policy chain (redact) → OTLP sender round-trip.
  - JSONL tail → policy chain → file sink with rotation.
  - Hooks adapter → policy → Loki sink with mock Loki via `wiremock-rs`.
  - WAL persistence across simulated process restart.
- **Style:** spin up Mara in-process or via `assert_cmd`; use `testcontainers-rs` for sinks that need real backends (Kafka, Elasticsearch, Splunk).
- **CI gate:** integration job that runs on every PR with Docker-in-Docker for `testcontainers`.

## Layer 3 — Golden-file tests per runtime

- **Location:** `crates/mara-runtimes/<runtime>/tests/golden/`.
- **Scope:** input fixture (recorded JSONL transcript, recorded OTLP request body, recorded hook stdio JSON) → expected canonical event stream.
- **Style:** `insta` snapshot tests with reviewable diffs. Snapshots are normalized for non-deterministic fields (timestamps replaced with stable ordinals; UUIDs replaced with `<uuid:N>` markers).
- **Maintenance:** when a runtime ships a payload-format change, regenerate snapshots, review diff in PR, link to upstream changelog.
- **CI gate:** part of `cargo test --workspace`.

## Layer 4 — Soak and performance tests (nightly)

- **Location:** `benches/` (criterion) and `tests/soak/` (long-running).
- **Scope:**
  - 1-hour sustained 50k EPS test (NFR-1.1).
  - 24-hour reliability soak with chaos injection (TSL-4).
  - SIGKILL durability test (NFR-2.2).
  - Memory-ceiling test (NFR-1.4).
- **Style:** dedicated GitHub Actions self-hosted runner with consistent hardware (or a sized cloud runner pinned). Results published to a dashboard.
- **CI gate:** nightly; PR-level perf check uses a short version (5 min sustained, 100 MB cap).

## Layer 5 — External fixture tests

- **Location:** `tests/external/` with fixtures gitignored if too large.
- **Scope:** recorded sessions from each of the six target runtimes (with prompt content redacted before commit).
- **Style:** end-to-end pipeline runs against recorded captures; assertions on a subset of canonical-event field invariants.
- **CI gate:** runs in nightly + on `release-*` branches.

## Special test categories

### Property-based tests (proptest)

- Normalizer: any valid `gen_ai.*` OTLP payload round-trips through canonical and back.
- Redaction: regex packs never emit the regex source pattern as a literal in output.
- Sampling: head/tail samplers preserve trace context contiguity.

### Snapshot tests (insta)

- Canonical events for golden inputs.
- Configuration validation error messages (so error wording doesn't regress).
- CLI help output.

### Fuzz tests (cargo-fuzz)

- OTLP receiver: malformed protobuf, malformed HTTP/JSON, oversized payloads.
- JSONL adapter: malformed JSON, mixed encodings, gigantic single lines.
- Hooks adapter: malformed JSON over stdio, stdin closed unexpectedly.
- Config parser: arbitrary TOML/YAML, schema fuzzing.
- WASM policy: untrusted module loading (ensure sandbox does not escape).
- **CI gate:** fuzz jobs run nightly for 30 min per target; corpus committed.

### Mutation tests (cargo-mutants)

- Run periodically (monthly) on `mara-core` to verify test suite catches mutations.

### Contract tests (where applicable)

- OTel `gen_ai.*` semconv conformance: a test that loads the pinned semconv YAML and validates that Mara's generated types match.
- Sink contracts: each sink has a contract test against its protocol (e.g., Loki HTTP push API expectations).

## Test data and fixtures

- Synthetic fixtures are checked into the repo at `tests/fixtures/`.
- Real recorded fixtures (post-redaction) are checked into `tests/external/`. If size is prohibitive, host on GitHub Releases and fetch in CI.
- Fixture generation scripts live in `tests/fixtures/gen/` so fixtures are reproducible.

## CI matrix

PR-blocking checks (must pass to merge):

- `cargo fmt --all --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace --all-targets` on linux/amd64, linux/arm64, macos-latest, windows-latest
- `cargo bench --bench pipeline_sustained -- --short` (smoke perf, 5 min)
- `cargo deny check`
- `cargo audit`
- license header check
- conventional commit lint
- DCO sign-off check

Nightly (not PR-blocking but tracked):

- 1-hour sustained perf test
- 24-hour soak with chaos
- mutation testing (monthly)
- fuzz corpus expansion
- external-fixture replay
- Trivy container scan
- SBOM generation
- SLSA provenance dry-run

## Regression budget

- Per NFR-1, perf regressions > 5% from rolling 30-day baseline block PRs.
- Per NFR-8, public-API regressions block PRs unless an ADR accompanies them.
- Per NFR-4, supply-chain regressions (new high/critical CVE) block releases.

## Test ownership

- `mara-core`: Core team.
- `mara-schema`: Schema team.
- `mara-policy`: Security team.
- `mara-adapters/<x>`: Integrations team.
- `mara-runtimes/<x>`: Integrations team + runtime-specific designated reviewer.
- `mara-sinks/<x>`: Sinks team (initially Core, splits later).

## When a test fails in CI

1. Triage within 1 business hour during work week.
2. Flaky tests get a `@flaky` annotation and an issue; three repeated flakes in a week → quarantine + rewrite.
3. Real failures block PR until fixed.

## Test anti-patterns we explicitly avoid

- **`thread::sleep`** in async tests — use `tokio::time::pause()` and `tokio::time::advance()`.
- **Real network in unit tests** — use `wiremock-rs` or `mockito`.
- **Shared global state** — every test sets up its own pipeline instance.
- **Skipping tests on Windows "because Windows"** — fix the test or fix the code; don't skip.
- **Hand-written JSON in fixtures when the type system can generate it** — use `serde_json::to_string_pretty` from typed Rust values.
