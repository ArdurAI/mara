# MVP — Implementation Plan

## Executive summary

Six-week schedule covering two runtimes (Claude Code and Ollama), one engineer, focused work. Each week closes one of the critical gaps from [`02-gap-analysis.md`](02-gap-analysis.md) and ends with a green sign-off criterion from [`01-scope-and-decision-criteria.md`](01-scope-and-decision-criteria.md). The plan is dependency-ordered: proto codegen + proxy scaffold first, then the receiver/sender/proxy fill-in, then sinks, then glue. Each week is independently shippable — if the engineer falls off the train at week 4, the project still has more than it had at the start of MVP.

This is the document the implementing engineer follows day-to-day. The MOS milestone view in [`../04-implementation/07-phased-milestones.md`](../04-implementation/07-phased-milestones.md) is for the longer horizon.

## Pre-conditions

Before MVP week 1 starts:

- [`01-scope-and-decision-criteria.md`](01-scope-and-decision-criteria.md) sign-off — Option A (two-runtime version with Claude Code + Ollama) confirmed by the user.
- [`11-pre-mvp-user-research.md`](11-pre-mvp-user-research.md) executed: 8–12 user interviews conducted, findings synthesized into a research report, and any MVP scope adjustments from §"Decision rules" applied to this plan and to `01-scope-and-decision-criteria.md`.
- M0–M5 scaffolding present (see `git log` on the `cursor/mara-m0-bootstrap` branch).
- Workspace contains stub crates for `mara-adapter-llm-proxy` and `mara-runtime-ollama` (added during MVP planning so the workspace stays cohesive even with the new design).
- A fresh feature branch (`cursor/mara-mvp` or similar) cut from where the planning lands.
- `cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo fmt --all --check` green at branch creation.

## Week 1 — OTLP HTTP/protobuf receiver

**Goal:** accept Claude Code's OTLP HTTP traffic on `127.0.0.1:4318/v1/logs` and `127.0.0.1:4318/v1/traces` and produce canonical Mara events with `gen_ai.*` and `mcp.*` attributes intact.

**Tasks:**

1. Add dependencies: `prost = "0.14"`, `opentelemetry-proto = "0.27"` (gives us pre-generated OTLP proto types), `hyper = "1"`, `http-body-util = "0.1"`, `bytes = "1"`.
2. New module `crates/mara-adapter-otlp/src/http.rs`:
   - hyper server on a configured bind address.
   - Route `POST /v1/logs` → deserialize `opentelemetry_proto::tonic::collector::logs::v1::ExportLogsServiceRequest`.
   - Route `POST /v1/traces` → deserialize `opentelemetry_proto::tonic::collector::trace::v1::ExportTraceServiceRequest`.
   - Support `Content-Encoding: gzip` (use `flate2::read::GzDecoder`).
   - Respect `Content-Type: application/x-protobuf`.
3. New module `crates/mara-adapter-otlp/src/normalize.rs`:
   - `fn log_record_to_event(resource_attrs, scope_attrs, lr: LogRecord) -> Event`.
   - `fn span_to_event(resource_attrs, scope_attrs, span: Span) -> Vec<Event>` (a span yields zero or more events depending on whether `gen_ai.*` attributes are present).
   - Honour ZDR-stability opt-in: read `OTEL_SEMCONV_STABILITY_OPT_IN` env at startup.
4. Wire the adapter into `mara-cli::run`: when config has `[[adapters.otlp]]`, instantiate the receiver.
5. Tests:
   - Decode a stored protobuf payload (`tests/fixtures/otlp/claude-code-completion.binpb`) and assert canonical event shape.
   - End-to-end: HTTP POST → event arrives on the adapter's sender channel.

**Exit:** `cargo test -p mara-adapter-otlp` passes ≥ 6 new tests; manual smoke (`curl -X POST -H 'content-type: application/x-protobuf' --data-binary @claude-code.binpb http://127.0.0.1:4318/v1/logs`) emits a canonical event to a configured stdout sink.

**Sign-off criteria advanced:** SC-2 (OTLP round-trip) partial — receive half done.

## Week 2 — OTLP HTTP/protobuf sender

**Goal:** ship canonical events out as OTLP HTTP to any OTLP-compatible backend.

**Tasks:**

1. New module `crates/mara-sink-otlp/src/http.rs`:
   - `reqwest::Client` with `rustls-tls` and `gzip` features.
   - Build `ExportLogsServiceRequest` from a batch of canonical events.
   - Send to configured endpoint (e.g., `https://api.honeycomb.io/v1/logs`).
   - Honour configured headers (e.g., `x-honeycomb-team: ...`).
   - Retry with exponential backoff and jitter on 429 / 5xx; honour `Retry-After`.
   - Bounded in-flight batches; back-pressure on send-channel fill.
2. Config schema additions in `mara-core::config::Sinks`: `[[sinks.otlp]]` with `endpoint`, `protocol` (always `http` at MVP), `headers`, `compression`, `batch_size`, `batch_max_age`, `retry_*`.
3. Wire into `mara-cli::run` builder.
4. Tests:
   - `wiremock`-based test: POST events through sink → assert mock receiver got expected protobuf body.
   - Retry behaviour: mock returns 429 twice then 200; assert eventual success and metric `mara_sink_retries_total` is 2.
   - Auth header passes through.

**Exit:** `cargo test -p mara-sink-otlp` passes ≥ 5 new tests; full e2e test added to `tests/e2e_pipeline.rs` (OTLP receive → policy → OTLP send to wiremock).

**Sign-off criteria advanced:** SC-2 (OTLP round-trip) green.

## Week 3 — Loki HTTP push sink + AI redactor finalize + cost compute

**Goal:** ship to Grafana Loki; complete AI-specific redaction; compute cost.

**Tasks:**

1. **`mara-sink-loki`** implementation:
   - `POST /loki/api/v1/push` with `Content-Type: application/x-protobuf` and `Content-Encoding: snappy` (Loki standard) or JSON fallback.
   - Build streams keyed by labels = `{runtime, event_kind}` only. All other attributes go into structured metadata (Loki 3.x feature) or the line body.
   - Bounded label cardinality: reject configurations with label sets that could explode.
   - Basic auth / bearer auth.
2. **AI redactor finalize**:
   - Expand `builtin_pii` pack with explicit `anthropic-key` (`sk-ant-*`), `openai-key` (`sk-proj-*` / `sk-*`), `bedrock-key`, `gcp-sa-json` patterns; ensure the existing patterns still hit.
   - Add a `tokenize` mode (deterministic HMAC-of-content replacement) to enable cross-event correlation without leaking the secret.
3. **Cost compute**:
   - New module `crates/mara-policy/src/builtin/cost.rs` with a `CostComputer` Policy.
   - Built-in price table for `claude-sonnet-4-5`, `claude-opus-4`, `claude-haiku-4` (and one OpenAI model for safety net) keyed on `gen_ai.system` + `gen_ai.request.model`.
   - Populate `mara.cost.usd` and `mara.cost.source = "mara_estimated"` when vendor cost is absent.
4. Tests:
   - Loki sink against `grafana/loki:3.4.0` testcontainer (or wiremock if testcontainer is too heavy).
   - Synthetic Anthropic / OpenAI key roundtrip ⇒ redacted.
   - Token-only event ⇒ `mara.cost.usd` computed.

**Exit:** `cargo test --workspace` passes ≥ 8 new tests; quickstart now has working Grafana Cloud Loki + cost dashboard.

**Sign-off criteria advanced:** SC-3 (redaction works), SC-4 (cost computed), SC-6 (Loki sink works).

## Week 4 — HTTP proxy adapter (the Ollama foundation)

**Goal:** ship the generic `mara-adapter-llm-proxy` crate so any HTTP-based AI upstream (Ollama at MVP; OpenAI-compat servers post-MVP) can be observed.

**Tasks:**

1. **Proxy adapter implementation** in `crates/mara-adapter-llm-proxy/`:
   - hyper server binding a configured listen address.
   - hyper-util client forwarding to a configured upstream.
   - Request body capture with bounded buffer (default 10 MiB; `mara.body.truncated` flag on overflow).
   - Response body capture for unary responses.
   - SSE streaming response capture (parse `data:` chunks, buffer entire stream, emit canonical event on final chunk while forwarding chunks to client in real time).
   - Pass-through headers (including auth); no inspection or mutation.
   - Pass-through status codes faithfully.
   - Per-request synthetic session id (UUID v7).
2. **Normalizer trait** in `mara-adapter-llm-proxy`:
   - `pub trait UpstreamNormalizer: Send + Sync { fn normalize(&self, req: ProxiedRequest, resp: ProxiedResponse) -> Vec<Event>; }`.
   - Runtime-supplied. Ollama implementation comes in Week 5.
3. **Failure-mode handling** per [`12-ollama-integration-design.md`](12-ollama-integration-design.md) §"Failure modes":
   - Upstream connection refused → 502 to client + error event.
   - Upstream 5xx → forwarded status + error event.
   - Client disconnect mid-stream → partial event with `mara.ollama.partial = true`.
4. **Tests:**
   - Bind localhost; forward to a hyper-test upstream; assert request/response captured and canonical event emitted via mock normalizer.
   - SSE stream test: chunks flow to client in real time; final canonical event includes full body.
   - Client disconnect test.
   - 502 test against unreachable upstream.
   - Pass-through fidelity: response checksums match upstream output.

**Exit:** `cargo test -p mara-adapter-llm-proxy` passes ≥ 7 new tests; smoke test against a hyper-based stub server returning fake Ollama-shaped JSON works end-to-end with a no-op normalizer.

**Sign-off criteria advanced:** none yet (proxy adapter is infrastructure for Ollama week).

## Week 5 — Ollama runtime preset + glue (`mara setup`, self-telemetry, `mara diag`)

**Goal:** finish the Ollama runtime + close the user-experience loop for both runtimes.

**Tasks:**

1. **Ollama normalizer** in `crates/mara-runtime-ollama/`:
   - Implement `UpstreamNormalizer` for each Ollama endpoint shape per [`12-ollama-integration-design.md`](12-ollama-integration-design.md) §"What we capture per request".
   - Native `/api/chat`, `/api/generate`, `/api/embed`; OpenAI-compat `/v1/chat/completions`, `/v1/completions`, `/v1/embeddings`.
   - Token mapping: `prompt_eval_count` → `gen_ai.usage.input_tokens`; `eval_count` → `gen_ai.usage.output_tokens`.
   - Latency mapping: divide all `*_duration` nanoseconds by 1_000_000 to milliseconds; populate `mara.ollama.*_ms` fields.
   - `mara.cost.usd = 0`, `mara.cost.source = "local_inference"`, `mara.compute.is_local = true`.
   - `mara.ollama.tokens_per_sec` computed as `eval_count / (eval_duration / 1_000_000_000)`.
2. **`mara setup claude-code`** and **`mara setup ollama`**:
   - Detect OS (`std::env::consts::OS`).
   - Resolve target config path (`~/Library/Application Support/mara/mara.toml` on macOS, `~/.config/mara/mara.toml` on Linux, etc.).
   - Write a runnable config from a baked-in template.
   - For Claude Code: print env-var setup (`CLAUDE_CODE_ENABLE_TELEMETRY=1`, `OTEL_EXPORTER_OTLP_PROTOCOL=http/protobuf`, `OTEL_EXPORTER_OTLP_ENDPOINT=http://127.0.0.1:4318`).
   - For Ollama: print `launchctl setenv OLLAMA_HOST 127.0.0.1:11435` (macOS) or systemd override snippet (Linux), then `brew services restart ollama` / `systemctl --user restart ollama`.
   - Both idempotent: refuse to overwrite without `--force`.
3. **Self-telemetry endpoint** in `mara-core::self_telemetry`:
   - hyper server on `127.0.0.1:9099`.
   - `GET /metrics`: Prometheus exposition with per-adapter labels (including `adapter="ollama-proxy"`).
   - `GET /healthz`: 200/503 health roll-up.
4. **`mara diag`**:
   - If a running Mara is reachable on `:9099`, scrape `/metrics` and `/healthz` and pretty-print.
   - If no running instance, inspect config and state dir.
   - `--output json` for machine consumption.
5. **Tests:**
   - Ollama golden test: fixture of a real Ollama `/api/chat` response → canonical event with correct token / latency mapping.
   - Ollama OpenAI-compat golden test against `/v1/chat/completions` shape.
   - Both `mara setup` variants write valid configs that `mara validate` accepts.
   - Self-telemetry endpoint serves valid Prometheus output with both adapter labels.

**Exit:** five-minute test passes locally and in CI for Ollama path; Ollama smoke E2E test green.

**Sign-off criteria advanced:** SC-1 (five-minute test for both runtimes), SC-2-Ollama (proxy round-trip), SC-4 (cost = 0 for Ollama), SC-5 (self-observability with adapter labels), SC-7 (zero phone-home re-verified), SC-9 (proxy transparency).

## Week 6 — Quickstart hardening, bench, polish, release

**Goal:** make MVP shippable as `v0.2.0-alpha`.

**Tasks:**

1. **Quickstart hardening**:
   - Rewrite [`../07-quickstarts/01-claude-code.md`](../07-quickstarts/01-claude-code.md) and [`../07-quickstarts/07-ollama.md`](../07-quickstarts/07-ollama.md) to be copy-paste-verbatim flows.
   - `tests/quickstart_claude_code.rs` and `tests/quickstart_ollama.rs`: scripted tests that run the CLI sequences in a clean tempdir and assert artifacts.
2. **Performance smoke**:
   - `benches/pipeline_smoke.rs` with `criterion`: simulate 10k events / second for 60 seconds with the full MVP path (OTLP-in → policy → OTLP-out to a noop sink).
   - `benches/proxy_smoke.rs`: simulate 1k Ollama-shaped requests/second through the proxy adapter to a stub upstream; measure pass-through latency overhead (target < 1 ms p99 over loopback).
   - PR gate in CI: throughput ≥ 8k EPS, RSS ≤ 384 MiB, proxy overhead p99 ≤ 1 ms, no panics.
3. **Release pipeline dry-run**:
   - Tag `v0.2.0-alpha` on the MVP branch.
   - Observe the release workflow produce: macOS arm64 + Linux x86_64 + Linux musl binaries; SBOMs; cosign signatures; SLSA L2 provenance; container image.
   - Verify Homebrew formula renders correctly.
4. **Documentation polish**:
   - Update `README.md` to reflect MVP capabilities for both runtimes.
   - Update `CHANGELOG.md` with the MVP delta.
   - Update `docs/compat-matrix.md`: Claude Code and Ollama move from "plan" status to "supported in v0.2.0-alpha"; the other five runtimes remain marked as "scaffold."
5. **Persona acceptance**:
   - Find at least 2 external users — one Claude Code user, one Ollama user (ideally the same person uses both).
   - Have them run both quickstarts on their machines.
   - Capture friction points; file as issues; address blockers before tagging `v0.2.0-alpha`.

**Exit:** `v0.2.0-alpha` tagged, signed, downloadable, and the persona acceptance test passed for both runtimes.

**Sign-off criteria advanced:** SC-3 (redaction works for both), SC-6 (Loki sink works with both runtime labels), SC-8 (signed release); MVP done.

## What MVP+1 looks like (preview, not in scope here)

So the engineer knows where MVP work is going next without scope creeping into MVP itself:

- gRPC OTLP receiver (`/v1/logs`, `/v1/traces` over tonic on `:4317`).
- Splunk HEC sink.
- Codex runtime preset activated (Claude Code-shape, OTLP receive).
- Gemini runtime preset activated (Claude Code-shape, OTLP receive).
- Hooks adapter + `mara cursor-hook` for Cursor.
- The `llm-proxy` adapter generalized to OpenAI-direct, Anthropic-direct, and Bedrock upstreams (re-using the Ollama proxy infrastructure).
- Ollama-specific extensions: `/v1/responses` endpoint capture, GPU memory correlation via `/api/ps` polling, watt-hour cost estimation for local inference.
- `mara test pipeline` and `mara dlq` subcommands.
- Windows packaging.

## What MVP+2 looks like

- Kimi and Augment runtimes activated.
- Object store (S3 / GCS / Azure Blob) sink with Parquet output.
- PHI and PCI redaction packs.
- Kafka sink.
- Bench upgrade: 50k EPS for 1h on a self-hosted runner.

## Day-to-day workflow

- Trunk-based on `cursor/mara-mvp` until MVP closes; then merge back to `main` via PR.
- One pull request per task, reviewed; no batched mega-PRs.
- PR template insists on `cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo fmt --all --check` green.
- `cargo bench --bench pipeline_smoke -- --short` runs on every PR after week 5 lands.
- Conventional commit messages; release notes generated from PR titles.

## Cross-references

- [`01-scope-and-decision-criteria.md`](01-scope-and-decision-criteria.md) — what we're building, what we're not.
- [`02-gap-analysis.md`](02-gap-analysis.md) — gaps closed by this plan.
- [`07-test-and-improve-loop.md`](07-test-and-improve-loop.md) — how we iterate.
- [`08-risk-register.md`](08-risk-register.md) — what can go wrong.
- [`../04-implementation/07-phased-milestones.md`](../04-implementation/07-phased-milestones.md) — overall MOS view.
