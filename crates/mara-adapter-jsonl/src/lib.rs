//! JSONL tail adapter.
//!
//! Tails one or more files containing newline-delimited JSON.
//! Each line is parsed as JSON, normalised into a canonical Mara
//! event, and forwarded to the pipeline's policy chain.  A per-file
//! offset is persisted to a checkpoint directory so the adapter
//! resumes correctly across restarts.
//!
//! M2 ships the synchronous reader pattern (open file, read from
//! offset, sleep, repeat). Optional **hot tail** via `notify` wakes
//! the loop sooner on Unix/macOS when `notify_hot_tail` is enabled.

#![doc(html_root_url = "https://docs.rs/mara-adapter-jsonl/0.1.0")]

use std::collections::BTreeMap;
use std::io::SeekFrom;
use std::path::{Path, PathBuf};
use std::time::Duration;

use async_trait::async_trait;
use mara_core::error::{Error, Result};
use mara_core::traits::{Adapter, EventSender};
use mara_schema::{AttrValue, Event, EventKind};
use tokio::fs::{self, OpenOptions};
use tokio::io::{AsyncBufReadExt, AsyncSeekExt, BufReader};
use tokio::sync::Notify;
use tracing::{debug, info, warn};

/// Configuration for the JSONL tail adapter.
#[derive(Clone, Debug)]
pub struct JsonlAdapterConfig {
    /// Logical adapter name (must be unique within the pipeline).
    pub name: String,
    /// File paths to tail.  v1 supports literal paths only; glob
    /// expansion is the caller's responsibility (mara-cli does it).
    pub paths: Vec<PathBuf>,
    /// Directory under which per-file offset checkpoints live.
    pub checkpoint_dir: PathBuf,
    /// Sleep interval between read attempts when EOF is reached.
    pub poll_interval: Duration,
    /// When `true` (Unix + `notify` feature), wake sooner on filesystem events.
    pub notify_hot_tail: bool,
}

impl JsonlAdapterConfig {
    /// Construct a config with sensible defaults.
    pub fn new(name: impl Into<String>, paths: Vec<PathBuf>, checkpoint_dir: PathBuf) -> Self {
        Self {
            name: name.into(),
            paths,
            checkpoint_dir,
            poll_interval: Duration::from_millis(200),
            notify_hot_tail: false,
        }
    }
}

/// The adapter.
pub struct JsonlAdapter {
    cfg: JsonlAdapterConfig,
    stop: Notify,
}

impl JsonlAdapter {
    /// Construct a new adapter.
    #[must_use]
    pub fn new(cfg: JsonlAdapterConfig) -> Self {
        Self { cfg, stop: Notify::new() }
    }
}

#[async_trait]
impl Adapter for JsonlAdapter {
    fn name(&self) -> &str {
        &self.cfg.name
    }

    async fn start(&self, out: EventSender) -> Result<()> {
        info!(adapter = %self.cfg.name, files = self.cfg.paths.len(), "jsonl adapter starting");
        fs::create_dir_all(&self.cfg.checkpoint_dir).await.map_err(|e| Error::Io {
            path: Some(self.cfg.checkpoint_dir.display().to_string()),
            source: e,
        })?;

        let mut tasks = Vec::new();
        for path in &self.cfg.paths {
            let path = path.clone();
            let ckpt_dir = self.cfg.checkpoint_dir.clone();
            let poll = self.cfg.poll_interval;
            let adapter_name = self.cfg.name.clone();
            let notify_hot_tail = self.cfg.notify_hot_tail;
            let out = out.clone();
            tasks.push(tokio::spawn(async move {
                tail_one(adapter_name, path, ckpt_dir, poll, notify_hot_tail, out).await
            }));
        }

        // Wait for shutdown notification.
        self.stop.notified().await;
        debug!(adapter = %self.cfg.name, "jsonl adapter shutdown requested");
        for t in tasks {
            t.abort();
        }
        Ok(())
    }

    async fn shutdown(&self) -> Result<()> {
        self.stop.notify_one();
        Ok(())
    }
}

#[cfg(all(feature = "notify", unix))]
fn spawn_fs_wake_thread(parent: PathBuf, wake: std::sync::Arc<Notify>) {
    use notify::Watcher;

    std::thread::Builder::new()
        .name("mara-jsonl-notify".into())
        .spawn(move || {
            let wake_cb = wake.clone();
            let mut watcher = match notify::RecommendedWatcher::new(
                move |res: std::result::Result<notify::Event, notify::Error>| {
                    let _ = res;
                    wake_cb.notify_one();
                },
                notify::Config::default(),
            ) {
                Ok(w) => w,
                Err(_) => return,
            };
            if watcher.watch(&parent, notify::RecursiveMode::NonRecursive).is_err() {
                return;
            }
            loop {
                std::thread::sleep(std::time::Duration::from_secs(3600));
            }
        })
        .ok();
}

async fn tail_one(
    adapter_name: String,
    path: PathBuf,
    ckpt_dir: PathBuf,
    poll: Duration,
    notify_hot_tail: bool,
    out: EventSender,
) -> Result<()> {
    let ckpt_path = checkpoint_path_for(&ckpt_dir, &path);
    let mut offset = load_checkpoint(&ckpt_path).await.unwrap_or(0);
    debug!(adapter = %adapter_name, path = ?path, start_offset = offset, "tailing file");

    #[cfg(all(feature = "notify", unix))]
    let fs_wake: Option<std::sync::Arc<Notify>> = if notify_hot_tail {
        let parent = path
            .parent()
            .map(std::path::Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from("."));
        let n = std::sync::Arc::new(Notify::new());
        spawn_fs_wake_thread(parent, n.clone());
        Some(n)
    } else {
        None
    };
    #[cfg(not(all(feature = "notify", unix)))]
    let fs_wake: Option<std::sync::Arc<Notify>> = None;

    loop {
        let file = match OpenOptions::new().read(true).open(&path).await {
            Ok(f) => f,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                tokio::time::sleep(poll).await;
                continue;
            }
            Err(e) => {
                warn!(adapter = %adapter_name, "open {path:?}: {e}");
                tokio::time::sleep(poll).await;
                continue;
            }
        };

        let mut reader = BufReader::new(file);
        if offset > 0
            && let Err(e) = reader.seek(SeekFrom::Start(offset)).await
        {
            warn!(adapter = %adapter_name, "seek failed; restarting from 0: {e}");
            offset = 0;
        }

        loop {
            let mut line = String::new();
            match reader.read_line(&mut line).await {
                Ok(0) => {
                    // EOF — wait and try again.
                    if let Err(e) = save_checkpoint(&ckpt_path, offset).await {
                        warn!(adapter = %adapter_name, "checkpoint save failed: {e}");
                    }
                    if let Some(ref w) = fs_wake {
                        tokio::select! {
                            _ = tokio::time::sleep(poll) => {}
                            _ = w.notified() => {
                                debug!(adapter = %adapter_name, path = ?path, "fs notify wake");
                            }
                        }
                    } else {
                        tokio::time::sleep(poll).await;
                    }
                    // If the file was truncated, reopen.
                    let f = reader.get_ref();
                    match f.metadata().await {
                        Ok(md) if md.len() < offset => {
                            debug!(adapter = %adapter_name, "file truncated; reopening from 0");
                            offset = 0;
                            break;
                        }
                        _ => {}
                    }
                }
                Ok(n) => {
                    offset += n as u64;
                    let trimmed = line.trim();
                    if trimmed.is_empty() {
                        continue;
                    }
                    match parse_jsonl_line(trimmed) {
                        Ok(event) => {
                            if let Err(send_err) = out.send(event).await {
                                warn!(adapter = %adapter_name, "downstream closed: {send_err}");
                                return Ok(());
                            }
                        }
                        Err(e) => {
                            warn!(adapter = %adapter_name, "parse error skipped: {e}");
                        }
                    }
                }
                Err(e) => {
                    warn!(adapter = %adapter_name, "read error: {e}");
                    tokio::time::sleep(poll).await;
                    break;
                }
            }
        }
    }
}

fn checkpoint_path_for(dir: &Path, file: &Path) -> PathBuf {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    use std::hash::{Hash, Hasher};
    file.hash(&mut hasher);
    let h = hasher.finish();
    dir.join(format!("ckpt-{h:016x}"))
}

async fn load_checkpoint(path: &Path) -> Option<u64> {
    let s = fs::read_to_string(path).await.ok()?;
    s.trim().parse::<u64>().ok()
}

async fn save_checkpoint(path: &Path, offset: u64) -> Result<()> {
    fs::write(path, offset.to_string())
        .await
        .map_err(|e| Error::Io { path: Some(path.display().to_string()), source: e })
}

/// Parse one JSONL line into a canonical [`Event`].
///
/// The mapping rules in M2 are intentionally lenient: any JSON
/// object is accepted; recognized field names are mapped into the
/// canonical schema; everything else lands in `attributes`.
/// Runtime-specific normalizers in `mara-runtime-*` crates override
/// this on a per-runtime basis.
pub fn parse_jsonl_line(line: &str) -> Result<Event> {
    let value: serde_json::Value = serde_json::from_str(line).map_err(|e| Error::Adapter {
        adapter: "jsonl".into(),
        message: format!("invalid JSON: {e}"),
    })?;

    let obj = value.as_object().ok_or_else(|| Error::Adapter {
        adapter: "jsonl".into(),
        message: "expected JSON object".into(),
    })?;

    let event_kind = match obj.get("event_kind").and_then(|v| v.as_str()) {
        Some("prompt") => EventKind::Prompt,
        Some("completion") => EventKind::Completion,
        Some("tool_call") => EventKind::ToolCall,
        Some("tool_result") => EventKind::ToolResult,
        Some("cost") => EventKind::Cost,
        Some("error") => EventKind::Error,
        Some("eval") => EventKind::Eval,
        Some("feedback") => EventKind::Feedback,
        _ => EventKind::System,
    };

    let mut event = Event::now(event_kind, "mara-adapter-jsonl");

    let mut attrs: BTreeMap<String, AttrValue> = BTreeMap::new();
    for (k, v) in obj {
        if k == "event_kind" {
            continue;
        }
        attrs.insert(k.clone(), json_to_attr(v.clone()));
    }
    event.attributes = attrs;
    Ok(event)
}

fn json_to_attr(v: serde_json::Value) -> AttrValue {
    match v {
        serde_json::Value::Null => AttrValue::Null,
        serde_json::Value::Bool(b) => AttrValue::Bool(b),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                AttrValue::Int(i)
            } else if let Some(f) = n.as_f64() {
                AttrValue::Float(f)
            } else {
                AttrValue::String(n.to_string())
            }
        }
        serde_json::Value::String(s) => AttrValue::String(s),
        serde_json::Value::Array(a) => AttrValue::Array(a.into_iter().map(json_to_attr).collect()),
        serde_json::Value::Object(o) => {
            AttrValue::Map(o.into_iter().map(|(k, v)| (k, json_to_attr(v))).collect())
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::time::Duration;

    use mara_core::traits::DEFAULT_CHANNEL_CAPACITY;
    use tokio::io::AsyncWriteExt;
    use tokio::sync::mpsc;

    use super::*;

    #[test]
    fn parse_line_minimal_object() {
        let ev = parse_jsonl_line(r#"{"event_kind":"prompt","model":"claude"}"#).unwrap();
        assert!(matches!(ev.event_kind, EventKind::Prompt));
        assert!(matches!(
            ev.attributes.get("model"),
            Some(AttrValue::String(s)) if s == "claude"
        ));
    }

    #[test]
    fn parse_line_defaults_to_system_kind() {
        let ev = parse_jsonl_line(r#"{"foo":"bar"}"#).unwrap();
        assert!(matches!(ev.event_kind, EventKind::System));
    }

    #[test]
    fn parse_line_rejects_non_object() {
        let err = parse_jsonl_line(r#"[1,2,3]"#).unwrap_err();
        assert!(err.to_string().contains("expected JSON object"));
    }

    #[tokio::test]
    async fn tails_a_file_to_completion() {
        let dir = tempfile::tempdir().unwrap();
        let log = dir.path().join("events.jsonl");
        let ckpt = dir.path().join("ckpt");

        // Pre-write two events.
        {
            let mut f = tokio::fs::File::create(&log).await.unwrap();
            f.write_all(b"{\"event_kind\":\"prompt\"}\n").await.unwrap();
            f.write_all(b"{\"event_kind\":\"completion\"}\n").await.unwrap();
            f.flush().await.unwrap();
        }

        let cfg = JsonlAdapterConfig {
            name: "test".into(),
            paths: vec![log.clone()],
            checkpoint_dir: ckpt,
            poll_interval: Duration::from_millis(20),
            notify_hot_tail: false,
        };
        let adapter = Arc::new(JsonlAdapter::new(cfg));

        let (tx, mut rx) = mpsc::channel::<Event>(DEFAULT_CHANNEL_CAPACITY);
        let adapter_clone = Arc::clone(&adapter);
        let handle = tokio::spawn(async move { adapter_clone.start(tx).await });

        let mut seen = Vec::new();
        for _ in 0..2 {
            let ev =
                tokio::time::timeout(Duration::from_secs(2), rx.recv()).await.unwrap().unwrap();
            seen.push(ev.event_kind);
        }
        assert!(seen.iter().any(|k| matches!(k, EventKind::Prompt)));
        assert!(seen.iter().any(|k| matches!(k, EventKind::Completion)));

        adapter.shutdown().await.unwrap();
        let _ = tokio::time::timeout(Duration::from_secs(2), handle).await;
    }
}
