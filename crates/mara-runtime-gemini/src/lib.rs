//! Gemini CLI runtime preset.
//!
//! Tier A integration via first-party OTel support in `gemini-cli`.
//! Configures the OTLP receiver and honours `logPrompts` opt-in.

#![doc(html_root_url = "https://docs.rs/mara-runtime-gemini/0.1.0")]

/// Stable runtime identifier emitted as `mara.source.runtime`.
pub const RUNTIME_ID: &str = "gemini";

/// Suggested `~/.gemini/settings.json` snippet to point gemini-cli at Mara.
pub fn suggested_gemini_settings() -> &'static str {
    r#"{
  "telemetry": {
    "enabled": true,
    "target": "otlp",
    "otlpEndpoint": "http://127.0.0.1:4317",
    "otlpProtocol": "grpc",
    "logPrompts": false
  }
}
"#
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runtime_id_is_stable() {
        assert_eq!(RUNTIME_ID, "gemini");
    }

    #[test]
    fn suggested_settings_keeps_log_prompts_off_by_default() {
        let s = suggested_gemini_settings();
        assert!(s.contains("\"logPrompts\": false"));
        assert!(s.contains("\"target\": \"otlp\""));
    }
}
