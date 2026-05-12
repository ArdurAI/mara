# Success Metrics

## Executive summary

How we measure whether Mara is succeeding. The metrics fall into four buckets: technical SLOs (does the agent meet its performance and reliability targets), feature coverage (does it actually do what the personas need), adoption (do real users install it), and ecosystem health (does the project remain open, governed, and contributable).

Each metric has a measurement methodology and a v1, 6-month, and 12-month target.

## Technical SLO metrics

### TSL-1 — Sustained throughput

- **Definition:** events per second sustained for 60 minutes on a single CI runner core (4 KiB events, default policy chain with PII redaction enabled, OTLP-in → OTLP-out).
- **v1 target:** ≥50,000 EPS.
- **6mo:** ≥75,000 EPS.
- **12mo:** ≥100,000 EPS.
- **Measurement:** nightly `cargo bench --bench pipeline_sustained` produces machine-readable JSON; published to a metrics dashboard.

### TSL-2 — Ingest-to-export latency

- **Definition:** time from event accepted at adapter to first sink-confirmed delivery.
- **v1 target:** p99 ≤ 2s, p999 ≤ 5s.
- **6mo:** p99 ≤ 1s, p999 ≤ 3s.
- **12mo:** p99 ≤ 500ms, p999 ≤ 2s.
- **Measurement:** `criterion` benchmarks with the OTLP loopback test.

### TSL-3 — Memory ceiling

- **Definition:** RSS at sustained SLO load.
- **v1 target:** ≤ 512 MiB.
- **6mo:** ≤ 384 MiB.
- **12mo:** ≤ 256 MiB.
- **Measurement:** `procfs` sampling in the sustained-throughput bench job.

### TSL-4 — Delivery reliability

- **Definition:** percentage of accepted events delivered to ≥1 configured sink under SLO load over a 24-hour run.
- **v1 target:** ≥ 99.9%.
- **6mo:** ≥ 99.95%.
- **12mo:** ≥ 99.99%.
- **Measurement:** weekly soak test with synthetic load + chaos injection.

### TSL-5 — Crash durability

- **Definition:** events lost during a SIGKILL test at sustained SLO load with WAL enabled.
- **v1 target:** ≤ 1 second of events.
- **6mo:** ≤ 500ms.
- **12mo:** ≤ 100ms.
- **Measurement:** dedicated crash-durability bench.

## Feature coverage metrics

### FC-1 — Runtime coverage

- **Definition:** number of runtimes meeting tier-appropriate acceptance per [`../05-evaluation/02-compatibility-matrix-spec.md`](../05-evaluation/02-compatibility-matrix-spec.md).
- **v1 target:** 6 of 6 (100%).
- **6mo:** 6 of 6 + at least 2 additional community-contributed runtimes.
- **12mo:** 10+ runtimes.
- **Measurement:** compatibility matrix published with each release.

### FC-2 — Sink coverage

- **Definition:** number of supported sinks shipping in tree with first-party tests.
- **v1 target:** 10 (per FR-7).
- **6mo:** 12.
- **12mo:** 15+, with at least 3 external-maintained.
- **Measurement:** crates audit per release.

### FC-3 — Schema completeness

- **Definition:** percentage of canonical fields populated for known event kinds across Tier A runtimes.
- **v1 target:** ≥ 98%.
- **6mo:** ≥ 99%.
- **12mo:** ≥ 99.5%.
- **Measurement:** schema-coverage analysis script run over the smoke test golden files.

### FC-4 — Policy primitive coverage

- **Definition:** number of built-in policy primitives shipping in tree.
- **v1 target:** 7 (redact, allow, deny, sample, rate_limit, transform, classify, route).
- **6mo:** 10+ including: hash-and-replace, deterministic-tokenize, k-anonymize.
- **12mo:** A policy marketplace prototype with at least 5 community bundles.

## Adoption metrics

### AD-1 — GitHub stars

- **v1 target:** 500 by v1.0.0-rc.1.
- **6mo:** 2,500.
- **12mo:** 10,000.

### AD-2 — Production users (named)

- **Definition:** organizations who have publicly stated they run Mara in production.
- **v1 target:** ≥ 1.
- **6mo:** ≥ 5.
- **12mo:** ≥ 25.
- **Measurement:** ADOPTERS.md file maintained in the repo.

### AD-3 — Container image pulls

- **v1 target:** 1,000/mo.
- **6mo:** 10,000/mo.
- **12mo:** 100,000/mo.

### AD-4 — Homebrew installs

- **v1 target:** N/A (no GA yet).
- **6mo:** ≥ 500/wk install-events.
- **12mo:** ≥ 5,000/wk.
- **Measurement:** Homebrew analytics (when tap is sufficiently mature to be on the analytics dashboard).

## Ecosystem health metrics

### EH-1 — External contributors

- **v1 target:** ≥ 3 external (non-ArdurAI) PR authors merged.
- **6mo:** ≥ 15.
- **12mo:** ≥ 50.

### EH-2 — Mean time to triage

- **Definition:** median time from issue open to first maintainer response.
- **v1 target:** ≤ 3 business days.
- **6mo:** ≤ 2 business days.
- **12mo:** ≤ 1 business day.

### EH-3 — Open issue half-life

- **Definition:** median time issues stay open before being resolved, closed-without-fix, or labeled `wontfix`.
- **v1 target:** ≤ 30 days.
- **6mo:** ≤ 21 days.
- **12mo:** ≤ 14 days.

### EH-4 — Release cadence

- **v1 target:** quarterly minor releases, monthly patch releases.
- **6mo:** same.
- **12mo:** same.
- **Measurement:** git tag history.

### EH-5 — Upstream contributions

- **Definition:** PRs Mara contributors have merged into OpenTelemetry semantic conventions, OpenTelemetry Collector, or other upstream OSS projects relevant to Mara's mission.
- **v1 target:** ≥ 3.
- **6mo:** ≥ 10.
- **12mo:** ≥ 25.

## Security metrics

### SEC-1 — Time to fix CVEs

- **Definition:** time from CVE assignment to released fix.
- **High/Critical:** ≤ 7 days.
- **Medium:** ≤ 30 days.
- **Low:** ≤ 90 days.

### SEC-2 — Open security findings on `main`

- **Definition:** any `cargo audit`, `cargo deny`, OSV scanner, or Trivy finding above `low` severity that has not been triaged.
- **Target at all times:** zero.

### SEC-3 — SLSA level

- **v1:** SLSA Level 2.
- **6mo:** SLSA Level 2 verified by third party.
- **12mo:** SLSA Level 3.

### SEC-4 — Compliance milestones

- **v1:** SOC 2 Type I control mapping draft.
- **6mo:** SOC 2 Type I audit complete.
- **12mo:** SOC 2 Type II audit complete; CNCF Sandbox accepted.

## Persona-aligned success criteria

For each persona in [`../03-value-proposition/04-target-personas.md`](../03-value-proposition/04-target-personas.md), success looks like:

- **Persona 2 (Rohan, indie):** time from `brew install mara` to first event in chosen sink ≤ 5 minutes. Measured in user testing during M5.
- **Persona 1 (Priya, platform):** Helm chart deploys to a 3-node test cluster; OTLP receive + Loki sink works; cardinality below configurable threshold. Measured in M3.
- **Persona 3 (Sasha, compliance):** SOC 2 Type I mapping complete; audit log integrity proof verified by external Merkle verifier. Measured in M4.
- **Persona 4 (Mira, enterprise):** v3 deferred metric.

## Anti-metrics (things we explicitly do not optimize for)

- **Lines of code.** Lean is better; we count complexity, not lines.
- **Feature checkboxes vs competitors.** We pick the narrow seat; feature-for-feature parity with Datadog is not a goal.
- **Marketing-driven adoption.** Real users beat synthetic GitHub stars; AD-2 (named production users) is the harder, truer metric.

## Reporting cadence

- Technical SLO metrics: nightly bench, weekly soak, dashboard published.
- Feature coverage: per release.
- Adoption: monthly project update post.
- Ecosystem health: quarterly retrospective.
- Security: real-time CI; quarterly external review.
