# ADR-0006: Thiserror enums in public APIs; anyhow only inside binaries

- **Status:** Accepted
- **Date:** 2026-05-12
- **Authors:** Mara M1 architecture review.

## Context

Public APIs need stable, programmatically inspectable error types so downstream consumers can react appropriately. Internal application code benefits from ergonomic error propagation. The Rust ecosystem has converged on two patterns: `thiserror` for library / public errors, `anyhow` for application / internal errors. Picking one consistently in each context avoids surprises.

## Decision

- **Public APIs in library crates** (`mara-core`, `mara-schema`, `mara-policy`, all adapter and sink crates) return concrete error enums derived with `thiserror`. Each crate defines a `pub enum Error` in `error.rs` and a `pub type Result<T> = std::result::Result<T, Error>;`.
- **Errors are `#[non_exhaustive]`** so adding new variants does not break SemVer.
- **Errors include source chains** via `#[from]` and `#[source]` attributes.
- **Binary crates** (`mara-cli`, `mara-gateway`, `xtask`) may use `anyhow::Error` for top-level error handling.
- **Tests** may use `anyhow::Error` freely.
- **Public traits** like `Adapter::start` and `Sink::start` return `Result<(), CrateError>` where `CrateError` is the crate's own enum, kept narrow.
- **Cross-crate error mapping** uses explicit `From` impls to convert between adapter/sink errors and `mara-core::Error`.

## Alternatives considered

- **`anyhow::Error` everywhere.** Pros: ergonomic. Cons: downstream cannot match on variants; opaque source chain; weak SemVer signal. Rejected for public APIs.
- **`Box<dyn std::error::Error>` everywhere.** Pros: even simpler. Cons: same problems as anyhow plus no `Send`/`Sync` constraints by default. Rejected.
- **`color_eyre` / `miette`.** Pros: prettier diagnostics. Cons: heavier dependencies; we already get good messages from `thiserror` + structured logging. Decision: not in core; `miette` can be a CLI-side enhancement for `mara validate` in v1.x.
- **Custom error trait per crate.** Pros: maximum flexibility. Cons: yet another foreign trait for downstream. Rejected.

## Consequences

- Every crate has an `error.rs` module with a single `pub enum Error`.
- `Error` variants include enough context to render a useful message without a debugger (e.g., `IoError { path, source }` rather than just `Io(io::Error)`).
- The CLI translates crate-specific errors into a unified user-facing presentation; `anyhow::Context::context` is used liberally inside CLI code.
- Lints encourage this discipline: `clippy::must_use_candidate` is allowed (errors are values to propagate), and `missing_errors_doc` is enforced on public APIs.

## References

- thiserror: <https://crates.io/crates/thiserror>.
- anyhow: <https://crates.io/crates/anyhow>.
- [Rust error handling design from the API guidelines](https://rust-lang.github.io/api-guidelines/).
- [`plans/04-implementation/03-architecture-blocks.md`](../../plans/04-implementation/03-architecture-blocks.md).
