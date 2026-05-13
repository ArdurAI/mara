//! Workspace-level end-to-end pipeline tests.
//!
//! Composes real adapters, the policy chain, and real sinks to
//! verify the M2 happy path: JSONL tail → PII redaction → file
//! sink writing the redacted events.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use mara_adapter_jsonl::{JsonlAdapter, JsonlAdapterConfig};
use mara_core::Pipeline;
use mara_core::policy::PolicyChain;
use mara_core::traits::{Adapter, Sink};
use mara_policy::builtin::RegexRedactor;
use mara_sink_file::{FileSink, FileSinkConfig};
use tokio::io::AsyncWriteExt;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn e2e_jsonl_to_file_with_pii_redaction() {
    let dir = tempfile::tempdir().expect("tmpdir");
    let log = dir.path().join("events.jsonl");
    let ckpt = dir.path().join("ckpt");
    let out = dir.path().join("out.jsonl");

    // Pre-write a JSONL event that contains a redaction target.
    {
        let mut f = tokio::fs::File::create(&log).await.unwrap();
        let line = r#"{"event_kind":"prompt","user_email":"alice@example.com","model":"claude"}"#;
        f.write_all(line.as_bytes()).await.unwrap();
        f.write_all(b"\n").await.unwrap();
        f.flush().await.unwrap();
    }

    let adapter: Arc<dyn Adapter> = Arc::new(JsonlAdapter::new(JsonlAdapterConfig::new(
        "in".to_owned(),
        vec![log.clone()],
        ckpt,
    )));

    let sink: Arc<dyn Sink> = Arc::new(FileSink::new(FileSinkConfig {
        name: "out".to_owned(),
        path: out.clone(),
        rotate_bytes: 100 * 1024,
    }));

    let chain = Arc::new(PolicyChain::new(
        "default",
        vec![Arc::new(RegexRedactor::builtin_pii()) as Arc<dyn mara_core::policy::Policy>],
    ));

    let pipeline = Pipeline::new("e2e", vec![adapter.clone()], chain, vec![sink.clone()]);
    let handle = pipeline.start().await.expect("pipeline start");

    // Give the pipeline a moment to process the line.
    let deadline = std::time::Instant::now() + Duration::from_secs(3);
    while !out.exists() || std::fs::metadata(&out).map(|m| m.len()).unwrap_or(0) == 0 {
        if std::time::Instant::now() >= deadline {
            break;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }

    handle.shutdown().await.expect("shutdown");

    let content = std::fs::read_to_string(&out).expect("read out");
    assert!(content.contains("[email]"), "expected redaction; got: {content}");
    assert!(!content.contains("alice@example.com"), "email leaked: {content}");
}

#[test]
fn config_loads_minimal_pipeline() {
    let toml = r#"
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
    let cfg = mara_core::Config::from_toml_str(toml, PathBuf::from("test")).expect("valid config");
    assert_eq!(cfg.pipelines.len(), 1);
    assert_eq!(cfg.pipelines[0].name, "p1");
}

#[test]
fn config_loads_llm_proxy_pipeline() {
    let toml = r#"
schema_version = "1"

[[adapters.llm_proxy]]
name = "ollama_proxy"
http_listen = "127.0.0.1:11435"
upstream = "http://127.0.0.1:11434"
normalizer = "ollama"

[[sinks.stdout]]
name = "out1"

[[pipelines]]
name = "ollama"
adapters = ["ollama_proxy"]
sinks = ["out1"]
"#;
    let cfg = mara_core::Config::from_toml_str(toml, PathBuf::from("test")).expect("valid config");
    assert_eq!(cfg.adapters.llm_proxy.len(), 1);
    assert_eq!(cfg.adapters.llm_proxy[0].name, "ollama_proxy");
    assert_eq!(cfg.adapters.llm_proxy[0].normalizer, "ollama");
    let ollama_p = cfg.pipelines.iter().find(|p| p.name == "ollama").expect("ollama pipeline");
    assert_eq!(ollama_p.adapters, vec!["ollama_proxy".to_owned()]);
}
