# OTLP HTTP/gRPC receiver — threat model and operator checklist

Mara’s `mara-adapter-otlp` binds a local HTTP/1 server for `POST /v1/logs` and `POST /v1/traces` (protobuf bodies) and, when configured, a gRPC server for OTLP logs and traces. This document mirrors the LLM proxy non-loopback pattern: **there is no authentication on the wire inside Mara**.

## Trust boundary

- **Loopback default**: Binds default to `127.0.0.1:4318` (HTTP) so only local processes can reach the receiver.
- **Non-loopback**: Binding `0.0.0.0`, a LAN IP, or a container bridge IP exposes protobuf ingestion to anyone who can reach the port. Set `allow_non_loopback_listen = true` on the adapter **only** after placing **TLS termination + network ACLs or mTLS** in front, and after reading this checklist.

## Abuse and DoS considerations

- **Body limits**: `max_body_bytes` rejects oversize bodies with **413** before full buffering where possible. Keep limits aligned with upstream exporter batch sizes.
- **Gzip**: `Content-Encoding: gzip` is supported; pathological compression ratios are bounded by the same `max_body_bytes` cap after decompression in the HTTP stack—still size-limit aggressively.
- **No auth**: Any client that can open a TCP connection to the bind address can post OTLP batches. Treat non-loopback like an **unauthenticated ingestion API**.

## Operator checklist (non-loopback)

1. Terminate TLS at a reverse proxy or service mesh; do not expose raw OTLP on the public Internet.
2. Restrict security groups / firewall rules to known collector IPs.
3. Monitor for unexpected traffic spikes and 413 rates.
4. Prefer separate listeners for internal vs DMZ collectors if you must expose OTLP at all.

## gRPC (`grpc_listen`)

Optional `grpc_listen` (e.g. `127.0.0.1:4317`) uses the same trust model as HTTP. The same `allow_non_loopback_listen` flag applies to **both** HTTP and gRPC bind addresses.
