//! Self-telemetry HTTP server: `GET /metrics` (Prometheus), `GET /healthz`, `GET /readyz`.
//!
//! `/readyz` semantics (what counts as “ready”) are documented in
//! `docs/observability/mara-readyz-semantics.md`.

use std::convert::Infallible;
use std::sync::Arc;

use bytes::Bytes;
use http_body_util::Full;
use hyper::body::Incoming;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{Request, Response, StatusCode};
use hyper_util::rt::TokioIo;
use mara_core::{PipelineSelfMetrics, render_prometheus};
use tokio::net::TcpListener;
use tokio::sync::{Notify, Semaphore};
use tracing::{debug, info, warn};

/// Options for [`serve_self_metrics_on`].
#[derive(Clone, Default)]
pub struct SelfMetricsListenOptions {
    /// When set, `/readyz` returns 200 only if the closure returns true (M2-09).
    pub readiness: Option<Arc<dyn Fn() -> bool + Send + Sync>>,
    /// Limit concurrent HTTP/1 handler tasks (M2-15). `None` means unlimited.
    pub max_in_flight_connections: Option<usize>,
}

/// Build HTTP status, `Content-Type`, and body for a self-metrics path (unit-testable).
#[must_use]
pub fn self_metrics_http_response(
    pipelines: &[Arc<PipelineSelfMetrics>],
    path: &str,
    readiness: Option<bool>,
) -> (u16, &'static str, String) {
    match path {
        "/metrics" => (
            200,
            "text/plain; version=0.0.4; charset=utf-8",
            render_prometheus(pipelines),
        ),
        "/healthz" => (200, "text/plain; charset=utf-8", "ok".into()),
        "/readyz" => match readiness {
            Some(true) => (200, "text/plain; charset=utf-8", "ready".into()),
            Some(false) => (503, "text/plain; charset=utf-8", "not ready".into()),
            None => (503, "text/plain; charset=utf-8", "readiness probe not configured".into()),
        },
        _ => (404, "text/plain; charset=utf-8", "not found".into()),
    }
}

/// Serve on an already-bound listener (tests can bind to `127.0.0.1:0` first).
pub async fn serve_self_metrics_on(
    listener: TcpListener,
    pipelines: Vec<Arc<PipelineSelfMetrics>>,
    shutdown: Arc<Notify>,
    opts: SelfMetricsListenOptions,
) -> std::io::Result<()> {
    let addr = listener.local_addr()?;
    let ready_note = if opts.readiness.is_some() { " /readyz" } else { "" };
    let cap_note = opts
        .max_in_flight_connections
        .map(|n| format!("; max_connections={n}"))
        .unwrap_or_default();
    info!(%addr, "mara self-telemetry listening (/metrics, /healthz{ready_note}{cap_note})");

    let in_flight: Option<Arc<Semaphore>> =
        opts.max_in_flight_connections.map(|n| Arc::new(Semaphore::new(n)));

    loop {
        tokio::select! {
            () = shutdown.notified() => {
                info!(%addr, "self-telemetry shutdown");
                return Ok(());
            }
            accept = listener.accept() => {
                let (stream, _) = match accept {
                    Ok(v) => v,
                    Err(e) => {
                        warn!(%addr, "accept error: {e}");
                        continue;
                    }
                };
                let pipes = pipelines.clone();
                let readiness = opts.readiness.clone();
                let sem = in_flight.clone();
                tokio::spawn(async move {
                    let _permit = match sem {
                        Some(s) => match s.acquire_owned().await {
                            Ok(g) => Some(g),
                            Err(_) => return,
                        },
                        None => None,
                    };
                    let io = TokioIo::new(stream);
                    let svc = service_fn(move |req: Request<Incoming>| {
                        let pipes = pipes.clone();
                        let readiness = readiness.clone();
                        async move {
                            let path = req.uri().path().to_owned();
                            let ready = readiness.as_ref().map(|f| f());
                            let (code, ct, body) =
                                self_metrics_http_response(&pipes, path.as_str(), ready);
                            let res = Response::builder()
                                .status(StatusCode::from_u16(code).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR))
                                .header("content-type", ct)
                                .body(Full::new(Bytes::from(body)))
                                .expect("response build");
                            Ok::<Response<Full<Bytes>>, Infallible>(res)
                        }
                    });
                    if let Err(e) = http1::Builder::new().serve_connection(io, svc).await {
                        debug!("metrics connection ended: {e}");
                    }
                });
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::time::Duration;

    use mara_core::PipelineSelfMetrics;
    use mara_schema::{AttrValue, Event, EventKind, SourceRuntime};
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::sync::Notify;

    use super::*;

    fn sample_gen_ai_completion() -> Event {
        let mut ev = Event::now(EventKind::Completion, "mara-adapter-llm-proxy");
        ev.resource.source_runtime = Some(SourceRuntime::Ollama);
        ev.gen_ai.system = Some("ollama".into());
        ev.gen_ai.operation_name = Some("chat".into());
        ev.gen_ai.usage.input_tokens = Some(2);
        ev.gen_ai.usage.output_tokens = Some(1);
        ev.attributes.insert("mara.ollama.total_duration_ms".into(), AttrValue::Float(9.0));
        ev
    }

    #[test]
    fn self_metrics_http_response_metrics_healthz_readyz() {
        let m = Arc::new(PipelineSelfMetrics::new("p"));
        m.record_delivered(&sample_gen_ai_completion());
        let (c, _ct, b) = self_metrics_http_response(&[m], "/metrics", None);
        assert_eq!(c, 200);
        assert!(b.contains("mara_gen_ai_requests_completed_total{pipeline=\"p\"} 1"));

        let (c2, _, b2) = self_metrics_http_response(&[], "/healthz", None);
        assert_eq!(c2, 200);
        assert_eq!(b2, "ok");

        let (c3, _, b3) = self_metrics_http_response(&[], "/readyz", None);
        assert_eq!(c3, 503);
        assert_eq!(b3, "readiness probe not configured");

        let (c4, _, b4) = self_metrics_http_response(&[], "/readyz", Some(false));
        assert_eq!(c4, 503);
        assert_eq!(b4, "not ready");

        let (c5, _, b5) = self_metrics_http_response(&[], "/readyz", Some(true));
        assert_eq!(c5, 200);
        assert_eq!(b5, "ready");

        let (c6, _, b6) = self_metrics_http_response(&[], "/nope", None);
        assert_eq!(c6, 404);
        assert_eq!(b6, "not found");
    }

    #[tokio::test]
    async fn tcp_roundtrip_healthz_and_metrics() {
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
        let addr = listener.local_addr().expect("addr");

        let m = Arc::new(PipelineSelfMetrics::new("tcp-pipe"));
        m.record_delivered(&sample_gen_ai_completion());
        let shutdown = Arc::new(Notify::new());
        let sd = shutdown.clone();
        let server = tokio::spawn(serve_self_metrics_on(
            listener,
            vec![m],
            sd,
            SelfMetricsListenOptions::default(),
        ));

        tokio::time::sleep(Duration::from_millis(30)).await;

        let mut stream = tokio::net::TcpStream::connect(addr).await.expect("connect");
        stream.write_all(b"GET /healthz HTTP/1.0\r\n\r\n").await.expect("write");
        let mut buf = vec![0u8; 512];
        let n = stream.read(&mut buf).await.expect("read");
        let text = String::from_utf8_lossy(&buf[..n]);
        assert!(text.contains("200 OK"), "{text}");
        assert!(text.contains("ok"), "{text}");

        let mut stream = tokio::net::TcpStream::connect(addr).await.expect("connect");
        stream.write_all(b"GET /metrics HTTP/1.0\r\n\r\n").await.expect("write");
        let mut buf = vec![0u8; 8192];
        let n = stream.read(&mut buf).await.expect("read");
        let text = String::from_utf8_lossy(&buf[..n]);
        assert!(text.contains("200 OK"), "{text}");
        assert!(text.contains("mara_gen_ai_requests_completed_total"), "{text}");

        shutdown.notify_one();
        let _ = tokio::time::timeout(Duration::from_secs(3), server).await;
    }
}
