//! Codex runtime preset.
//!
//! Configures Mara to capture from OpenAI Codex CLI via the
//! Codex `[otel]` config block (OTLP receive), `~/.codex/history.jsonl`
//! tail, and the experimental `notify` hook.  Honours
//! `log_user_prompt = false` default; supports opt-in.
//!
//! Tier A (OTLP) + Tier B (JSONL + notify hook).

#![doc(html_root_url = "https://docs.rs/mara-runtime-codex/0.1.0")]

use std::path::PathBuf;

use mara_adapter_jsonl::JsonlAdapterConfig;

/// Stable runtime identifier emitted as `mara.source.runtime`.
pub const RUNTIME_ID: &str = "codex";

/// Default Codex history file (Unix).
#[cfg(unix)]
pub const DEFAULT_HISTORY_PATH: &str = "$CODEX_HOME/history.jsonl";

/// Default Codex history file (Windows).
#[cfg(windows)]
pub const DEFAULT_HISTORY_PATH: &str = "%CODEX_HOME%\\history.jsonl";

/// Build a default JSONL adapter config for Codex history.
#[must_use]
pub fn default_jsonl_config(checkpoint_dir: PathBuf, paths: Vec<PathBuf>) -> JsonlAdapterConfig {
    JsonlAdapterConfig::new(format!("{RUNTIME_ID}_jsonl"), paths, checkpoint_dir)
}

/// Suggested Codex `[otel]` block to point Codex at a local Mara OTLP receiver.
pub fn suggested_codex_otel_config() -> &'static str {
    r#"# Append to ~/.codex/config.toml
[otel]
exporter = "otlp"
otlp_endpoint = "http://127.0.0.1:4317"
otlp_protocol = "grpc"
log_user_prompt = false

[notify]
type = "exec"
command = ["mara", "codex-hook"]

[analytics]
enabled = false
"#
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runtime_id_is_stable() {
        assert_eq!(RUNTIME_ID, "codex");
    }

    #[test]
    fn suggested_config_disables_vendor_analytics_and_logs() {
        let s = suggested_codex_otel_config();
        assert!(s.contains("analytics"));
        assert!(s.contains("enabled = false"));
        assert!(s.contains("log_user_prompt = false"));
    }
}
