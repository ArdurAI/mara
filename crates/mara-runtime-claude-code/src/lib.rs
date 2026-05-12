//! Claude Code runtime preset.
//!
//! Configures Mara to capture from Anthropic Claude Code via OTLP
//! receive (when `CLAUDE_CODE_ENABLE_TELEMETRY=1`) plus JSONL tail
//! of `~/.claude/projects/*.jsonl` as redundant signal.  Honours
//! ZDR toggles `OTEL_LOG_USER_PROMPTS`, `OTEL_LOG_TOOL_DETAILS`,
//! `OTEL_LOG_RAW_API_BODIES`.
//!
//! Tier A (OTLP) + Tier B (JSONL) in the integration model.

#![doc(html_root_url = "https://docs.rs/mara-runtime-claude-code/0.1.0")]

use std::path::PathBuf;

use mara_adapter_jsonl::JsonlAdapterConfig;

/// Stable runtime identifier emitted as `mara.source.runtime`.
pub const RUNTIME_ID: &str = "claude_code";

/// Default Claude Code transcript glob pattern (Unix paths).
#[cfg(unix)]
pub const DEFAULT_TRANSCRIPT_GLOB: &str = "$HOME/.claude/projects/**/*.jsonl";

/// Default Claude Code transcript glob pattern (Windows paths).
#[cfg(windows)]
pub const DEFAULT_TRANSCRIPT_GLOB: &str = "%USERPROFILE%\\.claude\\projects\\**\\*.jsonl";

/// Build a default JSONL adapter config for Claude Code.
#[must_use]
pub fn default_jsonl_config(checkpoint_dir: PathBuf, paths: Vec<PathBuf>) -> JsonlAdapterConfig {
    JsonlAdapterConfig::new(format!("{RUNTIME_ID}_jsonl"), paths, checkpoint_dir)
}

/// Environment variable names Mara reads to honour ZDR toggles.
pub mod env {
    /// Anthropic Claude Code prompt-capture opt-in.
    pub const LOG_USER_PROMPTS: &str = "OTEL_LOG_USER_PROMPTS";
    /// Anthropic Claude Code tool-detail capture opt-in.
    pub const LOG_TOOL_DETAILS: &str = "OTEL_LOG_TOOL_DETAILS";
    /// Anthropic Claude Code raw-API-body capture opt-in.
    pub const LOG_RAW_API_BODIES: &str = "OTEL_LOG_RAW_API_BODIES";
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runtime_id_is_stable() {
        assert_eq!(RUNTIME_ID, "claude_code");
    }

    #[test]
    fn default_glob_targets_user_home() {
        #[cfg(unix)]
        assert!(DEFAULT_TRANSCRIPT_GLOB.contains(".claude/projects"));
        #[cfg(windows)]
        assert!(DEFAULT_TRANSCRIPT_GLOB.contains(".claude\\projects"));
    }

    #[test]
    fn jsonl_config_uses_runtime_id_in_name() {
        let cfg =
            default_jsonl_config(PathBuf::from("/tmp/ckpt"), vec![PathBuf::from("/tmp/log.jsonl")]);
        assert_eq!(cfg.name, "claude_code_jsonl");
    }
}
