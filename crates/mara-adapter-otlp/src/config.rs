//! Configuration types for the OTLP receiver adapter.

use std::net::SocketAddr;

use serde::{Deserialize, Serialize};

/// Configuration for an [`OtlpHttpAdapter`](crate::OtlpHttpAdapter).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OtlpHttpAdapterConfig {
    /// Logical name (must be unique within the pipeline).
    pub name: String,
    /// Address to bind for HTTP/protobuf OTLP traffic.  Default
    /// is `127.0.0.1:4318` to match the OTel spec.
    pub http_listen: SocketAddr,
    /// Maximum compressed payload size accepted, in bytes.
    /// Requests larger than this return HTTP 413 without parsing.
    #[serde(default = "default_max_body_bytes")]
    pub max_body_bytes: usize,
}

impl OtlpHttpAdapterConfig {
    /// Construct a config with sensible defaults.  The `name` and
    /// listen address are required; other fields take MVP defaults.
    #[must_use]
    pub fn new(name: impl Into<String>, http_listen: SocketAddr) -> Self {
        Self { name: name.into(), http_listen, max_body_bytes: default_max_body_bytes() }
    }
}

const fn default_max_body_bytes() -> usize {
    // 16 MiB — enough for very large OTel batches; deliberately
    // smaller than the WAL spill threshold.
    16 * 1024 * 1024
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_max_body_bytes_is_16_mib() {
        let cfg = OtlpHttpAdapterConfig::new("test", "127.0.0.1:4318".parse().unwrap());
        assert_eq!(cfg.max_body_bytes, 16 * 1024 * 1024);
    }

    #[test]
    fn name_is_preserved() {
        let cfg = OtlpHttpAdapterConfig::new("ingest", "0.0.0.0:4318".parse().unwrap());
        assert_eq!(cfg.name, "ingest");
    }
}
