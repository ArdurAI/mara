//! Adapter, Sink, and shared trait surfaces.
//!
//! Per ADR-0005 the runtime is Tokio; channel types here are
//! Tokio-native.  Per ADR-0006 errors are concrete enums; both
//! `Adapter` and `Sink` return `Result<()>` rooted in
//! [`crate::Error`].

use async_trait::async_trait;
use tokio::sync::mpsc;

use crate::error::Result;
use crate::health::Health;
use mara_schema::Event;

/// Sender half of the canonical-event channel between an adapter
/// and the pipeline.
pub type EventSender = mpsc::Sender<Event>;

/// Receiver half of the canonical-event channel between the pipeline
/// and a sink.
pub type EventReceiver = mpsc::Receiver<Event>;

/// Default channel capacity for adapter → pipeline and pipeline →
/// sink channels.  Configurable per-pipeline at construction time.
pub const DEFAULT_CHANNEL_CAPACITY: usize = 1024;

/// A source of canonical events.
///
/// Implementations are responsible for:
///
/// - Listening on whatever transport their pattern uses (OTLP gRPC
///   port, file tail, hooks subprocess, REST poll).
/// - Translating incoming data into [`Event`] values.
/// - Sending events into the provided [`EventSender`].
/// - Honouring shutdown signals via [`Self::shutdown`].
/// - Reporting health on demand via [`Self::health`].
///
/// Adapters must be `Send + Sync` and concrete instances are
/// configured by the orchestrator before `start` is invoked.
#[async_trait]
pub trait Adapter: Send + Sync {
    /// Stable identifier (e.g., `"otlp-grpc"`).  Distinct adapter
    /// instances within a pipeline should have distinct names.
    fn name(&self) -> &str;

    /// Begin producing events.  Runs until the adapter terminates
    /// or [`Self::shutdown`] is invoked.  Returning `Ok(())`
    /// signals an orderly termination; returning an error
    /// terminates the adapter task with the error propagated to
    /// the orchestrator.
    async fn start(&self, out: EventSender) -> Result<()>;

    /// Request orderly shutdown.  Implementations should drain
    /// in-flight events before returning.
    async fn shutdown(&self) -> Result<()>;

    /// Report current health.  Default is [`Health::healthy`] (assume ready until you track real state).
    fn health(&self) -> Health {
        Health::healthy()
    }
}

/// A destination for canonical events.
///
/// Implementations are responsible for:
///
/// - Consuming events from the provided [`EventReceiver`].
/// - Serializing and shipping to the configured backend.
/// - Applying retry / backoff per the sink's own policy.
/// - Routing terminal failures to the dead-letter queue via the
///   orchestrator (in M2 the DLQ contract lives here).
/// - Reporting health on demand.
#[async_trait]
pub trait Sink: Send + Sync {
    /// Stable identifier (e.g., `"loki-local"`).
    fn name(&self) -> &str;

    /// Begin consuming events.  Runs until the input channel
    /// closes or [`Self::shutdown`] is invoked.
    async fn start(&self, input: EventReceiver) -> Result<()>;

    /// Request orderly shutdown.  Implementations should flush
    /// pending batches and close upstream connections before
    /// returning.
    async fn shutdown(&self) -> Result<()>;

    /// Report current health.  Default is [`Health::healthy`] (assume ready until you track real state).
    fn health(&self) -> Health {
        Health::healthy()
    }
}

#[cfg(test)]
mod tests {
    use mara_schema::EventKind;

    use super::*;

    /// A trivial adapter used in unit tests to exercise the trait
    /// object surface.
    struct TestAdapter;

    #[async_trait]
    impl Adapter for TestAdapter {
        fn name(&self) -> &str {
            "test"
        }

        async fn start(&self, out: EventSender) -> Result<()> {
            out.send(Event::now(EventKind::System, "test"))
                .await
                .map_err(|_| crate::Error::Internal("send failed".into()))?;
            Ok(())
        }

        async fn shutdown(&self) -> Result<()> {
            Ok(())
        }
    }

    #[tokio::test]
    async fn adapter_can_send_event_via_trait_object() {
        let (tx, mut rx) = mpsc::channel::<Event>(8);
        let adapter: Box<dyn Adapter> = Box::new(TestAdapter);
        adapter.start(tx).await.expect("start ok");
        let ev = rx.recv().await.expect("event received");
        assert_eq!(ev.scope.name, "test");
    }
}
