//! Mara core pipeline.
//!
//! This crate is the orchestration brain of the Mara agent. It owns:
//!
//! - The [`Adapter`], [`Sink`], and [`Policy`] traits that adapters,
//!   sinks, and policy stages implement.
//! - The async pipeline scheduler that wires adapters into policy
//!   chains into sinks (M2).
//! - The bounded-buffer + WAL implementation for durability and
//!   backpressure (M2).
//! - The configuration loader, JSON-schema validator, and hot-reload
//!   mechanism (M2).
//!
//! No adapter or sink I/O lives here.  Only orchestration and
//! contracts.
//!
//! ## Stability
//!
//! Traits in this crate are tagged `#[non_exhaustive]` and `sealed`
//! where appropriate.  Additive changes are allowed within a major
//! version; breaking changes require an ADR and deprecation cycle.

#![doc(html_root_url = "https://docs.rs/mara-core/0.1.0")]

pub mod config;
pub mod error;
pub mod health;
pub mod pipeline;
pub mod policy;
pub mod self_metrics;
pub mod traits;

pub use config::Config;
pub use error::{Error, Result};
pub use health::{Health, HealthStatus};
pub use mara_schema::Event;
pub use pipeline::{Pipeline, PipelineHandle, pipelines_aggregate_ready};
pub use policy::{Policy, PolicyChain, PolicyContext, PolicyOutcome};
pub use self_metrics::{PipelineSelfMetrics, render_prometheus};
pub use traits::{Adapter, EventReceiver, EventSender, Sink};

pub mod prelude {
    //! Prelude re-exports for downstream crates.
    pub use crate::{
        Adapter, Error, Event, EventReceiver, EventSender, Health, HealthStatus, PolicyContext,
        PolicyOutcome, Result, Sink, version,
    };
}

/// Build-time version string for this crate.
#[must_use]
pub const fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_is_non_empty() {
        assert!(!version().is_empty());
    }

    #[test]
    fn prelude_compiles() {
        let _err: Error = Error::Internal("test".into());
        let _ok: Result<()> = Ok(());
    }
}
