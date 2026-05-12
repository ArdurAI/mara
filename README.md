# Mara

**Mara** is an AI-native log shipper and telemetry agent for AI agents and LLM workloads — written in Rust, aligned with the OpenTelemetry `gen_ai.*` semantic conventions, and licensed Apache 2.0.

Think Fluent Bit for AI workloads: a small, fast, edge-deployable binary that collects, normalizes, governs, and ships AI-runtime telemetry to whatever observability backend you already use.

## Status

Pre-1.0. Active development on the v1 milestone plan ([MOS plan](plans/mara_mos_plan_d0da16c1.plan.md)).

## What Mara captures

Tier A (native OpenTelemetry receive): **Claude Code**, **Codex**, **Gemini CLI**.

Tier B (hooks / JSONL tail): **Cursor Agents**, **Kimi** — plus Claude Code and Codex as redundant signal.

Tier C (analytics REST best-effort): **Augment Code**.

Proxy tier (HTTP request/response capture for runtimes without native telemetry): **Ollama** (local LLM runtime).

**MVP target (in active development on `cursor/mara-mvp` branch):** Claude Code (Tier A) and Ollama (Proxy) end-to-end. The other five runtimes are scaffolded and activate in MVP+1. See [the MVP plan](plans/08-mvp/01-scope-and-decision-criteria.md).

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
└── .github/                        # CI and governance config
```

## Build

```bash
cargo check --workspace
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all --check
```

The Rust toolchain is pinned via `rust-toolchain.toml`. `rustup` installs the right version on first use.

## Try the CLI surface

```bash
cargo run --bin mara -- --help
cargo run --bin mara -- version
```

In M0 the CLI is a scaffolded skeleton; real behaviors arrive in M2+ per the [phased milestones](plans/04-implementation/07-phased-milestones.md).

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md). All contributions follow the [Apache 2.0 license](LICENSE), require DCO sign-off (`git commit -s`), and pass the CI gates above.

## Security

See [SECURITY.md](SECURITY.md). Report vulnerabilities privately to `security@ardurai.dev`.

## License

Apache License 2.0. See [LICENSE](LICENSE) and [NOTICE](NOTICE).

Mara is inspired at the pattern level by Fluentd, Fluent Bit, OpenTelemetry Collector, Grafana Loki, and Splunk Universal Forwarder / HEC. No source code from these projects is vendored.

---

Mara is a project of [ArdurAI](https://ardurai.dev).
