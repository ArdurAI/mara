//! HTTP proxy adapter.
//!
//! Binds a local port, forwards every request to a configured
//! upstream LLM endpoint, and captures the request and response
//! body pairs without mutation.  Each runtime crate supplies a
//! [`UpstreamNormalizer`] that translates the captured exchange
//! into canonical Mara events.
//!
//! Used at MVP for Ollama; generalises post-MVP to any OpenAI-compat
//! upstream (OpenAI direct, Anthropic via OpenAI shim, Bedrock,
//! Fireworks, Together, etc.).
//!
//! Detailed design in
//! `plans/08-mvp/12-ollama-integration-design.md`.
//!
//! M0/M3 status: stub.  Implementation lands in MVP week 4.

#![doc(html_root_url = "https://docs.rs/mara-adapter-llm-proxy/0.1.0")]

/// Marker for the proxy adapter's pattern.  Replaced by the real
/// trait surface in MVP week 4.
pub const ADAPTER_KIND: &str = "llm-proxy";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adapter_kind_is_stable() {
        assert_eq!(ADAPTER_KIND, "llm-proxy");
    }
}
