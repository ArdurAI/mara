//! Configuration for the OTLP HTTP/protobuf exporter sink.

/// Runtime configuration for [`super::OtlpHttpSink`].
#[derive(Clone, Debug)]
pub struct OtlpHttpSinkConfig {
    /// Logical sink name (unique within a pipeline).
    pub name: String,
    /// OTLP/HTTP base URL without path, e.g. `http://127.0.0.1:4318`.
    pub http_endpoint: String,
    /// Flush a batch after this many events (clamped to ≥1 at runtime).
    pub batch_max_events: usize,
    /// Per-request HTTP timeout in seconds.
    pub timeout_secs: u64,
    /// When `true`, gzip-compress the protobuf body and set
    /// `Content-Encoding: gzip`.
    pub gzip: bool,
}

impl OtlpHttpSinkConfig {
    /// Construct a sink config with defaults for batching, timeout,
    /// and compression.
    #[must_use]
    pub fn new(name: impl Into<String>, http_endpoint: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            http_endpoint: http_endpoint.into(),
            batch_max_events: 64,
            timeout_secs: 30,
            gzip: false,
        }
    }
}
