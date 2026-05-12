//! Augment Code runtime preset.
//!
//! Tier C integration: Augment ships no public hooks, no local
//! transcript path, and no OTLP exporter for the IDE agent loop.
//! Mara's only path is to poll the Augment Analytics REST API
//! (preview) where the operator opts in.  Gaps documented in the
//! compatibility matrix; graduates to higher tier when Augment
//! ships richer telemetry surfaces.

#![doc(html_root_url = "https://docs.rs/mara-runtime-augment/0.1.0")]

/// Stable runtime identifier emitted as `mara.source.runtime`.
pub const RUNTIME_ID: &str = "augment";

/// Default endpoint for Augment's Analytics REST API.
pub const DEFAULT_ANALYTICS_ENDPOINT: &str = "https://api.augmentcode.com/v1/analytics/events";

/// Default polling interval, in seconds.
pub const DEFAULT_POLL_INTERVAL_SECS: u64 = 60;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runtime_id_is_stable() {
        assert_eq!(RUNTIME_ID, "augment");
    }

    #[test]
    fn default_endpoint_is_https() {
        assert!(DEFAULT_ANALYTICS_ENDPOINT.starts_with("https://"));
    }
}
