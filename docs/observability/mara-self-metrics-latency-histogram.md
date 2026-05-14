# Self-metrics: `mara_gen_ai_request_duration_ms_*` semantics (M1-11)

This note explains how Mara exposes **request latency** derived from **`mara.ollama.total_duration_ms`** on delivered GenAI completion events, how that maps to Prometheus types, and how to write correct PromQL—especially `histogram_quantile`.

See also [`mara-self-metrics-m1-05.md`](mara-self-metrics-m1-05.md) for the full metric catalog and starter queries.

## Source signal

- **Latency value** (milliseconds) comes from the **`mara.ollama.total_duration_ms`** attribute on **`Completion`** events that count toward `mara_gen_ai_requests_completed_total` (Ollama / LLM proxy path; see `PipelineSelfMetrics::record_delivered` in `crates/mara-core/src/self_metrics.rs`).
- This is **engine-side** duration from Ollama’s JSON payload, not Mara's HTTP proxy wall-clock (split SLOs are M2-14).

## Three related expositions

| Exposition | Type (exposed) | Meaning |
|------------|------------------|--------|
| `mara_gen_ai_request_duration_ms_bucket{le="…"}` | histogram bucket | **Cumulative** counts since process start: for each observation `ms`, every bucket with `le >= ms` is incremented (classic Prometheus upper-bound semantics), including `le="+Inf"`. |
| `mara_gen_ai_request_duration_ms_sum` | histogram `_sum` | Sum of `ms` over the **retained in-memory sample ring** (at most `LATENCY_MAX_SAMPLES` = **4096** most recent latencies). Older samples are dropped from this vector when it overflows. |
| `mara_gen_ai_request_duration_ms_count` | histogram `_count` | Count of samples in that **same ring** (≤ 4096). |

Additionally, **`mara_gen_ai_request_duration_ms_p95`** is a **gauge**: nearest-rank **p95** computed by **sorting** the current ring buffer at scrape time (same values as `_count` / `_sum` window, not the lifetime bucket totals).

### Important: ring buffer vs cumulative buckets

Bucket counters **only increase** until the process restarts; they are **not** trimmed when the sample ring drops old latencies. After more than 4096 completions, **`mara_gen_ai_request_duration_ms_bucket{le="+Inf"}`** can exceed **`mara_gen_ai_request_duration_ms_count`** on the same scrape.

For that reason:

- Prefer **`histogram_quantile`** using **`_bucket`** series with **`rate(...[5m])`** (or `increase`) for **long-window** latency SLOs and dashboards.
- Treat **`_sum` / `_count` / `mara_gen_ai_request_duration_ms_p95`** as a **bounded, recent-window** view of the distribution (useful for “what tail looks like lately” without unbounded memory).

## PromQL: `histogram_quantile`

Standard pattern (per pipeline):

```promql
histogram_quantile(
  0.95,
  sum by (pipeline, le) (
    rate(mara_gen_ai_request_duration_ms_bucket[5m])
  )
)
```

- **`le`** must remain in the `sum` grouping so the quantile estimator sees the full cumulative ladder.
- Use **`rate`** (or **`increase`**) so resets on process restart do not spike quantiles; scrape interval should be stable enough for your chosen range.

The same expression works for other quantiles (0.50, 0.99) by changing the first argument.

## What `_sum` and `_count` are *not*

- They are **not** a second independent histogram: they summarize the **same floating-point samples** used for the **p95 gauge**, not a recomputation from buckets.
- They are **not** guaranteed to match **`+Inf`** bucket totals after the ring buffer has wrapped (see above).

## Operational notes

- **Scrape interval**: shorter intervals give fresher quantiles from `rate()` but more noise; 15–60s is typical.
- **CPU at scrape**: p95 is computed via a full sort of up to 4096 doubles whenever `/metrics` is scraped; see M2-10 for follow-up optimization options.
- **Cardinality**: only the `pipeline` label is used on these series; keep pipeline names bounded.

## Code references

- Histogram buckets: `LATENCY_MS_BUCKETS`, `LATENCY_MAX_SAMPLES` in `crates/mara-core/src/self_metrics.rs`.
- Rendering: `PipelineSelfMetrics::render_samples`, `render_prometheus`.
