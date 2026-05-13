//! Captured HTTP exchange passed to an [`crate::UpstreamNormalizer`].

use bytes::Bytes;

/// Client request observed at the proxy boundary.
#[derive(Clone, Debug)]
pub struct ProxiedRequest {
    /// HTTP method (e.g. `POST`).
    pub method: String,
    /// Path and query from the inbound URI (e.g. `/api/chat`).
    pub path_and_query: String,
    /// Selected header pairs (hop-by-hop headers omitted).
    pub headers: Vec<(String, String)>,
    /// Request body (possibly truncated).
    pub body: Bytes,
    /// `true` when `body` was cut at the configured byte cap.
    pub body_truncated: bool,
}

/// Upstream response after forwarding.
#[derive(Clone, Debug)]
pub struct ProxiedResponse {
    /// HTTP status code.
    pub status: u16,
    /// Selected header pairs from the upstream response.
    pub headers: Vec<(String, String)>,
    /// Response body (possibly truncated).
    pub body: Bytes,
    /// `true` when `body` was cut at the configured byte cap.
    pub body_truncated: bool,
    /// When the proxy returned a synthetic failure (e.g. 502), a stable reason code
    /// (`client_body_read`, `upstream_transport`, …).
    pub failure_kind: Option<String>,
    /// When the proxy failed after receiving upstream status line, that status (e.g. body read error after HTTP 200).
    pub upstream_status: Option<u16>,
    /// `true` when the client disconnected before the upstream body was fully forwarded (SSE path).
    pub stream_cut_short: bool,
}

impl ProxiedResponse {
    /// Response with no proxy-side failure metadata.
    #[must_use]
    pub fn from_upstream(
        status: u16,
        headers: Vec<(String, String)>,
        body: Bytes,
        body_truncated: bool,
    ) -> Self {
        Self {
            status,
            headers,
            body,
            body_truncated,
            failure_kind: None,
            upstream_status: None,
            stream_cut_short: false,
        }
    }
}
