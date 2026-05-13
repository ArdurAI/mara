//! Implementation of `mara run` and `mara validate`.
//!
//! Loads a TOML configuration, builds the configured adapters /
//! sinks / policy chains, composes them into [`Pipeline`]s, and
//! starts them.  Waits for SIGTERM / SIGINT for graceful drain.

use std::path::Path;
use std::sync::Arc;

use mara_adapter_jsonl::{JsonlAdapter, JsonlAdapterConfig};
use mara_adapter_llm_proxy::{
    LlmProxyAdapter, LlmProxyAdapterConfig, PassthroughNormalizer, UpstreamNormalizer,
};
use mara_adapter_otlp::{OtlpHttpAdapter, OtlpHttpAdapterConfig};
use mara_core::config::{
    Config, FileSinkConfig as ConfigFileSinkConfig, JsonlAdapterConfig as CfgJsonl,
    LlmProxyAdapterConfig as CfgLlmProxy, OtlpAdapterConfig as CfgOtlp,
    OtlpSinkConfig as CfgOtlpSink, PipelineConfig, PolicyStageConfig, ServerConfig,
    StdoutSinkConfig as ConfigStdoutSinkConfig,
};
use mara_core::policy::{Policy, PolicyChain};
use mara_core::traits::{Adapter, Sink};
use mara_core::{Error, Pipeline};
use mara_policy::builtin::{HeadSampler, RegexRedactor};
use mara_runtime_ollama::OllamaNormalizer;
use mara_sink_file::{FileSink, FileSinkConfig, StdoutSink};
use mara_sink_otlp::{OtlpHttpSink, OtlpHttpSinkConfig};
use tracing::{info, warn};

/// Run the agent against a configuration file.
pub async fn run(config_path: Option<&Path>) -> anyhow::Result<()> {
    let cfg = load_config(config_path)?;

    // Build adapters by name.
    let adapters_by_name = build_adapters(
        &cfg.server,
        &cfg.adapters.jsonl,
        &cfg.adapters.otlp,
        &cfg.adapters.llm_proxy,
    );

    // Build sinks by name.
    let sinks_by_name = build_sinks(&cfg.sinks.file, &cfg.sinks.stdout, &cfg.sinks.otlp);

    // Build policy chains.
    let chains_by_name = build_policy_chains(&cfg.policies);

    // Compose pipelines.
    let mut handles = Vec::new();
    for p in &cfg.pipelines {
        let handle =
            compose_pipeline(p, &adapters_by_name, &sinks_by_name, &chains_by_name).await?;
        handles.push(handle);
    }

    info!(pipelines = handles.len(), "all pipelines running; waiting for shutdown");
    wait_for_shutdown().await;

    info!("shutdown signal received; draining pipelines");
    for h in handles {
        if let Err(e) = h.shutdown().await {
            warn!("pipeline shutdown error: {e}");
        }
    }
    info!("clean shutdown complete");
    Ok(())
}

/// Validate a configuration without starting pipelines.
pub fn validate(config_path: Option<&Path>) -> anyhow::Result<()> {
    let _cfg = load_config(config_path)?;
    println!("configuration is valid");
    Ok(())
}

fn load_config(path: Option<&Path>) -> anyhow::Result<Config> {
    if let Some(p) = path {
        Config::from_file(p).map_err(Into::into)
    } else {
        info!("no --config provided; using built-in defaults (no pipelines)");
        Ok(Config::default())
    }
}

fn llm_proxy_normalizer(kind: &str, server: &ServerConfig) -> Arc<dyn UpstreamNormalizer> {
    match kind {
        "ollama" => Arc::new(OllamaNormalizer::from_server(server)),
        "passthrough" => Arc::new(PassthroughNormalizer),
        _ => {
            warn!(
                normalizer = %kind,
                "unknown llm_proxy normalizer; using passthrough"
            );
            Arc::new(PassthroughNormalizer)
        }
    }
}

fn build_adapters(
    server: &ServerConfig,
    jsonl_cfgs: &[CfgJsonl],
    otlp_cfgs: &[CfgOtlp],
    llm_proxy_cfgs: &[CfgLlmProxy],
) -> std::collections::HashMap<String, Arc<dyn Adapter>> {
    let mut out: std::collections::HashMap<String, Arc<dyn Adapter>> =
        std::collections::HashMap::new();
    for c in jsonl_cfgs {
        let paths: Vec<std::path::PathBuf> = c.globs.iter().map(std::path::PathBuf::from).collect();
        let cfg = JsonlAdapterConfig::new(c.name.clone(), paths, c.checkpoint_path.clone());
        out.insert(c.name.clone(), Arc::new(JsonlAdapter::new(cfg)));
    }
    for c in otlp_cfgs {
        match c.http_listen.parse() {
            Ok(addr) => {
                let mut cfg = OtlpHttpAdapterConfig::new(c.name.clone(), addr);
                cfg.max_body_bytes = c.max_body_bytes;
                out.insert(c.name.clone(), Arc::new(OtlpHttpAdapter::new(cfg)));
            }
            Err(err) => {
                warn!(adapter = %c.name, listen = %c.http_listen, "invalid http_listen address; skipping: {err}");
            }
        }
    }
    for c in llm_proxy_cfgs {
        let listen: std::net::SocketAddr = match c.http_listen.parse() {
            Ok(a) => a,
            Err(err) => {
                warn!(
                    adapter = %c.name,
                    listen = %c.http_listen,
                    "invalid llm_proxy http_listen; skipping: {err}"
                );
                continue;
            }
        };
        let upstream: http::Uri = match std::str::FromStr::from_str(c.upstream.trim()) {
            Ok(u) => u,
            Err(err) => {
                warn!(
                    adapter = %c.name,
                    upstream = %c.upstream,
                    "invalid llm_proxy upstream; skipping: {err}"
                );
                continue;
            }
        };
        let mut pcfg = LlmProxyAdapterConfig::new(c.name.clone(), listen, upstream);
        pcfg.max_body_bytes = c.max_body_bytes;
        let nz = llm_proxy_normalizer(c.normalizer.as_str(), server);
        out.insert(c.name.clone(), Arc::new(LlmProxyAdapter::new(pcfg, nz)));
    }
    out
}

fn build_sinks(
    files: &[ConfigFileSinkConfig],
    stdouts: &[ConfigStdoutSinkConfig],
    otlp: &[CfgOtlpSink],
) -> std::collections::HashMap<String, Arc<dyn Sink>> {
    let mut out: std::collections::HashMap<String, Arc<dyn Sink>> =
        std::collections::HashMap::new();
    for f in files {
        out.insert(
            f.name.clone(),
            Arc::new(FileSink::new(FileSinkConfig {
                name: f.name.clone(),
                path: f.path.clone(),
                rotate_bytes: f.rotate_bytes,
            })),
        );
    }
    for s in stdouts {
        out.insert(s.name.clone(), Arc::new(StdoutSink::new(s.name.clone(), s.pretty)));
    }
    for c in otlp {
        let mut sink_cfg = OtlpHttpSinkConfig::new(c.name.clone(), c.http_endpoint.clone());
        sink_cfg.batch_max_events = c.batch_max_events;
        sink_cfg.timeout_secs = c.timeout_secs;
        sink_cfg.gzip = c.gzip;
        match OtlpHttpSink::new(sink_cfg) {
            Ok(sink) => {
                out.insert(c.name.clone(), Arc::new(sink));
            }
            Err(e) => {
                warn!(sink = %c.name, error = %e, "invalid OTLP sink configuration; skipping");
            }
        }
    }
    out
}

fn build_policy_chains(
    cfg: &std::collections::BTreeMap<String, Vec<PolicyStageConfig>>,
) -> std::collections::HashMap<String, Arc<PolicyChain>> {
    let mut out: std::collections::HashMap<String, Arc<PolicyChain>> =
        std::collections::HashMap::new();
    for (chain_name, stages) in cfg {
        let mut built: Vec<Arc<dyn Policy>> = Vec::new();
        for stage in stages {
            match stage {
                PolicyStageConfig::Redact { pack } => {
                    if pack == "builtin.pii" {
                        built.push(Arc::new(RegexRedactor::builtin_pii()));
                    } else {
                        warn!("unknown redact pack '{pack}'; ignoring stage");
                    }
                }
                PolicyStageConfig::Sample { rate } => {
                    built.push(Arc::new(HeadSampler::new(*rate)));
                }
                PolicyStageConfig::Deny { reason: _ } => {
                    // Deny is not yet implemented as a built-in; a no-op for now.
                    warn!("policy stage 'deny' not yet implemented; ignoring");
                }
            }
        }
        out.insert(chain_name.clone(), Arc::new(PolicyChain::new(chain_name.clone(), built)));
    }
    // Provide a default empty chain if none configured.
    out.entry("default".into())
        .or_insert_with(|| Arc::new(PolicyChain::new("default", Vec::new())));
    out
}

async fn compose_pipeline(
    p: &PipelineConfig,
    adapters: &std::collections::HashMap<String, Arc<dyn Adapter>>,
    sinks: &std::collections::HashMap<String, Arc<dyn Sink>>,
    chains: &std::collections::HashMap<String, Arc<PolicyChain>>,
) -> anyhow::Result<mara_core::PipelineHandle> {
    let chosen_adapters = p
        .adapters
        .iter()
        .map(|name| {
            adapters.get(name).cloned().ok_or_else(|| Error::Config {
                message: format!("adapter '{name}' missing for pipeline '{}'", p.name),
                path: None,
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    let chosen_sinks = p
        .sinks
        .iter()
        .map(|name| {
            sinks.get(name).cloned().ok_or_else(|| Error::Config {
                message: format!("sink '{name}' missing for pipeline '{}'", p.name),
                path: None,
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    let chain = chains.get(&p.policy_chain).cloned().ok_or_else(|| Error::Config {
        message: format!("policy chain '{}' missing for pipeline '{}'", p.policy_chain, p.name),
        path: None,
    })?;

    let pipeline = Pipeline::new(p.name.clone(), chosen_adapters, chain, chosen_sinks);
    Ok(pipeline.start().await?)
}

#[cfg(unix)]
async fn wait_for_shutdown() {
    use tokio::signal::unix::{SignalKind, signal};
    let mut sigterm = signal(SignalKind::terminate()).expect("install SIGTERM handler");
    let mut sigint = signal(SignalKind::interrupt()).expect("install SIGINT handler");
    tokio::select! {
        _ = sigterm.recv() => info!("SIGTERM received"),
        _ = sigint.recv() => info!("SIGINT received"),
    }
}

#[cfg(not(unix))]
async fn wait_for_shutdown() {
    let _ = tokio::signal::ctrl_c().await;
    info!("Ctrl-C received");
}
