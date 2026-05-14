# Installing and verifying Mara

This document is the **canonical checklist** for anyone building from `main` (or any release tag). It mirrors what CI runs so you can reproduce a green build locally.

## Prerequisites

| Tool | Notes |
|------|--------|
| **Rust** | Use [rustup](https://rustup.rs/). The repo pins a toolchain in [`rust-toolchain.toml`](rust-toolchain.toml); `rustup` installs it automatically. |
| **Python 3** | For schema completeness and open-verification scripts (stdlib only). |
| **Hugo** (optional) | [Hugo Extended](https://gohugo.io/installation/) ≥ **0.120** to build the marketing site under [`website/`](website/README.md). |

## Install from source

```bash
git clone https://github.com/ArdurAI/mara.git
cd mara
cargo build --release -p mara-cli
```

The `mara` binary is at `target/release/mara`. Add it to your `PATH`, or run via `cargo run --release --bin mara -- …`.

### Example configuration

Copy [`examples/mara.toml`](examples/mara.toml) to `mara.toml`, adjust listen addresses and paths, then:

```bash
./target/release/mara validate --config mara.toml
./target/release/mara run --config mara.toml
```

## Verification (same gates as CI)

Run from the repository root, in order:

```bash
# 1. Formatting
cargo fmt --all --check

# 2. Linter (warnings denied)
cargo clippy --workspace --all-targets -- -D warnings

# 3. Unit + integration tests
cargo test --workspace

# 4. Schema completeness gate (fixture fill-rates)
python3 scripts/benchmarks/schema_completeness_gate.py

# 5. Open verification bundles (pinned SHA256)
bash scripts/captured/verify_open_verification.sh
```

**Expected:** every command exits with status **0**. The schema script prints a table (≥85% fill on required fields for qualifying runtimes). The verification script confirms `docs/captured/open-verification/SHASUMS256`.

### Optional: Hugo site

```bash
cd website
hugo --gc --minify
```

Output is `website/public/`. For local preview: `hugo server -D`.

### Optional: Ollama smoke (needs Ollama)

Short cloud smoke (see script header for environment variables):

```bash
bash scripts/realworld/ollama_cloud_smoke.sh
```

## Where results are recorded

| Artifact | Purpose |
|----------|---------|
| `cargo test` | Rust test output in your terminal; CI logs on GitHub Actions. |
| [`docs/captured/open-verification/`](docs/captured/open-verification/README.md) | Redacted JSONL + manifest for reproducible checks. |
| [`.github/workflows/ci.yml`](.github/workflows/ci.yml) | Automated runs on pushes and PRs. |

## Contributing

After the steps above pass, follow [CONTRIBUTING.md](CONTRIBUTING.md) (DCO sign-off, focused PRs).
