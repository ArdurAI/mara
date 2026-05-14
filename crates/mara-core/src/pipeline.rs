//! Pipeline scheduler.
//!
//! Wires a set of adapters into a policy chain and a set of sinks.
//! Each adapter runs in its own tokio task; events flow through a
//! bounded mpsc channel into a single dispatcher task, which runs
//! the policy chain and fans out to the sinks via per-sink mpsc
//! channels.  Each sink runs in its own task and is responsible
//! for its own retry / batching.
//!
//! M2 ships the in-memory pipeline.  WAL durability and per-sink
//! offsets land in M2 follow-up work; the API is shaped to admit
//! them without changes.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

use futures::future::join_all;
use time::OffsetDateTime;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tracing::{debug, error, info, warn};

use crate::error::{Error, Result};
use crate::policy::{ChainOutcome, PolicyChain};
use crate::self_metrics::PipelineSelfMetrics;
use crate::traits::{Adapter, DEFAULT_CHANNEL_CAPACITY, EventReceiver, EventSender, Sink};
use mara_schema::{AttrValue, Event, EventKind};

/// A configured pipeline ready to be started.
pub struct Pipeline {
    /// Pipeline name (matches config).
    pub name: String,
    /// Adapters that feed this pipeline.
    pub adapters: Vec<Arc<dyn Adapter>>,
    /// Policy chain applied to every event.
    pub policy_chain: Arc<PolicyChain>,
    /// Sinks that receive events.
    pub sinks: Vec<Arc<dyn Sink>>,
    /// Per-pipeline self-telemetry (M1-05); updated when events are delivered after policy.
    pub self_metrics: Option<Arc<PipelineSelfMetrics>>,
    /// Per-channel capacity.  Defaults to [`DEFAULT_CHANNEL_CAPACITY`].
    pub channel_capacity: usize,
    /// When true, emit a minimal [`EventKind::System`] audit event to sinks on every policy drop.
    pub audit_policy_drops: bool,
    /// Optional directory: append one JSON line per post-policy delivered event (`*.wal` per UTC day).
    pub wal_spool_path: Option<PathBuf>,
}

impl Pipeline {
    /// Convenience constructor.  Sets `channel_capacity` to the default.
    #[must_use]
    pub fn new(
        name: impl Into<String>,
        adapters: Vec<Arc<dyn Adapter>>,
        policy_chain: Arc<PolicyChain>,
        sinks: Vec<Arc<dyn Sink>>,
    ) -> Self {
        Self {
            name: name.into(),
            adapters,
            policy_chain,
            sinks,
            self_metrics: None,
            channel_capacity: DEFAULT_CHANNEL_CAPACITY,
            audit_policy_drops: false,
            wal_spool_path: None,
        }
    }

    /// Attach optional post-policy WAL spool directory (see `docs/observability/pipeline-wal-spool.md`).
    #[must_use]
    pub fn with_wal_spool_path(mut self, dir: impl Into<PathBuf>) -> Self {
        self.wal_spool_path = Some(dir.into());
        self
    }

    /// When enabled, policy [`ChainOutcome::Drop`](crate::policy::ChainOutcome::Drop) emits a
    /// minimal audit event (no body) to sinks for operator visibility.
    #[must_use]
    pub fn with_audit_policy_drops(mut self, audit: bool) -> Self {
        self.audit_policy_drops = audit;
        self
    }

    /// Attach Prometheus self-metrics for this pipeline (optional).
    #[must_use]
    pub fn with_self_metrics(mut self, m: Arc<PipelineSelfMetrics>) -> Self {
        self.self_metrics = Some(m);
        self
    }

    /// Start the pipeline.  Returns a [`PipelineHandle`] that
    /// supervises the child tasks until shutdown.
    #[allow(
        clippy::unused_async,
        reason = "Async preserved for forward compatibility; WAL replay in M2 follow-up uses await."
    )]
    pub async fn start(self) -> Result<PipelineHandle> {
        let Self {
            name,
            adapters,
            policy_chain,
            sinks,
            self_metrics,
            channel_capacity,
            audit_policy_drops,
            wal_spool_path,
        } = self;

        info!(
            pipeline = %name,
            adapters = adapters.len(),
            sinks = sinks.len(),
            stages = policy_chain.profile(),
            wal_spool = wal_spool_path.is_some(),
            "starting pipeline"
        );

        // Channel from all adapters into the dispatcher.
        let (in_tx, in_rx) = mpsc::channel::<Event>(channel_capacity);

        // Per-sink channels.
        let mut sink_txs: Vec<EventSender> = Vec::with_capacity(sinks.len());
        let mut sink_tasks: Vec<JoinHandle<Result<()>>> = Vec::with_capacity(sinks.len());
        for sink in &sinks {
            let (tx, rx) = mpsc::channel::<Event>(channel_capacity);
            sink_txs.push(tx);
            let sink_clone = Arc::clone(sink);
            let task_name = sink.name().to_owned();
            sink_tasks.push(tokio::spawn(async move {
                debug!(sink = %task_name, "sink task starting");
                sink_clone.start(rx).await
            }));
        }

        // Dispatcher: applies policy chain and fans out to sinks.
        let dispatcher_chain = Arc::clone(&policy_chain);
        let pipeline_name = name.clone();
        let metrics_for_dispatcher = self_metrics.clone();
        let wal_dir = wal_spool_path.clone();
        let dispatcher = tokio::spawn(async move {
            run_dispatcher(
                pipeline_name,
                dispatcher_chain,
                in_rx,
                sink_txs,
                metrics_for_dispatcher,
                audit_policy_drops,
                wal_dir,
            )
            .await
        });

        // Adapters: each adapter feeds the dispatcher channel.
        let mut adapter_tasks: Vec<JoinHandle<Result<()>>> = Vec::with_capacity(adapters.len());
        for adapter in &adapters {
            let adapter_clone = Arc::clone(adapter);
            let in_tx_clone = in_tx.clone();
            let adapter_name = adapter.name().to_owned();
            adapter_tasks.push(tokio::spawn(async move {
                debug!(adapter = %adapter_name, "adapter task starting");
                adapter_clone.start(in_tx_clone).await
            }));
        }
        drop(in_tx);

        Ok(PipelineHandle {
            name,
            adapters,
            sinks,
            policy_chain,
            adapter_tasks,
            sink_tasks,
            dispatcher,
        })
    }
}

fn event_kind_slug(kind: EventKind) -> &'static str {
    match kind {
        EventKind::Prompt => "prompt",
        EventKind::Completion => "completion",
        EventKind::ToolCall => "tool_call",
        EventKind::ToolResult => "tool_result",
        EventKind::Cost => "cost",
        EventKind::Error => "error",
        EventKind::System => "system",
        EventKind::Eval => "eval",
        EventKind::Feedback => "feedback",
        _ => "other",
    }
}

/// Minimal audit record for a policy drop: correlation + policy decisions only (no body).
fn build_policy_drop_audit_event(dropped: Event, chain_reason: &str, pipeline_name: &str) -> Event {
    let mut audit = Event::now(EventKind::System, "mara.policy.audit");
    audit.timestamp_ns = dropped.timestamp_ns;
    audit.observed_timestamp_ns = dropped.observed_timestamp_ns;
    audit.trace_id = dropped.trace_id;
    audit.span_id = dropped.span_id;
    audit.parent_span_id = dropped.parent_span_id;
    audit.resource = dropped.resource;
    audit.severity = dropped.severity;
    audit.mara.request_id = dropped.mara.request_id.clone();
    audit.mara.session_id = dropped.mara.session_id.clone();
    audit.mara.turn_id = dropped.mara.turn_id.clone();
    audit.mara.tenant_id = dropped.mara.tenant_id.clone();
    audit.mara.policy_profile = dropped.mara.policy_profile.clone();
    audit.mara.policy_capture_optin = false;
    audit.mara.policy_decisions = dropped.mara.policy_decisions.clone();
    audit.mara.source_adapter = dropped.mara.source_adapter.clone();
    let mut reason_short = chain_reason.chars().take(512).collect::<String>();
    if reason_short.is_empty() {
        reason_short = "policy:drop".into();
    }
    audit.attributes.insert("mara.policy_audit.kind".into(), AttrValue::String("drop".into()));
    audit.attributes.insert(
        "mara.policy_audit.pipeline".into(),
        AttrValue::String(pipeline_name.to_owned()),
    );
    audit.attributes.insert(
        "mara.policy_audit.chain_reason".into(),
        AttrValue::String(reason_short),
    );
    audit.attributes.insert(
        "mara.policy_audit.base_event_kind".into(),
        AttrValue::String(event_kind_slug(dropped.event_kind).into()),
    );
    audit
}

fn wal_append_delivered(spool_dir: &std::path::Path, pipeline_name: &str, ev: &Event) -> std::io::Result<()> {
    std::fs::create_dir_all(spool_dir)?;
    let safe: String = pipeline_name
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '-' || c == '_' { c } else { '_' })
        .collect();
    let date = OffsetDateTime::now_utc().date();
    let fname = format!("{safe}-{date}.wal");
    let path = spool_dir.join(fname);
    let mut line = serde_json::to_vec(ev)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string()))?;
    line.push(b'\n');
    use std::io::Write;
    let mut f = std::fs::OpenOptions::new().create(true).append(true).open(path)?;
    f.write_all(&line)?;
    f.sync_data()?;
    Ok(())
}

async fn run_dispatcher(
    pipeline_name: String,
    chain: Arc<PolicyChain>,
    mut input: EventReceiver,
    sink_txs: Vec<EventSender>,
    self_metrics: Option<Arc<PipelineSelfMetrics>>,
    audit_policy_drops: bool,
    wal_spool_path: Option<PathBuf>,
) -> Result<()> {
    while let Some(event) = input.recv().await {
        match chain.run(event).await {
            Ok(ChainOutcome::Deliver(ev)) => {
                if let Some(dir) = wal_spool_path.clone() {
                    let ev_clone = ev.clone();
                    let pname_wal = pipeline_name.clone();
                    let pname_log = pipeline_name.clone();
                    tokio::spawn(async move {
                        match tokio::task::spawn_blocking(move || {
                            wal_append_delivered(&dir, &pname_wal, &ev_clone)
                        })
                        .await
                        {
                            Ok(Ok(())) => {}
                            Ok(Err(e)) => warn!(pipeline = %pname_log, "wal append: {e}"),
                            Err(e) => warn!(pipeline = %pname_log, "wal append join: {e:?}"),
                        }
                    });
                }
                if let Some(ref m) = self_metrics {
                    m.record_delivered(&ev);
                }
                fanout(&pipeline_name, &sink_txs, ev, self_metrics.as_ref()).await;
            }
            Ok(ChainOutcome::Drop { reason, event }) => {
                debug!(pipeline = %pipeline_name, reason = %reason, "policy dropped event");
                if audit_policy_drops {
                    let audit = build_policy_drop_audit_event(event, &reason, &pipeline_name);
                    if let Some(ref m) = self_metrics {
                        m.record_delivered(&audit);
                    }
                    fanout(&pipeline_name, &sink_txs, audit, self_metrics.as_ref()).await;
                }
            }
            Err(e) => {
                error!(pipeline = %pipeline_name, error = %e, "policy chain errored; event discarded");
            }
        }
    }
    debug!(pipeline = %pipeline_name, "dispatcher input closed; exiting");
    Ok(())
}

async fn fanout(
    pipeline_name: &str,
    sink_txs: &[EventSender],
    event: Event,
    self_metrics: Option<&Arc<PipelineSelfMetrics>>,
) {
    let t0 = Instant::now();
    let futs: Vec<_> = sink_txs
        .iter()
        .map(|tx| {
            let ev = event.clone();
            let tx = tx.clone();
            let pname = pipeline_name.to_owned();
            async move {
                if let Err(e) = tx.send(ev).await {
                    warn!(pipeline = %pname, "sink channel closed while fanning out: {e}");
                    return false;
                }
                true
            }
        })
        .collect();
    let results = join_all(futs).await;
    if let Some(m) = self_metrics {
        for ok in results {
            if !ok {
                m.record_sink_send_error();
            }
        }
        m.record_fanout_wall_ms(t0.elapsed().as_secs_f64() * 1000.0);
    }
}

/// A running pipeline.  Drop to stop; call [`Self::shutdown`] for
/// graceful drain.
pub struct PipelineHandle {
    name: String,
    adapters: Vec<Arc<dyn Adapter>>,
    sinks: Vec<Arc<dyn Sink>>,
    #[allow(dead_code, reason = "Held for the chain's lifetime; consumed at shutdown.")]
    policy_chain: Arc<PolicyChain>,
    adapter_tasks: Vec<JoinHandle<Result<()>>>,
    sink_tasks: Vec<JoinHandle<Result<()>>>,
    dispatcher: JoinHandle<Result<()>>,
}

impl PipelineHandle {
    /// True when every adapter and sink reports a non-failed aggregate health (M2-09).
    #[must_use]
    pub fn readiness_aggregate_ok(&self) -> bool {
        self.adapters.iter().all(|a| a.health().is_aggregate_ready())
            && self.sinks.iter().all(|s| s.health().is_aggregate_ready())
    }

    /// The pipeline's name (matches its configuration entry).
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Request orderly shutdown.  Stops adapters first, drains the
    /// dispatcher, then stops sinks.
    pub async fn shutdown(self) -> Result<()> {
        info!(pipeline = %self.name, "shutting down pipeline");
        for adapter in &self.adapters {
            if let Err(e) = adapter.shutdown().await {
                warn!(adapter = %adapter.name(), "shutdown error: {e}");
            }
        }
        for handle in self.adapter_tasks {
            if let Err(e) = handle.await {
                warn!(pipeline = %self.name, "adapter task error: {e}");
            }
        }
        if let Err(e) = self.dispatcher.await {
            warn!(pipeline = %self.name, "dispatcher task error: {e}");
        }
        for sink in &self.sinks {
            if let Err(e) = sink.shutdown().await {
                warn!(sink = %sink.name(), "shutdown error: {e}");
            }
        }
        for handle in self.sink_tasks {
            if let Err(e) = handle.await {
                warn!(pipeline = %self.name, "sink task error: {e}");
            }
        }
        Ok(())
    }
}

impl std::fmt::Debug for PipelineHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PipelineHandle").field("name", &self.name).finish_non_exhaustive()
    }
}

// Make `Error` Send/Sync compatible explicitly for the error path,
// belt-and-suspenders for older Rust toolchains.
fn _assert_error_send_sync() {
    fn assert_send<T: Send>() {}
    fn assert_sync<T: Sync>() {}
    assert_send::<Error>();
    assert_sync::<Error>();
}

/// Aggregate readiness for Kubernetes `/readyz` style probes (M2-09).
#[must_use]
pub fn pipelines_aggregate_ready(handles: &[PipelineHandle]) -> bool {
    handles.iter().all(PipelineHandle::readiness_aggregate_ok)
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::Duration;

    use async_trait::async_trait;
    use mara_schema::EventKind;
    use tokio::sync::Notify;

    use super::*;
    use crate::policy::{Policy, PolicyContext, PolicyOutcome, PolicyChain};
    use crate::self_metrics::{PipelineSelfMetrics, render_prometheus};
    use crate::traits::{Adapter, Sink};

    /// Policy stage that always drops (for audit tests).
    struct DropAllPolicy;

    #[async_trait]
    impl Policy for DropAllPolicy {
        fn name(&self) -> &str {
            "drop-all"
        }

        async fn apply(&self, _ctx: &PolicyContext, ev: Event) -> Result<PolicyOutcome> {
            Ok(PolicyOutcome::drop(ev, "dropped-by-test"))
        }
    }

    #[tokio::test]
    async fn policy_drop_without_audit_sends_nothing_to_sinks() {
        let counter = Arc::new(AtomicUsize::new(0));
        let chain = Arc::new(PolicyChain::new(
            "drop-chain",
            vec![Arc::new(DropAllPolicy) as Arc<dyn Policy>],
        ));
        let pipeline = Pipeline::new(
            "pd-off",
            vec![Arc::new(ProducerAdapter { count: 1 })],
            chain,
            vec![Arc::new(CountingSink {
                counter: counter.clone(),
                done: Arc::new(Notify::new()),
                expected: usize::MAX,
            })],
        );
        let handle = pipeline.start().await.expect("pipeline started");
        tokio::time::sleep(Duration::from_millis(150)).await;
        handle.shutdown().await.expect("shutdown ok");
        assert_eq!(counter.load(Ordering::Relaxed), 0);
    }

    #[tokio::test]
    async fn policy_drop_with_audit_emits_system_event() {
        let counter = Arc::new(AtomicUsize::new(0));
        let done = Arc::new(Notify::new());
        let chain = Arc::new(PolicyChain::new(
            "drop-chain-a",
            vec![Arc::new(DropAllPolicy) as Arc<dyn Policy>],
        ));
        let pipeline = Pipeline::new(
            "pd-on",
            vec![Arc::new(ProducerAdapter { count: 1 })],
            chain,
            vec![Arc::new(CountingSink {
                counter: counter.clone(),
                done: done.clone(),
                expected: 1,
            })],
        )
        .with_audit_policy_drops(true);
        let handle = pipeline.start().await.expect("pipeline started");
        tokio::time::timeout(Duration::from_secs(2), done.notified())
            .await
            .expect("audit event delivered");
        handle.shutdown().await.expect("shutdown ok");
        assert_eq!(counter.load(Ordering::Relaxed), 1);
    }

    /// Test adapter that emits a fixed number of events and exits.
    struct ProducerAdapter {
        count: usize,
    }

    #[async_trait]
    impl Adapter for ProducerAdapter {
        fn name(&self) -> &str {
            "producer"
        }
        async fn start(&self, out: EventSender) -> Result<()> {
            for _ in 0..self.count {
                out.send(Event::now(EventKind::System, "producer"))
                    .await
                    .map_err(|_| Error::Internal("send failed".into()))?;
            }
            Ok(())
        }
        async fn shutdown(&self) -> Result<()> {
            Ok(())
        }
    }

    /// Test sink that counts events.
    struct CountingSink {
        counter: Arc<AtomicUsize>,
        done: Arc<Notify>,
        expected: usize,
    }

    #[async_trait]
    impl Sink for CountingSink {
        fn name(&self) -> &str {
            "counter"
        }
        async fn start(&self, mut input: EventReceiver) -> Result<()> {
            while let Some(_ev) = input.recv().await {
                let new = self.counter.fetch_add(1, Ordering::Relaxed) + 1;
                if new == self.expected {
                    self.done.notify_one();
                }
            }
            Ok(())
        }
        async fn shutdown(&self) -> Result<()> {
            Ok(())
        }
    }

    #[tokio::test]
    async fn pipeline_delivers_events_through_empty_chain() {
        let counter = Arc::new(AtomicUsize::new(0));
        let done = Arc::new(Notify::new());

        let pipeline = Pipeline::new(
            "test",
            vec![Arc::new(ProducerAdapter { count: 10 })],
            Arc::new(PolicyChain::new("default", vec![])),
            vec![Arc::new(CountingSink {
                counter: counter.clone(),
                done: done.clone(),
                expected: 10,
            })],
        );
        let handle = pipeline.start().await.expect("pipeline started");

        tokio::time::timeout(Duration::from_secs(2), done.notified())
            .await
            .expect("counter notified");

        handle.shutdown().await.expect("shutdown ok");
        assert_eq!(counter.load(Ordering::Relaxed), 10);
    }

    #[tokio::test]
    async fn pipeline_fans_out_to_multiple_sinks() {
        let c1 = Arc::new(AtomicUsize::new(0));
        let c2 = Arc::new(AtomicUsize::new(0));
        let d1 = Arc::new(Notify::new());
        let d2 = Arc::new(Notify::new());

        let pipeline = Pipeline::new(
            "test-fanout",
            vec![Arc::new(ProducerAdapter { count: 5 })],
            Arc::new(PolicyChain::new("default", vec![])),
            vec![
                Arc::new(CountingSink { counter: c1.clone(), done: d1.clone(), expected: 5 }),
                Arc::new(CountingSink { counter: c2.clone(), done: d2.clone(), expected: 5 }),
            ],
        );
        let handle = pipeline.start().await.expect("pipeline started");

        tokio::time::timeout(Duration::from_secs(2), async {
            d1.notified().await;
            d2.notified().await;
        })
        .await
        .expect("both sinks notified");

        handle.shutdown().await.expect("shutdown ok");
        assert_eq!(c1.load(Ordering::Relaxed), 5);
        assert_eq!(c2.load(Ordering::Relaxed), 5);
    }

    #[tokio::test]
    async fn pipeline_records_self_metrics_on_deliver() {
        let m = Arc::new(PipelineSelfMetrics::new("self-m"));
        let counter = Arc::new(AtomicUsize::new(0));
        let done = Arc::new(Notify::new());

        let pipeline = Pipeline::new(
            "self-m",
            vec![Arc::new(ProducerAdapter { count: 2 })],
            Arc::new(PolicyChain::new("default", vec![])),
            vec![Arc::new(CountingSink {
                counter: counter.clone(),
                done: done.clone(),
                expected: 2,
            })],
        )
        .with_self_metrics(m.clone());

        let handle = pipeline.start().await.expect("pipeline started");

        tokio::time::timeout(Duration::from_secs(2), done.notified())
            .await
            .expect("counter notified");

        handle.shutdown().await.expect("shutdown ok");
        let body = render_prometheus(std::slice::from_ref(&m));
        assert!(body.contains("mara_pipeline_events_delivered_total{pipeline=\"self-m\"} 2"));
    }
}
