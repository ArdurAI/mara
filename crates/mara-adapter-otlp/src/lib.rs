//! OTLP receiver adapter.
//!
//! Receives OpenTelemetry Protocol traffic and emits canonical
//! Mara events.  MVP ships HTTP/protobuf on `127.0.0.1:4318`
//! (`POST /v1/logs` and `POST /v1/traces`); gRPC on `:4317`
//! lands in MVP+1.
//!
//! Tier A in the integration-tier model.  Primary capture path
//! for Claude Code, Codex, and Gemini CLI (all three ship
//! first-party OTLP exporters per
//! `plans/01-landscape/08-ai-runtime-telemetry-surfaces.md`).
//!
//! Implementation contract:
//! - Bind a configured local address.
//! - Accept `application/x-protobuf` request bodies, decoded with
//!   `opentelemetry-proto`.
//! - Support `Content-Encoding: gzip` and identity.
//! - Translate each `LogRecord` and `Span` into a canonical
//!   [`Event`](mara_schema::Event) and emit it via the configured
//!   [`EventSender`](mara_core::traits::EventSender).
//! - Honour the `OTEL_SEMCONV_STABILITY_OPT_IN` toggle by
//!   preserving the upstream attribute names verbatim.

#![doc(html_root_url = "https://docs.rs/mara-adapter-otlp/0.1.0")]

pub mod config;
pub mod grpc;
pub mod http;
pub mod normalize;

pub use config::OtlpHttpAdapterConfig;
pub use http::OtlpHttpAdapter;
