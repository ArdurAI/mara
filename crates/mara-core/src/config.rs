//! Configuration types and TOML loader.
//!
//! Per ADR-0007 the canonical configuration format is TOML; YAML
//! is supported as an alternate.  M2 ships the TOML loader and the
//! typed configuration tree.  JSON Schema export and YAML support
//! land in M2 follow-up work.

use std::collections::BTreeMap;
use std::fs;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};

/// Top-level Mara configuration.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    /// Configuration schema version.  Currently `"1"`.
    pub schema_version: SchemaVersion,
    /// Process-wide server settings.
    pub server: ServerConfig,
    /// Adapters keyed by logical name.
    pub adapters: Adapters,
    /// Sinks keyed by logical name.
    pub sinks: Sinks,
    /// Policy chains keyed by chain name.
    pub policies: BTreeMap<String, Vec<PolicyStageConfig>>,
    /// Pipelines that compose adapters, a policy chain, and sinks.
    pub pipelines: Vec<PipelineConfig>,
}

/// Wrapper for the `schema_version` field.  Defaults to `"1"`.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SchemaVersion(pub String);

impl Default for SchemaVersion {
    fn default() -> Self {
        Self("1".into())
    }
}

/// One row of per-model token pricing (USD per 1M tokens) for M1-04 cost estimates.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GenAiModelPriceRow {
    /// Model id prefix; longest matching `starts_with` on the effective model wins.
    pub prefix: String,
    /// USD per 1M input tokens when this row matches.
    pub input_per_million_usd: f64,
    /// USD per 1M output tokens when this row matches.
    pub output_per_million_usd: f64,
}

/// `[server.gen_ai_pricing]` — estimate `mara.cost_usd` from usage + rates.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct GenAiPricingConfig {
    /// When `false`, `mara.cost_usd` is set to `0.0` (placeholder) after normalization.
    pub estimate_enabled: bool,
    /// Default USD per 1M input tokens when no model row matches.
    pub default_input_per_million_usd: f64,
    /// Default USD per 1M output tokens when no model row matches.
    pub default_output_per_million_usd: f64,
    /// Optional prefix rows (longest prefix wins on the effective model id).
    pub models: Vec<GenAiModelPriceRow>,
}

impl Default for GenAiPricingConfig {
    fn default() -> Self {
        Self {
            estimate_enabled: false,
            default_input_per_million_usd: 0.25,
            default_output_per_million_usd: 1.0,
            models: Vec::new(),
        }
    }
}

/// Process-wide server settings.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct ServerConfig {
    /// Address for the self-telemetry metrics endpoint.
    pub metrics_addr: String,
    /// Log format: `"text"` or `"json"`.
    pub log_format: String,
    /// Optional `service.name` on emitted events when the Ollama normalizer runs.
    ///
    /// Non-empty TOML value wins over the `MARA_SERVICE_NAME` environment variable.
    #[serde(default)]
    pub telemetry_service_name: Option<String>,
    /// Optional `service.version` on emitted events when the Ollama normalizer runs.
    ///
    /// Non-empty TOML value wins over `MARA_SERVICE_VERSION`.
    #[serde(default)]
    pub telemetry_service_version: Option<String>,
    /// Optional `GenAI` token pricing for `mara.cost_usd` estimates (M1-04).
    #[serde(default)]
    pub gen_ai_pricing: GenAiPricingConfig,
    /// Max concurrent HTTP connections for [`Self::metrics_addr`] when non-loopback (M2-15).
    /// When unset and the bind address is not loopback, Mara defaults to 64.
    #[serde(default)]
    pub metrics_max_in_flight_connections: Option<usize>,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            metrics_addr: "127.0.0.1:9099".into(),
            log_format: "text".into(),
            telemetry_service_name: None,
            telemetry_service_version: None,
            gen_ai_pricing: GenAiPricingConfig::default(),
            metrics_max_in_flight_connections: None,
        }
    }
}

impl GenAiPricingConfig {
    /// Validate rates and model rows (finite, non-negative; non-empty prefixes).
    pub fn validate(&self, path: &str) -> Result<()> {
        let p = Some(path.to_owned());
        let check_rate = |v: f64, label: &str| -> Result<()> {
            if !v.is_finite() || v < 0.0 {
                return Err(Error::Config {
                    message: format!("{label} must be finite and >= 0 (got {v})"),
                    path: p.clone(),
                });
            }
            Ok(())
        };
        check_rate(self.default_input_per_million_usd, "server.gen_ai_pricing.default_input_per_million_usd")?;
        check_rate(self.default_output_per_million_usd, "server.gen_ai_pricing.default_output_per_million_usd")?;
        for (i, row) in self.models.iter().enumerate() {
            if row.prefix.trim().is_empty() {
                return Err(Error::Config {
                    message: format!("server.gen_ai_pricing.models[{i}].prefix must not be empty"),
                    path: p.clone(),
                });
            }
            check_rate(row.input_per_million_usd, &format!("server.gen_ai_pricing.models[{i}].input_per_million_usd"))?;
            check_rate(row.output_per_million_usd, &format!("server.gen_ai_pricing.models[{i}].output_per_million_usd"))?;
        }
        Ok(())
    }
}

/// Collection of all configured adapters, grouped by adapter kind.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Adapters {
    /// JSONL tail adapters.
    pub jsonl: Vec<JsonlAdapterConfig>,
    /// OTLP HTTP/protobuf receivers.
    pub otlp: Vec<OtlpAdapterConfig>,
    /// HTTP reverse-proxy adapters for LLM upstreams (Ollama, OpenAI-compat).
    pub llm_proxy: Vec<LlmProxyAdapterConfig>,
}

/// Configuration for an OTLP HTTP/protobuf receiver adapter.
///
/// Listens on a configured local address and accepts `POST /v1/logs`
/// and `POST /v1/traces` requests with `application/x-protobuf`
/// bodies, decoded against the OTel protocol schema and translated
/// into canonical Mara events.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OtlpAdapterConfig {
    /// Logical name (must be unique across all adapters).
    pub name: String,
    /// Address to bind for HTTP/protobuf OTLP traffic.  Default
    /// `127.0.0.1:4318` per the OTel spec.
    #[serde(default = "default_otlp_http_listen")]
    pub http_listen: String,
    /// Maximum body size accepted in bytes (default 16 MiB).
    #[serde(default = "default_otlp_max_body_bytes")]
    pub max_body_bytes: usize,
}

fn default_otlp_http_listen() -> String {
    "127.0.0.1:4318".to_owned()
}

const fn default_otlp_max_body_bytes() -> usize {
    16 * 1024 * 1024
}

/// Configuration for the HTTP LLM reverse-proxy adapter.
///
/// Binds `http_listen`, forwards requests to `upstream` preserving
/// path and query, and emits canonical events via the configured
/// `normalizer` (`ollama` or `passthrough`).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LlmProxyAdapterConfig {
    /// Logical name (must be unique across all adapters).
    pub name: String,
    /// Local bind address (e.g. `127.0.0.1:11434`).
    pub http_listen: String,
    /// Upstream base URL (scheme + authority, no path), e.g.
    /// `http://127.0.0.1:11434` for a local Ollama daemon.
    pub upstream: String,
    /// Normalizer profile: `ollama` (default) or `passthrough`.
    #[serde(default = "default_llm_proxy_normalizer")]
    pub normalizer: String,
    /// Maximum request/response body capture per direction (default 10 MiB).
    #[serde(default = "default_llm_proxy_max_body_bytes")]
    pub max_body_bytes: usize,
    /// Allow binding [`http_listen`](LlmProxyAdapterConfig::http_listen) on a non-loopback
    /// address (`0.0.0.0`, LAN IP, etc.). Default `false`; see
    /// `docs/llm-proxy-non-loopback-threat-model.md`.
    #[serde(default)]
    pub allow_non_loopback_listen: bool,
}

fn default_llm_proxy_normalizer() -> String {
    "ollama".into()
}

const fn default_llm_proxy_max_body_bytes() -> usize {
    10 * 1024 * 1024
}

/// Configuration for a JSONL tail adapter.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct JsonlAdapterConfig {
    /// Logical name (must be unique across all adapters).
    pub name: String,
    /// File globs to tail.
    pub globs: Vec<String>,
    /// Path under which to persist per-file offset checkpoints.
    pub checkpoint_path: PathBuf,
    /// Optional parser variant; defaults to plain JSONL.
    #[serde(default)]
    pub parser: ParserVariant,
}

/// Parser variant for the JSONL adapter.
#[derive(Copy, Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ParserVariant {
    /// Plain JSONL.
    #[default]
    Plain,
    /// CRI container log format (timestamp + stream + log).
    Cri,
}

/// Collection of all configured sinks, grouped by sink kind.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Sinks {
    /// File rotation sinks.
    pub file: Vec<FileSinkConfig>,
    /// Stdout / stderr debug sinks.
    pub stdout: Vec<StdoutSinkConfig>,
    /// OTLP HTTP/protobuf log exporters.
    pub otlp: Vec<OtlpSinkConfig>,
}

/// Configuration for the file sink.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FileSinkConfig {
    /// Logical name (must be unique across all sinks).
    pub name: String,
    /// Output path template.  Supports `${YYYY}`, `${MM}`, `${DD}`,
    /// `${HH}`, `${MM}`, `${runtime}` interpolation tokens.
    pub path: PathBuf,
    /// Output format: `"jsonl"` (default) or `"parquet"` (M2 follow-up).
    #[serde(default = "default_jsonl")]
    pub format: String,
    /// Roll to a new file after this many bytes.  Default 64 MiB.
    #[serde(default = "default_rotate_bytes")]
    pub rotate_bytes: u64,
}

fn default_jsonl() -> String {
    "jsonl".into()
}

const fn default_rotate_bytes() -> u64 {
    64 * 1024 * 1024
}

/// Configuration for the stdout sink.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct StdoutSinkConfig {
    /// Logical name.
    pub name: String,
    /// Whether to pretty-print JSON.
    #[serde(default)]
    pub pretty: bool,
}

/// Configuration for an OTLP HTTP/protobuf log sink.
///
/// POSTs `ExportLogsServiceRequest` bodies to `{http_endpoint}/v1/logs`
/// with `Content-Type: application/x-protobuf`.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OtlpSinkConfig {
    /// Logical name (must be unique across all sinks).
    pub name: String,
    /// OTLP/HTTP base URL without path, e.g. `http://127.0.0.1:4318`.
    pub http_endpoint: String,
    /// Flush after this many events (default 64).
    #[serde(default = "default_otlp_sink_batch_max")]
    pub batch_max_events: usize,
    /// HTTP client timeout in seconds (default 30).
    #[serde(default = "default_otlp_sink_timeout_secs")]
    pub timeout_secs: u64,
    /// When `true`, gzip-compress request bodies.
    #[serde(default)]
    pub gzip: bool,
}

const fn default_otlp_sink_batch_max() -> usize {
    64
}

const fn default_otlp_sink_timeout_secs() -> u64 {
    30
}

/// How a `privacy` policy stage treats optional `Event.body` payloads (M1-07).
#[derive(Copy, Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PrivacyCaptureMode {
    /// Drop raw `body` and clear `body_hashes` before sinks.
    #[default]
    MetadataOnly,
    /// Replace `body` with SHA-256 fingerprints in `mara.body_hashes` (hex, lowercase).
    HashedBodies,
    /// Keep `body` only when `mara.policy_capture_optin` is true; otherwise behave like `metadata_only`.
    BodyOptIn,
}

/// A single stage in a policy chain.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum PolicyStageConfig {
    /// Drop or transform captured bodies per [`PrivacyCaptureMode`].
    Privacy {
        /// Body handling mode for downstream sinks.
        #[serde(default)]
        mode: PrivacyCaptureMode,
    },
    /// Built-in regex redaction.
    Redact {
        /// Built-in pack name (`builtin.pii`, etc.).
        pack: String,
    },
    /// Built-in head sampler.
    Sample {
        /// Sampling rate in `[0.0, 1.0]`.
        rate: f64,
    },
    /// Drop matching events.
    Deny {
        /// Optional reason recorded in the audit log.
        #[serde(default)]
        reason: Option<String>,
    },
}

/// A pipeline composes adapters, a policy chain, and sinks.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PipelineConfig {
    /// Pipeline name.
    pub name: String,
    /// Adapter logical names that feed this pipeline.
    pub adapters: Vec<String>,
    /// Name of the policy chain to apply.
    #[serde(default = "default_chain")]
    pub policy_chain: String,
    /// Sink logical names that receive events from this pipeline.
    pub sinks: Vec<String>,
    /// When true, emit a minimal `System` audit event to sinks whenever a policy stage drops an
    /// event (no original body). Default: false (kill-switch / ZDR friendly).
    #[serde(default)]
    pub audit_policy_drops: bool,
}

fn default_chain() -> String {
    "default".into()
}

impl Config {
    /// Load a TOML configuration file.
    pub fn from_file(path: impl AsRef<Path>) -> Result<Self> {
        let path_ref = path.as_ref();
        let raw = fs::read_to_string(path_ref)
            .map_err(|e| Error::Io { path: Some(path_ref.display().to_string()), source: e })?;
        Self::from_toml_str(&raw, path_ref)
    }

    /// Parse a TOML configuration string.
    pub fn from_toml_str(raw: &str, source_path: impl AsRef<Path>) -> Result<Self> {
        let path_str = source_path.as_ref().display().to_string();
        let cfg: Self = toml::from_str(raw)
            .map_err(|e| Error::Config { message: e.to_string(), path: Some(path_str.clone()) })?;
        cfg.validate(&path_str)?;
        Ok(cfg)
    }

    /// Perform structural validation that toml deserialization
    /// cannot express (cross-reference checks, duplicate names).
    pub fn validate(&self, path: &str) -> Result<()> {
        if self.schema_version.0 != "1" {
            return Err(Error::Config {
                message: format!(
                    "unsupported schema_version {:?} (expected \"1\")",
                    self.schema_version.0
                ),
                path: Some(path.into()),
            });
        }

        self.server.gen_ai_pricing.validate(path)?;

        let mut adapter_names = std::collections::HashSet::new();
        for a in &self.adapters.jsonl {
            if !adapter_names.insert(a.name.clone()) {
                return Err(Error::Config {
                    message: format!("duplicate adapter name: {}", a.name),
                    path: Some(path.into()),
                });
            }
        }
        for a in &self.adapters.otlp {
            if !adapter_names.insert(a.name.clone()) {
                return Err(Error::Config {
                    message: format!("duplicate adapter name: {}", a.name),
                    path: Some(path.into()),
                });
            }
        }
        for a in &self.adapters.llm_proxy {
            if !adapter_names.insert(a.name.clone()) {
                return Err(Error::Config {
                    message: format!("duplicate adapter name: {}", a.name),
                    path: Some(path.into()),
                });
            }
        }

        for (idx, a) in self.adapters.llm_proxy.iter().enumerate() {
            let addr: SocketAddr = a.http_listen.trim().parse().map_err(|e| Error::Config {
                message: format!(
                    "adapters.llm_proxy[{idx}].http_listen {:?} is not a valid socket address (expected host:port, e.g. 127.0.0.1:11434): {e}",
                    a.http_listen
                ),
                path: Some(path.into()),
            })?;
            if !addr.ip().is_loopback() && !a.allow_non_loopback_listen {
                return Err(Error::Config {
                    message: format!(
                        "adapters.llm_proxy[{idx}] '{}' listens on {addr}, which is not loopback-only. \
That exposes an OpenAI/Ollama-compatible HTTP surface on the network without in-proxy authentication. \
Set `allow_non_loopback_listen = true` on this adapter only after reading docs/llm-proxy-non-loopback-threat-model.md and placing TLS + access control in front.",
                        a.name
                    ),
                    path: Some(path.into()),
                });
            }
        }

        let mut sink_names = std::collections::HashSet::new();
        for s in &self.sinks.file {
            if !sink_names.insert(s.name.clone()) {
                return Err(Error::Config {
                    message: format!("duplicate sink name: {}", s.name),
                    path: Some(path.into()),
                });
            }
        }
        for s in &self.sinks.stdout {
            if !sink_names.insert(s.name.clone()) {
                return Err(Error::Config {
                    message: format!("duplicate sink name: {}", s.name),
                    path: Some(path.into()),
                });
            }
        }
        for s in &self.sinks.otlp {
            if !sink_names.insert(s.name.clone()) {
                return Err(Error::Config {
                    message: format!("duplicate sink name: {}", s.name),
                    path: Some(path.into()),
                });
            }
        }

        for p in &self.pipelines {
            for a in &p.adapters {
                if !adapter_names.contains(a) {
                    return Err(Error::Config {
                        message: format!("pipeline '{}' references unknown adapter '{a}'", p.name),
                        path: Some(path.into()),
                    });
                }
            }
            for s in &p.sinks {
                if !sink_names.contains(s) {
                    return Err(Error::Config {
                        message: format!("pipeline '{}' references unknown sink '{s}'", p.name),
                        path: Some(path.into()),
                    });
                }
            }
            if !self.policies.contains_key(&p.policy_chain) && p.policy_chain != "default" {
                return Err(Error::Config {
                    message: format!(
                        "pipeline '{}' references unknown policy chain '{}'",
                        p.name, p.policy_chain
                    ),
                    path: Some(path.into()),
                });
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MIN_CONFIG: &str = r#"
schema_version = "1"

[[adapters.jsonl]]
name = "in1"
globs = ["/tmp/events*.jsonl"]
checkpoint_path = "/tmp/mara-ckpt"

[[sinks.stdout]]
name = "out1"

[[pipelines]]
name = "p1"
adapters = ["in1"]
sinks = ["out1"]
"#;

    #[test]
    fn parses_minimal_config() {
        let cfg = Config::from_toml_str(MIN_CONFIG, "test").expect("parse ok");
        assert_eq!(cfg.adapters.jsonl.len(), 1);
        assert_eq!(cfg.sinks.stdout.len(), 1);
        assert_eq!(cfg.pipelines.len(), 1);
        assert_eq!(cfg.pipelines[0].policy_chain, "default");
        assert!(!cfg.pipelines[0].audit_policy_drops);
    }

    #[test]
    fn parses_pipeline_audit_policy_drops() {
        let raw = r#"
schema_version = "1"

[[adapters.jsonl]]
name = "in1"
globs = ["/tmp/events*.jsonl"]
checkpoint_path = "/tmp/mara-ckpt"

[[sinks.stdout]]
name = "out1"

[[pipelines]]
name = "p1"
adapters = ["in1"]
sinks = ["out1"]
audit_policy_drops = true
"#;
        let cfg = Config::from_toml_str(raw, "test").expect("parse ok");
        assert!(cfg.pipelines[0].audit_policy_drops);
    }

    #[test]
    fn parses_server_telemetry_fields() {
        let raw = r#"
schema_version = "1"

[server]
telemetry_service_name = "fixture-mara"
telemetry_service_version = "0.0.0-test"

[[adapters.jsonl]]
name = "in1"
globs = ["/tmp/events*.jsonl"]
checkpoint_path = "/tmp/mara-ckpt"

[[sinks.stdout]]
name = "out1"

[[pipelines]]
name = "p1"
adapters = ["in1"]
sinks = ["out1"]
"#;
        let cfg = Config::from_toml_str(raw, "test").expect("parse ok");
        assert_eq!(cfg.server.telemetry_service_name.as_deref(), Some("fixture-mara"));
        assert_eq!(cfg.server.telemetry_service_version.as_deref(), Some("0.0.0-test"));
    }

    #[test]
    fn rejects_unknown_adapter_reference() {
        let bad = r#"
schema_version = "1"
[[sinks.stdout]]
name = "out1"
[[pipelines]]
name = "p1"
adapters = ["nonexistent"]
sinks = ["out1"]
"#;
        let err = Config::from_toml_str(bad, "test").unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("unknown adapter"), "got: {msg}");
    }

    #[test]
    fn rejects_unknown_schema_version() {
        let bad = r#"
schema_version = "2"
"#;
        let err = Config::from_toml_str(bad, "test").unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("schema_version"), "got: {msg}");
    }

    #[test]
    fn rejects_duplicate_otlp_sink_names() {
        let bad = r#"
schema_version = "1"
[[sinks.otlp]]
name = "dup"
http_endpoint = "http://127.0.0.1:1"
[[sinks.otlp]]
name = "dup"
http_endpoint = "http://127.0.0.1:2"
"#;
        let err = Config::from_toml_str(bad, "test").unwrap_err();
        assert!(err.to_string().contains("duplicate sink"), "got: {err}");
    }

    #[test]
    fn rejects_duplicate_sink_names() {
        let bad = r#"
schema_version = "1"
[[sinks.stdout]]
name = "dup"
[[sinks.stdout]]
name = "dup"
"#;
        let err = Config::from_toml_str(bad, "test").unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("duplicate sink"), "got: {msg}");
    }

    #[test]
    fn rejects_duplicate_llm_proxy_adapter_names() {
        let bad = r#"
schema_version = "1"
[[adapters.llm_proxy]]
name = "dup"
http_listen = "127.0.0.1:11435"
upstream = "http://127.0.0.1:11434"
[[adapters.llm_proxy]]
name = "dup"
http_listen = "127.0.0.1:11436"
upstream = "http://127.0.0.1:11434"
"#;
        let err = Config::from_toml_str(bad, "test").unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("duplicate adapter"), "got: {msg}");
    }

    #[test]
    fn parses_privacy_policy_stage() {
        let raw = r#"
schema_version = "1"

[[policies.zdr]]
type = "privacy"
mode = "hashed_bodies"

[[policies.zdr]]
type = "privacy"

[[policies.zdr]]
type = "sample"
rate = 1.0
"#;
        let cfg = Config::from_toml_str(raw, "test").expect("parse ok");
        let stages = cfg.policies.get("zdr").expect("chain");
        assert!(matches!(
            stages[0],
            PolicyStageConfig::Privacy {
                mode: PrivacyCaptureMode::HashedBodies
            }
        ));
        assert!(matches!(
            stages[1],
            PolicyStageConfig::Privacy {
                mode: PrivacyCaptureMode::MetadataOnly
            }
        ));
        assert!(matches!(stages[2], PolicyStageConfig::Sample { rate } if rate == 1.0));
    }

    #[test]
    fn parses_deny_policy_stage() {
        let raw = r#"
schema_version = "1"

[[policies.block_all]]
type = "deny"
reason = "maintenance window"

[[policies.block_all]]
type = "deny"
"#;
        let cfg = Config::from_toml_str(raw, "test").expect("parse ok");
        let stages = cfg.policies.get("block_all").expect("chain");
        assert!(matches!(
            &stages[0],
            PolicyStageConfig::Deny { reason } if reason.as_deref() == Some("maintenance window")
        ));
        assert!(matches!(&stages[1], PolicyStageConfig::Deny { reason } if reason.is_none()));
    }

    #[test]
    fn parses_gen_ai_pricing_from_toml() {
        let raw = r#"
schema_version = "1"

[server.gen_ai_pricing]
estimate_enabled = true
default_input_per_million_usd = 0.5
default_output_per_million_usd = 1.5

[[server.gen_ai_pricing.models]]
prefix = "llama"
input_per_million_usd = 0.1
output_per_million_usd = 0.2
"#;
        let cfg = Config::from_toml_str(raw, "test").expect("parse");
        assert!(cfg.server.gen_ai_pricing.estimate_enabled);
        assert!((cfg.server.gen_ai_pricing.default_input_per_million_usd - 0.5).abs() < f64::EPSILON);
        assert_eq!(cfg.server.gen_ai_pricing.models.len(), 1);
        assert_eq!(cfg.server.gen_ai_pricing.models[0].prefix, "llama");
    }

    #[test]
    fn rejects_negative_gen_ai_pricing_rate() {
        let raw = r#"
schema_version = "1"

[server.gen_ai_pricing]
default_input_per_million_usd = -1.0
"#;
        let err = Config::from_toml_str(raw, "test").unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("finite") || msg.contains(">="), "got {msg}");
    }

    #[test]
    fn rejects_duplicate_adapter_name_across_otlp_and_llm_proxy() {
        let bad = r#"
schema_version = "1"
[[adapters.otlp]]
name = "same"
http_listen = "127.0.0.1:4318"
[[adapters.llm_proxy]]
name = "same"
http_listen = "127.0.0.1:11435"
upstream = "http://127.0.0.1:11434"
"#;
        let err = Config::from_toml_str(bad, "test").unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("duplicate adapter"), "got: {msg}");
    }

    #[test]
    fn rejects_llm_proxy_non_loopback_listen_without_opt_in() {
        let bad = r#"
schema_version = "1"
[[adapters.llm_proxy]]
name = "edge"
http_listen = "0.0.0.0:11435"
upstream = "http://127.0.0.1:11434"
"#;
        let err = Config::from_toml_str(bad, "test").unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("not loopback"), "got: {msg}");
        assert!(msg.contains("allow_non_loopback_listen"), "got: {msg}");
    }

    #[test]
    fn allows_llm_proxy_non_loopback_listen_with_opt_in() {
        let raw = r#"
schema_version = "1"
[[adapters.llm_proxy]]
name = "edge"
http_listen = "0.0.0.0:11435"
upstream = "http://127.0.0.1:11434"
allow_non_loopback_listen = true
"#;
        let cfg = Config::from_toml_str(raw, "test").expect("parse");
        assert!(cfg.adapters.llm_proxy[0].allow_non_loopback_listen);
    }

    #[test]
    fn rejects_llm_proxy_invalid_http_listen() {
        let bad = r#"
schema_version = "1"
[[adapters.llm_proxy]]
name = "bad"
http_listen = "not-a-socket"
upstream = "http://127.0.0.1:11434"
"#;
        let err = Config::from_toml_str(bad, "test").unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("not a valid socket address"), "got: {msg}");
    }
}
