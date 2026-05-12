# MVP — Test and Improve Loop

## Executive summary

Once the MVP ships, we need a tight, repeatable loop that catches regressions, validates the value claim against real users, and surfaces what to build next. This document describes the loop in concrete terms: what runs, when it runs, who looks at it, and what the exit signals are.

The loop is structured as three nested cadences: **per-PR** (minutes), **nightly** (1 hour), and **per-release** (weekly). Each catches a different class of regression at a different cost.

## The three cadences

### Per-PR (target: green in < 15 minutes)

Runs on every pull request before merge. Fast feedback, narrow scope.

- `cargo fmt --all --check`.
- `cargo clippy --workspace --all-targets -- -D warnings`.
- `cargo test --workspace --all-targets`.
- `cargo bench --bench pipeline_smoke -- --short` — 60-second smoke at 10k EPS. Throughput ≥ 8k EPS, RSS ≤ 384 MiB, p99 latency ≤ 3s. PR fails if any number regresses > 5 % from the rolling 7-day baseline.
- `cargo deny check` — license + advisory + bans.
- `cargo audit` — RustSec.
- Quickstart-scripted test: `tests/quickstart_claude_code.rs` runs the documented setup commands in a tempdir on macOS and Linux runners.

**Exit signal:** all checks green; one human reviewer LGTM.

### Nightly (target: complete in < 1 hour)

Runs on the `main` branch on a schedule. Catches what PR-gate is too fast to catch.

- 15-minute sustained-throughput bench at 10k EPS. Track p50 / p99 / p999 latency, RSS, CPU. Publish to `bench-results/` orphan branch.
- 5-minute sustained-throughput bench at 25k EPS (stretch target). Pass/fail loose; we just want to see the trend.
- Chaos injection test: kill the Mara process during the 10-minute soak; restart; assert ≤ 1 second of events lost from the in-memory queue (note: MVP has no WAL; this floor is the in-memory channel depth).
- Full smoke against each runtime preset (only Claude Code at MVP; others are skipped with "not in MVP" annotation that will activate post-MVP).
- `cargo audit` + OSV scanner against the full lockfile.
- `trivy fs .` against the workspace + `trivy image ghcr.io/ardurai/mara:latest`.

**Exit signal:** automated GitHub Issue opened with results; on regression, the team triages in the morning standup.

### Per-release (target: weekly during MVP iteration)

Runs when cutting a release candidate (typically Friday afternoons).

- Full nightly bench suite plus a 1-hour soak.
- Manual persona acceptance test: a human external to ArdurAI runs the Claude Code quickstart on a clean machine. Time-to-first-event ≤ 5 minutes. Friction points captured as issues.
- Compat matrix update: `docs/compat-matrix.md` cells for Claude Code refreshed against the release.
- Release-notes drafting: PR titles since last tag aggregated into `CHANGELOG.md`.
- SBOM + provenance verification: `cosign verify-blob` + `slsa-verifier verify-artifact` succeed against all release artifacts.

**Exit signal:** tag pushed; release workflow produces signed artifacts; release notes posted.

## What we measure

### Throughput / latency / memory

Three metrics dominate the dashboard:

- `mvp_throughput_eps` — events per second sustained on the smoke bench.
- `mvp_p99_latency_ms` — ingest-to-export p99.
- `mvp_rss_max_bytes` — max RSS during the bench.

Tracked daily; trend visible on a single Grafana panel. PR-gate thresholds are 5 % deltas from the rolling 7-day median.

### Reliability

- `mara_pipeline_events_total{outcome="dispatched"}` / `mara_pipeline_events_total{outcome="dropped"}` — drop rate.
- `mara_sink_errors_total` per sink.
- `mara_policy_traps_total` — policy panics or timeouts.

### Adoption (post-release)

- GitHub stars (vanity, but tracked).
- `homebrew analytics` install events.
- `ghcr.io/ardurai/mara` image pulls.
- Unique IP count to the optional public `mara stats` opt-in endpoint (default off — see `SC-7`).
- ADOPTERS.md PRs.

### Quality (anti-vanity)

- Mean time to triage from issue open → first maintainer response. Target ≤ 3 business days during MVP iteration.
- Open issue half-life. Target ≤ 21 days during MVP iteration.
- External contributors (non-ArdurAI) PRs merged. Target ≥ 1 during MVP iteration.

## Dogfooding

ArdurAI engineers running Claude Code locally route their telemetry through Mara from week 4 onward. Two reasons:

1. Real-world stress test on the team's daily-use machines.
2. Forces fast feedback on UX friction — anything that annoys the team gets fixed quickly because the team has to live with it.

The dogfooding instance ships to a Mara-internal Grafana Cloud account (separate from any external observability). Aggregate metrics from dogfooding are published in the weekly retro, never the raw events.

## Improve loop

When something is wrong, we follow the same loop:

1. **Reproduce** in a unit or integration test. If we can't reproduce, we don't ship a fix; we improve telemetry until we can.
2. **Fix** in a small PR.
3. **Regression test** added to the suite so it never re-occurs.
4. **Backport** if the issue is severe and a previous tagged release is widely used (post-MVP-only concern; pre-MVP we move forward).
5. **Postmortem** for any incident affecting two or more external users. Published under `docs/security-postmortems/<date>-<slug>.md` (security) or `docs/postmortems/<date>-<slug>.md` (non-security).

## When to declare MVP "done iterating"

MVP iteration closes when:

1. All eight sign-off criteria from [`01-scope-and-decision-criteria.md`](01-scope-and-decision-criteria.md) are green for two consecutive nightlies.
2. At least three external (non-ArdurAI) production users have validated the quickstart and given a green light on `ADOPTERS.md`.
3. The 7-day rolling perf baseline has held within 5 % for two weeks.
4. No P0/P1 issues open.

At that point we either:

- Begin Option B (gRPC + Cursor + Splunk + Windows), or
- Declare `v0.2.0` (drop the `-alpha`) and continue iterating.

That decision is driven by adoption signals from the dogfooding + external-user feedback loops.

## Cross-references

- [`06-mvp-implementation-plan.md`](06-mvp-implementation-plan.md) — what we built that we're now testing.
- [`../04-implementation/08-success-metrics.md`](../04-implementation/08-success-metrics.md) — long-horizon metric definitions.
- [`../05-evaluation/01-benchmark-methodology.md`](../05-evaluation/01-benchmark-methodology.md) — full bench methodology.
- [`../05-evaluation/02-compatibility-matrix-spec.md`](../05-evaluation/02-compatibility-matrix-spec.md) — how the matrix is maintained.
