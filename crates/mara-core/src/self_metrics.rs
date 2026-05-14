//! Derived workload metrics for Mara self-telemetry (M1-05).
//!
//! Aggregates selected `gen_ai` proxy events after the policy chain and exposes
//! Prometheus text exposition for `GET /metrics`.

use std::fmt::Write as _;
use std::sync::atomic::{AtomicU64, Ordering};

use mara_schema::{AttrValue, CostConfidence, Event, EventKind, SourceRuntime};

/// Histogram bucket upper bounds in milliseconds (Prometheus cumulative `_bucket`).
pub const LATENCY_MS_BUCKETS: &[f64] = &[
    5.0, 10.0, 25.0, 50.0, 100.0, 250.0, 500.0, 1000.0, 2500.0, 5000.0, 10_000.0,
];

const INF_BUCKET: usize = LATENCY_MS_BUCKETS.len();

/// Nearest-rank percentile: `samples` sorted ascending. `p` in (0, 1].
#[must_use]
pub fn percentile_sorted(samples: &[f64], p: f64) -> Option<f64> {
    if samples.is_empty() || !p.is_finite() || p <= 0.0 {
        return None;
    }
    let n = samples.len();
    let k = (p * n as f64).ceil() as usize;
    let idx = k.saturating_sub(1).min(n - 1);
    Some(samples[idx])
}

#[must_use]
fn escape_label_value(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

#[must_use]
fn proxy_style_error(ev: &Event) -> bool {
    ev.attributes.contains_key("mara.proxy.failure_kind")
        || ev.attributes.contains_key("http.status_code")
}

#[must_use]
fn is_llm_proxy_gen_ai(ev: &Event) -> bool {
    matches!(ev.resource.source_runtime, Some(SourceRuntime::Ollama))
        || ev.gen_ai.system.as_deref() == Some("ollama")
        || ev.scope.name.contains("llm-proxy")
}

fn record_ms_into_buckets(
    ms: f64,
    buckets: &[AtomicU64; INF_BUCKET + 1],
    sum_micro: &AtomicU64,
    count: &AtomicU64,
) {
    if !ms.is_finite() || ms < 0.0 {
        return;
    }
    let micro = (ms * 1_000_000.0).round() as u64;
    sum_micro.fetch_add(micro, Ordering::Relaxed);
    count.fetch_add(1, Ordering::Relaxed);
    for (i, &upper) in LATENCY_MS_BUCKETS.iter().enumerate() {
        if ms <= upper {
            buckets[i].fetch_add(1, Ordering::Relaxed);
        }
    }
    buckets[INF_BUCKET].fetch_add(1, Ordering::Relaxed);
}

/// p95 from per-bucket cumulative counts (same layout as [`record_ms_into_buckets`]).
#[must_use]
fn p95_from_hist(bucket_counts: &[AtomicU64; INF_BUCKET + 1]) -> f64 {
    let total = bucket_counts[INF_BUCKET].load(Ordering::Relaxed);
    if total == 0 {
        return 0.0;
    }
    let target = ((total as f64) * 0.95).ceil() as u64;
    let mut prev_upper = 0.0_f64;
    let mut prev_count = 0u64;
    for (i, &upper) in LATENCY_MS_BUCKETS.iter().enumerate() {
        let c = bucket_counts[i].load(Ordering::Relaxed);
        if c >= target {
            if c == prev_count {
                return upper;
            }
            let frac = (target - prev_count) as f64 / (c - prev_count) as f64;
            return prev_upper + frac * (upper - prev_upper);
        }
        prev_upper = upper;
        prev_count = c;
    }
    LATENCY_MS_BUCKETS.last().copied().unwrap_or(0.0)
}

fn render_histogram_block(
    buf: &mut String,
    pipeline: &str,
    name: &str,
    bucket_counts: &[AtomicU64; INF_BUCKET + 1],
    sum_micro: u64,
    count: u64,
) {
    let p = escape_label_value(pipeline);
    let p95 = p95_from_hist(bucket_counts);
    let _ = writeln!(buf, "{name}_p95{{pipeline=\"{p}\"}} {p95}");
    for (i, &upper) in LATENCY_MS_BUCKETS.iter().enumerate() {
        let cnt = bucket_counts[i].load(Ordering::Relaxed);
        let _ = writeln!(buf, "{name}_bucket{{pipeline=\"{p}\",le=\"{upper}\"}} {cnt}");
    }
    let inf_cnt = bucket_counts[INF_BUCKET].load(Ordering::Relaxed);
    let _ = writeln!(buf, "{name}_bucket{{pipeline=\"{p}\",le=\"+Inf\"}} {inf_cnt}");
    let sum = sum_micro as f64 / 1_000_000.0;
    let _ = writeln!(buf, "{name}_sum{{pipeline=\"{p}\"}} {sum}");
    let _ = writeln!(buf, "{name}_count{{pipeline=\"{p}\"}} {count}");
}

#[derive(Debug)]
struct Inner {
    delivered_total: AtomicU64,
    gen_ai_completed_total: AtomicU64,
    gen_ai_failed_total: AtomicU64,
    input_tokens_total: AtomicU64,
    output_tokens_total: AtomicU64,
    cost_micro_usd_total: AtomicU64,
    cost_low_confidence_total: AtomicU64,
    /// Engine-side latency (`mara.ollama.total_duration_ms`).
    latency_bucket_counts: [AtomicU64; INF_BUCKET + 1],
    latency_sum_micro: AtomicU64,
    latency_count: AtomicU64,
    /// Proxy wall clock (`mara.proxy.gateway_duration_ms`).
    gateway_latency_bucket_counts: [AtomicU64; INF_BUCKET + 1],
    gateway_latency_sum_micro: AtomicU64,
    gateway_latency_count: AtomicU64,
    /// Dispatcher fan-out wall time (M2-08 / M2-13 signal).
    fanout_bucket_counts: [AtomicU64; INF_BUCKET + 1],
    fanout_sum_micro: AtomicU64,
    fanout_count: AtomicU64,
    sink_send_errors_total: AtomicU64,
}

impl Default for Inner {
    fn default() -> Self {
        Self {
            delivered_total: AtomicU64::new(0),
            gen_ai_completed_total: AtomicU64::new(0),
            gen_ai_failed_total: AtomicU64::new(0),
            input_tokens_total: AtomicU64::new(0),
            output_tokens_total: AtomicU64::new(0),
            cost_micro_usd_total: AtomicU64::new(0),
            cost_low_confidence_total: AtomicU64::new(0),
            latency_bucket_counts: std::array::from_fn(|_| AtomicU64::new(0)),
            latency_sum_micro: AtomicU64::new(0),
            latency_count: AtomicU64::new(0),
            gateway_latency_bucket_counts: std::array::from_fn(|_| AtomicU64::new(0)),
            gateway_latency_sum_micro: AtomicU64::new(0),
            gateway_latency_count: AtomicU64::new(0),
            fanout_bucket_counts: std::array::from_fn(|_| AtomicU64::new(0)),
            fanout_sum_micro: AtomicU64::new(0),
            fanout_count: AtomicU64::new(0),
            sink_send_errors_total: AtomicU64::new(0),
        }
    }
}

/// Per-pipeline counters and latency histogram derived from delivered events.
#[derive(Debug)]
pub struct PipelineSelfMetrics {
    pipeline: String,
    inner: Inner,
}

impl PipelineSelfMetrics {
    /// Pipeline name used as the `pipeline` Prometheus label.
    #[must_use]
    pub fn new(pipeline: impl Into<String>) -> Self {
        Self { pipeline: pipeline.into(), inner: Inner::default() }
    }

    /// Configured pipeline name (label value).
    #[must_use]
    pub fn pipeline(&self) -> &str {
        &self.pipeline
    }

    /// Increment when a sink `send` fails during dispatcher fan-out (M2-13).
    pub fn record_sink_send_error(&self) {
        self.inner.sink_send_errors_total.fetch_add(1, Ordering::Relaxed);
    }

    /// Record wall time spent waiting on all sink channels for one event (M2-13).
    pub fn record_fanout_wall_ms(&self, ms: f64) {
        record_ms_into_buckets(
            ms,
            &self.inner.fanout_bucket_counts,
            &self.inner.fanout_sum_micro,
            &self.inner.fanout_count,
        );
    }

    /// Record an event that passed the policy chain and is being sent to sinks.
    pub fn record_delivered(&self, ev: &Event) {
        self.inner.delivered_total.fetch_add(1, Ordering::Relaxed);

        if matches!(ev.event_kind, EventKind::Error) {
            if proxy_style_error(ev) {
                self.inner.gen_ai_failed_total.fetch_add(1, Ordering::Relaxed);
            }
            return;
        }

        if !matches!(ev.event_kind, EventKind::Completion) {
            return;
        }

        let op = ev.gen_ai.operation_name.as_deref();
        if !matches!(op, Some("chat") | Some("text_completion") | Some("embeddings")) {
            return;
        }

        if !is_llm_proxy_gen_ai(ev) {
            return;
        }

        self.inner.gen_ai_completed_total.fetch_add(1, Ordering::Relaxed);

        let in_t = ev.gen_ai.usage.input_tokens.unwrap_or(0);
        let out_t = ev.gen_ai.usage.output_tokens.unwrap_or(0);
        self.inner.input_tokens_total.fetch_add(in_t, Ordering::Relaxed);
        self.inner.output_tokens_total.fetch_add(out_t, Ordering::Relaxed);

        if let Some(usd) = ev.mara.cost_usd
            && usd.is_finite()
            && usd >= 0.0
        {
            let micro = (usd * 1_000_000.0).round() as u128;
            let micro_u64 = micro.min(u64::MAX as u128) as u64;
            self.inner.cost_micro_usd_total.fetch_add(micro_u64, Ordering::Relaxed);
        }
        if ev.mara.cost_confidence == Some(CostConfidence::Low) {
            self.inner.cost_low_confidence_total.fetch_add(1, Ordering::Relaxed);
        }

        if let Some(AttrValue::Float(ms)) = ev.attributes.get("mara.ollama.total_duration_ms")
            && ms.is_finite()
            && *ms >= 0.0
        {
            record_ms_into_buckets(
                *ms,
                &self.inner.latency_bucket_counts,
                &self.inner.latency_sum_micro,
                &self.inner.latency_count,
            );
        }
        if let Some(AttrValue::Float(ms)) = ev.attributes.get("mara.proxy.gateway_duration_ms")
            && ms.is_finite()
            && *ms >= 0.0
        {
            record_ms_into_buckets(
                *ms,
                &self.inner.gateway_latency_bucket_counts,
                &self.inner.gateway_latency_sum_micro,
                &self.inner.gateway_latency_count,
            );
        }
    }

    /// Append exposition lines for this pipeline (metric HELP/TYPE emitted by [`render_prometheus`]).
    pub fn render_samples(&self, buf: &mut String) {
        let p = escape_label_value(&self.pipeline);
        let d = self.inner.delivered_total.load(Ordering::Relaxed);
        let _ = writeln!(buf, "mara_pipeline_events_delivered_total{{pipeline=\"{p}\"}} {d}");

        let c = self.inner.gen_ai_completed_total.load(Ordering::Relaxed);
        let _ = writeln!(buf, "mara_gen_ai_requests_completed_total{{pipeline=\"{p}\"}} {c}");

        let f = self.inner.gen_ai_failed_total.load(Ordering::Relaxed);
        let _ = writeln!(buf, "mara_gen_ai_requests_failed_total{{pipeline=\"{p}\"}} {f}");

        let it = self.inner.input_tokens_total.load(Ordering::Relaxed);
        let _ = writeln!(buf, "mara_gen_ai_input_tokens_total{{pipeline=\"{p}\"}} {it}");

        let ot = self.inner.output_tokens_total.load(Ordering::Relaxed);
        let _ = writeln!(buf, "mara_gen_ai_output_tokens_total{{pipeline=\"{p}\"}} {ot}");

        let cm = self.inner.cost_micro_usd_total.load(Ordering::Relaxed);
        let _ = writeln!(buf, "mara_gen_ai_cost_micro_usd_total{{pipeline=\"{p}\"}} {cm}");

        let cl = self.inner.cost_low_confidence_total.load(Ordering::Relaxed);
        let _ = writeln!(buf, "mara_gen_ai_cost_low_confidence_completions_total{{pipeline=\"{p}\"}} {cl}");

        let se = self.inner.sink_send_errors_total.load(Ordering::Relaxed);
        let _ = writeln!(buf, "mara_pipeline_sink_channel_send_errors_total{{pipeline=\"{p}\"}} {se}");

        let eng = "mara_gen_ai_request_duration_ms";
        let gw = "mara_gen_ai_gateway_duration_ms";
        let fo = "mara_pipeline_fanout_duration_ms";

        let lm = self.inner.latency_sum_micro.load(Ordering::Relaxed);
        let lc = self.inner.latency_count.load(Ordering::Relaxed);
        let gm = self.inner.gateway_latency_sum_micro.load(Ordering::Relaxed);
        let gc = self.inner.gateway_latency_count.load(Ordering::Relaxed);
        let fm = self.inner.fanout_sum_micro.load(Ordering::Relaxed);
        let fc = self.inner.fanout_count.load(Ordering::Relaxed);

        render_histogram_block(buf, &self.pipeline, eng, &self.inner.latency_bucket_counts, lm, lc);
        render_histogram_block(buf, &self.pipeline, gw, &self.inner.gateway_latency_bucket_counts, gm, gc);
        render_histogram_block(buf, &self.pipeline, fo, &self.inner.fanout_bucket_counts, fm, fc);
    }
}

/// Full Prometheus text for all configured pipelines.
#[must_use]
pub fn render_prometheus(pipelines: &[std::sync::Arc<PipelineSelfMetrics>]) -> String {
    let mut buf = String::new();
    let _ = writeln!(
        buf,
        "# HELP mara_pipeline_events_delivered_total Events delivered to sinks after policy"
    );
    let _ = writeln!(buf, "# TYPE mara_pipeline_events_delivered_total counter");
    let _ = writeln!(
        buf,
        "# HELP mara_gen_ai_requests_completed_total GenAI completions (chat/text_completion/embeddings) after policy"
    );
    let _ = writeln!(buf, "# TYPE mara_gen_ai_requests_completed_total counter");
    let _ = writeln!(
        buf,
        "# HELP mara_gen_ai_requests_failed_total Proxy or upstream HTTP errors attributed to gen_ai traffic"
    );
    let _ = writeln!(buf, "# TYPE mara_gen_ai_requests_failed_total counter");
    let _ = writeln!(buf, "# HELP mara_gen_ai_input_tokens_total Sum of gen_ai.usage.input_tokens on completions");
    let _ = writeln!(buf, "# TYPE mara_gen_ai_input_tokens_total counter");
    let _ = writeln!(buf, "# HELP mara_gen_ai_output_tokens_total Sum of gen_ai.usage.output_tokens on completions");
    let _ = writeln!(buf, "# TYPE mara_gen_ai_output_tokens_total counter");
    let _ = writeln!(
        buf,
        "# HELP mara_gen_ai_cost_micro_usd_total Sum of mara.cost_usd in micro-dollars (usd * 1e6)"
    );
    let _ = writeln!(buf, "# TYPE mara_gen_ai_cost_micro_usd_total counter");
    let _ = writeln!(
        buf,
        "# HELP mara_gen_ai_cost_low_confidence_completions_total Completions with mara.cost_confidence=low"
    );
    let _ = writeln!(buf, "# TYPE mara_gen_ai_cost_low_confidence_completions_total counter");
    let _ = writeln!(
        buf,
        "# HELP mara_pipeline_sink_channel_send_errors_total Sink channel closed during fan-out"
    );
    let _ = writeln!(buf, "# TYPE mara_pipeline_sink_channel_send_errors_total counter");
    let _ = writeln!(
        buf,
        "# HELP mara_gen_ai_request_duration_ms_p95 Approximate p95 engine latency from histogram buckets (ms)"
    );
    let _ = writeln!(buf, "# TYPE mara_gen_ai_request_duration_ms_p95 gauge");
    let _ = writeln!(
        buf,
        "# HELP mara_gen_ai_gateway_duration_ms_p95 Approximate p95 gateway latency from histogram buckets (ms)"
    );
    let _ = writeln!(buf, "# TYPE mara_gen_ai_gateway_duration_ms_p95 gauge");
    let _ = writeln!(
        buf,
        "# HELP mara_pipeline_fanout_duration_ms_p95 Approximate p95 fan-out wall time from histogram buckets (ms)"
    );
    let _ = writeln!(buf, "# TYPE mara_pipeline_fanout_duration_ms_p95 gauge");
    let _ = writeln!(
        buf,
        "# HELP mara_gen_ai_request_duration_ms Histogram of mara.ollama.total_duration_ms (ms)"
    );
    let _ = writeln!(buf, "# TYPE mara_gen_ai_request_duration_ms histogram");
    let _ = writeln!(
        buf,
        "# HELP mara_gen_ai_gateway_duration_ms Histogram of mara.proxy.gateway_duration_ms (ms)"
    );
    let _ = writeln!(buf, "# TYPE mara_gen_ai_gateway_duration_ms histogram");
    let _ = writeln!(
        buf,
        "# HELP mara_pipeline_fanout_duration_ms Dispatcher sink fan-out wall time (ms)"
    );
    let _ = writeln!(buf, "# TYPE mara_pipeline_fanout_duration_ms histogram");

    for m in pipelines {
        m.render_samples(&mut buf);
    }
    buf
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use mara_schema::{Event, EventKind};

    use super::*;

    #[test]
    fn percentile_sorted_empty() {
        assert_eq!(percentile_sorted(&[], 0.95), None);
    }

    #[test]
    fn percentile_sorted_single() {
        assert_eq!(percentile_sorted(&[42.0], 0.95), Some(42.0));
    }

    #[test]
    fn percentile_sorted_uniform_100() {
        let v: Vec<f64> = (1..=100).map(|i| i as f64).collect();
        assert_eq!(percentile_sorted(&v, 0.95), Some(95.0));
        assert_eq!(percentile_sorted(&v, 1.0), Some(100.0));
    }

    #[test]
    fn percentile_rejects_non_positive_p() {
        let v = [1.0, 2.0, 3.0];
        assert_eq!(percentile_sorted(&v, 0.0), None);
    }

    #[test]
    fn escape_label_escapes_quote() {
        assert_eq!(escape_label_value(r#"a"b"#), r#"a\"b"#);
    }

    fn sample_completion(latency_ms: f64, in_t: u64, out_t: u64, cost: f64) -> Event {
        let mut ev = Event::now(EventKind::Completion, "mara-adapter-llm-proxy");
        ev.resource.source_runtime = Some(SourceRuntime::Ollama);
        ev.gen_ai.system = Some("ollama".into());
        ev.gen_ai.operation_name = Some("text_completion".into());
        ev.gen_ai.usage.input_tokens = Some(in_t);
        ev.gen_ai.usage.output_tokens = Some(out_t);
        ev.mara.cost_usd = Some(cost);
        ev.attributes.insert("mara.ollama.total_duration_ms".into(), AttrValue::Float(latency_ms));
        ev
    }

    #[test]
    fn records_completion_tokens_cost_latency_histogram() {
        let m = Arc::new(PipelineSelfMetrics::new("p-test"));
        m.record_delivered(&sample_completion(12.0, 100, 50, 0.001));
        m.record_delivered(&sample_completion(3.0, 10, 5, 0.0));
        let s = render_prometheus(&[m]);
        assert!(s.contains("mara_gen_ai_requests_completed_total{pipeline=\"p-test\"} 2"));
        assert!(s.contains("mara_gen_ai_input_tokens_total{pipeline=\"p-test\"} 110"));
        assert!(s.contains("mara_gen_ai_output_tokens_total{pipeline=\"p-test\"} 55"));
        assert!(s.contains("mara_gen_ai_cost_micro_usd_total{pipeline=\"p-test\"} 1000"));
        assert!(s.contains("mara_gen_ai_request_duration_ms_bucket{pipeline=\"p-test\",le=\"5\"} 1"));
        assert!(s.contains("mara_gen_ai_request_duration_ms_bucket{pipeline=\"p-test\",le=\"25\"} 2"));
    }

    #[test]
    fn records_proxy_error() {
        let m = Arc::new(PipelineSelfMetrics::new("p-err"));
        let mut ev = Event::now(EventKind::Error, "mara-adapter-llm-proxy");
        ev.resource.source_runtime = Some(SourceRuntime::Ollama);
        ev.attributes.insert("http.status_code".into(), AttrValue::Int(502));
        m.record_delivered(&ev);
        let s = render_prometheus(&[m]);
        assert!(s.contains("mara_pipeline_events_delivered_total{pipeline=\"p-err\"} 1"));
        assert!(s.contains("mara_gen_ai_requests_failed_total{pipeline=\"p-err\"} 1"));
        assert!(s.contains("mara_gen_ai_requests_completed_total{pipeline=\"p-err\"} 0"));
    }

    #[test]
    fn skips_completion_without_ollama_signal() {
        let m = Arc::new(PipelineSelfMetrics::new("p-skip"));
        let mut ev = sample_completion(1.0, 1, 1, 0.0);
        ev.resource.source_runtime = None;
        ev.gen_ai.system = None;
        ev.scope.name = "other".into();
        m.record_delivered(&ev);
        let s = render_prometheus(&[m]);
        assert!(s.contains("mara_gen_ai_requests_completed_total{pipeline=\"p-skip\"} 0"));
        assert!(s.contains("mara_pipeline_events_delivered_total{pipeline=\"p-skip\"} 1"));
    }

    #[test]
    fn render_prometheus_contains_series() {
        let m = Arc::new(PipelineSelfMetrics::new("ollama"));
        m.record_delivered(&sample_completion(50.0, 2, 3, 0.5));
        let body = render_prometheus(&[m]);
        assert!(body.contains("mara_gen_ai_requests_completed_total{pipeline=\"ollama\"} 1"));
        assert!(body.contains("mara_gen_ai_input_tokens_total{pipeline=\"ollama\"} 2"));
        assert!(body.contains("mara_gen_ai_output_tokens_total{pipeline=\"ollama\"} 3"));
        assert!(body.contains("mara_gen_ai_request_duration_ms_bucket{pipeline=\"ollama\",le=\"100\"}"));
        assert!(body.contains("mara_gen_ai_request_duration_ms_p95{pipeline=\"ollama\"}"));
    }

    #[test]
    fn records_gateway_histogram_when_attribute_present() {
        let m = Arc::new(PipelineSelfMetrics::new("gw"));
        let mut ev = sample_completion(10.0, 1, 1, 0.0);
        ev.attributes.insert("mara.proxy.gateway_duration_ms".into(), AttrValue::Float(42.0));
        m.record_delivered(&ev);
        let s = render_prometheus(&[m]);
        assert!(s.contains("mara_gen_ai_gateway_duration_ms_count{pipeline=\"gw\"} 1"));
    }

    #[test]
    fn records_fanout_and_sink_errors() {
        let m = Arc::new(PipelineSelfMetrics::new("fan"));
        m.record_fanout_wall_ms(2.5);
        m.record_sink_send_error();
        let s = render_prometheus(&[m]);
        assert!(s.contains("mara_pipeline_sink_channel_send_errors_total{pipeline=\"fan\"} 1"));
        assert!(s.contains("mara_pipeline_fanout_duration_ms_count{pipeline=\"fan\"} 1"));
    }

    #[test]
    fn records_proxy_error_with_failure_kind_only() {
        let m = Arc::new(PipelineSelfMetrics::new("fk"));
        let mut ev = Event::now(EventKind::Error, "mara-adapter-llm-proxy");
        ev.resource.source_runtime = Some(SourceRuntime::Ollama);
        ev.attributes.insert("mara.proxy.failure_kind".into(), AttrValue::String("upstream_transport".into()));
        m.record_delivered(&ev);
        let s = render_prometheus(&[m]);
        assert!(s.contains("mara_gen_ai_requests_failed_total{pipeline=\"fk\"} 1"));
    }

    #[test]
    fn skips_non_finite_cost() {
        let m = Arc::new(PipelineSelfMetrics::new("nan-cost"));
        let ev = sample_completion(1.0, 1, 1, f64::NAN);
        m.record_delivered(&ev);
        let s = render_prometheus(&[m]);
        assert!(s.contains("mara_gen_ai_cost_micro_usd_total{pipeline=\"nan-cost\"} 0"));
    }

    #[test]
    fn render_multiple_pipelines() {
        let a = Arc::new(PipelineSelfMetrics::new("a"));
        let b = Arc::new(PipelineSelfMetrics::new("b"));
        a.record_delivered(&sample_completion(1.0, 1, 0, 0.0));
        b.record_delivered(&sample_completion(2.0, 0, 1, 0.0));
        let s = render_prometheus(&[a, b]);
        assert!(s.contains("pipeline=\"a\""));
        assert!(s.contains("pipeline=\"b\""));
    }

    #[test]
    fn render_many_completions_without_sorting_large_vector() {
        let m = Arc::new(PipelineSelfMetrics::new("scale"));
        for i in 0u64..3000 {
            m.record_delivered(&sample_completion((i % 50) as f64, 1, 1, 0.0));
        }
        let s = render_prometheus(&[m]);
        assert!(s.contains("mara_gen_ai_requests_completed_total{pipeline=\"scale\"} 3000"));
        assert!(s.contains("mara_gen_ai_request_duration_ms_count{pipeline=\"scale\"} 3000"));
    }
}
