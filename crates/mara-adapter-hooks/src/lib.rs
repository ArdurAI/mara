//! HTTP hooks adapter (Tier B).
//!
//! Listens for `POST` requests with `Content-Type: application/json`
//! and ingests either a single canonical [`Event`] or
//! `{"events":[...]}`.

#![doc(html_root_url = "https://docs.rs/mara-adapter-hooks/0.1.0")]

use std::convert::Infallible;
use std::net::SocketAddr;
use std::sync::Arc;

use async_trait::async_trait;
use bytes::Bytes;
use http_body_util::{BodyExt, Full};
use hyper::body::Incoming;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{Method, Request, Response, StatusCode};
use hyper_util::rt::TokioIo;
use mara_core::error::{Error, Result};
use mara_core::health::Health;
use mara_core::traits::{Adapter, EventSender};
use mara_schema::Event;
use serde_json::Value;
use tokio::net::TcpListener;
use tokio::sync::Notify;
use tracing::{debug, info, warn};

/// Runtime configuration for [`HooksHttpAdapter`].
#[derive(Clone, Debug)]
pub struct HooksHttpAdapterConfig {
    /// Logical adapter name.
    pub name: String,
    /// Bind address for the hooks HTTP server.
    pub http_listen: SocketAddr,
    /// Maximum JSON body size per request.
    pub max_body_bytes: usize,
}

impl HooksHttpAdapterConfig {
    /// Construct with an 8 MiB default body cap.
    #[must_use]
    pub fn new(name: impl Into<String>, http_listen: SocketAddr) -> Self {
        Self { name: name.into(), http_listen, max_body_bytes: 8 * 1024 * 1024 }
    }
}

/// HTTP `POST` ingest for Cursor-style hook payloads.
pub struct HooksHttpAdapter {
    cfg: HooksHttpAdapterConfig,
    stop: Arc<Notify>,
}

impl HooksHttpAdapter {
    /// Create a hooks adapter from configuration.
    #[must_use]
    pub fn new(cfg: HooksHttpAdapterConfig) -> Self {
        Self { cfg, stop: Arc::new(Notify::new()) }
    }
}

#[async_trait]
impl Adapter for HooksHttpAdapter {
    fn name(&self) -> &str {
        &self.cfg.name
    }

    async fn start(&self, out: EventSender) -> Result<()> {
        let listener = TcpListener::bind(self.cfg.http_listen).await.map_err(|e| {
            Error::Io { path: Some(self.cfg.http_listen.to_string()), source: e }
        })?;
        info!(adapter = %self.cfg.name, addr = %self.cfg.http_listen, "hooks http listening");
        let stop = self.stop.clone();
        let max = self.cfg.max_body_bytes;
        let adapter_name = self.cfg.name.clone();
        loop {
            tokio::select! {
                () = stop.notified() => {
                    debug!(adapter = %adapter_name, "hooks shutdown");
                    return Ok(());
                }
                acc = listener.accept() => {
                    let Ok((stream, _)) = acc else { continue; };
                    let io = TokioIo::new(stream);
                    let out = out.clone();
                    let name = adapter_name.clone();
                    let log_name = adapter_name.clone();
                    tokio::spawn(async move {
                        let service = service_fn(move |req| {
                            let out = out.clone();
                            let n = name.clone();
                            async move { handle_post(req, out, max, n).await }
                        });
                        if let Err(e) = http1::Builder::new().serve_connection(io, service).await {
                            debug!(adapter = %log_name, "hooks conn ended: {e}");
                        }
                    });
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

async fn handle_post(
    req: Request<Incoming>,
    out: EventSender,
    max_body_bytes: usize,
    adapter_name: String,
) -> Result<Response<Full<Bytes>>, Infallible> {
    if req.method() != Method::POST {
        return Ok(Response::builder()
            .status(StatusCode::METHOD_NOT_ALLOWED)
            .body(Full::default())
            .unwrap());
    }
    let body = req.into_body();
    let collected = match body.collect().await {
        Ok(c) => c.to_bytes(),
        Err(_) => {
            return Ok(Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .body(Full::from("invalid body"))
                .unwrap());
        }
    };
    if collected.len() > max_body_bytes {
        return Ok(Response::builder()
            .status(StatusCode::PAYLOAD_TOO_LARGE)
            .body(Full::from("payload too large"))
            .unwrap());
    }
    let events = match parse_events(&collected) {
        Ok(e) => e,
        Err(msg) => {
            warn!(adapter = %adapter_name, "{msg}");
            return Ok(Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .body(Full::from(msg))
                .unwrap());
        }
    };
    let mut sent = 0usize;
    for mut ev in events {
        if ev.mara.source_adapter.is_none() {
            ev.mara.source_adapter = Some(adapter_name.clone());
        }
        if out.send(ev).await.is_err() {
            break;
        }
        sent += 1;
    }
    Ok(Response::builder()
        .status(StatusCode::ACCEPTED)
        .body(Full::from(format!("accepted {sent} events")))
        .unwrap())
}

fn parse_events(bytes: &[u8]) -> std::result::Result<Vec<Event>, &'static str> {
    if let Ok(ev) = serde_json::from_slice::<Event>(bytes) {
        return Ok(vec![ev]);
    }
    let v: Value = serde_json::from_slice(bytes).map_err(|_| "invalid json")?;
    if let Some(arr) = v.get("events").and_then(Value::as_array) {
        let mut out = Vec::with_capacity(arr.len());
        for item in arr {
            let ev: Event = serde_json::from_value(item.clone()).map_err(|_| "invalid event in events[]")?;
            out.push(ev);
        }
        if out.is_empty() {
            return Err("events array is empty");
        }
        return Ok(out);
    }
    Err("expected Event JSON or {\"events\":[...]}")
}
