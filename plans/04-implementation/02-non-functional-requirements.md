# Non-functional Requirements

## Executive summary

Non-functional requirements (NFRs) define how Mara behaves under load, failure, attack, and time. They translate directly into CI gates, SLO definitions, and acceptance criteria. Each NFR is testable.

## NFR-1 — Performance (edge agent)

Baseline target hardware for SLO measurement: a single x86_64 core at 2.4 GHz (e.g., `c5.large`), 4 KiB average event size, 50 % policy-chain hit rate (one regex redaction stage active).

- **NFR-1.1** Sustained throughput: ≥**50,000 events/sec** ingest-to-export for 60 minutes without dropping non-policy-rejected events.
- **NFR-1.2** Ingest-to-export latency p99 ≤ **2 seconds** at SLO throughput.
- **NFR-1.3** Ingest-to-export latency p999 ≤ **5 seconds** at SLO throughput.
- **NFR-1.4** Memory: ≤**128 MiB RSS** at idle, ≤**512 MiB RSS** at sustained SLO load.
- **NFR-1.5** CPU: ≤**5 %** of one core at 10,000 EPS steady-state.
- **NFR-1.6** Startup time: ≤**500 ms** from `mara run` to first event accepted (cold start, no WAL replay).
- **NFR-1.7** WAL replay time: ≤**10 seconds** for 1 GiB WAL on local SSD.

CI gate: `cargo bench --bench pipeline_sustained` and `cargo bench --bench memory_ceiling` produce machine-readable JSON results; PR is blocked if any NFR-1 number regresses by more than 5 % from the trailing 30-day baseline.

## NFR-2 — Reliability and durability

- **NFR-2.1** Zero event loss on **graceful shutdown** (SIGTERM with default 30 s drain timeout).
- **NFR-2.2** ≤1 second of event loss on **ungraceful shutdown** (SIGKILL or power loss) when WAL is enabled.
- **NFR-2.3** Sink failures of up to 10 minutes MUST NOT cause upstream event loss when WAL is enabled and within size budget.
- **NFR-2.4** At-least-once delivery semantics per sink (sink-side dedup is the caller's job).
- **NFR-2.5** ≥99.9 % of accepted events delivered to ≥1 configured sink under SLO load.

## NFR-3 — Security

- **NFR-3.1** No phone-home telemetry by default. Default install with no configured sinks MUST produce zero outbound traffic.
- **NFR-3.2** Prompt and raw-API-body capture MUST be opt-in per pipeline; default off.
- **NFR-3.3** Each runtime's ZDR toggle MUST be honored agent-side. Capture MUST be disabled when the upstream runtime ZDR flag is set.
- **NFR-3.4** All network sinks MUST support TLS 1.3 (fallback to 1.2 with explicit opt-in only).
- **NFR-3.5** All `https://` sink URLs MUST verify certificates by default. `insecure_skip_verify` MUST be a per-sink explicit opt-in with a startup log warning.
- **NFR-3.6** mTLS MUST be supported on OTLP receiver, OTLP sender, gateway client (v2), and webhook sink.
- **NFR-3.7** Secrets in configuration MUST never appear in logs, error messages, metrics, or self-telemetry traces.
- **NFR-3.8** Policy bundles MUST be `cosign`-verifiable before load; unsigned bundles require explicit `--allow-unsigned-policy` flag.
- **NFR-3.9** Mara MUST run as an unprivileged user by default; root MUST NOT be required for any v1 functionality.
- **NFR-3.10** File system reads MUST be capability-scoped: a JSONL adapter configured to read `~/.claude/projects/` MUST NOT be able to read `/etc/shadow`.

## NFR-4 — Supply chain

- **NFR-4.1** Every tagged release MUST publish a CycloneDX SBOM and an SPDX SBOM.
- **NFR-4.2** Every tagged release MUST publish SLSA Level 2 provenance attestations.
- **NFR-4.3** Every tagged release MUST be `cosign`-signed (keyless via Fulcio + Rekor).
- **NFR-4.4** Builds MUST be reproducible on best-effort basis (Linux first, others later).
- **NFR-4.5** Every dependency MUST be tracked in `cargo-deny` allowlist; PRs adding deps MUST pass `cargo deny check`.
- **NFR-4.6** `cargo audit`, `cargo deny`, `cargo vet`, and OSV scanner MUST pass in CI before merge.
- **NFR-4.7** Container images MUST be built from distroless base; `trivy fs` and `trivy image` MUST report zero high/critical CVEs in published images.

## NFR-5 — Compatibility

- **NFR-5.1** macOS 13 Ventura or later (both Intel and Apple Silicon).
- **NFR-5.2** Linux: glibc 2.31+ (Ubuntu 20.04+, RHEL 9+, Debian 11+) and musl (Alpine 3.16+).
- **NFR-5.3** Windows 10 21H2+ and Windows Server 2019+ (amd64 only in v1).
- **NFR-5.4** Kubernetes 1.27+ (DaemonSet pattern requires container runtime log access).
- **NFR-5.5** Rust toolchain pinned via `rust-toolchain.toml`; upgrade cadence quarterly.
- **NFR-5.6** OTel `gen_ai.*` semconv pinned per release; semconv version bump requires ADR.

## NFR-6 — Configuration ergonomics

- **NFR-6.1** Default configuration MUST be valid (no required fields without sensible defaults).
- **NFR-6.2** `mara run` with no config MUST start, run an OTLP receiver on localhost, and dump to stderr — useful for "does it work" testing.
- **NFR-6.3** `mara validate` MUST complete in ≤100 ms for typical configurations.
- **NFR-6.4** Configuration error messages MUST include: file path, line/column, the value seen, the expected value/type, and a suggested fix.

## NFR-7 — Operational ergonomics

- **NFR-7.1** Single binary, no runtime dependencies (no JVM, no Python, no Node, no shared libraries beyond OS standard).
- **NFR-7.2** Binary size ≤30 MiB for the macOS universal2 release artifact (target; larger acceptable initially).
- **NFR-7.3** Log lines emitted by Mara MUST be parseable with `jq` when `--log-format json` is set.
- **NFR-7.4** `mara diag` output MUST be both human-readable and machine-parseable (`--output json` flag).

## NFR-8 — Backward compatibility

- **NFR-8.1** Configuration schema MUST be versioned. v1 configs MUST be loadable by v1.x agents.
- **NFR-8.2** Canonical schema additions MUST be additive within a major version.
- **NFR-8.3** Breaking changes MUST go through a deprecation cycle: announce in N, warn in N+1, remove in N+2 (where N is a minor version).

## NFR-9 — Internationalization

- **NFR-9.1** UTF-8 throughout. No code page assumptions.
- **NFR-9.2** Log file paths and content MUST be UTF-8 safe on Windows (no GBK or Shift-JIS surprises).
- **NFR-9.3** PII redaction packs MUST handle non-ASCII names, addresses, and identifiers. Default packs target English, Mandarin, Hindi, Arabic, Spanish, French, German, Japanese; pluggable for others.

## NFR-10 — Observability of the agent itself

- **NFR-10.1** Self-telemetry overhead MUST be ≤2 % of agent CPU and ≤8 MiB RSS.
- **NFR-10.2** Self-metrics MUST follow Prometheus naming conventions (`mara_pipeline_events_total`, `mara_sink_errors_total`, etc.).
- **NFR-10.3** Self-traces MUST not capture user event content under any default configuration.

## NFR-11 — Time-to-X SLOs (developer experience)

- **NFR-11.1** Time-to-first-event for Persona 2 (indie dev): ≤**5 minutes** from `brew install` to first event in chosen sink.
- **NFR-11.2** Time-to-integrate a new AI runtime: ≤**2 weeks** scaffold (Tier B equivalent), ≤**4 weeks** production-ready (Tier A equivalent).
- **NFR-11.3** Time-to-fix-a-CVE: ≤**7 days** for high/critical, ≤**30 days** for medium, ≤**90 days** for low.

## NFR-12 — License compliance

- **NFR-12.1** Mara core and all first-party crates MUST be Apache 2.0.
- **NFR-12.2** No AGPL or SSPL dependencies in compiled core.
- **NFR-12.3** GPL-3 dependencies acceptable only at runtime via dynamic loading (and even then prefer alternatives).
- **NFR-12.4** Mozilla-licensed (MPL 2.0) deps are acceptable.
- **NFR-12.5** Patent-grant-equivalent deps preferred (Apache 2.0, MPL 2.0, ISC).
- **NFR-12.6** Third-party WASM policy bundles can be any license; their license is the bundle author's responsibility, surfaced in the policy registry metadata.
