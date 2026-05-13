//! HTTP/1.1 reverse proxy: accept, forward, capture, normalize.

use std::convert::Infallible;
use std::net::SocketAddr;
use std::sync::Arc;

use async_trait::async_trait;
use bytes::Buf;
use bytes::Bytes;
use http::header::{self, HeaderMap, HeaderValue};
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
use tokio::net::TcpListener;
use tokio::sync::Notify;
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
}

impl LlmProxyAdapterConfig {
    /// Construct a new config.
    #[must_use]
    pub fn new(name: impl Into<String>, http_listen: SocketAddr, upstream: Uri) -> Self {
        Self { name: name.into(), http_listen, upstream, max_body_bytes: 10 * 1024 * 1024 }
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
        ev.mara.source_adapter = Some(adapter_name.to_owned());
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
    let preq = proxied_request(parts, req_body, req_trunc);
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
    upstream: Uri,
    max_body: usize,
    normalizer: Arc<dyn UpstreamNormalizer>,
    out: EventSender,
    adapter_name: String,
) -> Result<Response<ProxyBody>, Infallible> {
    let session_id = Uuid::now_v7().to_string();
    let (parts, body_in) = req.into_parts();
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

    let preq = proxied_request(&parts, req_body.clone(), req_trunc);

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

    let up_resp = match client.request(up_req).await {
        Ok(r) => r,
        Err(e) => {
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
    };

    let (up_parts, up_body_in) = up_resp.into_parts();
    let up_status = up_parts.status.as_u16();
    let up_resp_header_map = up_parts.headers.clone();

    if is_event_stream(&up_parts.headers) {
        let mut resp_parts = up_parts;
        let hdrs = streaming_client_headers(&resp_parts);
        resp_parts.headers = hdrs;
        resp_parts.version = Version::HTTP_11;

        let (mut tx, rx_body) = Channel::new(32);
        let body_for_client = box_channel(rx_body);

        let normalizer_c = Arc::clone(&normalizer);
        let out_c = out.clone();
        let adapter_c = adapter_name.clone();
        let session_c = session_id.clone();
        let preq_c = preq.clone();
        let max_agg = max_body;

        tokio::spawn(async move {
            let mut agg: Vec<u8> = Vec::new();
            let mut truncated = false;
            let mut stream_cut_short = false;
            let mut upstream_body = up_body_in;

            while let Some(frame_res) = upstream_body.frame().await {
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
            normalize_send(&session_c, &preq_c, &presp, &normalizer_c, &out_c, &adapter_c).await;
        });

        return Ok(Response::from_parts(resp_parts, body_for_client));
    }

    let (resp_body, resp_trunc) = match collect_limited_body(up_body_in, max_body).await {
        Ok(v) => v,
        Err(e) => {
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
    };

    let presp = ProxiedResponse::from_upstream(
        up_status,
        header_pairs(&up_resp_header_map),
        resp_body.clone(),
        resp_trunc,
    );

    normalize_send(&session_id, &preq, &presp, &normalizer, &out, &adapter_name).await;

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

        let client: Client<HttpConnector, Full<Bytes>> =
            Client::builder(TokioExecutor::new()).build_http();
        let upstream = self.cfg.upstream.clone();
        let max_body = self.cfg.max_body_bytes;
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
                    let io = TokioIo::new(stream);
                    let client = client.clone();
                    let upstream = upstream.clone();
                    let normalizer = Arc::clone(&normalizer);
                    let out = out.clone();
                    let name = adapter_name.clone();
                    let log_name = adapter_name.clone();
                    tokio::spawn(async move {
                        let service = service_fn(move |req: Request<Incoming>| {
                            let client = client.clone();
                            let upstream = upstream.clone();
                            let normalizer = Arc::clone(&normalizer);
                            let out = out.clone();
                            let name = name.clone();
                            async move { process_one(req, client, upstream, max_body, normalizer, out, name).await }
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
}
