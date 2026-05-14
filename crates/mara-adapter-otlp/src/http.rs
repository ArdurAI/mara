//! OTLP HTTP/protobuf receiver.
//!
//! Binds the configured address and serves the OTel-spec
//! `POST /v1/logs` and `POST /v1/traces` routes.  Each request
//! body is decoded with `prost` against the `opentelemetry-proto`
//! schema, then translated into canonical Mara events by the
//! [`normalize`](crate::normalize) module.
//!
//! Supports `Content-Encoding: gzip` and identity.  Rejects
//! payloads larger than the configured `max_body_bytes` with
//! HTTP 413.  Returns 415 for unsupported media types.

use std::convert::Infallible;
use std::io::Read;
use std::sync::Arc;

use async_trait::async_trait;
use bytes::Bytes;
use flate2::read::GzDecoder;
use http_body_util::{BodyExt, Full};
use hyper::body::Incoming;
use hyper::header::{CONTENT_ENCODING, CONTENT_TYPE};
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{Method, Request, Response, StatusCode};
use hyper_util::rt::TokioIo;
use mara_core::error::{Error, Result};
use mara_core::health::Health;
use mara_core::traits::{Adapter, EventSender};
use opentelemetry_proto::tonic::collector::logs::v1::ExportLogsServiceRequest;
use opentelemetry_proto::tonic::collector::trace::v1::ExportTraceServiceRequest;
use prost::Message;
use tokio::net::TcpListener;
use tokio::sync::Notify;
use tracing::{debug, error, info, warn};

use crate::config::OtlpHttpAdapterConfig;
use crate::normalize::{log_record_to_event, span_to_event};

/// OTLP HTTP/protobuf receiver adapter.
pub struct OtlpHttpAdapter {
    cfg: OtlpHttpAdapterConfig,
    stop: Arc<Notify>,
}

impl OtlpHttpAdapter {
    /// Construct a new adapter from configuration.
    #[must_use]
    pub fn new(cfg: OtlpHttpAdapterConfig) -> Self {
        Self { cfg, stop: Arc::new(Notify::new()) }
    }
}

#[async_trait]
impl Adapter for OtlpHttpAdapter {
    fn name(&self) -> &str {
        &self.cfg.name
    }

    async fn start(&self, out: EventSender) -> Result<()> {
        if let Some(addr) = self.cfg.grpc_listen {
            let out_g = out.clone();
            let name = self.cfg.name.clone();
            let stop_g = self.stop.clone();
            info!(adapter = %name, %addr, "otlp grpc receiver listening");
            tokio::spawn(async move {
                if let Err(e) = crate::grpc::serve(addr, out_g, name, stop_g).await {
                    error!(error = %e, "otlp grpc server exited with error");
                }
            });
        }

        let listener = TcpListener::bind(self.cfg.http_listen)
            .await
            .map_err(|e| Error::Io { path: Some(self.cfg.http_listen.to_string()), source: e })?;
        info!(
            adapter = %self.cfg.name,
            addr = %self.cfg.http_listen,
            "otlp http receiver listening"
        );

        let stop = self.stop.clone();
        let max_body = self.cfg.max_body_bytes;
        let adapter_name = self.cfg.name.clone();

        loop {
            tokio::select! {
                () = stop.notified() => {
                    debug!(adapter = %adapter_name, "shutdown signal received");
                    return Ok(());
                }
                accept = listener.accept() => {
                    match accept {
                        Ok((stream, _peer)) => {
                            let io = TokioIo::new(stream);
                            let out = out.clone();
                            let name = adapter_name.clone();
                            let log_name = adapter_name.clone();
                            tokio::spawn(async move {
                                let service = service_fn(move |req| {
                                    handle_request(req, out.clone(), max_body, name.clone())
                                });
                                if let Err(e) =
                                    http1::Builder::new().serve_connection(io, service).await
                                {
                                    debug!(adapter = %log_name, "conn ended: {e}");
                                }
                            });
                        }
                        Err(e) => {
                            warn!(adapter = %adapter_name, "accept error: {e}");
                        }
                    }
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

async fn handle_request(
    req: Request<Incoming>,
    out: EventSender,
    max_body_bytes: usize,
    adapter_name: String,
) -> std::result::Result<Response<Full<Bytes>>, Infallible> {
    let result = route(req, out, max_body_bytes, &adapter_name).await;
    Ok(result.unwrap_or_else(|status| {
        let body = match status {
            StatusCode::PAYLOAD_TOO_LARGE => "payload too large".to_owned(),
            StatusCode::UNSUPPORTED_MEDIA_TYPE => "unsupported media type".to_owned(),
            StatusCode::BAD_REQUEST => "bad request".to_owned(),
            StatusCode::NOT_FOUND => "not found".to_owned(),
            StatusCode::METHOD_NOT_ALLOWED => "method not allowed".to_owned(),
            StatusCode::INTERNAL_SERVER_ERROR => "internal server error".to_owned(),
            _ => "error".to_owned(),
        };
        Response::builder()
            .status(status)
            .body(Full::new(Bytes::from(body)))
            .expect("static response builds")
    }))
}

async fn route(
    req: Request<Incoming>,
    out: EventSender,
    max_body_bytes: usize,
    adapter_name: &str,
) -> std::result::Result<Response<Full<Bytes>>, StatusCode> {
    if req.method() != Method::POST {
        return Err(StatusCode::METHOD_NOT_ALLOWED);
    }
    let path = req.uri().path().to_owned();

    let content_type =
        req.headers().get(CONTENT_TYPE).and_then(|h| h.to_str().ok()).unwrap_or("").to_owned();
    if !content_type.starts_with("application/x-protobuf") {
        return Err(StatusCode::UNSUPPORTED_MEDIA_TYPE);
    }

    let content_encoding =
        req.headers().get(CONTENT_ENCODING).and_then(|h| h.to_str().ok()).unwrap_or("").to_owned();

    let body_bytes = read_body(req.into_body(), max_body_bytes).await?;

    let payload = match content_encoding.as_str() {
        "" | "identity" => body_bytes,
        "gzip" => decode_gzip(&body_bytes).map_err(|_| StatusCode::BAD_REQUEST)?,
        _ => return Err(StatusCode::UNSUPPORTED_MEDIA_TYPE),
    };

    let accepted = match path.as_str() {
        "/v1/logs" => {
            handle_logs(&payload, &out, adapter_name).await.map_err(|_| StatusCode::BAD_REQUEST)?
        }
        "/v1/traces" => handle_traces(&payload, &out, adapter_name)
            .await
            .map_err(|_| StatusCode::BAD_REQUEST)?,
        _ => return Err(StatusCode::NOT_FOUND),
    };

    debug!(adapter = %adapter_name, path = %path, accepted, "otlp request processed");

    // OTel spec: success response is an empty *ExportServiceResponse* protobuf;
    // an empty body is acceptable for a 200 in practice for most receivers.
    Ok(Response::builder()
        .status(StatusCode::OK)
        .header(CONTENT_TYPE, "application/x-protobuf")
        .body(Full::new(Bytes::new()))
        .expect("response builds"))
}

async fn read_body(
    body: Incoming,
    max_body_bytes: usize,
) -> std::result::Result<Vec<u8>, StatusCode> {
    let collected = body.collect().await.map_err(|_| StatusCode::BAD_REQUEST)?;
    let bytes = collected.to_bytes();
    if bytes.len() > max_body_bytes {
        return Err(StatusCode::PAYLOAD_TOO_LARGE);
    }
    Ok(bytes.to_vec())
}

fn decode_gzip(data: &[u8]) -> std::io::Result<Vec<u8>> {
    let mut decoder = GzDecoder::new(data);
    let mut out = Vec::new();
    decoder.read_to_end(&mut out)?;
    Ok(out)
}

async fn handle_logs(
    payload: &[u8],
    out: &EventSender,
    adapter_name: &str,
) -> std::result::Result<usize, prost::DecodeError> {
    let request = ExportLogsServiceRequest::decode(payload)?;
    Ok(dispatch_export_logs(request, out, adapter_name).await)
}

/// Shared by HTTP and gRPC OTLP ingest.
pub(crate) async fn dispatch_export_logs(
    request: ExportLogsServiceRequest,
    out: &EventSender,
    adapter_name: &str,
) -> usize {
    let mut count = 0;
    for resource_logs in &request.resource_logs {
        let resource = resource_logs.resource.as_ref();
        for scope_logs in &resource_logs.scope_logs {
            let scope = scope_logs.scope.as_ref();
            for log_record in &scope_logs.log_records {
                let mut event = log_record_to_event(resource, scope, log_record);
                event.mara.source_adapter = Some(adapter_name.to_owned());
                if let Err(send_err) = out.send(event).await {
                    error!(adapter = %adapter_name, "downstream closed: {send_err}");
                    return count;
                }
                count += 1;
            }
        }
    }
    count
}

async fn handle_traces(
    payload: &[u8],
    out: &EventSender,
    adapter_name: &str,
) -> std::result::Result<usize, prost::DecodeError> {
    let request = ExportTraceServiceRequest::decode(payload)?;
    Ok(dispatch_export_traces(request, out, adapter_name).await)
}

pub(crate) async fn dispatch_export_traces(
    request: ExportTraceServiceRequest,
    out: &EventSender,
    adapter_name: &str,
) -> usize {
    let mut count = 0;
    for resource_spans in &request.resource_spans {
        let resource = resource_spans.resource.as_ref();
        for scope_spans in &resource_spans.scope_spans {
            let scope = scope_spans.scope.as_ref();
            for span in &scope_spans.spans {
                let mut event = span_to_event(resource, scope, span);
                event.mara.source_adapter = Some(adapter_name.to_owned());
                if let Err(send_err) = out.send(event).await {
                    error!(adapter = %adapter_name, "downstream closed: {send_err}");
                    return count;
                }
                count += 1;
            }
        }
    }
    count
}

#[cfg(test)]
mod tests {
    use std::net::SocketAddr;
    use std::time::Duration;

    use mara_core::Event;
    use mara_schema::EventKind;
    use opentelemetry_proto::tonic::collector::logs::v1::ExportLogsServiceRequest;
    use opentelemetry_proto::tonic::common::v1::any_value::Value as AnyValueInner;
    use opentelemetry_proto::tonic::common::v1::{AnyValue, InstrumentationScope, KeyValue};
    use opentelemetry_proto::tonic::logs::v1::{LogRecord, ResourceLogs, ScopeLogs};
    use opentelemetry_proto::tonic::resource::v1::Resource as ProtoResource;
    use prost::Message;
    use tokio::sync::mpsc;

    use super::*;

    fn str_kv(key: &str, value: &str) -> KeyValue {
        KeyValue {
            key: key.to_owned(),
            value: Some(AnyValue { value: Some(AnyValueInner::StringValue(value.to_owned())) }),
        }
    }

    fn int_kv(key: &str, value: i64) -> KeyValue {
        KeyValue {
            key: key.to_owned(),
            value: Some(AnyValue { value: Some(AnyValueInner::IntValue(value)) }),
        }
    }

    fn build_claude_code_payload() -> Vec<u8> {
        let request = ExportLogsServiceRequest {
            resource_logs: vec![ResourceLogs {
                resource: Some(ProtoResource {
                    attributes: vec![
                        str_kv("service.name", "claude-code"),
                        str_kv("mara.source.runtime", "claude_code"),
                    ],
                    dropped_attributes_count: 0,
                }),
                scope_logs: vec![ScopeLogs {
                    scope: Some(InstrumentationScope {
                        name: "claude-code".to_owned(),
                        version: "0.43.1".to_owned(),
                        attributes: vec![],
                        dropped_attributes_count: 0,
                    }),
                    log_records: vec![LogRecord {
                        time_unix_nano: 1_700_000_000_000_000_000,
                        observed_time_unix_nano: 1_700_000_000_000_000_000,
                        severity_number: 9,
                        severity_text: "INFO".to_owned(),
                        body: None,
                        attributes: vec![
                            str_kv("gen_ai.system", "anthropic"),
                            str_kv("gen_ai.operation.name", "chat"),
                            str_kv("gen_ai.request.model", "claude-sonnet-4-5"),
                            int_kv("gen_ai.usage.input_tokens", 1024),
                            int_kv("gen_ai.usage.output_tokens", 768),
                        ],
                        dropped_attributes_count: 0,
                        flags: 0,
                        trace_id: vec![1u8; 16],
                        span_id: vec![2u8; 8],
                    }],
                    schema_url: String::new(),
                }],
                schema_url: String::new(),
            }],
        };
        let mut buf = Vec::new();
        request.encode(&mut buf).expect("encodes");
        buf
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn end_to_end_post_logs_emits_canonical_event() {
        let cfg = OtlpHttpAdapterConfig::new(
            "test_otlp",
            "127.0.0.1:0".parse::<SocketAddr>().expect("addr"),
        );
        let listener = TcpListener::bind(cfg.http_listen).await.expect("bind");
        let actual_addr = listener.local_addr().expect("addr");
        drop(listener);

        let cfg = OtlpHttpAdapterConfig { http_listen: actual_addr, ..cfg };
        let adapter = Arc::new(OtlpHttpAdapter::new(cfg));
        let (tx, mut rx) = mpsc::channel::<Event>(8);
        let adapter_clone = Arc::clone(&adapter);
        let handle = tokio::spawn(async move { adapter_clone.start(tx).await });

        // Wait a beat for the listener to come back up on the same port.
        tokio::time::sleep(Duration::from_millis(50)).await;

        let payload = build_claude_code_payload();
        let client = reqwest::Client::new();
        let response = client
            .post(format!("http://{actual_addr}/v1/logs"))
            .header("content-type", "application/x-protobuf")
            .body(payload)
            .send()
            .await
            .expect("request sent");
        assert_eq!(response.status().as_u16(), 200);

        let event =
            tokio::time::timeout(Duration::from_secs(2), rx.recv()).await.expect("event arrived");
        let event = event.expect("event");
        assert_eq!(event.gen_ai.system.as_deref(), Some("anthropic"));
        assert_eq!(event.gen_ai.request.model.as_deref(), Some("claude-sonnet-4-5"));
        assert_eq!(event.gen_ai.usage.input_tokens, Some(1024));
        assert_eq!(event.gen_ai.usage.output_tokens, Some(768));
        assert!(matches!(event.event_kind, EventKind::Completion));
        assert_eq!(event.mara.source_adapter.as_deref(), Some("test_otlp"));

        adapter.shutdown().await.expect("shutdown ok");
        let _ = tokio::time::timeout(Duration::from_secs(2), handle).await;
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn rejects_wrong_content_type() {
        let cfg = OtlpHttpAdapterConfig::new(
            "test_otlp",
            "127.0.0.1:0".parse::<SocketAddr>().expect("addr"),
        );
        let listener = TcpListener::bind(cfg.http_listen).await.expect("bind");
        let actual_addr = listener.local_addr().expect("addr");
        drop(listener);

        let cfg = OtlpHttpAdapterConfig { http_listen: actual_addr, ..cfg };
        let adapter = Arc::new(OtlpHttpAdapter::new(cfg));
        let (tx, _rx) = mpsc::channel::<Event>(8);
        let adapter_clone = Arc::clone(&adapter);
        let handle = tokio::spawn(async move { adapter_clone.start(tx).await });
        tokio::time::sleep(Duration::from_millis(50)).await;

        let client = reqwest::Client::new();
        let response = client
            .post(format!("http://{actual_addr}/v1/logs"))
            .header("content-type", "application/json")
            .body("{}")
            .send()
            .await
            .expect("sent");
        assert_eq!(response.status().as_u16(), 415);

        adapter.shutdown().await.expect("shutdown");
        let _ = tokio::time::timeout(Duration::from_secs(2), handle).await;
    }

    async fn bind_ephemeral_otlp_addr() -> SocketAddr {
        let cfg = OtlpHttpAdapterConfig::new(
            "test_otlp",
            "127.0.0.1:0".parse::<SocketAddr>().expect("addr"),
        );
        let listener = TcpListener::bind(cfg.http_listen).await.expect("bind");
        listener.local_addr().expect("addr")
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn get_method_returns_405() {
        let actual_addr = bind_ephemeral_otlp_addr().await;
        let cfg = OtlpHttpAdapterConfig { http_listen: actual_addr, ..OtlpHttpAdapterConfig::new(
            "test_otlp",
            actual_addr,
        ) };
        let adapter = Arc::new(OtlpHttpAdapter::new(cfg));
        let (tx, _rx) = mpsc::channel::<Event>(8);
        let handle = tokio::spawn({
            let adapter = Arc::clone(&adapter);
            async move { adapter.start(tx).await }
        });
        tokio::time::sleep(Duration::from_millis(50)).await;

        let client = reqwest::Client::new();
        let response = client
            .get(format!("http://{actual_addr}/v1/logs"))
            .send()
            .await
            .expect("sent");
        assert_eq!(response.status().as_u16(), 405);

        adapter.shutdown().await.expect("shutdown");
        let _ = tokio::time::timeout(Duration::from_secs(2), handle).await;
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn unknown_path_returns_404() {
        let actual_addr = bind_ephemeral_otlp_addr().await;
        let cfg = OtlpHttpAdapterConfig { http_listen: actual_addr, ..OtlpHttpAdapterConfig::new(
            "test_otlp",
            actual_addr,
        ) };
        let adapter = Arc::new(OtlpHttpAdapter::new(cfg));
        let (tx, _rx) = mpsc::channel::<Event>(8);
        let handle = tokio::spawn({
            let adapter = Arc::clone(&adapter);
            async move { adapter.start(tx).await }
        });
        tokio::time::sleep(Duration::from_millis(50)).await;

        let client = reqwest::Client::new();
        let response = client
            .post(format!("http://{actual_addr}/v1/metrics"))
            .header("content-type", "application/x-protobuf")
            .body(vec![0u8; 8])
            .send()
            .await
            .expect("sent");
        assert_eq!(response.status().as_u16(), 404);

        adapter.shutdown().await.expect("shutdown");
        let _ = tokio::time::timeout(Duration::from_secs(2), handle).await;
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn oversized_body_returns_413() {
        let actual_addr = bind_ephemeral_otlp_addr().await;
        let cfg = OtlpHttpAdapterConfig {
            http_listen: actual_addr,
            max_body_bytes: 32,
            ..OtlpHttpAdapterConfig::new("test_otlp", actual_addr)
        };
        let adapter = Arc::new(OtlpHttpAdapter::new(cfg));
        let (tx, _rx) = mpsc::channel::<Event>(8);
        let handle = tokio::spawn({
            let adapter = Arc::clone(&adapter);
            async move { adapter.start(tx).await }
        });
        tokio::time::sleep(Duration::from_millis(50)).await;

        let client = reqwest::Client::new();
        let response = client
            .post(format!("http://{actual_addr}/v1/logs"))
            .header("content-type", "application/x-protobuf")
            .body(vec![0u8; 64])
            .send()
            .await
            .expect("sent");
        assert_eq!(response.status().as_u16(), 413);

        adapter.shutdown().await.expect("shutdown");
        let _ = tokio::time::timeout(Duration::from_secs(2), handle).await;
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn unsupported_content_encoding_returns_415() {
        let actual_addr = bind_ephemeral_otlp_addr().await;
        let cfg = OtlpHttpAdapterConfig { http_listen: actual_addr, ..OtlpHttpAdapterConfig::new(
            "test_otlp",
            actual_addr,
        ) };
        let adapter = Arc::new(OtlpHttpAdapter::new(cfg));
        let (tx, _rx) = mpsc::channel::<Event>(8);
        let handle = tokio::spawn({
            let adapter = Arc::clone(&adapter);
            async move { adapter.start(tx).await }
        });
        tokio::time::sleep(Duration::from_millis(50)).await;

        let client = reqwest::Client::new();
        let response = client
            .post(format!("http://{actual_addr}/v1/logs"))
            .header("content-type", "application/x-protobuf")
            .header("content-encoding", "br")
            .body(vec![1, 2, 3])
            .send()
            .await
            .expect("sent");
        assert_eq!(response.status().as_u16(), 415);

        adapter.shutdown().await.expect("shutdown");
        let _ = tokio::time::timeout(Duration::from_secs(2), handle).await;
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn gzip_with_invalid_payload_returns_400() {
        let actual_addr = bind_ephemeral_otlp_addr().await;
        let cfg = OtlpHttpAdapterConfig { http_listen: actual_addr, ..OtlpHttpAdapterConfig::new(
            "test_otlp",
            actual_addr,
        ) };
        let adapter = Arc::new(OtlpHttpAdapter::new(cfg));
        let (tx, _rx) = mpsc::channel::<Event>(8);
        let handle = tokio::spawn({
            let adapter = Arc::clone(&adapter);
            async move { adapter.start(tx).await }
        });
        tokio::time::sleep(Duration::from_millis(50)).await;

        let client = reqwest::Client::new();
        let response = client
            .post(format!("http://{actual_addr}/v1/logs"))
            .header("content-type", "application/x-protobuf")
            .header("content-encoding", "gzip")
            .body(b"not a gzip stream".to_vec())
            .send()
            .await
            .expect("sent");
        assert_eq!(response.status().as_u16(), 400);

        adapter.shutdown().await.expect("shutdown");
        let _ = tokio::time::timeout(Duration::from_secs(2), handle).await;
    }
}
