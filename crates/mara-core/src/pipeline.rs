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

use std::sync::Arc;

use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tracing::{debug, error, info, warn};

use crate::error::{Error, Result};
use crate::policy::{ChainOutcome, PolicyChain};
use crate::traits::{Adapter, DEFAULT_CHANNEL_CAPACITY, EventReceiver, EventSender, Sink};
use mara_schema::Event;

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
    /// Per-channel capacity.  Defaults to [`DEFAULT_CHANNEL_CAPACITY`].
    pub channel_capacity: usize,
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
            channel_capacity: DEFAULT_CHANNEL_CAPACITY,
        }
    }

    /// Start the pipeline.  Returns a [`PipelineHandle`] that
    /// supervises the child tasks until shutdown.
    #[allow(
        clippy::unused_async,
        reason = "Async preserved for forward compatibility; WAL replay in M2 follow-up uses await."
    )]
    pub async fn start(self) -> Result<PipelineHandle> {
        let Self { name, adapters, policy_chain, sinks, channel_capacity } = self;

        info!(
            pipeline = %name,
            adapters = adapters.len(),
            sinks = sinks.len(),
            stages = policy_chain.profile(),
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
        let dispatcher = tokio::spawn(async move {
            run_dispatcher(pipeline_name, dispatcher_chain, in_rx, sink_txs).await
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

async fn run_dispatcher(
    pipeline_name: String,
    chain: Arc<PolicyChain>,
    mut input: EventReceiver,
    sink_txs: Vec<EventSender>,
) -> Result<()> {
    while let Some(event) = input.recv().await {
        match chain.run(event).await {
            Ok(ChainOutcome::Deliver(ev)) => {
                fanout(&pipeline_name, &sink_txs, ev).await;
            }
            Ok(ChainOutcome::Drop(reason)) => {
                debug!(pipeline = %pipeline_name, reason = %reason, "policy dropped event");
            }
            Err(e) => {
                error!(pipeline = %pipeline_name, error = %e, "policy chain errored; event discarded");
            }
        }
    }
    debug!(pipeline = %pipeline_name, "dispatcher input closed; exiting");
    Ok(())
}

async fn fanout(pipeline_name: &str, sink_txs: &[EventSender], event: Event) {
    // For N sinks we clone once per sink.  M2 follow-up may switch
    // to `Arc<Event>` to avoid clones when N grows; v1 sinks
    // consume typed values directly.
    for tx in sink_txs {
        let to_send = event.clone();
        if let Err(e) = tx.send(to_send).await {
            warn!(
                pipeline = %pipeline_name,
                "sink channel closed while fanning out: {e}"
            );
        }
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

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::Duration;

    use async_trait::async_trait;
    use mara_schema::EventKind;
    use tokio::sync::Notify;

    use super::*;
    use crate::policy::PolicyChain;
    use crate::traits::{Adapter, Sink};

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
}
