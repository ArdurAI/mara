//! Local file rotation sink and a stdout debug sink.
//!
//! The file sink writes canonical events as JSONL to a configured
//! path, rolling to a new file once the active file passes a
//! byte threshold or a time boundary.  Useful for local debug,
//! CI artifact capture, and dev-loop introspection.
//!
//! The stdout sink writes events to standard output, optionally
//! pretty-printed.  Useful for `mara test pipeline` and `--inspect`-
//! style flows.

#![doc(html_root_url = "https://docs.rs/mara-sink-file/0.1.0")]

use std::path::PathBuf;

use async_trait::async_trait;
use mara_core::error::{Error, Result};
use mara_core::traits::{EventReceiver, Sink};
use mara_schema::Event;
use parking_lot::Mutex;
use tokio::fs::{self, File, OpenOptions};
use tokio::io::{AsyncWriteExt, BufWriter};
use tracing::{debug, info, warn};

/// Configuration knobs for the [`FileSink`].
#[derive(Clone, Debug)]
pub struct FileSinkConfig {
    /// Logical sink name (must be unique within a pipeline).
    pub name: String,
    /// Target path.  Currently treated as a single file; rotation
    /// appends a `.<seq>` suffix when [`Self::rotate_bytes`] is
    /// reached.
    pub path: PathBuf,
    /// Roll to a new file once the active file exceeds this many
    /// bytes.
    pub rotate_bytes: u64,
}

/// JSONL file rotation sink.
pub struct FileSink {
    cfg: FileSinkConfig,
    state: Mutex<RotState>,
}

#[derive(Default)]
struct RotState {
    seq: u64,
}

impl FileSink {
    /// Construct a new file sink.
    #[must_use]
    pub fn new(cfg: FileSinkConfig) -> Self {
        Self { cfg, state: Mutex::new(RotState::default()) }
    }

    fn current_path(&self) -> PathBuf {
        let seq = self.state.lock().seq;
        if seq == 0 {
            self.cfg.path.clone()
        } else {
            let mut p = self.cfg.path.clone();
            let stem = p.file_name().map(std::ffi::OsStr::to_os_string).unwrap_or_default();
            let mut new_name = stem;
            new_name.push(format!(".{seq}"));
            p.set_file_name(new_name);
            p
        }
    }

    async fn open_active(&self) -> Result<BufWriter<File>> {
        let path = self.current_path();
        if let Some(parent) = path.parent()
            && !parent.as_os_str().is_empty()
        {
            fs::create_dir_all(parent)
                .await
                .map_err(|e| Error::Io { path: Some(parent.display().to_string()), source: e })?;
        }
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .await
            .map_err(|e| Error::Io { path: Some(path.display().to_string()), source: e })?;
        Ok(BufWriter::new(file))
    }

    async fn write_event(
        &self,
        writer: &mut BufWriter<File>,
        bytes_written: &mut u64,
        event: &Event,
    ) -> Result<()> {
        let line = serde_json::to_vec(event).map_err(|e| Error::Sink {
            sink: self.cfg.name.clone(),
            message: format!("encode: {e}"),
        })?;
        writer.write_all(&line).await.map_err(|e| Error::Io { path: None, source: e })?;
        writer.write_all(b"\n").await.map_err(|e| Error::Io { path: None, source: e })?;
        *bytes_written += line.len() as u64 + 1;
        Ok(())
    }
}

#[async_trait]
impl Sink for FileSink {
    fn name(&self) -> &str {
        &self.cfg.name
    }

    async fn start(&self, mut input: EventReceiver) -> Result<()> {
        info!(sink = %self.cfg.name, path = ?self.cfg.path, "file sink starting");
        let mut writer = self.open_active().await?;
        let mut bytes_written: u64 = 0;

        while let Some(event) = input.recv().await {
            if let Err(e) = self.write_event(&mut writer, &mut bytes_written, &event).await {
                warn!(sink = %self.cfg.name, "write failed: {e}");
                continue;
            }
            if bytes_written >= self.cfg.rotate_bytes {
                debug!(sink = %self.cfg.name, bytes = bytes_written, "rotating file");
                writer.flush().await.map_err(|e| Error::Io { path: None, source: e })?;
                {
                    let mut st = self.state.lock();
                    st.seq += 1;
                }
                writer = self.open_active().await?;
                bytes_written = 0;
            }
        }

        writer.flush().await.map_err(|e| Error::Io { path: None, source: e })?;
        debug!(sink = %self.cfg.name, "file sink input closed; exiting");
        Ok(())
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }
}

/// Stdout sink: writes events as JSONL (or pretty JSON) to stdout.
pub struct StdoutSink {
    name: String,
    pretty: bool,
}

impl StdoutSink {
    /// Construct a stdout sink.
    #[must_use]
    pub fn new(name: impl Into<String>, pretty: bool) -> Self {
        Self { name: name.into(), pretty }
    }
}

#[async_trait]
impl Sink for StdoutSink {
    fn name(&self) -> &str {
        &self.name
    }

    async fn start(&self, mut input: EventReceiver) -> Result<()> {
        info!(sink = %self.name, "stdout sink starting");
        while let Some(event) = input.recv().await {
            let line = if self.pretty {
                serde_json::to_string_pretty(&event)
            } else {
                serde_json::to_string(&event)
            };
            match line {
                Ok(s) => println!("{s}"),
                Err(e) => warn!(sink = %self.name, "encode failed: {e}"),
            }
        }
        debug!(sink = %self.name, "stdout sink input closed; exiting");
        Ok(())
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use mara_core::traits::DEFAULT_CHANNEL_CAPACITY;
    use mara_schema::EventKind;
    use tokio::sync::mpsc;

    use super::*;

    #[tokio::test]
    async fn file_sink_writes_jsonl_and_rotates() {
        let dir = tempfile::tempdir().expect("tmp dir");
        let path = dir.path().join("out.jsonl");
        let sink = Arc::new(FileSink::new(FileSinkConfig {
            name: "test-file".into(),
            path: path.clone(),
            rotate_bytes: 200,
        }));

        let (tx, rx) = mpsc::channel::<Event>(DEFAULT_CHANNEL_CAPACITY);
        let sink_clone = Arc::clone(&sink);
        let task = tokio::spawn(async move { sink_clone.start(rx).await });

        // Send enough events to trigger rotation.
        for _ in 0..20 {
            tx.send(Event::now(EventKind::System, "test")).await.unwrap();
        }
        drop(tx);
        task.await.unwrap().unwrap();

        // Original file exists.
        assert!(path.exists());
        let main = std::fs::read_to_string(&path).unwrap();
        assert!(!main.is_empty(), "main file should have content");
        // Rotation produced at least the .1 segment.
        let rotated = path.with_file_name("out.jsonl.1");
        assert!(rotated.exists(), "expected rotated file at {rotated:?}");
    }
}
