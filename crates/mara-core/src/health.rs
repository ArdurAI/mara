//! Health and readiness reporting for adapters, sinks, and policy stages.

use serde::{Deserialize, Serialize};

/// Coarse health classification of a component.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum HealthStatus {
    /// The component has not yet started or is initializing.
    Starting,
    /// The component is running normally.
    Healthy,
    /// The component is running but degraded (e.g., lagging behind ingestion).
    Degraded,
    /// The component has failed and is not currently processing.
    Failed,
    /// The component is shutting down.
    Stopping,
    /// The component has terminated.
    Stopped,
}

/// Detailed health report.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Health {
    /// Coarse status.
    pub status: HealthStatus,
    /// Human-readable status detail.
    pub message: String,
    /// Time since last successful operation, in milliseconds, if applicable.
    pub last_success_ms_ago: Option<u64>,
    /// Number of consecutive failures observed, if applicable.
    pub consecutive_failures: u32,
}

impl Default for Health {
    fn default() -> Self {
        Self {
            status: HealthStatus::Starting,
            message: String::from("initializing"),
            last_success_ms_ago: None,
            consecutive_failures: 0,
        }
    }
}

impl Health {
    /// Construct a healthy report with a default message.
    #[must_use]
    pub fn healthy() -> Self {
        Self {
            status: HealthStatus::Healthy,
            message: String::from("healthy"),
            last_success_ms_ago: Some(0),
            consecutive_failures: 0,
        }
    }

    /// Used for HTTP `/readyz`: only **running** components count as ready.
    ///
    /// Ready: [`HealthStatus::Healthy`] or [`HealthStatus::Degraded`]. Not ready:
    /// [`HealthStatus::Starting`], [`HealthStatus::Stopping`], [`HealthStatus::Stopped`],
    /// [`HealthStatus::Failed`]. See `docs/observability/mara-readyz-semantics.md`.
    #[must_use]
    pub fn is_aggregate_ready(&self) -> bool {
        matches!(self.status, HealthStatus::Healthy | HealthStatus::Degraded)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_starting() {
        let h = Health::default();
        assert_eq!(h.status, HealthStatus::Starting);
        assert_eq!(h.consecutive_failures, 0);
    }

    #[test]
    fn aggregate_ready_excludes_failed() {
        let h = Health {
            status: HealthStatus::Failed,
            message: "x".into(),
            last_success_ms_ago: None,
            consecutive_failures: 1,
        };
        assert!(!h.is_aggregate_ready());
        assert!(Health::healthy().is_aggregate_ready());
        assert!(!Health::default().is_aggregate_ready());
        let degraded = Health {
            status: HealthStatus::Degraded,
            message: "lag".into(),
            last_success_ms_ago: Some(500),
            consecutive_failures: 0,
        };
        assert!(degraded.is_aggregate_ready());
        let stopping = Health {
            status: HealthStatus::Stopping,
            message: "drain".into(),
            last_success_ms_ago: None,
            consecutive_failures: 0,
        };
        assert!(!stopping.is_aggregate_ready());
        let stopped = Health {
            status: HealthStatus::Stopped,
            message: "done".into(),
            last_success_ms_ago: None,
            consecutive_failures: 0,
        };
        assert!(!stopped.is_aggregate_ready());
    }

    #[test]
    fn healthy_constructor_sets_status() {
        let h = Health::healthy();
        assert_eq!(h.status, HealthStatus::Healthy);
        assert_eq!(h.last_success_ms_ago, Some(0));
    }
}
