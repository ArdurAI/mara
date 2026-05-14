//! Stable string codes for synthetic proxy failures (`mara.proxy.failure_kind`).

/// Reason the LLM proxy returned a synthetic **502** (or failed after observing upstream status).
///
/// Values are emitted as lowercase `snake_case` strings on events. Do not rename existing
/// variants without a semver note and a doc update in `docs/ollama-proxy-error-taxonomy.md`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ProxyFailureKind {
    /// Failed to read the inbound HTTP body from the client.
    ClientBodyRead,
    /// Upstream base URI is missing an authority component.
    UpstreamConfig,
    /// Could not join inbound path/query with upstream base.
    UpstreamJoin,
    /// Failed while building forwardable hop-by-hop headers.
    HeaderForward,
    /// Hyper could not build the outbound upstream request.
    UpstreamRequestBuild,
    /// TCP/TLS/DNS or other transport error calling upstream.
    UpstreamTransport,
    /// Upstream returned headers but reading the body failed (non-SSE path).
    UpstreamBodyRead,
    /// Upstream connect, header, body, or SSE idle deadline exceeded.
    UpstreamTimeout,
}

impl ProxyFailureKind {
    /// Stable `snake_case` string stored on events as `mara.proxy.failure_kind`.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ClientBodyRead => "client_body_read",
            Self::UpstreamConfig => "upstream_config",
            Self::UpstreamJoin => "upstream_join",
            Self::HeaderForward => "header_forward",
            Self::UpstreamRequestBuild => "upstream_request_build",
            Self::UpstreamTransport => "upstream_transport",
            Self::UpstreamBodyRead => "upstream_body_read",
            Self::UpstreamTimeout => "upstream_timeout",
        }
    }

    /// Every variant that may appear on events today (regression guard).
    #[must_use]
    pub const fn all_variants() -> &'static [ProxyFailureKind] {
        &[
            Self::ClientBodyRead,
            Self::UpstreamConfig,
            Self::UpstreamJoin,
            Self::HeaderForward,
            Self::UpstreamRequestBuild,
            Self::UpstreamTransport,
            Self::UpstreamBodyRead,
            Self::UpstreamTimeout,
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn failure_kind_strings_are_unique_and_stable() {
        let kinds = ProxyFailureKind::all_variants();
        let mut seen = std::collections::HashSet::new();
        for k in kinds {
            assert!(seen.insert(k.as_str()), "duplicate as_str: {}", k.as_str());
        }
    }
}
