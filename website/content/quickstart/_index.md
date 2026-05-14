---
title: Quickstart
description: Build Mara from source and run with an example configuration.
---

## Prerequisites

- [Rust](https://www.rust-lang.org/tools/install) with `rustup` (the repo pins a toolchain in [`rust-toolchain.toml`](https://github.com/ArdurAI/mara/blob/main/rust-toolchain.toml)).

## Build

```bash
git clone https://github.com/ArdurAI/mara.git
cd mara
cargo build --release -p mara-cli
```

The `mara` binary is at `target/release/mara`.

## Configure

Copy an example from [`docs/quickstarts/`](https://github.com/ArdurAI/mara/tree/main/docs/quickstarts) or start from the checked-in sample:

- [`examples/mara.toml`](https://github.com/ArdurAI/mara/blob/main/examples/mara.toml)

Adjust `http_listen` ports, file paths, and upstream URLs for your machine.

## Run

```bash
./target/release/mara run --config /path/to/mara.toml
```

Validate configuration without starting pipelines:

```bash
./target/release/mara validate --config /path/to/mara.toml
```

## Next steps

- **Ollama + proxy smoke:** see [`scripts/realworld/ollama_cloud_smoke.sh`](https://github.com/ArdurAI/mara/blob/main/scripts/realworld/ollama_cloud_smoke.sh) (script header documents environment variables).
- **Open verification bundles:** [`docs/captured/open-verification/README.md`](https://github.com/ArdurAI/mara/blob/main/docs/captured/open-verification/README.md)

`cargo install` from crates.io is planned for post–1.0 releases; until then, build from source.
