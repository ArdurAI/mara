# Contributing to Mara

Thanks for your interest in contributing. Mara is an AI-native telemetry shipper for AI agent runtimes, distributed under the Apache License 2.0.

## Quick start

```bash
git clone https://github.com/ArdurAI/mara.git
cd mara
cargo check --workspace
cargo test --workspace
bash scripts/captured/verify_open_verification.sh
```

The Rust toolchain is pinned in `rust-toolchain.toml`. `rustup` will install it automatically.

## Workflow

1. Open an issue describing the change before any non-trivial PR.
2. Fork, branch off `main`, keep commits focused.
3. Run `cargo fmt --all`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace` before pushing.
4. Sign your commits with `git commit -s` (Developer Certificate of Origin) — see `https://developercertificate.org`.
5. Open the PR against `main`. CI must be green before review.

## Commit and PR style

- Conventional Commits style is encouraged: `feat:`, `fix:`, `docs:`, `refactor:`, `perf:`, `test:`, `chore:`.
- One logical change per PR. Split mechanical refactors from behavioral changes.
- Include tests with every behavior change. Performance changes require a `criterion` benchmark in `benches/`.

## Architecture decisions

Material design decisions are captured as Architecture Decision Records in [`docs/adr/`](docs/adr/). If your change touches an existing decision, update the ADR or supersede it with a new one.

## Schema changes

The canonical event schema lives in `crates/mara-schema/` and is aligned with the OpenTelemetry `gen_ai.*` semantic conventions. Schema changes require:

1. An ADR or amendment in `docs/adr/`.
2. CI passes the semconv drift check.
3. Golden-file tests in `crates/mara-runtimes/<runtime>/tests/` are regenerated and reviewed.

## Security

Do **not** open public issues for security vulnerabilities. See [`SECURITY.md`](SECURITY.md).

## Code of conduct

This project follows the [Contributor Covenant](CODE_OF_CONDUCT.md).
