# Ollama LLM proxy: error taxonomy

This document lists **stable** `mara.proxy.failure_kind` values emitted when the `llm_proxy` adapter returns a synthetic **502** or fails after reading the upstream status line. Renaming these strings is a breaking change for dashboards and alerts.

## Synthetic 502 (`ProxiedResponse.status == 502`)

| `mara.proxy.failure_kind` | When it happens | `mara.proxy.upstream_status` |
|-----------------------------|-----------------|------------------------------|
| `client_body_read` | Reading the inbound HTTP body from the client failed. | absent |
| `upstream_config` | The configured upstream base URI has no authority (host/port). | absent |
| `upstream_join` | The proxy could not join the client path with the upstream base. | absent |
| `header_forward` | Building forwardable request headers failed. | absent |
| `upstream_request_build` | Hyper could not construct the outbound upstream request. | absent |
| `upstream_transport` | TCP/TLS/DNS or other transport error before a complete response. | absent |
| `upstream_body_read` | Upstream returned a status line but reading the response body failed (non-SSE path). | set to the HTTP status that was received (e.g. 200) |
| `upstream_timeout` | Connect, headers, body read, or SSE idle timeout fired while waiting on the upstream. | absent (synthetic **502** before upstream status is known, or mid-stream as documented in proxy code paths) |

## Passthrough upstream HTTP errors (no synthetic 502)

When the upstream returns **4xx/5xx** with a body, the proxy forwards that status. The normalizer emits an **error** event with:

- `http.status_code` — upstream HTTP status (e.g. 503).
- `mara.proxy.failure_kind` — **absent** (the failure is not classified as a proxy-side synthetic error).
- `mara.proxy.upstream_status` — **absent** unless set on a synthetic failure path above.

## SSE streaming

When the client disconnects early, the proxy may still emit a success-path event with `mara.proxy.stream_cut_short` / `mara.ollama.partial` depending on normalizer; there is no `failure_kind` for that case today.

## Source of truth

Rust enum: `mara_adapter_llm_proxy::ProxyFailureKind` (`crates/mara-adapter-llm-proxy/src/proxy_failure_kind.rs`).
