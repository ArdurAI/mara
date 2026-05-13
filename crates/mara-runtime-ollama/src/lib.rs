//! Ollama runtime preset.
//!
//! Captures local LLM activity from Ollama (<https://ollama.com>)
//! via the `mara-adapter-llm-proxy` HTTP proxy adapter.  Ollama
//! ships no first-party OTel exporter and no Prometheus `/metrics`
//! endpoint as of release v0.21.0 (April 2026); proxy-based
//! capture is the only path that gets per-request token counts
//! and latency from the API response body.
//!
//! Mode of operation (drop-in replacement): Ollama is reconfigured
//! to listen on `127.0.0.1:11435` via `OLLAMA_HOST`; Mara binds
//! the conventional `127.0.0.1:11434` and forwards.  Clients
//! (the `ollama` CLI, `Open WebUI`, `Continue.dev`, OpenAI-SDK code)
//! see no behavioural change.
//!
//! Endpoint coverage:
//! - Native: `/api/chat`, `/api/generate`, `/api/embed`.
//! - OpenAI-compat: `/v1/chat/completions`, `/v1/completions`,
//!   `/v1/embeddings`.
//! - `/v1/responses` (Ollama v0.13.3+) and `/v1/images/generations`
//!   (experimental) deferred to MVP+1.
//!
//! Field mapping (from response body):
//! - `prompt_eval_count` -> `gen_ai.usage.input_tokens`.
//! - `eval_count` -> `gen_ai.usage.output_tokens`.
//! - `total_duration` ns -> `mara.ollama.total_duration_ms`.
//! - `load_duration` ns -> `mara.ollama.load_duration_ms`.
//! - `prompt_eval_duration` ns -> `mara.ollama.prompt_eval_duration_ms`.
//! - `eval_duration` ns -> `mara.ollama.eval_duration_ms`.
//! - `eval_count / (eval_duration / 1e9)` -> `mara.ollama.tokens_per_sec`.
//! - `mara.cost.usd = 0`, `mara.cost.source = "mara_estimated"`,
//!   `mara.compute.is_local = true`.
//!
//! References:
//! - <https://docs.ollama.com/api> (endpoint index)
//! - <https://docs.ollama.com/api/usage> (response telemetry fields)
//! - <https://docs.ollama.com/api/openai-compatibility> (OpenAI-compat routes)
//! - <https://docs.ollama.com/faq> (env vars, reverse-proxy patterns)
//! - <https://github.com/ollama/ollama/blob/main/LICENSE> (MIT)
//!
//! M0/M3 status: normalizer + `mara setup ollama` wired; see MVP week 5 plan.

#![doc(html_root_url = "https://docs.rs/mara-runtime-ollama/0.1.0")]

mod normalizer;

pub use normalizer::OllamaNormalizer;

/// Stable runtime identifier emitted as `mara.source.runtime`.
pub const RUNTIME_ID: &str = "ollama";

/// Default Mara-side listen address when proxying Ollama.
pub const DEFAULT_PROXY_LISTEN: &str = "127.0.0.1:11434";

/// Default Ollama upstream address when Mara owns the conventional port.
pub const DEFAULT_OLLAMA_UPSTREAM: &str = "127.0.0.1:11435";

/// Suggested macOS reconfiguration to move Ollama off the conventional port.
pub fn suggested_macos_reconfig() -> &'static str {
    r#"# Move Ollama to port 11435 so Mara can proxy on 11434
sudo launchctl setenv OLLAMA_HOST '127.0.0.1:11435'
brew services restart ollama
"#
}

/// Suggested Linux (systemd user) reconfiguration.
pub fn suggested_linux_reconfig() -> &'static str {
    r#"# Move Ollama to port 11435 via a systemd override
mkdir -p ~/.config/systemd/user/ollama.service.d
cat > ~/.config/systemd/user/ollama.service.d/override.conf <<'EOF'
[Service]
Environment="OLLAMA_HOST=127.0.0.1:11435"
EOF
systemctl --user daemon-reload
systemctl --user restart ollama
"#
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runtime_id_is_stable() {
        assert_eq!(RUNTIME_ID, "ollama");
    }

    #[test]
    fn default_ports_split_conventional_and_upstream() {
        assert!(DEFAULT_PROXY_LISTEN.ends_with(":11434"));
        assert!(DEFAULT_OLLAMA_UPSTREAM.ends_with(":11435"));
    }

    #[test]
    fn macos_reconfig_uses_launchctl() {
        let s = suggested_macos_reconfig();
        assert!(s.contains("launchctl setenv OLLAMA_HOST"));
        assert!(s.contains("11435"));
    }

    #[test]
    fn linux_reconfig_uses_systemd_override() {
        let s = suggested_linux_reconfig();
        assert!(s.contains("systemd/user/ollama.service.d"));
        assert!(s.contains("OLLAMA_HOST=127.0.0.1:11435"));
    }
}
