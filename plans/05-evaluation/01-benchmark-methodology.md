# Benchmark Methodology

## Executive summary

Mara's performance claims are verifiable. This document defines exactly how throughput, latency, memory, CPU, and durability benchmarks are run, what inputs they use, how results are reported, and how regressions block PRs. The methodology is deliberately public so external reproducers can match it.

## Hardware baselines

- **PR-gate runner (short bench, 5-minute):** GitHub Actions `ubuntu-latest` `c2-standard-2` equivalent (2 vCPU, 8 GiB RAM). Used for fast regression detection.
- **Nightly runner (full bench, 1-hour):** self-hosted runner pinned to a `c5.large` or `c6a.large` AWS instance (2 vCPU, 4 GiB RAM, EBS gp3 SSD). Used for SLO verification.
- **Soak runner (24-hour):** self-hosted runner pinned to a `c6a.xlarge` (4 vCPU, 8 GiB RAM). Used for stability claims.

Local-developer "do not regress" runs are documented for an M1 MacBook (8 perf cores, 16 GB RAM); numbers there are not authoritative but track relative deltas.

## Workload mix

Three workload profiles cover the main use cases:

- **Profile A — Tier A round-trip**: OTLP-in (gRPC, gzip) → policy chain (one regex redaction, sampling rate 1.0) → OTLP-out (gRPC, gzip). 4 KiB events. Mimics Claude Code / Codex / Gemini.
- **Profile B — Tier B JSONL**: JSONL tail (rotation every 100 MB) → policy chain (PII redaction pack) → Loki HTTP sink (gzip). 2 KiB lines. Mimics Cursor + Kimi.
- **Profile C — Mixed**: 60% Profile A, 30% Profile B, 10% Profile C (analytics REST poll, mock server). Mimics multi-runtime fleet.

## Metric definitions (precise)

- **Throughput:** events accepted at adapter divided by wall-clock seconds, averaged over a steady-state window (last 50% of the run after a 30 s warmup).
- **End-to-end latency:** time from `adapter.accept_ts` (set in adapter) to `sink.ack_ts` (set when sink confirms downstream commit). Measured per event; histograms reported at p50, p90, p99, p99.9.
- **Memory:** RSS sampled from `procfs` (`/proc/<pid>/status:VmRSS`) every second; max and p99 reported.
- **CPU:** Mara process CPU% (from `procfs` `proc/<pid>/stat`) sampled every second; mean of last 50%.
- **Crash durability:** Events accepted but not yet sink-acked at SIGKILL time, divided by event acceptance rate; lower bound on data loss.

## Test scenarios

### Scenario 1 — Sustained throughput at SLO load (NFR-1.1)

- Profile A, 50,000 EPS target, 60 minutes.
- **Pass:** sustained ≥ 50,000 EPS measured throughput; ≤ 0.01% non-policy-rejected event loss.
- **Reported:** mean, p99 latency, max RSS, mean CPU.

### Scenario 2 — Latency at lower loads (NFR-1.2)

- Profile A, 10,000 EPS, 30 minutes.
- **Pass:** p99 ≤ 2s, p99.9 ≤ 5s.

### Scenario 3 — Memory ceiling (NFR-1.4)

- Profile B, 10,000 EPS, 30 minutes.
- **Pass:** max RSS ≤ 512 MiB.

### Scenario 4 — Idle footprint (NFR-1.4)

- Mara running with default config, no input load, 5 minutes.
- **Pass:** RSS ≤ 128 MiB after 60 seconds.

### Scenario 5 — Startup time (NFR-1.6)

- Time from `mara run` to first event accepted (Profile A producer ready, no WAL replay).
- **Pass:** ≤ 500 ms.

### Scenario 6 — WAL replay (NFR-1.7)

- Pre-populate WAL with 1 GiB of events; restart Mara; measure time to replay.
- **Pass:** ≤ 10 seconds.

### Scenario 7 — Crash durability (NFR-2.2)

- Profile A at 50,000 EPS; SIGKILL after 5 minutes; restart; measure events lost.
- **Pass:** ≤ 1 s of events lost.

### Scenario 8 — Sink-outage tolerance (NFR-2.3)

- Profile A at 10,000 EPS; sink simulated as offline for 10 minutes; measure event loss and recovery time.
- **Pass:** zero loss; events replayed in ≤ 30 s after sink recovery; WAL stays within configured budget.

### Scenario 9 — Reliability soak (TSL-4)

- Profile C, 24 hours, with chaos injection: random adapter kill every 30 min, random policy-WASM trap every 60 min, random sink 30-second hang every 15 min.
- **Pass:** ≥ 99.9% delivery rate; agent does not crash; metrics reflect every event.

## Reporting format

Each bench job emits a single JSON document with the schema:

```json
{
  "scenario": "sustained_throughput",
  "profile": "A",
  "duration_s": 3600,
  "throughput": { "mean": 51234, "median": 51100, "min": 49800, "max": 52900 },
  "latency_ns": { "p50": 12_000_000, "p90": 320_000_000, "p99": 1_400_000_000, "p999": 4_800_000_000 },
  "memory_bytes": { "rss_max": 480_000_000, "rss_p99": 460_000_000 },
  "cpu_percent": { "mean": 78.4 },
  "event_loss": { "policy_rejected": 0, "other": 12 },
  "git_sha": "abc1234",
  "runner": "self-hosted-c5large",
  "started_at": "2026-05-12T18:00:00Z"
}
```

Results are committed to a `bench-results/` branch (orphan branch) keyed by date and SHA, plus published to a Grafana dashboard fed from that branch.

## Regression detection

- Each metric has a rolling 30-day baseline (median of last 30 nightly runs on the same scenario + runner).
- A PR-level smoke bench (5 minute version of Scenario 1) blocks the PR if throughput drops > 5% from baseline OR if RSS rises > 5% OR if p99 latency rises > 10%.
- Nightly runs publish a regression report; sustained regressions over 3 consecutive nights open a tracking issue.

## Repeatability rules

- Single-tenant runner (no other jobs running concurrently).
- CPU governor set to `performance` on Linux nightly runner.
- Disk I/O isolated to a dedicated EBS volume.
- Network: a mock OTLP sink and mock Loki sink run on the same host (no network variability).

## Real-world calibration

In addition to synthetic benchmarks, Mara is calibrated against real-runtime fixture replay:

- **Claude Code session replay:** a recorded 30-minute coding session through Mara measured end-to-end.
- **Codex CI replay:** a recorded non-interactive `codex exec --json` run.
- **Cursor agent replay:** a recorded series of hook events captured from a real Cursor session.

Calibration runs are reported but do not gate PRs; they're a "sanity check that synthetic ≈ real."

## Anti-benchmarks (what we deliberately do not measure)

- Single-event latency (microbenchmark distraction).
- Maximum-burst throughput (not the SLO).
- Best-case-no-policy latency (we always run the policy chain in benches).
- Memory under no-load with sinks disabled (we measure realistic configurations).

## Public reproducibility

The bench harness in `benches/` is itself part of the repo. External users can run:

```bash
cargo bench --bench pipeline_sustained -- --duration 3600 --profile A --eps 50000
```

…and replicate the published number on equivalent hardware within ≈ 10%.
