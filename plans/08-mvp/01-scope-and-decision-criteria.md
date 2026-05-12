# MVP — Scope and Decision Criteria

## Executive summary

The Mara MVP is **"two real paths, end-to-end."** A developer running **Claude Code** (cloud LLM via Anthropic) and **Ollama** (local LLM, fully on-device) on a laptop installs `mara`, runs the per-runtime setup command, and within five minutes sees both classes of AI activity in a Honeycomb / Grafana Cloud / Loki dashboard with cost tracking populated, Anthropic/OpenAI API keys redacted before any byte leaves the machine, and local-inference token-per-second metrics from Ollama side-by-side with cloud-inference token usage from Claude Code. Two runtimes was deliberately chosen because Claude Code represents the "cloud AI via OTLP" pattern (Tier A) and Ollama represents the "local AI via HTTP proxy" pattern (the same shape we'll reuse for OpenAI-compat proxies post-MVP). Nothing else is in MVP scope. This document defines the MVP's boundaries, sign-off criteria, and the three follow-on scopes (B and C) that extend it.

## MVP target (Option A — "Two real paths")

Persona: **Rohan, the indie / startup developer** from [`../03-value-proposition/04-target-personas.md`](../03-value-proposition/04-target-personas.md). Refined for MVP based on the two runtimes:

- Runs **Claude Code** on macOS (Apple Silicon) or Linux x86_64 for cloud-LLM-assisted development.
- Runs **Ollama** on the same machine for local-LLM use cases (cost-conscious, privacy-conscious, or offline scenarios).
- Already pays for Honeycomb, Grafana Cloud, or Logfire — or wants to use a local Docker Compose stack with Loki + Grafana.
- Has five minutes and wants to see both cloud and local AI activity in a single dashboard.

## In scope (MVP)

1. **OTLP HTTP/protobuf receiver** on `127.0.0.1:4318` accepting log records that Claude Code (and any OTel-compatible source) emits when `CLAUDE_CODE_ENABLE_TELEMETRY=1` and `OTEL_EXPORTER_OTLP_PROTOCOL=http/protobuf`. gRPC receiver follows in MVP+1.
2. **OTLP HTTP/protobuf sender** that maps Mara canonical events back to OTLP and POSTs them with retry, gzip, and bearer-token auth (Honeycomb header style, also works for Logfire / Grafana Cloud OTLP).
3. **Loki HTTP push sink** for operators using the Grafana stack. Streams keyed by `runtime` and `event_kind` labels with the rest as structured metadata to avoid cardinality blowups.
4. **HTTP proxy adapter (`mara-adapter-llm-proxy`)** that binds on `127.0.0.1:11434`, forwards to Ollama on `127.0.0.1:11435`, and captures every request/response pair without payload mutation. Generic enough to handle any OpenAI-compat upstream post-MVP. Detailed design in [`12-ollama-integration-design.md`](12-ollama-integration-design.md).
5. **Ollama runtime preset (`mara-runtime-ollama`)** mapping native `/api/chat`, `/api/generate`, `/api/embed` and OpenAI-compat `/v1/chat/completions`, `/v1/completions`, `/v1/embeddings` request/response shapes into canonical `gen_ai.*` + `mara.ollama.*` events. Token counts from `prompt_eval_count` / `eval_count`. Latency from `*_duration` fields. Cost = 0 with `mara.cost.source = "local_inference"`.
6. **Built-in `gen_ai.*` redaction pack** extension: keep the nine patterns we already have; add `anthropic-key`, `openai-key`, `bedrock-key`, `gcp-sa-json` validators. (Already mostly done in `mara-policy::builtin::redact`.)
7. **`mara setup claude-code`** writes a real, runnable config to the OS-appropriate location and prints next steps (set the OTel env vars, restart Claude Code).
8. **`mara setup ollama`** writes a real, runnable config; prints instructions to set `OLLAMA_HOST=127.0.0.1:11435` and restart Ollama so Mara takes over the default port.
9. **Self-telemetry endpoint** at `127.0.0.1:9099`: `/metrics` (Prometheus exposition format) and `/healthz` (200/503).
10. **`mara diag`** prints a human-readable view of adapter, policy, sink, and channel-depth state.
11. **End-to-end smoke tests** in CI for both runtimes: synthetic Claude Code OTLP → Mara → wiremocked OTLP backend; synthetic Ollama HTTP traffic through proxy → Mara → wiremocked OTLP backend. Both assert all `gen_ai.*` attributes round-tripped and all redactions applied.
12. **Per-runtime quickstarts** ([`../07-quickstarts/01-claude-code.md`](../07-quickstarts/01-claude-code.md) and [`../07-quickstarts/07-ollama.md`](../07-quickstarts/07-ollama.md)) updated to be runnable verbatim and tested by CI.
13. **macOS Homebrew tap** + **Linux deb/rpm** publishing wired into the release workflow with a real signed tag (`v0.2.0-alpha`).

## Explicitly out of MVP scope

Tracked as immediate post-MVP, not deleted:

- gRPC OTLP receiver (4317).
- The other five runtimes (Codex, Cursor, Kimi, Augment, Gemini). Their preset crates stay; they're activated in MVP+1.
- All other sinks: Splunk HEC, Elasticsearch, S3/GCS/Azure, Kafka, Prom RW, generic webhook.
- WASM-hosted policy bundles. (Built-in primitives suffice for MVP.)
- Segmented WAL. (In-memory bounded mpsc only; sink outage = drops with a metric.)
- Hooks adapter, analytics REST adapter.
- Windows packaging. (Linux + macOS only at MVP; Windows in MVP+2.)
- SOC 2 Type I audit kick-off, CNCF Sandbox submission, EU AI Act formal alignment doc.
- Performance bench beyond a 60-second smoke at 10k EPS.

## Sign-off criteria for MVP

Every item below is testable. The MVP is "done" when all of them are green for two consecutive nightly CI runs.

- **SC-1 — Five-minute test.** A fresh macOS or Linux user runs each quickstart copy-paste and produces ≥1 event from each runtime (Claude Code and Ollama) in their chosen sink within 5 minutes of starting. Measured by an integration test that simulates the steps.
- **SC-2 — OTLP round-trip (Claude Code).** A representative Claude Code OTLP payload survives ingest → policy → export with all `gen_ai.*` attributes preserved; goldens reviewed.
- **SC-2-Ollama — Proxy round-trip (Ollama).** A representative Ollama native + OpenAI-compat request/response flows through the proxy adapter producing canonical events with `gen_ai.usage.input_tokens` / `output_tokens` from `prompt_eval_count` / `eval_count`, all `mara.ollama.*` latency fields populated, and `mara.cost.source = "local_inference"`. The proxied response back to the client is byte-identical to what Ollama emitted (verified by checksum).
- **SC-3 — Redaction works.** Synthetic Anthropic and OpenAI keys appearing anywhere in either runtime's payload are replaced with `[anthropic-key]` / `[openai-key]` before sink dispatch. Verified by integration test for both runtimes.
- **SC-4 — Cost computed.** For Claude Code: when token usage is present but vendor-emitted cost is not, `mara.cost.usd` is populated from a built-in price table. For Ollama: `mara.cost.usd = 0`, `mara.cost.source = "local_inference"`, and `mara.compute.is_local = true` are populated correctly.
- **SC-5 — Self-observability.** `curl 127.0.0.1:9099/metrics` returns Prometheus exposition with at least `mara_pipeline_events_total`, `mara_sink_errors_total`, `mara_policy_decisions_total`, plus per-adapter labels including `adapter="ollama-proxy"` and `adapter="otlp-http"`. `/healthz` returns 200.
- **SC-6 — Loki sink works.** A 10k-event run into a local Loki container produces no cardinality-related errors. Events are queryable by `{runtime="claude_code"}` and `{runtime="ollama"}`.
- **SC-7 — Zero phone-home.** With Mara running and no sink configured, `tcpdump` over a 60-second window shows zero outbound packets to non-loopback addresses. With the Ollama proxy running, traffic only flows between `127.0.0.1:11434` (Mara) and `127.0.0.1:11435` (Ollama).
- **SC-8 — Signed release.** A `v0.2.0-alpha` tag produces signed artifacts with SBOM, cosign signature, SLSA provenance attached on GitHub Releases.
- **SC-9 — Proxy transparency.** Ollama clients (the `ollama` CLI, Open WebUI if available in CI, an OpenAI-SDK script) operate against Mara on `:11434` identically to operating against Ollama directly. No client-visible behavioural change other than the configured port move.

## Decision matrix (Options A, B, C)

Three scopes to choose from. Recommend Option A; document B and C as follow-on scopes so we know what we're explicitly deferring.

### Option A — two real paths (recommended, ~6 weeks)

- **Claude Code** via OTLP HTTP receive.
- **Ollama** via HTTP proxy adapter (`mara-adapter-llm-proxy`).
- OTLP HTTP sender + Loki HTTP push as sinks.
- macOS + Linux packaging.
- This document's in-scope list (items 1–13).

Two runtimes was chosen over one because they exercise the two integration shapes Mara will reuse for the rest of v1 (OTLP-receive and proxy-capture). One alone would leave us guessing about the second shape.

### Option B — two tiers, two runtimes (~10 weeks)

Everything in A plus:

- gRPC OTLP receiver.
- Hooks adapter + `mara cursor-hook` glue subcommand.
- Cursor runtime preset activated end-to-end.
- Splunk HEC sink.
- Windows packaging.

### Option C — honest v1.0-rc.1 (~20 weeks, matches the MOS plan)

Everything in B plus:

- Remaining four runtimes: Codex, Kimi, Augment, Gemini.
- All remaining sinks: Elasticsearch, S3/GCS/Azure, Kafka, Prom RW, webhook.
- Segmented WAL per ADR-0003.
- WASM-hosted signed policy bundles per ADR-0002.
- Tamper-evident audit log.
- Full performance bench harness (50k EPS for 1 h).
- SOC 2 Type I control mapping exercised, CNCF Sandbox application drafted.

## Why Option A and not directly Option C

1. **Validation before investment.** Building out C without ever shipping A risks shipping something nobody wants. The MVP smoke-tests the value claim ("AI-native shipper") with a single real user before we spend months on every runtime and sink.
2. **Smallest correct scope.** Two months of work eliminate the gap between "the README is aspirational" and "the README is true for one path."
3. **Forces the right primitives.** OTLP HTTP receive + send + Loki push are the three protocols every other sink will lean on (or contrast with). Getting these right de-risks B and C.
4. **Persona priority.** Persona 2 (Rohan, indie) is the v1 primary; Option A serves only that persona. Persona 1 (platform engineer) is well-served by B. Persona 3 (compliance) needs C.

## Confirmation needed

Confirm Option A as the MVP scope. If A is wrong, the rest of this section needs revision.

If A is confirmed, [`06-mvp-implementation-plan.md`](06-mvp-implementation-plan.md) defines the week-by-week execution.

## Cross-references

- [`02-gap-analysis.md`](02-gap-analysis.md) — what we have vs need.
- [`03-language-choice.md`](03-language-choice.md) — why Rust.
- [`04-ai-native-features.md`](04-ai-native-features.md) — AI-specific behaviour shipping in MVP.
- [`05-problem-statement.md`](05-problem-statement.md) — what we're solving and for whom.
- [`06-mvp-implementation-plan.md`](06-mvp-implementation-plan.md) — concrete week-by-week.
- [`07-test-and-improve-loop.md`](07-test-and-improve-loop.md) — the iteration loop.
- [`08-risk-register.md`](08-risk-register.md) — MVP-specific risks.
- [`09-differentiation-and-moat.md`](09-differentiation-and-moat.md) — what makes Mara not a fork.
- [`10-owasp-alignment.md`](10-owasp-alignment.md) — security overlay.
- [`11-pre-mvp-user-research.md`](11-pre-mvp-user-research.md) — talk to users before week 1.
- [`12-ollama-integration-design.md`](12-ollama-integration-design.md) — detailed Ollama proxy design.
