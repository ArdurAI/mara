//! OTLP HTTP/protobuf exporter sink.
//!
//! Consumes canonical [`Event`](mara_core::Event) values from the
//! pipeline and POSTs `ExportLogsServiceRequest` bodies to the
//! configured OTLP/HTTP endpoint (`/v1/logs`), using
//! `application/x-protobuf` encoding.  Optional gzip compression
//! matches the receiver path in `mara-adapter-otlp`.
//!
//! gRPC (`:4317`) and HTTP/JSON export are deferred to MVP+1.

#![doc(html_root_url = "https://docs.rs/mara-sink-otlp/0.1.0")]

mod config;
mod encode;

pub use config::OtlpHttpSinkConfig;

use std::io::Write;
use std::time::Duration;

use async_trait::async_trait;
use encode::events_to_export_request;
use flate2::Compression;
use flate2::write::GzEncoder;
use mara_core::error::{Error, Result};
use mara_core::traits::{EventReceiver, Sink};
use prost::Message;
use reqwest::Url;
use tracing::{debug, info, warn};

/// OTLP/HTTP protobuf log exporter.
pub struct OtlpHttpSink {
    cfg: OtlpHttpSinkConfig,
    http: reqwest::Client,
    logs_url: Url,
}

impl OtlpHttpSink {
    /// Build a sink from configuration.  Fails if `http_endpoint` is
    /// not a valid base URL.
    pub fn new(cfg: OtlpHttpSinkConfig) -> Result<Self> {
        let base = cfg.http_endpoint.trim_end_matches('/');
        let logs_url = Url::parse(base)
            .map_err(|e| Error::Sink {
                sink: cfg.name.clone(),
                message: format!("invalid http_endpoint: {e}"),
            })?
            .join("v1/logs")
            .map_err(|e| Error::Sink {
                sink: cfg.name.clone(),
                message: format!("could not build /v1/logs URL: {e}"),
            })?;

        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(cfg.timeout_secs.max(1)))
            .build()
            .map_err(|e| Error::Sink {
                sink: cfg.name.clone(),
                message: format!("http client: {e}"),
            })?;

        Ok(Self { cfg, http, logs_url })
    }

    fn batch_cap(&self) -> usize {
        self.cfg.batch_max_events.max(1)
    }

    async fn flush_batch(&self, batch: &mut Vec<mara_core::Event>) -> Result<()> {
        if batch.is_empty() {
            return Ok(());
        }
        let req = events_to_export_request(batch);
        let mut body = Vec::new();
        req.encode(&mut body).map_err(|e| Error::Sink {
            sink: self.cfg.name.clone(),
            message: format!("protobuf encode: {e}"),
        })?;

        let (body, content_encoding) = if self.cfg.gzip {
            let mut enc = GzEncoder::new(Vec::new(), Compression::default());
            enc.write_all(&body).map_err(|e| Error::Sink {
                sink: self.cfg.name.clone(),
                message: format!("gzip: {e}"),
            })?;
            let gz = enc.finish().map_err(|e| Error::Sink {
                sink: self.cfg.name.clone(),
                message: format!("gzip finish: {e}"),
            })?;
            (gz, Some("gzip"))
        } else {
            (body, None)
        };

        let mut rb = self
            .http
            .post(self.logs_url.clone())
            .header(reqwest::header::CONTENT_TYPE, "application/x-protobuf");
        if let Some(ce) = content_encoding {
            rb = rb.header(reqwest::header::CONTENT_ENCODING, ce);
        }

        let resp = rb.body(body).send().await.map_err(|e| Error::Sink {
            sink: self.cfg.name.clone(),
            message: format!("http: {e}"),
        })?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body_hint = resp.text().await.unwrap_or_default();
            return Err(Error::Sink {
                sink: self.cfg.name.clone(),
                message: format!("upstream returned {status}: {body_hint}"),
            });
        }

        debug!(sink = %self.cfg.name, n = batch.len(), "otlp logs export ok");
        batch.clear();
        Ok(())
    }
}

#[async_trait]
impl Sink for OtlpHttpSink {
    fn name(&self) -> &str {
        &self.cfg.name
    }

    async fn start(&self, mut input: EventReceiver) -> Result<()> {
        info!(sink = %self.cfg.name, url = %self.logs_url, "otlp http sink starting");
        let cap = self.batch_cap();
        let mut batch: Vec<mara_core::Event> = Vec::with_capacity(cap);

        while let Some(ev) = input.recv().await {
            batch.push(ev);
            if batch.len() >= cap
                && let Err(e) = self.flush_batch(&mut batch).await
            {
                warn!(sink = %self.cfg.name, error = %e, "export failed");
            }
        }

        self.flush_batch(&mut batch).await?;
        debug!(sink = %self.cfg.name, "otlp http sink input closed");
        Ok(())
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::convert::Infallible;
    use std::net::SocketAddr;

    use bytes::Bytes;
    use http_body_util::{BodyExt, Full};
    use hyper::body::Incoming;
    use hyper::service::service_fn;
    use hyper::{Method, Request, Response, StatusCode};
    use hyper_util::rt::TokioIo;
    use mara_adapter_otlp::normalize::log_record_to_event;
    use mara_schema::{EventKind, GenAi, Resource, Severity, SourceRuntime};
    use opentelemetry_proto::tonic::collector::logs::v1::ExportLogsServiceRequest;
    use prost::Message;
    use tokio::net::TcpListener;

    use super::*;

    #[tokio::test]
    async fn round_trip_gen_ai_through_encode_and_normalize() {
        let ev = mara_core::Event {
            resource: Resource {
                service_name: Some("claude-code".into()),
                source_runtime: Some(SourceRuntime::ClaudeCode),
                ..Default::default()
            },
            scope: mara_schema::Scope { name: "scope".into(), version: Some("0.1".into()) },
            timestamp_ns: 1_700_000_000_000_000_000,
            observed_timestamp_ns: 1_700_000_000_000_000_000,
            trace_id: None,
            span_id: None,
            parent_span_id: None,
            event_kind: EventKind::Completion,
            severity: Severity::INFO,
            gen_ai: GenAi {
                system: Some("anthropic".into()),
                operation_name: Some("chat".into()),
                usage: mara_schema::GenAiUsage {
                    input_tokens: Some(10),
                    output_tokens: Some(20),
                    ..Default::default()
                },
                ..Default::default()
            },
            mcp: None,
            mara: mara_schema::MaraExtensions::default(),
            attributes: Default::default(),
            body: None,
        };

        let req = events_to_export_request(std::slice::from_ref(&ev));
        let mut buf = Vec::new();
        req.encode(&mut buf).expect("encode");

        let decoded = ExportLogsServiceRequest::decode(buf.as_slice()).expect("decode");
        let rl = decoded.resource_logs.first().expect("rl");
        let resource = rl.resource.as_ref();
        let sl = rl.scope_logs.first().expect("sl");
        let scope = sl.scope.as_ref();
        let lr = sl.log_records.first().expect("lr");
        let back = log_record_to_event(resource, scope, lr);

        assert_eq!(back.gen_ai.system.as_deref(), Some("anthropic"));
        assert_eq!(back.gen_ai.operation_name.as_deref(), Some("chat"));
        assert_eq!(back.gen_ai.usage.input_tokens, Some(10));
        assert_eq!(back.gen_ai.usage.output_tokens, Some(20));
        assert_eq!(back.resource.service_name.as_deref(), Some("claude-code"));
        assert_eq!(back.resource.source_runtime, Some(SourceRuntime::ClaudeCode));
        assert!(matches!(back.event_kind, EventKind::Completion));
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn http_post_reaches_receiver() {
        use std::sync::{Arc, Mutex};

        let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
        let addr: SocketAddr = listener.local_addr().expect("addr");
        let captured: Arc<Mutex<Option<Vec<u8>>>> = Arc::new(Mutex::new(None));
        let cap = Arc::clone(&captured);

        let server = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.expect("accept");
            let io = TokioIo::new(stream);
            let service = service_fn(move |req: Request<Incoming>| {
                let cap = Arc::clone(&cap);
                async move {
                    let resp = if req.method() == Method::POST && req.uri().path() == "/v1/logs" {
                        let body = req.into_body().collect().await.expect("read").to_bytes();
                        *cap.lock().expect("lock") = Some(body.to_vec());
                        Response::builder()
                            .status(StatusCode::OK)
                            .body(Full::new(Bytes::new()))
                            .expect("resp")
                    } else {
                        Response::builder()
                            .status(StatusCode::NOT_FOUND)
                            .body(Full::new(Bytes::from("not found")))
                            .expect("resp")
                    };
                    Ok::<_, Infallible>(resp)
                }
            });
            let _ = hyper::server::conn::http1::Builder::new().serve_connection(io, service).await;
        });

        let sink = OtlpHttpSink::new(OtlpHttpSinkConfig {
            name: "t".into(),
            http_endpoint: format!("http://{addr}"),
            batch_max_events: 8,
            timeout_secs: 5,
            gzip: false,
        })
        .expect("sink");

        let (in_tx, in_rx) = tokio::sync::mpsc::channel(4);
        let sink_clone = std::sync::Arc::new(sink);
        let h = tokio::spawn({
            let s = std::sync::Arc::clone(&sink_clone);
            async move { s.start(in_rx).await }
        });

        in_tx.send(mara_core::Event::now(EventKind::System, "probe")).await.unwrap();
        drop(in_tx);

        for _ in 0..100 {
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            if captured.lock().expect("lock").is_some() {
                break;
            }
        }

        let received =
            captured.lock().expect("lock").take().expect("server should have received POST body");

        let decoded = ExportLogsServiceRequest::decode(received.as_slice()).expect("proto");
        assert_eq!(decoded.resource_logs.len(), 1);

        let _ = h.await;
        server.abort();
    }
}
