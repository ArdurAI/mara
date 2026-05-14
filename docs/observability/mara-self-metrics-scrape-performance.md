# Self-metrics scrape cost (M2-10)

`/metrics` avoids sorting thousands of latency samples on each scrape. Engine (`mara.ollama.total_duration_ms`), gateway (`mara.proxy.gateway_duration_ms`), and fan-out timings use **fixed histogram buckets**; `_sum` / `_count` and `_p95` gauges are derived from those buckets in **O(buckets)** time per pipeline.

## Operator guidance

- Prefer scrape intervals of **15–60s** for development; sub-second scrapes are rarely useful for LLM workloads.
- For PromQL, pair `histogram_quantile` on `_bucket` series with the documented bucket layout in `crates/mara-core/src/self_metrics.rs` (`LATENCY_MS_BUCKETS`).

## Regression guard

Unit test `render_many_completions_without_sorting_large_vector` (in `self_metrics.rs`) exercises 3k delivered events and full `render_prometheus` output generation without allocating a per-scrape sort buffer.
