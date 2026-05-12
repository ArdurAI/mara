# Rust Crate Ecosystem for Mara

## Executive summary

This document inventories the Rust crates Mara depends on (or considers depending on), organized by category, with maturity notes and licensing posture. Each entry is verifiable on `crates.io` and `docs.rs`. Pin versions are recommended in M2; specific patch-level pins follow from the workspace's `Cargo.lock`. Where two crates compete (e.g., wasmtime vs wasmer), this document records the rationale for our pick.

The full list of pinned versions lives in the workspace `Cargo.toml` under `[workspace.dependencies]`; this document explains why.

## Async runtime + I/O

- **`tokio`** — the de facto async runtime. License Apache 2.0 / MIT. Stable, widely used. Mara picks `tokio` with the `rt-multi-thread`, `macros`, `signal`, `sync`, `fs`, `io-util`, `process`, `time`, `net` features.
- **`tokio-uring`** — io_uring support on Linux. Promising but optional; consider for v1.x perf gains.
- **`smol`** / **`async-std`** — alternative runtimes. `async-std` is in maintenance mode; `smol` is healthy but smaller community. Mara picks `tokio` for ecosystem reach.
- **`hyper`** + **`hyper-util`** — HTTP/1 and HTTP/2 server/client. Apache 2.0 / MIT. Used for HTTP sinks and the HTTP/protobuf OTLP transport.
- **`tonic`** — gRPC over `hyper`. Apache 2.0. Used for OTLP gRPC.
- **`prost`** — protobuf codegen + runtime. Apache 2.0.
- **`reqwest`** — high-level HTTP client. Apache 2.0 / MIT. Used for webhook + analytics REST sinks.
- **`rustls`** — pure-Rust TLS. Apache 2.0 / MIT / ISC. Mara picks `rustls` over `native-tls` to avoid OpenSSL build complexity and to support FIPS via `rustls-rustcrypto-fips` once stable.
- **`rustls-platform-verifier`** — uses OS-native trust stores with `rustls`.

## OpenTelemetry in Rust

- **`opentelemetry`** + **`opentelemetry_sdk`** — official OTel SDK for Rust. Apache 2.0. Used both for self-telemetry and for understanding/consuming `gen_ai.*` events.
- **`opentelemetry-otlp`** — OTLP exporter. Apache 2.0.
- **`opentelemetry-stdout`** — stdout exporter for debug.
- **`opentelemetry-semantic-conventions`** — generated semconv constants. Apache 2.0. Mara consumes this and extends in `mara-schema`.
- **`tracing`** — structured logging facade. Apache 2.0.
- **`tracing-opentelemetry`** — bridge `tracing` to OTel.
- **`tracing-subscriber`** — sinks and filters for `tracing`. Apache 2.0.
- **`tracing-journald`** — journald output on Linux.
- **`tracing-etw`** — ETW output on Windows.

## Serialization

- **`serde`** + **`serde_json`** — universal serialization. Apache 2.0 / MIT.
- **`simd-json`** — fast JSON parsing for hot paths. Apache 2.0 / MIT. Use selectively; not a drop-in replacement for `serde_json` in all cases.
- **`prost`** — protobuf (as above).
- **`rmp-serde`** — MessagePack.
- **`apache-avro`** — Avro for sinks that need it (Kafka with schema registry).
- **`ciborium`** — CBOR; useful for some bundle formats.
- **`rkyv`** — zero-copy serialization. Considered for WAL on-disk format.

## WASM hosts

- **`wasmtime`** — Bytecode Alliance's WASM runtime. Apache 2.0. Mara picks `wasmtime` over `wasmer` for governance neutrality and CNCF-adjacency.
- **`wasmer`** — alternative; commercial entity behind it. Not picked for the core; available as a contrib option later.
- **`wasmi`** — smaller, interpreter-only. Considered for environments with `MemoryDenyWriteExecute=true` (systemd hardening). Useful as a fallback.
- **`extism`** — plugin SDK with multi-host support. Considered for v1.x plugin marketplace.

## Policy / configuration

- **`regex`** — Rust regex engine with linear-time guarantees. Unicode-DFS-2016 / MIT / Apache 2.0. Linear-time is a hard requirement for Mara to avoid ReDoS.
- **`regex-automata`** — lower-level DFA APIs for advanced use.
- **`aho-corasick`** — multi-pattern literal matching; used for fast secret-prefix detection.
- **`jsonpath-rust`** — JSONPath evaluation over `serde_json::Value`.
- **`cel-rust`** — CEL (Common Expression Language) evaluation; alternative DSL for simple conditions.
- **`minijinja`** — Jinja-compatible templating for sink body templates.

## Storage / WAL

- **`redb`** — pure-Rust embedded KV store; ACID. Apache 2.0 / MIT. Candidate for WAL index.
- **`sled`** — embedded KV store. Apache 2.0 / MIT. **Status concern:** project went into a multi-year stewardship gap; check current health before relying on it for production.
- **`fjall`** — newer LSM-based embedded KV. Apache 2.0 / MIT. Promising alternative.
- **`rust-rocksdb`** — bindings to RocksDB (C++). Apache 2.0 / MIT (with RocksDB Apache 2.0 / GPL 2.0). Heavyweight but proven.
- **`heed`** — LMDB bindings. Used for KV.
- **`crc32fast`** — CRC32 for WAL record integrity.
- **`xxhash-rust`** — xxhash for fast non-crypto hashing.

**WAL recommendation:** custom append-only segmented file format on top of `tokio::fs`, with `crc32fast` per record and `redb` for the offset index. Keep dependencies minimal.

## Compression

- **`zstd`** — zstandard bindings. License BSD-3-Clause / GPL (depends on link mode).
- **`lz4_flex`** — pure-Rust LZ4. MIT.
- **`snap`** — pure-Rust Snappy. BSD-3-Clause.
- **`flate2`** — zlib/gzip. MIT / Apache 2.0.

## Sink-specific clients

- **`aws-sdk-s3`**, **`aws-sdk-cloudwatchlogs`**, **`aws-sdk-kinesis`** — official AWS SDK. Apache 2.0.
- **`object_store`** — Apache Arrow's multi-cloud blob store abstraction. Apache 2.0. **First choice for S3/GCS/Azure Blob** sink.
- **`rdkafka`** — librdkafka bindings. MIT. Heavy native dependency; minimize.
- **`elasticsearch`** — official Elastic client. Apache 2.0 / Elastic License v2 (recently). Verify license direction at time of pin.
- **`google-cloud-storage`** — community GCS client.
- **`rusoto`** — DEPRECATED; do not use.

## CLI and observability

- **`clap`** — CLI parser. MIT / Apache 2.0. Standard.
- **`clap_complete`** — shell completion generation.
- **`console`** — terminal UI helpers.
- **`indicatif`** — progress bars.
- **`tracing-flame`** — flamegraph generation from tracing spans.
- **`pprof`** — CPU profiling.
- **`mimalloc`** or **`jemalloc-sys`** — alternative allocators. Mimalloc tends to be faster and simpler on modern systems; consider for the release binary.

## Testing and benchmarking

- **`criterion`** — statistical benchmarking. Apache 2.0 / MIT.
- **`iai-callgrind`** — instruction-count benchmarking via callgrind. For memory/CPU regressions.
- **`proptest`** — property-based testing. MIT / Apache 2.0.
- **`insta`** — snapshot testing. Apache 2.0.
- **`testcontainers`** — manage real containerized services in integration tests. MIT / Apache 2.0.
- **`assert_cmd`** — test CLI executions. MIT / Apache 2.0.
- **`wiremock`** — HTTP mock server. MIT / Apache 2.0.
- **`tempfile`** — temporary files in tests.

## Security and supply chain (CLI tools, not crates Mara depends on at runtime)

- **`cargo-deny`** — license + ban + advisory checks.
- **`cargo-audit`** — RustSec advisories.
- **`cargo-vet`** — supply-chain audit.
- **`cargo-cyclonedx`** — CycloneDX SBOM.
- **`cargo-sbom`** — SPDX SBOM.
- **`cargo-supply-chain`** — author/dependency graph.

## Recommended dependency picks for Mara M2

1. `tokio` (rt-multi-thread, all features needed; no `unstable`).
2. `tonic` + `prost` for OTLP gRPC.
3. `hyper` + `hyper-util` + `rustls` for HTTP/2 + TLS.
4. `serde` + `serde_json` (default JSON) + `simd-json` (hot-path opt-in).
5. `wasmtime` for WASM policy host.
6. `regex` (linear-time guarantee) + `aho-corasick`.
7. `object_store` for S3/GCS/Azure unified.
8. `criterion` + `iai-callgrind` for perf bench.
9. `proptest` + `insta` for tests.
10. `tracing` + `tracing-subscriber` + `tracing-opentelemetry` for self-telemetry.

## Crates to avoid and why

1. **`rusoto`** — deprecated.
2. **`openssl`-direct** — prefer `rustls` to avoid build complexity.
3. **`actix-web`** — heavier than needed; `axum` is the standard.
4. **`anyhow`** in public APIs — fine internally, but public types use `thiserror`-generated errors.
5. **`sled`** without verifying current maintenance status — use `redb` or `fjall` if `sled` is dormant.

## Crate version pin policy

- Patch updates: auto-merge if CI green (Dependabot/Renovate).
- Minor updates: PR with manual review, weekly cadence.
- Major updates: ADR if API surface changes.
- Pin in `Cargo.lock`; check in `Cargo.lock` (we ship a binary).
- `cargo update --aggressive` not done routinely; targeted updates only.

## Build performance

- Mara workspace builds in CI in ≤ 5 min from cold cache (target).
- `sccache` or `cargo-cache` to share artifacts across CI runs.
- `cargo-chef` for Docker layer caching.

## Cross-compilation toolchain

- `cross` or `cargo-zigbuild` for cross-compilation in CI.
- `cargo-zigbuild` is preferred for static glibc/musl targets and macOS universal2.

## Notes on emerging crates

- **`gpui`** — Zed's UI crate; relevant if Mara ever ships a desktop UI (it won't in v1).
- **`bevy`** — game engine; not relevant.
- **`leptos`**, **`dioxus`** — web UI crates; only relevant for v3 hosted control plane UI.
- **`shuttle`** — deployment toolchain; not core but interesting for the gateway-tier story.

## Release engineering crates / tools

- **`cargo-release`** — automated version bumps and tagging.
- **`cargo-dist`** — release artifact generator.
- **`cross`** — cross-compilation.
- **`cargo-zigbuild`** — universal2 macOS + static Linux.
- **`cargo-deb`** — Debian package generation.
- **`cargo-generate-rpm`** — RPM generation.
- **`goreleaser` equivalent in Rust** — `cargo-dist` is the closest.

## References

- crates.io: <https://crates.io>.
- docs.rs: <https://docs.rs>.
- "Are we async yet?": <https://areweasyncyet.rs>.
- The Rust Foundation: <https://foundation.rust-lang.org>.
- RustSec advisories: <https://rustsec.org>.
