//! HTTP/1.1 reverse proxy: accept, forward, capture, normalize.

use std::convert::Infallible;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};

use async_trait::async_trait;
use bytes::Buf;
use bytes::Bytes;
use http::header::{self, HeaderMap, HeaderName, HeaderValue};
use http::uri::Uri;
use http::{Request, Response, StatusCode, Version};
use http_body_util::BodyExt;
use http_body_util::Full;
use http_body_util::channel::Channel;
use http_body_util::combinators::UnsyncBoxBody;
use hyper::body::Incoming;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper_util::client::legacy::Client;
use hyper_util::client::legacy::connect::HttpConnector;
use hyper_util::rt::{TokioExecutor, TokioIo};
use mara_core::error::{Error, Result};
use mara_core::health::Health;
use mara_core::traits::{Adapter, EventSender};
use mara_schema::AttrValue;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpListener;
use tokio::net::TcpStream;
use tokio::sync::{Notify, Semaphore};
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use crate::ProxyFailureKind;
use crate::exchange::{ProxiedRequest, ProxiedResponse};
use crate::normalizer::UpstreamNormalizer;

/// Response body type returned to the inbound HTTP/1 client.
pub type ProxyBody = UnsyncBoxBody<Bytes, Infallible>;

fn box_full(full: Full<Bytes>) -> ProxyBody {
    UnsyncBoxBody::new(full)
}

fn box_channel(ch: Channel<Bytes, Infallible>) -> ProxyBody {
    UnsyncBoxBody::new(ch)
}

/// Runtime configuration for [`LlmProxyAdapter`].
#[derive(Clone, Debug)]
pub struct LlmProxyAdapterConfig {
    /// Logical adapter name (pipeline + telemetry).
    pub name: String,
    /// Local bind address.
    pub http_listen: SocketAddr,
    /// Upstream base URI (`http://host:port` only).
    pub upstream: Uri,
    /// Maximum bytes buffered per direction (request / response).
    pub max_body_bytes: usize,
    /// TCP connect timeout to upstream.
    pub upstream_connect_timeout: Duration,
    /// Timeout waiting for upstream HTTP response headers after dispatching the request.
    pub upstream_headers_timeout: Duration,
    /// Timeout for buffering a unary upstream response body.
    pub upstream_body_read_timeout: Duration,
    /// Max idle time between frames while reading an upstream SSE stream.
    pub upstream_sse_frame_idle_timeout: Duration,
    /// Max concurrent inbound connections. `0` disables the limit.
    pub max_in_flight_connections: usize,
}

impl LlmProxyAdapterConfig {
    /// Construct a new config with defaults for timeouts and connection cap.
    #[must_use]
    pub fn new(name: impl Into<String>, http_listen: SocketAddr, upstream: Uri) -> Self {
        Self {
            name: name.into(),
            http_listen,
            upstream,
            max_body_bytes: 10 * 1024 * 1024,
            upstream_connect_timeout: Duration::from_secs(30),
            upstream_headers_timeout: Duration::from_secs(300),
            upstream_body_read_timeout: Duration::from_secs(900),
            upstream_sse_frame_idle_timeout: Duration::from_secs(600),
            max_in_flight_connections: 512,
        }
    }
}

/// HTTP reverse-proxy adapter with pluggable normalization.
pub struct LlmProxyAdapter {
    cfg: LlmProxyAdapterConfig,
    normalizer: Arc<dyn UpstreamNormalizer>,
    stop: Arc<Notify>,
}

impl LlmProxyAdapter {
    /// Create a proxy adapter with the given normalizer implementation.
    #[must_use]
    pub fn new(cfg: LlmProxyAdapterConfig, normalizer: Arc<dyn UpstreamNormalizer>) -> Self {
        Self { cfg, normalizer, stop: Arc::new(Notify::new()) }
    }
}

fn correlation_request_id(headers: &HeaderMap) -> String {
    const IDS: &[&str] = &["x-mara-request-id", "x-request-id"];
    for name in IDS {
        if let Some(v) = headers.get(*name).and_then(|h| h.to_str().ok()) {
            let t = v.trim();
            if !t.is_empty() {
                return t.to_owned();
            }
        }
    }
    Uuid::now_v7().to_string()
}

/// Value safe for `x-mara-request-id` on the HTTP response (visible ASCII subset). The canonical
/// correlation string on events may still be the original `rid` from [`correlation_request_id`].
fn response_request_id_header_value(rid: &str) -> HeaderValue {
    let t = rid.trim();
    if !t.is_empty() && t.is_ascii() && let Ok(v) = HeaderValue::from_str(t) {
        return v;
    }
    const MAX: usize = 128;
    let ascii: String = rid
        .chars()
        .filter(|c| c.is_ascii_graphic() && !matches!(*c, '"' | '\\'))
        .take(MAX)
        .collect();
    if !ascii.is_empty() && let Ok(v) = HeaderValue::from_str(&ascii) {
        return v;
    }
    HeaderValue::from_str(&Uuid::now_v7().to_string()).expect("uuid v7 is header-safe ascii")
}

fn insert_request_id_header(parts: &mut http::response::Parts, rid: &str) {
    let val = response_request_id_header_value(rid);
    parts.headers.insert(HeaderName::from_static("x-mara-request-id"), val);
}
fn hop_by_hop(name: &str) -> bool {
    matches!(
        name.to_ascii_lowercase().as_str(),
        "connection"
            | "keep-alive"
            | "proxy-authenticate"
            | "proxy-authorization"
            | "te"
            | "trailers"
            | "transfer-encoding"
            | "upgrade"
    )
}

/// Join upstream base (`http://host:port`) with the inbound path and query.
pub(crate) fn join_upstream(base: &Uri, req_uri: &Uri) -> Option<Uri> {
    let authority = base.authority()?.as_str();
    let scheme = base.scheme_str().unwrap_or("http");
    let pq = req_uri
        .path_and_query()
        .map(http::uri::PathAndQuery::as_str)
        .filter(|s| !s.is_empty())
        .unwrap_or("/");
    let s = format!("{scheme}://{authority}{pq}");
    s.parse().ok()
}

fn header_pairs(map: &HeaderMap) -> Vec<(String, String)> {
    map.iter()
        .filter_map(|(k, v)| {
            if hop_by_hop(k.as_str()) {
                return None;
            }
            let vs = v.to_str().ok()?.to_owned();
            Some((k.as_str().to_owned(), vs))
        })
        .collect()
}

fn forward_request_headers(in_headers: &HeaderMap, upstream_host: &str) -> Result<HeaderMap> {
    let mut out = HeaderMap::new();
    for (key, value) in in_headers.iter() {
        if hop_by_hop(key.as_str()) || key == header::HOST {
            continue;
        }
        out.append(key, value.clone());
    }
    let host_val = HeaderValue::from_str(upstream_host).map_err(|e| Error::Adapter {
        adapter: "llm-proxy".into(),
        message: format!("invalid upstream host header: {e}"),
    })?;
    out.insert(header::HOST, host_val);
    Ok(out)
}

async fn collect_limited_body(
    mut body: Incoming,
    max: usize,
) -> Result<(Bytes, bool), hyper::Error> {
    let mut buf = Vec::<u8>::new();
    let mut truncated = false;
    while let Some(frame) = body.frame().await {
        let frame = frame?;
        if let Some(chunk) = frame.data_ref() {
            let remaining = max.saturating_sub(buf.len());
            if remaining == 0 {
                truncated = true;
                break;
            }
            if chunk.len() <= remaining {
                buf.extend_from_slice(chunk);
            } else {
                buf.extend_from_slice(&chunk[..remaining]);
                truncated = true;
                break;
            }
        }
    }
    Ok((Bytes::from(buf), truncated))
}

fn bad_gateway() -> Response<ProxyBody> {
    Response::builder()
        .status(StatusCode::BAD_GATEWAY)
        .body(box_full(Full::new(Bytes::from("Bad Gateway"))))
        .expect("static response")
}

async fn reject_overloaded(mut stream: TcpStream) {
    let _ = stream
        .write_all(
            b"HTTP/1.1 503 Service Unavailable\r\nConnection: close\r\nContent-Length: 0\r\n\r\n",
        )
        .await;
}

fn is_event_stream(headers: &HeaderMap) -> bool {
    headers
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .is_some_and(|s| s.to_ascii_lowercase().contains("text/event-stream"))
}

/// Strip hop-by-hop headers and set `Content-Length` for a unary body.
fn response_to_client(mut parts: http::response::Parts, body: Bytes) -> Response<ProxyBody> {
    let saved = parts.headers.clone();
    parts.headers.clear();
    for (key, val) in saved.iter() {
        let k = key.as_str();
        if hop_by_hop(k)
            || k.eq_ignore_ascii_case("content-length")
            || k.eq_ignore_ascii_case("transfer-encoding")
        {
            continue;
        }
        parts.headers.append(key, val.clone());
    }
    parts.headers.insert(
        header::CONTENT_LENGTH,
        HeaderValue::from_str(&body.len().to_string()).expect("ascii len"),
    );
    parts.version = Version::HTTP_11;
    Response::from_parts(parts, box_full(Full::new(body)))
}

/// Forwardable upstream response headers for streaming (no content-length rewrite).
fn streaming_client_headers(parts: &http::response::Parts) -> HeaderMap {
    let mut out = HeaderMap::new();
    for (key, val) in parts.headers.iter() {
        let k = key.as_str();
        if hop_by_hop(k) {
            continue;
        }
        out.append(key, val.clone());
    }
    out
}

fn proxied_request(
    parts: &http::request::Parts,
    body: Bytes,
    body_truncated: bool,
) -> ProxiedRequest {
    ProxiedRequest {
        method: parts.method.to_string(),
        path_and_query: parts
            .uri
            .path_and_query()
            .map(|pq| pq.as_str().to_owned())
            .unwrap_or_else(|| "/".into()),
        headers: header_pairs(&parts.headers),
        body,
        body_truncated,
        request_id: None,
        gateway_duration_ms: None,
    }
}

async fn normalize_send(
    session_id: &str,
    preq: &ProxiedRequest,
    presp: &ProxiedResponse,
    normalizer: &Arc<dyn UpstreamNormalizer>,
    out: &EventSender,
    adapter_name: &str,
) {
    let events = normalizer.normalize(session_id, preq, presp);
    for mut ev in events {
        crate::w3c_traceparent::apply_traceparent_from_request(&mut ev, preq);
        ev.mara.source_adapter = Some(adapter_name.to_owned());
        if let Some(ref id) = preq.request_id {
            ev.mara.request_id = Some(id.clone());
        }
        if let Some(ms) = preq.gateway_duration_ms
            && ms.is_finite()
            && ms >= 0.0
        {
            ev.attributes.insert("mara.proxy.gateway_duration_ms".into(), AttrValue::Float(ms));
        }
        if let Err(send_err) = out.send(ev).await {
            error!("downstream closed: {send_err}");
            break;
        }
    }
}

#[allow(clippy::too_many_arguments)] // one-off telemetry emission at several early-return sites
async fn emit_proxy_failure(
    session_id: &str,
    parts: &http::request::Parts,
    req_body: Bytes,
    req_trunc: bool,
    failure_kind: ProxyFailureKind,
    message: &str,
    upstream_status: Option<u16>,
    normalizer: &Arc<dyn UpstreamNormalizer>,
    out: &EventSender,
    adapter_name: &str,
) {
    let mut preq = proxied_request(parts, req_body, req_trunc);
    preq.request_id = Some(correlation_request_id(&parts.headers));
    let presp = ProxiedResponse {
        status: 502,
        headers: Vec::new(),
        body: Bytes::copy_from_slice(message.as_bytes()),
        body_truncated: false,
        failure_kind: Some(failure_kind.as_str().to_owned()),
        upstream_status,
        stream_cut_short: false,
    };
    normalize_send(session_id, &preq, &presp, normalizer, out, adapter_name).await;
}

async fn process_one(
    req: Request<Incoming>,
    client: Client<HttpConnector, Full<Bytes>>,
    cfg: Arc<LlmProxyAdapterConfig>,
    normalizer: Arc<dyn UpstreamNormalizer>,
    out: EventSender,
) -> Result<Response<ProxyBody>, Infallible> {
    let adapter_name = cfg.name.clone();
    let session_id = Uuid::now_v7().to_string();
    let t0 = Instant::now();
    let (parts, body_in) = req.into_parts();
    let rid = correlation_request_id(&parts.headers);
    let max_body = cfg.max_body_bytes;
    let (req_body, req_trunc) = match collect_limited_body(body_in, max_body).await {
        Ok(v) => v,
        Err(e) => {
            warn!("read client body: {e}");
            emit_proxy_failure(
                &session_id,
                &parts,
                Bytes::new(),
                false,
                ProxyFailureKind::ClientBodyRead,
                "read client body failed",
                None,
                &normalizer,
                &out,
                &adapter_name,
            )
            .await;
            return Ok(bad_gateway());
        }
    };

    let mut preq = proxied_request(&parts, req_body.clone(), req_trunc);
    preq.request_id = Some(rid.clone());

    let upstream = cfg.upstream.clone();

    let upstream_host = if let Some(a) = upstream.authority() {
        a.to_string()
    } else {
        warn!("upstream uri missing authority");
        emit_proxy_failure(
            &session_id,
            &parts,
            req_body,
            req_trunc,
            ProxyFailureKind::UpstreamConfig,
            "upstream uri missing authority",
            None,
            &normalizer,
            &out,
            &adapter_name,
        )
        .await;
        return Ok(bad_gateway());
    };

    let upstream_uri = if let Some(u) = join_upstream(&upstream, &parts.uri) {
        u
    } else {
        warn!("could not join upstream uri");
        emit_proxy_failure(
            &session_id,
            &parts,
            req_body,
            req_trunc,
            ProxyFailureKind::UpstreamJoin,
            "could not join upstream uri",
            None,
            &normalizer,
            &out,
            &adapter_name,
        )
        .await;
        return Ok(bad_gateway());
    };

    let fwd_headers = match forward_request_headers(&parts.headers, &upstream_host) {
        Ok(h) => h,
        Err(e) => {
            warn!("header forward: {e}");
            emit_proxy_failure(
                &session_id,
                &parts,
                req_body,
                req_trunc,
                ProxyFailureKind::HeaderForward,
                &e.to_string(),
                None,
                &normalizer,
                &out,
                &adapter_name,
            )
            .await;
            return Ok(bad_gateway());
        }
    };

    let up_req = match Request::builder()
        .method(parts.method.clone())
        .uri(upstream_uri)
        .body(Full::new(req_body.clone()))
    {
        Ok(mut r) => {
            *r.headers_mut() = fwd_headers;
            r
        }
        Err(e) => {
            warn!("build upstream request: {e}");
            emit_proxy_failure(
                &session_id,
                &parts,
                req_body,
                req_trunc,
                ProxyFailureKind::UpstreamRequestBuild,
                &e.to_string(),
                None,
                &normalizer,
                &out,
                &adapter_name,
            )
            .await;
            return Ok(bad_gateway());
        }
    };

    let up_resp = match tokio::time::timeout(cfg.upstream_headers_timeout, client.request(up_req)).await {
        Ok(Ok(r)) => r,
        Ok(Err(e)) => {
            warn!("upstream request failed: {e}");
            emit_proxy_failure(
                &session_id,
                &parts,
                req_body,
                req_trunc,
                ProxyFailureKind::UpstreamTransport,
                &e.to_string(),
                None,
                &normalizer,
                &out,
                &adapter_name,
            )
            .await;
            return Ok(bad_gateway());
        }
        Err(_) => {
            warn!("upstream response headers timed out");
            emit_proxy_failure(
                &session_id,
                &parts,
                req_body,
                req_trunc,
                ProxyFailureKind::UpstreamTimeout,
                "upstream response headers timed out",
                None,
                &normalizer,
                &out,
                &adapter_name,
            )
            .await;
            return Ok(bad_gateway());
        }
    };

    let (up_parts, up_body_in) = up_resp.into_parts();
    let up_status = up_parts.status.as_u16();
    let up_resp_header_map = up_parts.headers.clone();

    if is_event_stream(&up_parts.headers) {
        let mut resp_parts = up_parts;
        let hdrs = streaming_client_headers(&resp_parts);
        resp_parts.headers = hdrs;
        resp_parts.version = Version::HTTP_11;
        insert_request_id_header(&mut resp_parts, &rid);

        let (mut tx, rx_body) = Channel::new(32);
        let body_for_client = box_channel(rx_body);

        let normalizer_c = Arc::clone(&normalizer);
        let out_c = out.clone();
        let adapter_c = adapter_name.clone();
        let session_c = session_id.clone();
        let preq_c = preq.clone();
        let max_agg = max_body;
        let sse_start = Instant::now();
        let sse_idle = cfg.upstream_sse_frame_idle_timeout;

        tokio::spawn(async move {
            let mut agg: Vec<u8> = Vec::new();
            let mut truncated = false;
            let mut stream_cut_short = false;
            let mut upstream_body = up_body_in;

            loop {
                let frame_opt = tokio::time::timeout(sse_idle, upstream_body.frame()).await;
                let frame_res = if let Ok(f) = frame_opt {
                    f
                } else {
                    stream_cut_short = true;
                    break;
                };
                let Some(frame_res) = frame_res else { break };
                let frame = match frame_res {
                    Ok(f) => f,
                    Err(e) => {
                        warn!("upstream sse frame read: {e}");
                        break;
                    }
                };
                if let Some(chunk) = frame.data_ref() {
                    let chunk = chunk.chunk();
                    if tx.send_data(Bytes::copy_from_slice(chunk)).await.is_err() {
                        stream_cut_short = true;
                        break;
                    }
                    let remaining = max_agg.saturating_sub(agg.len());
                    if remaining == 0 {
                        truncated = true;
                    } else {
                        let n = remaining.min(chunk.len());
                        agg.extend_from_slice(&chunk[..n]);
                        if n < chunk.len() {
                            truncated = true;
                        }
                    }
                }
            }
            drop(tx);

            let presp = ProxiedResponse {
                status: up_status,
                headers: header_pairs(&up_resp_header_map),
                body: Bytes::from(agg),
                body_truncated: truncated,
                failure_kind: None,
                upstream_status: None,
                stream_cut_short,
            };
            let mut preq_for = preq_c.clone();
            preq_for.gateway_duration_ms = Some(sse_start.elapsed().as_secs_f64() * 1000.0);
            normalize_send(&session_c, &preq_for, &presp, &normalizer_c, &out_c, &adapter_c).await;
        });

        return Ok(Response::from_parts(resp_parts, body_for_client));
    }

    let body_deadline = cfg.upstream_body_read_timeout;
    let (resp_body, resp_trunc) =
        match tokio::time::timeout(body_deadline, collect_limited_body(up_body_in, max_body)).await
        {
            Ok(Ok(v)) => v,
            Ok(Err(e)) => {
                warn!("read upstream body: {e}");
                let presp = ProxiedResponse {
                    status: 502,
                    headers: Vec::new(),
                    body: Bytes::copy_from_slice(b"read upstream body failed"),
                    body_truncated: false,
                    failure_kind: Some(ProxyFailureKind::UpstreamBodyRead.as_str().to_owned()),
                    upstream_status: Some(up_status),
                    stream_cut_short: false,
                };
                normalize_send(&session_id, &preq, &presp, &normalizer, &out, &adapter_name).await;
                return Ok(bad_gateway());
            }
            Err(_) => {
                warn!("upstream unary body read timed out");
                emit_proxy_failure(
                    &session_id,
                    &parts,
                    req_body,
                    req_trunc,
                    ProxyFailureKind::UpstreamTimeout,
                    "upstream unary body read timed out",
                    Some(up_status),
                    &normalizer,
                    &out,
                    &adapter_name,
                )
                .await;
                return Ok(bad_gateway());
            }
        };

    let presp = ProxiedResponse::from_upstream(
        up_status,
        header_pairs(&up_resp_header_map),
        resp_body.clone(),
        resp_trunc,
    );

    preq.gateway_duration_ms = Some(t0.elapsed().as_secs_f64() * 1000.0);
    normalize_send(&session_id, &preq, &presp, &normalizer, &out, &adapter_name).await;

    let mut up_parts = up_parts;
    insert_request_id_header(&mut up_parts, &rid);
    Ok(response_to_client(up_parts, resp_body))
}

#[async_trait]
impl Adapter for LlmProxyAdapter {
    fn name(&self) -> &str {
        &self.cfg.name
    }

    async fn start(&self, out: EventSender) -> Result<()> {
        let listener = TcpListener::bind(self.cfg.http_listen)
            .await
            .map_err(|e| Error::Io { path: Some(self.cfg.http_listen.to_string()), source: e })?;
        info!(
            adapter = %self.cfg.name,
            addr = %self.cfg.http_listen,
            upstream = %self.cfg.upstream,
            "llm http proxy listening"
        );

        let mut connector = HttpConnector::new();
        connector.set_connect_timeout(Some(self.cfg.upstream_connect_timeout));
        let client: Client<HttpConnector, Full<Bytes>> =
            Client::builder(TokioExecutor::new()).build(connector);

        let cfg = Arc::new(self.cfg.clone());
        let limiter = if cfg.max_in_flight_connections > 0 {
            Some(Arc::new(Semaphore::new(cfg.max_in_flight_connections)))
        } else {
            None
        };

        let normalizer = Arc::clone(&self.normalizer);
        let stop = self.stop.clone();
        let adapter_name = self.cfg.name.clone();

        loop {
            tokio::select! {
                () = stop.notified() => {
                    debug!(adapter = %adapter_name, "llm proxy shutdown");
                    return Ok(());
                }
                acc = listener.accept() => {
                    let (stream, _) = acc.map_err(|e| Error::Io {
                        path: Some(self.cfg.http_listen.to_string()),
                        source: e,
                    })?;
                    let permit: Option<tokio::sync::OwnedSemaphorePermit> = if let Some(s) = &limiter {
                        if let Ok(p) = s.clone().try_acquire_owned() {
                            Some(p)
                        } else {
                            reject_overloaded(stream).await;
                            continue;
                        }
                    } else {
                        None
                    };
                    let io = TokioIo::new(stream);
                    let client = client.clone();
                    let cfg = Arc::clone(&cfg);
                    let normalizer = Arc::clone(&normalizer);
                    let out = out.clone();
                    let log_name = adapter_name.clone();
                    tokio::spawn(async move {
                        let _permit = permit;
                        let service = service_fn(move |req: Request<Incoming>| {
                            let client = client.clone();
                            let cfg = Arc::clone(&cfg);
                            let normalizer = Arc::clone(&normalizer);
                            let out = out.clone();
                            async move { process_one(req, client, cfg, normalizer, out).await }
                        });
                        if let Err(e) = http1::Builder::new().serve_connection(io, service).await {
                            debug!(adapter = %log_name, "connection ended: {e}");
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn join_upstream_appends_path() {
        let base: Uri = "http://127.0.0.1:11435".parse().unwrap();
        let req: Uri = "/api/chat?x=1".parse().unwrap();
        let u = join_upstream(&base, &req).expect("uri");
        assert_eq!(u.to_string(), "http://127.0.0.1:11435/api/chat?x=1");
    }

    #[test]
    fn response_request_id_preserves_header_safe_ascii() {
        let v = response_request_id_header_value("abc-def_1.2~");
        assert_eq!(v.to_str().unwrap(), "abc-def_1.2~");
    }

    #[test]
    fn response_request_id_strips_non_ascii_then_truncates() {
        let v = response_request_id_header_value("café-track");
        assert_eq!(v.to_str().unwrap(), "caf-track");
    }

    #[test]
    fn response_request_id_falls_back_to_uuid_when_unusable() {
        let v = response_request_id_header_value("\n\t ");
        let s = v.to_str().unwrap();
        assert_eq!(s.len(), 36, "expected uuid string, got {s:?}");
    }
}
