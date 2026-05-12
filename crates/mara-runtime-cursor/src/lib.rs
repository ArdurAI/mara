//! Cursor Agents runtime preset.
//!
//! Tier B integration: Cursor ships hooks-over-stdio as the only
//! programmatic surface (no transcript file, no OTLP exporter for
//! the agent loop).  Mara configures the hooks adapter to receive
//! Agent and Cmd+K hook events.

#![doc(html_root_url = "https://docs.rs/mara-runtime-cursor/0.1.0")]

/// Stable runtime identifier emitted as `mara.source.runtime`.
pub const RUNTIME_ID: &str = "cursor";

/// Suggested Cursor hook configuration JSON to be merged into the
/// user's Cursor settings.
pub fn suggested_cursor_hook_config() -> &'static str {
    r#"{
  "cursor.hooks": {
    "Agent": {
      "PostMessage":  { "command": "mara cursor-hook --event agent.post_message" },
      "PreToolCall":  { "command": "mara cursor-hook --event agent.pre_tool_call" },
      "PostToolCall": { "command": "mara cursor-hook --event agent.post_tool_call" }
    },
    "Cmd+K": {
      "PostEdit": { "command": "mara cursor-hook --event cmdk.post_edit" }
    }
  }
}
"#
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runtime_id_is_stable() {
        assert_eq!(RUNTIME_ID, "cursor");
    }

    #[test]
    fn suggested_hook_config_mentions_canonical_events() {
        let s = suggested_cursor_hook_config();
        assert!(s.contains("agent.post_message"));
        assert!(s.contains("agent.pre_tool_call"));
        assert!(s.contains("cmdk.post_edit"));
    }
}
