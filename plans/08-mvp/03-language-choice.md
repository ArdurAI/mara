# MVP — Language Choice

## Executive summary

Mara is and will remain **Rust**. The choice is constrained by the value-claim conjunction in [`../03-value-proposition/03-unique-value-claims.md`](../03-value-proposition/03-unique-value-claims.md): single statically-linked binary, ≤128 MiB idle RSS, ≥50k EPS sustained on a single core, no runtime VM dependency, memory-safe, multi-platform (macOS, Linux, Windows), and licensable under Apache 2.0 with a clean dependency tree. Only Rust meets all of these for an edge-agent shape as of May 2026. This document records the alternatives we considered and why each was rejected, so future contributors can revisit the decision against new evidence rather than re-litigate it from first principles.

## What we're optimising for

Eight constraints, ranked by priority:

1. **Single statically-linked binary**, deployable with no runtime VM, interpreter, or DLL footprint.
2. **Memory safety** without GC pauses, because tail latency matters at high EPS.
3. **Concurrency safety** because Mara is a fan-in / fan-out async system with shared state in the policy chain.
4. **Footprint** ≤ 128 MiB idle RSS, ≤ 512 MiB at sustained SLO load.
5. **Performance ceiling** of ≥ 50k EPS on a single core.
6. **Ecosystem reach** — high-quality crates for OTel, gRPC, HTTP/2, TLS, WASM hosting, regex, JSON, Parquet, Kafka, S3.
7. **Cross-platform** — macOS, Linux, Windows; ideally including ARM64 across all three.
8. **Talent + community** — enough contributors and stable enough tooling to maintain the project for 5+ years.

## Why Rust

### Hard requirements satisfied

- **Static binary**: `cargo build --release` with `target-cpu=native` and `strip = "symbols"` produces a single binary, no runtime deps beyond OS-standard.
- **Memory safety**: borrow checker prevents use-after-free, double-free, data races. No null pointers. `unsafe_code = forbid` workspace-wide.
- **Concurrency safety**: `Send` + `Sync` markers, ownership rules, and `tokio` make the kind of races that ruin C++ telemetry agents structurally impossible.
- **No GC**: deterministic destruction; no stop-the-world pauses; predictable tail latency.
- **Footprint**: with `mimalloc` allocator and careful `Vec`/`String` use, our 24-crate workspace baseline at idle is well under 100 MiB. Field-validated.
- **Cross-platform**: first-class targets for x86_64 + aarch64 on Linux (glibc and musl), macOS (universal2), Windows (MSVC). `cargo-zigbuild` and `cargo-dist` ergonomically cross-compile.

### Ecosystem fit

The May 2026 Rust crate ecosystem covers Mara's needs end-to-end. See [`../01-landscape/07-rust-crate-ecosystem.md`](../01-landscape/07-rust-crate-ecosystem.md). The picks we are committed to: `tokio`, `tonic`, `hyper`, `rustls`, `opentelemetry-rust`, `prost`, `serde`, `regex`, `wasmtime`, `redb`, `object_store`, `criterion`. Each is at 1.0+ or close to it, actively maintained, Apache 2.0 / MIT licensed, and used in production by other vendor-neutral CNCF-adjacent projects.

### Production precedent for the same shape

Three Rust-implemented telemetry agents already exist in production at scale: **Vector** (Datadog), **Quickwit**'s `quickwit-cli`, and **Grafana Alloy**'s Rust components. All ship single-binary, hit ≥30k EPS per core, and run on the same platform matrix Mara needs. The shape is proven.

### Operational considerations

- **Build times**: not a virtue, but `sccache` + `cargo-chef` Docker layer caching keep our CI workspace build to under 5 minutes from cold.
- **Compile-error feedback**: `rust-analyzer` + `clippy` are world-class. New contributors are productive in days, not weeks.
- **Refactor safety**: large refactors are tractable in Rust in a way they aren't in dynamic languages. Concrete example: the M2 `non_exhaustive` migration we made in mara-schema took 30 minutes plus tests.

## Alternatives considered

### Go — runner-up

Pros:

- Tonic / hyper equivalents (`grpc-go`, `net/http`) are mature.
- OpenTelemetry Collector is Go; precedent is overwhelming.
- Easier hiring than Rust.
- Cross-platform binary builds are simpler than Rust.

Cons (decisive):

- GC pauses introduce tail-latency jitter at high EPS. The OTel Collector community ships workarounds; we'd rather not start with the workaround.
- Memory footprint is ≥ 2× Rust at the same workload. The Vector-vs-OTel-Collector benchmarks document this.
- Generics-since-1.18 closed one gap but the type system is still less expressive than Rust's, which matters for the schema and trait surfaces.
- Concurrency safety is weaker (no `Send` / `Sync` analogue at compile time; data races compile cleanly).
- Binary size is larger; we'd lose the ≤30 MiB binary claim.

We considered Go seriously and rejected it on tail-latency and footprint grounds. If we ever build the v2 gateway as a separate codebase, Go is a credible re-evaluation point.

### Zig — too early

Pros: comparable performance to Rust; smaller learning curve; explicit allocator threading; no GC; great cross-compilation.

Cons (decisive):

- Pre-1.0 (currently 0.14.x). Breaking changes between minor versions.
- Ecosystem for telemetry (OTel, gRPC, OTLP proto) is thin to non-existent.
- Maintainer pool an order of magnitude smaller than Rust.
- Tooling immature compared to `rust-analyzer` + `clippy` + `cargo`.

Excellent language; we'd revisit in 2028+.

### C++ — rejected

Pros: maximum performance ceiling; rich ecosystem; mature.

Cons (decisive):

- Memory safety is the operator's job, not the compiler's. The CVE history of Fluent Bit and Logstash (see [`../01-landscape/06-security-and-compliance.md`](../01-landscape/06-security-and-compliance.md)) shows what happens at this surface area.
- Build complexity (CMake, vcpkg, Conan) is a contributor barrier.
- Modern C++ (C++23) closes some gaps but the toolchain churn is its own tax.

A non-starter for a v1 edge agent that runs in user-trusted contexts.

### C — rejected

Same arguments as C++, more so. Fluent Bit is excellent C but every Fluent Bit CVE is a reminder of the cost.

### Java / Kotlin (JVM) — rejected

Pros: huge ecosystem, mature OTel SDK, easy hiring.

Cons (decisive):

- JVM start-up time and warmup latency conflict with the "edge agent" form factor.
- Memory floor is 80–200 MiB even for a no-op JVM.
- GraalVM Native Image is interesting but adds toolchain complexity and ecosystem caveats.
- Logstash is the cautionary tale: same shape, JVM, hundreds of MiB RSS baseline, slow.

Rejected for "edge agent" goals; viable for backends.

### Python / Node.js / TypeScript — rejected

Pros: enormous talent pool; OTel SDKs are first-class; rapid prototyping.

Cons (decisive):

- Interpreter / runtime dependency contradicts "static binary."
- GIL (Python) or single-threaded event loop (Node) caps concurrency profile.
- Footprint and startup time both unacceptable.
- Maintenance: dependency hell, Python 2/3-style migration risks.

These are great for the application code that Mara observes, not for Mara itself.

### Elixir / Erlang — rejected

Pros: actor model maps elegantly to a fan-in / fan-out pipeline; BEAM concurrency is bulletproof; hot code swapping is magical.

Cons (decisive):

- BEAM runtime adds 50+ MiB baseline.
- Single-static-binary story is workable (releases) but not idiomatic and lags Rust's ergonomics.
- Talent pool smaller than Rust at the systems level.

Beautiful for the gateway tier, possibly. Not the edge agent.

### OCaml / Haskell — rejected

Lovely; tiny ecosystem for what we need; talent pool too small for project longevity.

## How language choice shapes the codebase

Choosing Rust forces (and enables) specific architectural patterns we lean into:

- **Typed canonical schema.** `gen_ai.*` attributes are typed structs, not free-form maps. The compiler enforces field correctness.
- **Trait-based pluggability.** Adapter / Sink / Policy are object-safe traits; new adapter ships as a crate that implements one trait.
- **Async by default.** All I/O paths are `async fn`; `tokio` task per adapter / sink.
- **`#[non_exhaustive]` on enums** for additive evolution within a major version.
- **`#[deny(unsafe_code)]` workspace lint** to enforce the safety claim.
- **Error model**: `thiserror` enums in public APIs (per ADR-0006); no `anyhow` leaking.

## When we'd revisit this decision

- If Zig 1.0 ships with a mature telemetry ecosystem.
- If GraalVM Native Image becomes the default JVM deployment and a Java OTel agent demonstrably hits <30 MiB binary, ≤128 MiB RSS, ≤50k EPS in production at a comparable shape.
- If Rust's library ecosystem fragments materially around async runtimes (e.g., Tokio and Smol incompatibility tax becomes prohibitive).

None of these are credible signals as of May 2026.

## Cross-references

- [`../01-landscape/07-rust-crate-ecosystem.md`](../01-landscape/07-rust-crate-ecosystem.md) — specific crate picks.
- [`../04-implementation/03-architecture-blocks.md`](../04-implementation/03-architecture-blocks.md) — workspace shape.
- [`../../docs/adr/0005-async-runtime.md`](../../docs/adr/0005-async-runtime.md) — Tokio decision.
- [`../../docs/adr/0006-error-model.md`](../../docs/adr/0006-error-model.md) — error-model decision.
