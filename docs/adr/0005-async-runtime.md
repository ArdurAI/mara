# ADR-0005: Use Tokio multi-thread as the async runtime

- **Status:** Accepted
- **Date:** 2026-05-12
- **Authors:** Mara M1 architecture review.

## Context

Mara is I/O-bound across the board: file tails, network sinks, gRPC and HTTP receivers, policy WASM, WAL fsync. A multi-threaded async runtime is essential. Three credible runtimes exist in the Rust ecosystem: Tokio, async-std, and smol.

## Decision

Mara uses **Tokio** with the `rt-multi-thread` runtime as its async executor.

Specifically, Mara enables these Tokio features (centralized in the workspace `Cargo.toml`):

- `rt-multi-thread` — the scheduler.
- `macros` — `#[tokio::main]`, `tokio::select!`, etc.
- `signal` — SIGHUP, SIGTERM handlers.
- `sync` — bounded channels, mutex, RwLock.
- `fs` — async file I/O.
- `io-util` — buffered async readers/writers.
- `process` — subprocess (hooks adapter).
- `time` — sleep, timeout, intervals.
- `net` — TCP, UDP, Unix sockets.

The runtime is configured at startup with worker thread count = number of CPU cores (clamped to a configurable max), `max_blocking_threads` set generously for file I/O and WASM execution.

## Alternatives considered

- **async-std.** Pros: clean API. Cons: project in maintenance mode; ecosystem reach is much smaller; many libraries (tonic, hyper, reqwest, etc.) target Tokio. Rejected.
- **smol.** Pros: small, fast, healthy community. Cons: ecosystem reach. Rejected.
- **Glommio.** Pros: thread-per-core io_uring on Linux. Cons: Linux-only; ecosystem reach limited; we need macOS and Windows. Rejected for default; may be evaluated as a Linux-only fast path in v1.x.
- **Custom executor.** Pros: optimal control. Cons: vast undertaking; opportunity cost. Rejected.

## Consequences

- Tokio version is pinned in the workspace `Cargo.toml`. Major version bumps require an ADR amendment.
- All public traits in `mara-core` that need async are `async_trait`-based for object safety; `async fn in trait` (stable since 1.75) is used inside the same crate where dynamic dispatch isn't needed.
- The runtime is started in `mara-cli::main`; library crates do not assume a particular runtime exists, but in practice they will only run in Tokio environments.
- Benchmarks use Tokio's multi-thread runtime to reflect production conditions.
- Future work: investigate Tokio's experimental `unstable` features (e.g., task budget API for better latency control) once they stabilize.

## References

- Tokio docs: <https://tokio.rs>.
- Rust async ecosystem state: <https://areweasyncyet.rs>.
- [`plans/04-implementation/03-architecture-blocks.md`](../../plans/04-implementation/03-architecture-blocks.md) "Pipeline scheduler".
