//! Vendor analytics HTTP poller (Tier C).
//!
//! `GET`s a configured URL on an interval, parses a JSON array of
//! [`Event`] or newline-delimited JSON events, and forwards them to
//! the pipeline. Uses a small checkpoint file for HTTP validators
//! (`ETag` / `Last-Modified`) to avoid re-ingesting unchanged bodies.

#![doc(html_root_url = "https://docs.rs/mara-adapter-analytics/0.1.0")]

use std::path::PathBuf;
use std::time::Duration;

use async_trait::async_trait;
use mara_core::error::{Error, Result};
use mara_core::health::Health;
use mara_core::traits::{Adapter, EventSender};
use mara_schema::Event;
use reqwest::header::{ETAG, IF_MODIFIED_SINCE, IF_NONE_MATCH, LAST_MODIFIED};
use reqwest::Url;
use tokio::fs;
use tokio::sync::Notify;
use tokio::time::sleep;
use tracing::{debug, info, warn};

/// Configuration for [`AnalyticsHttpAdapter`].
#[derive(Clone, Debug)]
pub struct AnalyticsHttpAdapterConfig {
    /// Logical adapter name.
    pub name: String,
    /// Full URL polled with `GET` each interval.
    pub url: Url,
    /// Successful-poll interval in seconds.
    pub poll_interval_secs: u64,
    /// Checkpoint file for `ETag` / `Last-Modified` validators.
    pub checkpoint_path: PathBuf,
}

impl AnalyticsHttpAdapterConfig {
    /// Construct from parsed URL and poll settings.
    #[must_use]
    pub fn new(name: impl Into<String>, url: Url, poll_interval_secs: u64, checkpoint_path: PathBuf) -> Self {
        Self { name: name.into(), url, poll_interval_secs, checkpoint_path }
    }
}

/// Polls a remote HTTP endpoint for event batches.
pub struct AnalyticsHttpAdapter {
    cfg: AnalyticsHttpAdapterConfig,
    stop: std::sync::Arc<Notify>,
}

impl AnalyticsHttpAdapter {
    /// Create an analytics poller from configuration.
    #[must_use]
    pub fn new(cfg: AnalyticsHttpAdapterConfig) -> Self {
        Self { cfg, stop: std::sync::Arc::new(Notify::new()) }
    }
}

#[async_trait]
impl Adapter for AnalyticsHttpAdapter {
    fn name(&self) -> &str {
        &self.cfg.name
    }

    async fn start(&self, out: EventSender) -> Result<()> {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(120))
            .build()
            .map_err(|e| Error::Config {
                message: format!("analytics http client: {e}"),
                path: None,
            })?;
        info!(adapter = %self.cfg.name, url = %self.cfg.url, "analytics poller starting");
        let stop = self.stop.clone();
        let mut backoff = Duration::from_secs(1);
        let interval = Duration::from_secs(self.cfg.poll_interval_secs.max(1));
        let mut next_sleep = Duration::ZERO;

        loop {
            tokio::select! {
                () = stop.notified() => {
                    debug!(adapter = %self.cfg.name, "analytics shutdown");
                    return Ok(());
                }
                _ = sleep(next_sleep) => {}
            }

            let ck = read_checkpoint(&self.cfg.checkpoint_path).await;
            let mut req = client.get(self.cfg.url.clone());
            if let Some(ref etag) = ck.etag {
                req = req.header(IF_NONE_MATCH, etag);
            }
            if let Some(ref ims) = ck.last_modified {
                req = req.header(IF_MODIFIED_SINCE, ims);
            }
            match req.send().await {
                Ok(resp) => {
                    let status = resp.status();
                    if status == reqwest::StatusCode::NOT_MODIFIED {
                        backoff = Duration::from_secs(1);
                        next_sleep = interval;
                        continue;
                    }
                    if !status.is_success() {
                        warn!(adapter = %self.cfg.name, %status, "analytics GET failed; backing off");
                        next_sleep = backoff;
                        backoff = (backoff * 2).min(Duration::from_secs(300));
                        continue;
                    }
                    let etag = resp.headers().get(ETAG).and_then(|v| v.to_str().ok()).map(str::to_owned);
                    let last_modified =
                        resp.headers().get(LAST_MODIFIED).and_then(|v| v.to_str().ok()).map(str::to_owned);
                    let body = match resp.bytes().await {
                        Ok(b) => b,
                        Err(e) => {
                            warn!(adapter = %self.cfg.name, "read body: {e}");
                            next_sleep = backoff;
                            backoff = (backoff * 2).min(Duration::from_secs(300));
                            continue;
                        }
                    };
                    match parse_events_body(&body) {
                        Ok(events) => {
                            for mut ev in events {
                                if ev.mara.source_adapter.is_none() {
                                    ev.mara.source_adapter = Some(self.cfg.name.clone());
                                }
                                if out.send(ev).await.is_err() {
                                    return Ok(());
                                }
                            }
                            if let Err(e) =
                                write_checkpoint(&self.cfg.checkpoint_path, etag, last_modified).await
                            {
                                warn!(adapter = %self.cfg.name, "checkpoint write: {e}");
                            }
                            backoff = Duration::from_secs(1);
                            next_sleep = interval;
                        }
                        Err(e) => {
                            warn!(adapter = %self.cfg.name, "parse: {e}");
                            next_sleep = backoff;
                            backoff = (backoff * 2).min(Duration::from_secs(300));
                        }
                    }
                }
                Err(e) => {
                    warn!(adapter = %self.cfg.name, "request: {e}");
                    next_sleep = backoff;
                    backoff = (backoff * 2).min(Duration::from_secs(300));
                }
            }
        }
    }

    async fn shutdown(&self) -> Result<()> {
        self.stop.notify_waiters();
        Ok(())
    }

    fn health(&self) -> Health {
        Health::healthy()
    }
}

#[derive(Default, Clone)]
struct Checkpoint {
    etag: Option<String>,
    last_modified: Option<String>,
}

async fn read_checkpoint(path: &std::path::Path) -> Checkpoint {
    let raw = match fs::read_to_string(path).await {
        Ok(s) => s,
        Err(_) => return Checkpoint::default(),
    };
    let mut ck = Checkpoint::default();
    for line in raw.lines() {
        let line = line.trim();
        if let Some(v) = line.strip_prefix("etag:") {
            ck.etag = Some(v.trim().to_owned());
        } else if let Some(v) = line.strip_prefix("last_modified:") {
            ck.last_modified = Some(v.trim().to_owned());
        }
    }
    ck
}

async fn write_checkpoint(
    path: &std::path::Path,
    etag: Option<String>,
    last_modified: Option<String>,
) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).await.map_err(|e| Error::Io {
            path: Some(parent.display().to_string()),
            source: e,
        })?;
    }
    let mut s = String::new();
    if let Some(e) = etag {
        s.push_str("etag: ");
        s.push_str(&e);
        s.push('\n');
    }
    if let Some(l) = last_modified {
        s.push_str("last_modified: ");
        s.push_str(&l);
        s.push('\n');
    }
    fs::write(path, s).await.map_err(|e| Error::Io {
        path: Some(path.display().to_string()),
        source: e,
    })
}

fn parse_events_body(body: &[u8]) -> std::result::Result<Vec<Event>, String> {
    let trimmed = std::str::from_utf8(body).map_err(|e| e.to_string())?.trim();
    if trimmed.is_empty() {
        return Ok(Vec::new());
    }
    if trimmed.starts_with('[') {
        return serde_json::from_str(trimmed).map_err(|e| e.to_string());
    }
    let mut out = Vec::new();
    for line in trimmed.lines() {
        let t = line.trim();
        if t.is_empty() {
            continue;
        }
        out.push(serde_json::from_str(t).map_err(|e| e.to_string())?);
    }
    Ok(out)
}
