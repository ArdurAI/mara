//! HTTP proxy adapter.
//!
//! Binds a local port, forwards every request to a configured
//! upstream LLM endpoint, and captures the request and response
//! body pairs without mutation.  Each runtime crate supplies a
//! [`UpstreamNormalizer`](crate::UpstreamNormalizer) that translates
//! the captured exchange into canonical Mara events.
//!
//! Used at MVP for Ollama; generalises post-MVP to any OpenAI-compat
//! upstream (OpenAI direct, Anthropic via OpenAI shim, Bedrock,
//! Fireworks, Together, etc.).
//!
//! Detailed design in
//! `plans/08-mvp/12-ollama-integration-design.md`.

#![doc(html_root_url = "https://docs.rs/mara-adapter-llm-proxy/0.1.0")]

mod exchange;
mod http_proxy;
mod normalizer;
mod proxy_failure_kind;
mod w3c_traceparent;

pub use exchange::{ProxiedRequest, ProxiedResponse};
pub use http_proxy::{LlmProxyAdapter, LlmProxyAdapterConfig};
pub use normalizer::{PassthroughNormalizer, UpstreamNormalizer};
pub use proxy_failure_kind::ProxyFailureKind;

/// Marker for the proxy adapter's pattern.
pub const ADAPTER_KIND: &str = "llm-proxy";

#[cfg(test)]
mod tests {
    use mara_core::Adapter;
    use mara_core::traits::DEFAULT_CHANNEL_CAPACITY;
    use mara_schema::{AttrValue, EventKind};
    use tokio::sync::mpsc;

    use super::*;

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn traceparent_header_sets_trace_and_span_ids() {
        use std::convert::Infallible;
        use std::net::SocketAddr;

        use bytes::Bytes;
        use http_body_util::{BodyExt, Full};
        use hyper::body::Incoming;
        use hyper::service::service_fn;
        use hyper_util::rt::TokioIo;
        use tokio::net::TcpListener;

        let upstream = TcpListener::bind("127.0.0.1:0").await.expect("bind");
        let up_addr: SocketAddr = upstream.local_addr().unwrap();
        let _up_task = tokio::spawn(async move {
            let (stream, _) = upstream.accept().await.expect("accept");
            let io = TokioIo::new(stream);
            let svc = service_fn(|req: http::Request<Incoming>| async move {
                let (_parts, body) = req.into_parts();
                let b = body.collect().await.expect("read").to_bytes();
                Ok::<_, Infallible>(
                    http::Response::builder()
                        .status(200)
                        .header("content-type", "application/json")
                        .body(Full::new(Bytes::from(format!(
                            r#"{{"echo":"{}"}}"#,
                            String::from_utf8_lossy(&b)
                        ))))
                        .unwrap(),
                )
            });
            let _ = hyper::server::conn::http1::Builder::new().serve_connection(io, svc).await;
        });

        tokio::time::sleep(std::time::Duration::from_millis(20)).await;

        let proxy_listen = TcpListener::bind("127.0.0.1:0").await.expect("bind");
        let proxy_addr = proxy_listen.local_addr().unwrap();
        drop(proxy_listen);

        let cfg = LlmProxyAdapterConfig {
            name: "test-proxy".into(),
            http_listen: proxy_addr,
            upstream: format!("http://{up_addr}").parse().expect("uri"),
            max_body_bytes: 1024 * 1024,
        };
        let adapter = std::sync::Arc::new(LlmProxyAdapter::new(
            cfg,
            std::sync::Arc::new(PassthroughNormalizer),
        ));
        let (tx, mut rx) = mpsc::channel(DEFAULT_CHANNEL_CAPACITY);
        let h = tokio::spawn({
            let a = std::sync::Arc::clone(&adapter);
            async move { a.start(tx).await }
        });
        tokio::time::sleep(std::time::Duration::from_millis(30)).await;

        let tp = "00-0af7651916cd43dd8448eb211c80319c-b7ad6b7169203331-01";
        let client = reqwest::Client::new();
        let r = client
            .post(format!("http://{proxy_addr}/test"))
            .header("traceparent", tp)
            .body(r#"{"hello":"world"}"#)
            .send()
            .await
            .expect("client");
        assert_eq!(r.status(), 200);

        let ev = tokio::time::timeout(std::time::Duration::from_secs(2), rx.recv())
            .await
            .expect("timeout")
            .expect("event");
        assert_eq!(
            ev.trace_id.map(|t| t.0),
            Some([
                0x0a, 0xf7, 0x65, 0x19, 0x16, 0xcd, 0x43, 0xdd, 0x84, 0x48, 0xeb, 0x21, 0x1c, 0x80,
                0x31, 0x9c
            ])
        );
        assert_eq!(ev.span_id.map(|s| s.0), Some([0xb7, 0xad, 0x6b, 0x71, 0x69, 0x20, 0x33, 0x31]));

        adapter.shutdown().await.expect("shutdown");
        let _ = tokio::time::timeout(std::time::Duration::from_secs(2), h).await;
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn passthrough_emits_system_event() {
        use std::convert::Infallible;
        use std::net::SocketAddr;

        use bytes::Bytes;
        use http_body_util::{BodyExt, Full};
        use hyper::body::Incoming;
        use hyper::service::service_fn;
        use hyper_util::rt::TokioIo;
        use tokio::net::TcpListener;

        let upstream = TcpListener::bind("127.0.0.1:0").await.expect("bind");
        let up_addr: SocketAddr = upstream.local_addr().unwrap();
        let _up_task = tokio::spawn(async move {
            let (stream, _) = upstream.accept().await.expect("accept");
            let io = TokioIo::new(stream);
            let svc = service_fn(|req: http::Request<Incoming>| async move {
                let (_parts, body) = req.into_parts();
                let b = body.collect().await.expect("read").to_bytes();
                Ok::<_, Infallible>(
                    http::Response::builder()
                        .status(200)
                        .header("content-type", "application/json")
                        .body(Full::new(Bytes::from(format!(
                            r#"{{"echo":"{}"}}"#,
                            String::from_utf8_lossy(&b)
                        ))))
                        .unwrap(),
                )
            });
            let _ = hyper::server::conn::http1::Builder::new().serve_connection(io, svc).await;
        });

        tokio::time::sleep(std::time::Duration::from_millis(20)).await;

        let proxy_listen = TcpListener::bind("127.0.0.1:0").await.expect("bind");
        let proxy_addr = proxy_listen.local_addr().unwrap();
        drop(proxy_listen);

        let cfg = LlmProxyAdapterConfig {
            name: "test-proxy".into(),
            http_listen: proxy_addr,
            upstream: format!("http://{up_addr}").parse().expect("uri"),
            max_body_bytes: 1024 * 1024,
        };
        let adapter = std::sync::Arc::new(LlmProxyAdapter::new(
            cfg,
            std::sync::Arc::new(PassthroughNormalizer),
        ));
        let (tx, mut rx) = mpsc::channel(DEFAULT_CHANNEL_CAPACITY);
        let h = tokio::spawn({
            let a = std::sync::Arc::clone(&adapter);
            async move { a.start(tx).await }
        });
        tokio::time::sleep(std::time::Duration::from_millis(30)).await;

        let client = reqwest::Client::new();
        let r = client
            .post(format!("http://{proxy_addr}/test"))
            .body(r#"{"hello":"world"}"#)
            .send()
            .await
            .expect("client");
        assert_eq!(r.status(), 200);

        let ev = tokio::time::timeout(std::time::Duration::from_secs(2), rx.recv())
            .await
            .expect("timeout")
            .expect("event");
        assert!(matches!(ev.event_kind, EventKind::System));
        assert_eq!(ev.mara.source_adapter.as_deref(), Some("test-proxy"));

        adapter.shutdown().await.expect("shutdown");
        let _ = tokio::time::timeout(std::time::Duration::from_secs(2), h).await;
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn upstream_transport_failure_is_502_with_error_event() {
        use std::net::SocketAddr;

        use tokio::net::TcpListener;

        let proxy_listen = TcpListener::bind("127.0.0.1:0").await.expect("bind");
        let proxy_addr: SocketAddr = proxy_listen.local_addr().unwrap();
        drop(proxy_listen);

        let cfg = LlmProxyAdapterConfig {
            name: "test-proxy".into(),
            http_listen: proxy_addr,
            upstream: "http://127.0.0.1:1".parse().expect("uri"),
            max_body_bytes: 1024 * 1024,
        };
        let adapter = std::sync::Arc::new(LlmProxyAdapter::new(
            cfg,
            std::sync::Arc::new(PassthroughNormalizer),
        ));
        let (tx, mut rx) = mpsc::channel(DEFAULT_CHANNEL_CAPACITY);
        let h = tokio::spawn({
            let a = std::sync::Arc::clone(&adapter);
            async move { a.start(tx).await }
        });
        tokio::time::sleep(std::time::Duration::from_millis(40)).await;

        let client = reqwest::Client::new();
        let r = client
            .post(format!("http://{proxy_addr}/api/chat"))
            .body("{}")
            .send()
            .await
            .expect("client");
        assert_eq!(r.status(), reqwest::StatusCode::BAD_GATEWAY);

        let ev = tokio::time::timeout(std::time::Duration::from_secs(2), rx.recv())
            .await
            .expect("timeout")
            .expect("event");
        assert!(matches!(ev.event_kind, EventKind::Error));
        assert_eq!(
            ev.attributes.get("mara.proxy.failure_kind"),
            Some(&AttrValue::String("upstream_transport".into()))
        );

        adapter.shutdown().await.expect("shutdown");
        let _ = tokio::time::timeout(std::time::Duration::from_secs(2), h).await;
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn upstream_503_is_forwarded_with_error_event() {
        use std::convert::Infallible;
        use std::net::SocketAddr;

        use bytes::Bytes;
        use http_body_util::{BodyExt, Full};
        use hyper::body::Incoming;
        use hyper::service::service_fn;
        use hyper_util::rt::TokioIo;
        use tokio::net::TcpListener;

        let upstream = TcpListener::bind("127.0.0.1:0").await.expect("bind");
        let up_addr: SocketAddr = upstream.local_addr().unwrap();
        let _up_task = tokio::spawn(async move {
            let (stream, _) = upstream.accept().await.expect("accept");
            let io = TokioIo::new(stream);
            let svc = service_fn(|req: http::Request<Incoming>| async move {
                let (_parts, body) = req.into_parts();
                let _ = body.collect().await.expect("read").to_bytes();
                Ok::<_, Infallible>(
                    http::Response::builder()
                        .status(503)
                        .header("content-type", "application/json")
                        .body(Full::new(Bytes::from_static(br#"{"error":"overload"}"#)))
                        .unwrap(),
                )
            });
            let _ = hyper::server::conn::http1::Builder::new().serve_connection(io, svc).await;
        });

        tokio::time::sleep(std::time::Duration::from_millis(20)).await;

        let proxy_listen = TcpListener::bind("127.0.0.1:0").await.expect("bind");
        let proxy_addr = proxy_listen.local_addr().unwrap();
        drop(proxy_listen);

        let cfg = LlmProxyAdapterConfig {
            name: "test-proxy".into(),
            http_listen: proxy_addr,
            upstream: format!("http://{up_addr}").parse().expect("uri"),
            max_body_bytes: 1024 * 1024,
        };
        let adapter = std::sync::Arc::new(LlmProxyAdapter::new(
            cfg,
            std::sync::Arc::new(PassthroughNormalizer),
        ));
        let (tx, mut rx) = mpsc::channel(DEFAULT_CHANNEL_CAPACITY);
        let h = tokio::spawn({
            let a = std::sync::Arc::clone(&adapter);
            async move { a.start(tx).await }
        });
        tokio::time::sleep(std::time::Duration::from_millis(30)).await;

        let client = reqwest::Client::new();
        let r = client
            .post(format!("http://{proxy_addr}/v1/chat/completions"))
            .body("{}")
            .send()
            .await
            .expect("client");
        assert_eq!(r.status(), reqwest::StatusCode::SERVICE_UNAVAILABLE);

        let ev = tokio::time::timeout(std::time::Duration::from_secs(2), rx.recv())
            .await
            .expect("timeout")
            .expect("event");
        assert!(matches!(ev.event_kind, EventKind::Error));
        assert_eq!(ev.attributes.get("http.status_code"), Some(&AttrValue::Int(503)));

        adapter.shutdown().await.expect("shutdown");
        let _ = tokio::time::timeout(std::time::Duration::from_secs(2), h).await;
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn sse_response_streams_and_emits_event() {
        use std::convert::Infallible;
        use std::net::SocketAddr;

        use bytes::Bytes;
        use http_body_util::{BodyExt, Full};
        use hyper::body::Incoming;
        use hyper::service::service_fn;
        use hyper_util::rt::TokioIo;
        use tokio::net::TcpListener;

        let sse_payload = "data: {\"token\":\"hi\"}\n\ndata: [DONE]\n\n";
        let upstream = TcpListener::bind("127.0.0.1:0").await.expect("bind");
        let up_addr: SocketAddr = upstream.local_addr().unwrap();
        let body_bytes = Bytes::copy_from_slice(sse_payload.as_bytes());
        let _up_task = tokio::spawn(async move {
            let (stream, _) = upstream.accept().await.expect("accept");
            let io = TokioIo::new(stream);
            let svc = service_fn({
                let body_bytes = body_bytes.clone();
                move |req: http::Request<Incoming>| {
                    let bb = body_bytes.clone();
                    async move {
                        let (_parts, body) = req.into_parts();
                        let _ = body.collect().await.expect("read").to_bytes();
                        Ok::<_, Infallible>(
                            http::Response::builder()
                                .status(200)
                                .header("content-type", "text/event-stream; charset=utf-8")
                                .body(Full::new(bb))
                                .unwrap(),
                        )
                    }
                }
            });
            let _ = hyper::server::conn::http1::Builder::new().serve_connection(io, svc).await;
        });

        tokio::time::sleep(std::time::Duration::from_millis(20)).await;

        let proxy_listen = TcpListener::bind("127.0.0.1:0").await.expect("bind");
        let proxy_addr = proxy_listen.local_addr().unwrap();
        drop(proxy_listen);

        let cfg = LlmProxyAdapterConfig {
            name: "test-proxy".into(),
            http_listen: proxy_addr,
            upstream: format!("http://{up_addr}").parse().expect("uri"),
            max_body_bytes: 1024 * 1024,
        };
        let adapter = std::sync::Arc::new(LlmProxyAdapter::new(
            cfg,
            std::sync::Arc::new(PassthroughNormalizer),
        ));
        let (tx, mut rx) = mpsc::channel(DEFAULT_CHANNEL_CAPACITY);
        let h = tokio::spawn({
            let a = std::sync::Arc::clone(&adapter);
            async move { a.start(tx).await }
        });
        tokio::time::sleep(std::time::Duration::from_millis(30)).await;

        let client = reqwest::Client::new();
        let r = client
            .post(format!("http://{proxy_addr}/api/chat"))
            .body("{}")
            .send()
            .await
            .expect("client");
        assert_eq!(r.status(), reqwest::StatusCode::OK);
        let text = r.text().await.expect("body");
        assert!(text.contains("data:"), "client should see sse: {text:?}");

        let ev = tokio::time::timeout(std::time::Duration::from_secs(2), rx.recv())
            .await
            .expect("timeout")
            .expect("event");
        assert!(matches!(ev.event_kind, EventKind::System));
        let rb = ev.attributes.get("mara.proxy.response_bytes");
        assert!(matches!(rb, Some(AttrValue::Int(n)) if *n > 0));

        adapter.shutdown().await.expect("shutdown");
        let _ = tokio::time::timeout(std::time::Duration::from_secs(2), h).await;
    }

    #[test]
    fn adapter_kind_is_stable() {
        assert_eq!(ADAPTER_KIND, "llm-proxy");
    }
}
