# Mara

**Mara** is an AI-native log shipper and telemetry agent for AI agents and LLM workloads — written in Rust, aligned with the OpenTelemetry `gen_ai.*` semantic conventions, and licensed Apache 2.0.

Think Fluent Bit for AI workloads: a small, fast, edge-deployable binary that collects, normalizes, governs, and ships AI-runtime telemetry to whatever observability backend you already use.

## Status

Pre-1.0. **`main` is expected to stay green** with the checks in [INSTALL.md](INSTALL.md) (format, clippy, tests, schema gate, open-verification SHA256). Feature work may land on `dev` first; releases and default clone experience track **`main`**. Roadmap detail: [MOS plan](plans/mara_mos_plan_d0da16c1.plan.md).

## What Mara captures

Tier A (native OpenTelemetry receive): **Claude Code**, **Codex**, **Gemini CLI**.

Tier B (hooks / JSONL tail): **Cursor Agents**, **Kimi** — plus Claude Code and Codex as redundant signal.

Tier C (analytics REST best-effort): **Augment Code**.

Proxy tier (HTTP request/response capture for runtimes without native telemetry): **Ollama** (local LLM runtime).

**First-class paths on `main` today:** OTLP HTTP/protobuf (+ optional gRPC when configured), LLM HTTP proxy in front of Ollama/OpenAI-compatible servers, JSONL tail, file/stdout sinks, policy chain (redact, privacy, sample, deny), self-metrics (`/metrics`, `/healthz`, `/readyz`). Tier B/C adapters (hooks HTTP ingest, analytics polling) are configurable from TOML; see [docs/compat-matrix.md](docs/compat-matrix.md) and [the MVP plan](plans/08-mvp/01-scope-and-decision-criteria.md) for depth per runtime.

See the [compatibility matrix specification](plans/05-evaluation/02-compatibility-matrix-spec.md) for details.

## Where Mara ships

OpenTelemetry Protocol (gRPC + HTTP), Grafana Loki, Splunk HEC, Elasticsearch / OpenSearch, S3 / GCS / Azure Blob (JSONL + Parquet), Kafka, Prometheus Remote Write, local files, stdout, generic webhook.

## Design pillars

- Edge-first single-binary Rust agent (v1). Optional self-hostable gateway (v2). Optional hosted control plane (v3).
- OpenTelemetry `gen_ai.*` as the canonical schema. No invented parallel namespace.
- WASM-sandboxed policy stage with built-in PII/PHI/PCI redaction packs and signed policy bundles.
- Zero phone-home. Prompt and raw-body capture opt-in everywhere. ZDR-respecting per runtime.
- Apache 2.0 with a CNCF Sandbox track.

## Quickstart (per AI runtime)

- [Claude Code](plans/07-quickstarts/01-claude-code.md) — MVP target
- [Codex](plans/07-quickstarts/02-codex.md)
- [Cursor Agents](plans/07-quickstarts/03-cursor.md)
- [Kimi](plans/07-quickstarts/04-kimi.md)
- [Augment Code](plans/07-quickstarts/05-augment.md)
- [Gemini CLI](plans/07-quickstarts/06-gemini-cli.md)
- [Ollama (local LLMs)](plans/07-quickstarts/07-ollama.md) — MVP target

## Deployment blueprints

- [macOS launchd](plans/06-deployment-blueprints/01-macos-launchd.md)
- [Linux systemd](plans/06-deployment-blueprints/02-linux-systemd.md)
- [Windows Service](plans/06-deployment-blueprints/03-windows-service.md)
- [Kubernetes DaemonSet](plans/06-deployment-blueprints/04-kubernetes-daemonset.md)
- [Kubernetes sidecar](plans/06-deployment-blueprints/05-kubernetes-sidecar.md)
- [AWS Lambda Extension and serverless](plans/06-deployment-blueprints/06-serverless-lambda-extension.md)
- [Docker Compose](plans/06-deployment-blueprints/07-docker-compose.md)
- [CI runners (GitHub Actions, GitLab, Buildkite)](plans/06-deployment-blueprints/08-ci-runners.md)

## Repository layout

```
.
├── crates/                         # Rust workspace members
│   ├── mara-core/                  # pipeline orchestration
│   ├── mara-schema/                # canonical event schema
│   ├── mara-policy/                # WASM-sandboxed policy
│   ├── mara-adapter-{otlp,jsonl,hooks,analytics}/
│   ├── mara-runtime-{claude-code,codex,cursor,kimi,augment,gemini}/
│   ├── mara-sink-{otlp,loki,splunk-hec,elasticsearch,object-store,kafka,prom-rw,file,webhook}/
│   ├── mara-cli/                   # the `mara` binary
│   └── mara-gateway/               # v2 placeholder
├── xtask/                          # internal codegen + release tooling
├── plans/                          # planning encyclopedia (48 documents)
│   ├── 00-overview/                # mission, non-goals, glossary
│   ├── 01-landscape/               # competitive landscape
│   ├── 02-gaps/                    # AI observability gaps
│   ├── 03-value-proposition/       # positioning + feature matrix
│   ├── 04-implementation/          # requirements + architecture + milestones
│   ├── 05-evaluation/              # benchmarks + SOC 2 + EU AI Act mapping
│   ├── 06-deployment-blueprints/   # per-platform install patterns
│   └── 07-quickstarts/             # per-runtime onboarding
├── docs/                           # engineering reference (ADRs, runbooks)
├── website/                        # Hugo static site (project homepage)
├── INSTALL.md                      # install + verification checklist for main
└── .github/                        # CI and governance config
```

## Project website

A **Hugo** static site (home, docs hub, community, quickstart) lives in [`website/`](website/). Preview locally:

```bash
cd website && hugo server -D
```

See [`website/README.md`](website/README.md) for theme notes, production build, and CI. Set `baseURL` in `website/hugo.toml` before deploying.

## Install and verify

See **[INSTALL.md](INSTALL.md)** for a full install-from-source walkthrough and the exact command list used to validate `main` (including schema completeness and open-verification bundles).

Quick check:

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

The Rust toolchain is pinned via `rust-toolchain.toml`. `rustup` installs the right version on first use.

Optional live check with **Ollama Cloud** models (local Ollama daemon + signed-in cloud + pulled `*-cloud` model): `bash scripts/realworld/ollama_cloud_smoke.sh` (see the script header for `MODEL`, `PROXY_PORT`, and other env vars).

**Longer varied workload (≥15 min default):** `python3 scripts/realworld/ollama_cloud_varied_workload.py` — rotates generate/chat, OpenAI-compat chat, a real HTTP fetch + summarize, and sequential “planner → specialist” calls; writes `results.ndjson` and serves a tiny dashboard (`--dashboard-port`, default on). Use `--duration 900` (default) or shorter for dry runs.

**Open verification (redacted JSONL + pinned SHA256):** see [`docs/captured/open-verification/README.md`](docs/captured/open-verification/README.md) and run `bash scripts/captured/verify_open_verification.sh` from the repo root.

## Try the CLI surface

```bash
cargo run --bin mara -- --help
cargo run --bin mara -- version
```

The CLI implements **`mara run`**, **`mara validate`**, **`mara version`**, and **`mara setup`** (presets); behavior is driven by `mara.toml`. See [phased milestones](plans/04-implementation/07-phased-milestones.md) for history and upcoming work.

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md). All contributions follow the [Apache 2.0 license](LICENSE), require DCO sign-off (`git commit -s`), and pass the CI gates above.

## Security

See [SECURITY.md](SECURITY.md). Report vulnerabilities privately to `security@ardurai.dev`.

## License

Apache License 2.0. See [LICENSE](LICENSE) and [NOTICE](NOTICE).

Mara is inspired at the pattern level by Fluentd, Fluent Bit, OpenTelemetry Collector, Grafana Loki, and Splunk Universal Forwarder / HEC. No source code from these projects is vendored.

---

Mara is a project of [ArdurAI](https://ardurai.dev).
