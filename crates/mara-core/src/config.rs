//! Configuration types and TOML loader.
//!
//! Per ADR-0007 the canonical configuration format is TOML; YAML
//! is supported as an alternate.  M2 ships the TOML loader and the
//! typed configuration tree.  JSON Schema export and YAML support
//! land in M2 follow-up work.

use std::collections::BTreeMap;
use std::fs;
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

/// Process-wide server settings.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct ServerConfig {
    /// Address for the self-telemetry metrics endpoint.
    pub metrics_addr: String,
    /// Log format: `"text"` or `"json"`.
    pub log_format: String,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self { metrics_addr: "127.0.0.1:9099".into(), log_format: "text".into() }
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

/// A single stage in a policy chain.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum PolicyStageConfig {
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
}
