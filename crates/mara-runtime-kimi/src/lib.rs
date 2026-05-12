//! Kimi runtime preset.
//!
//! Tier B integration via JSONL tail of `~/.kimi/logs/kimi.log`
//! (requires `--debug`) plus opportunistic ingest of `kimi export`
//! ZIPs.  Optional `stream-json` mode parser for piped invocations.
//! Graduates to Tier A when Moonshot ships stable OTel export.

#![doc(html_root_url = "https://docs.rs/mara-runtime-kimi/0.1.0")]

use std::path::PathBuf;

use mara_adapter_jsonl::JsonlAdapterConfig;

/// Stable runtime identifier emitted as `mara.source.runtime`.
pub const RUNTIME_ID: &str = "kimi";

/// Default Kimi log path (Unix).
#[cfg(unix)]
pub const DEFAULT_LOG_PATH: &str = "$HOME/.kimi/logs/kimi.log";

/// Default Kimi log path (Windows).
#[cfg(windows)]
pub const DEFAULT_LOG_PATH: &str = "%USERPROFILE%\\.kimi\\logs\\kimi.log";

/// Build a default JSONL adapter config for Kimi.
#[must_use]
pub fn default_jsonl_config(checkpoint_dir: PathBuf, paths: Vec<PathBuf>) -> JsonlAdapterConfig {
    JsonlAdapterConfig::new(format!("{RUNTIME_ID}_jsonl"), paths, checkpoint_dir)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runtime_id_is_stable() {
        assert_eq!(RUNTIME_ID, "kimi");
    }
}
