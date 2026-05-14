# Mara Milestone Board (M0/M1/M2)

This board converts recent test findings into a tracked execution plan. **M2-16+** rows capture the 2026 market-gap backlog (correlation IDs, semconv governance, OpenInference / eval exports, optional Presidio-class PII, upstream semantics, vector/GPU spike).

## Status Legend

- [ ] Not started
- [~] In progress
- [x] Done
- [!] Blocked

## Owner Key (Suggested)

- **Runtime**: runtime normalizers and schema mapping
- **Platform**: pipeline/reliability/CI plumbing
- **Observability**: metrics, dashboards, telemetry analysis
- **Security/Policy**: privacy modes, redaction, governance controls
- **Integrations**: external runtime and agent-framework onboarding/docs

## Priority Key

- **P0**: Must land to keep roadmap credibility / unblock downstream work
- **P1**: High value, should land in planned milestone window
- **P2**: Important, but can slip without blocking core adoption

## M0 — Prove Core Value + Prevent Regressions (0–2 weeks)

### Milestone Outcome

Mara reliably captures meaningful Ollama telemetry and flags regressions early.

| Ticket | Priority | Status | Task | Owner | ETA | Acceptance Criteria |
|---|---|---|---|---|---|---|
| M0-01 | P0 | [x] | Promote 30-min real-world run into a repeatable harness | Platform | 2026-05-17 | Script runs end-to-end on a clean machine and consistently outputs `events.jsonl`, `run.log`, `mara-run.log`, and `FINDINGS.md`. |
| M0-02 | P0 | [x] | Add CI smoke benchmark (3–5 min) | Platform | 2026-05-20 | CI fails when event count is zero or required field fill-rate drops below threshold. |
| M0-03 | P0 | [x] | Add field-completeness guardrail tests | Runtime | 2026-05-16 | Tests enforce non-null `request.model`, `response.model`, `operation_name`, and `usage` for known payloads. |
| M0-04 | P1 | [x] | Generate baseline telemetry quality report | Observability | 2026-05-21 | Report emits null/fill-rate by key fields and is checked in under `docs/captured/`. |
| M0-05 | P0 | [x] | Publish operator doc for null-field behavior | Runtime | 2026-05-16 | Doc explains why fields are null and how to populate/enrich them in practice. |

## M1 — Make It Operational (2–6 weeks)

### Milestone Outcome

Mara is practical as a telemetry + derived-metrics layer for real operations.

| Ticket | Priority | Status | Task | Owner | ETA | Acceptance Criteria |
|---|---|---|---|---|---|---|
| M1-01 | P0 | [x] | Add `resource.*` enrichment defaults | Runtime | 2026-05-28 | `host_name` + `process_pid` always on Ollama events; optional `service_name` / `service_version` via `[server].telemetry_*` or `MARA_SERVICE_*`. |
| M1-02 | P0 | [x] | Add conversation/turn correlation fields | Runtime | 2026-05-30 | `gen_ai.conversation_id` and `mara.turn_id` are captured from client JSON (`conversation_id` / `turn_id` / `metadata.*`) or correlation headers when present. |
| M1-03 | P0 | [x] | Add trace propagation support | Platform | 2026-06-03 | Inbound `traceparent` maps to non-null `trace_id`/`span_id` in emitted events. |
| M1-04 | P1 | [x] | Implement cost estimator v1 | Observability | 2026-06-05 | `cost_usd` is computed from model usage + pricing map with documented assumptions. |
| M1-05 | P1 | [x] | Ship derived metrics pack and starter dashboard | Observability | 2026-06-06 | Request rate, tokens, p95 latency, error rate, and cost are queryable out of the box. |
| M1-06 | P0 | [x] | Harden error taxonomy and mappings | Runtime | 2026-05-29 | `ProxyFailureKind` enum + `docs/ollama-proxy-error-taxonomy.md`; synthetic 502 `failure_kind` strings are stable and unit-tested. |
| M1-07 | P1 | [x] | Add privacy modes (metadata-only/hashed/body-opt-in) | Security/Policy | 2026-06-10 | Policy and config toggles are tested and documented for each mode. |
| M1-08 | P0 | [x] | Implement or reject-at-parse `deny` policy stage (no silent skip) | Security/Policy | 2026-06-14 | `[[policies.*]]` stages of type `deny` either enforce a deny outcome in the pipeline or `mara validate` / startup fails with a clear error; no warn-and-ignore. Optional **drop audit** to sinks: `[[pipelines]] audit_policy_drops = true` emits minimal `System` audit events (no body); see `docs/observability/mara-policy-drop-audit.md`. |
| M1-09 | P0 | [x] | LLM proxy threat model for non-loopback bind | Security/Policy + Platform | 2026-06-18 | Doc under `docs/` covers client trust boundary; defaults remain loopback-only; non-local bind requires explicit opt-in and documents upstream timeouts, body limits, connection limits, and optional auth/TLS termination pattern. |
| M1-10 | P1 | [x] | Semgrep security workflow: fail on findings or mandatory SARIF | Platform | 2026-06-20 | `.github/workflows/security.yml` no longer hides Semgrep failures without an alternate mandatory signal (for example SARIF upload + required check). |
| M1-11 | P2 | [x] | Document self-metrics latency exposition semantics | Observability | 2026-06-22 | `docs/observability/` explains how `mara_gen_ai_request_duration_ms_*` relates to PromQL `histogram_quantile` and what `_sum` / `_count` represent relative to the in-process sample window. |

## M2 — Cross-Runtime + Agent-Native Differentiation (6–12 weeks)

### Milestone Outcome

Mara is clearly differentiated through cross-runtime and agent-level observability.

| Ticket | Priority | Status | Task | Owner | ETA | Acceptance Criteria |
|---|---|---|---|---|---|---|
| M2-01 | P1 | [x] | Publish runtime parity matrix (Claude/Codex/Kimi/Cursor/Ollama) | Integrations | 2026-06-17 | Matrix includes required fields and completeness score per runtime. |
| M2-02 | P0 | [x] | Reach minimum schema completeness per runtime | Runtime | 2026-06-24 | At least 3 runtimes meet production threshold (for example >=85% required-field fill-rate). |
| M2-03 | P1 | [x] | Add agent semantics (`agent_id`, `step_id`, `tool_name`, `tool_outcome`) | Runtime | 2026-06-26 | Agent-context fields appear in end-to-end runs with tests for extraction. |
| M2-04 | P1 | [x] | Add Hermes/OpenClaw integration guide | Integrations | 2026-06-27 | Guide and sample config produce usable telemetry on a reference workflow. |
| M2-05 | P1 | [x] | Build agent run summary materializer | Observability | 2026-07-02 | Per-run summary reports steps, tokens, cost, failures, and latency hotspots. |
| M2-06 | P2 | [x] | Build cross-runtime comparative dashboard | Observability | 2026-07-03 | Single dashboard compares model/runtime latency, cost, and error rates for same workload. |
| M2-07 | P1 | [x] | Publish adoption quickstarts pack | Integrations | 2026-07-08 | Templates for Ollama-heavy, mixed-runtime, and privacy-first setups pass smoke tests. |
| M2-08 | P1 | [x] | Parallel sink fan-out (latency + fairness) | Platform | 2026-07-10 | Dispatcher fans out to N sinks with concurrent `send` (or shared `Arc<Event>`) so one slow sink does not block others; tests cover ordering where required and no duplicate side effects. |
| M2-09 | P1 | [x] | Readiness and liveness for self-telemetry HTTP | Observability + Platform | 2026-07-12 | `/healthz` remains process liveness; `/readyz` aggregates `Adapter::health` / `Sink::health` (200 only when every component is **healthy** or **degraded**; `starting` / `stopping` / `stopped` / `failed` → 503). Defaults assume ready (`Health::healthy()`); documented in `docs/observability/mara-readyz-semantics.md` and K8s probe fragment. |
| M2-10 | P2 | [x] | Reduce `/metrics` scrape CPU (latency quantiles) | Observability | 2026-07-15 | Replace or augment full-vector sort per scrape with bounded-cost quantile sketch, native histogram, or documented scrape interval guidance; criterion or bench shows regression guard. |
| M2-11 | P2 | [x] | Cost confidence metadata | Observability + Runtime | 2026-07-18 | When body truncation or missing usage breaks estimates, events carry explicit low-confidence cost metadata (schema field + docs) so dashboards and chargeback queries stay honest. |
| M2-12 | P2 | [x] | Minimal Helm / K8s operator doc | Platform + Integrations | 2026-07-22 | Example chart or `docs/` fragment: Deployment, `Service` for metrics scrape, `livenessProbe` / `readinessProbe` wiring to M2-09 endpoints; copy-paste values for `metrics_addr`. |
| M2-13 | P2 | [x] | Backpressure / saturation metrics (USE) | Observability | 2026-07-24 | **Shipped:** bounded Prometheus signals—proxy gateway latency (`mara_gen_ai_gateway_duration_ms*`), dispatcher sink fan-out wall time (`mara_pipeline_fanout_duration_ms*`), and sink `mpsc::send` failures (`mara_pipeline_sink_channel_send_errors_total`)—without high-cardinality labels; Grafana starter includes these. **Deferred:** literal channel-depth / utilization gauges (true USE “saturation”) if we later expose bounded queue stats from adapters/dispatcher. |
| M2-14 | P2 | [x] | Dual latency: gateway vs engine | Observability + Runtime | 2026-07-26 | Separate metrics or labels distinguish proxy wall-clock from `mara.ollama.total_duration_ms` (or upstream equivalent) for split SLOs. |
| M2-15 | P2 | [x] | Cap concurrent connections on metrics server | Platform | 2026-07-28 | `serve_self_metrics` path limits in-flight HTTP/1 tasks (semaphore or equivalent) when `metrics_addr` is non-loopback; documented default. |
| M2-16 | P1 | [x] | Gateway correlation ID (`x-mara-request-id` or configurable) | Platform + Runtime | 2026-07-30 | LLM proxy generates or forwards a stable per-request ID header; value appears on canonical events (and structured logs) for end-to-end tracing when clients omit `traceparent`; Prometheus remains low-cardinality (no per-request labels). |
| M2-17 | P1 | [x] | OTel GenAI semconv pin + CI drift | Platform + Runtime | 2026-08-02 | `semconv.lock` / codegen tied to a documented OTel GenAI version (e.g. v1.37+); CI fails on uncommitted drift; version bump procedure recorded in `CHANGELOG` or `docs/`. |
| M2-18 | P2 | [x] | OpenInference / Phoenix compatibility bridge | Observability + Runtime | 2026-08-05 | Doc + optional normalizer or export mapping so Arize Phoenix and OpenInference consumers can use Mara OTLP with minimal attribute translation; one golden fixture or sample pipeline config. |
| M2-19 | P2 | [x] | Optional Presidio-class PII path | Security/Policy | 2026-08-08 | Documented integration pattern (sidecar or HTTP policy hook) for Microsoft Presidio–style masking **before** upstream; latency and failure-mode notes; remains opt-in vs builtin regex redactor. |
| M2-20 | P2 | [x] | Eval-platform OTLP export profile | Observability + Integrations | 2026-08-12 | Checklist + doc for LangSmith / Phoenix / Honeycomb: which `gen_ai.*` and `mara.*` fields those tools expect for useful LLM traces; explicitly **no** in-product eval UI in Mara—export-only positioning. |
| M2-21 | P2 | [x] | Document single-upstream proxy semantics | Platform | 2026-08-14 | Operator doc states today’s **one upstream** behavior (no automatic failover); if multi-upstream is introduced later, retry/failover rules must be specified—addresses market complaints about opaque gateway routing. |
| M2-22 | P2 | [x] | Vector DB / GPU telemetry spike | Integrations | 2026-08-18 | Time-boxed spike: whether Mara gains a **separate** adapter/scraper path for vector-store and GPU counters (OpenLIT-style breadth) without bloating `llm-proxy`; output is ADR or `FINDINGS.md` recommendation, not necessarily code. |

## Sprint Buckets (Suggested)

### This Sprint

- [x] P0 M0-01 Promote 30-min real-world run into repeatable harness
- [x] P0 M0-02 Add CI smoke benchmark
- [x] P0 M0-03 Add field-completeness guardrail tests
- [x] P1 M0-04 Generate baseline telemetry quality report
- [x] P0 M0-05 Publish null-field operator doc
- [x] P0 M1-01 Add `resource.*` enrichment defaults
- [x] P0 M1-06 Harden error taxonomy and mappings

### Next Sprint

- [x] P0 M1-02 Add conversation/turn correlation fields
- [x] P0 M1-03 Add trace propagation support
- [x] P1 M1-04 Implement cost estimator v1
- [x] P1 M1-05 Ship derived metrics pack and starter dashboard
- [x] P1 M1-07 Add privacy modes
- [x] P0 M1-08 Implement or reject-at-parse `deny` policy stage
- [x] P0 M1-09 LLM proxy threat model (non-loopback)
- [x] P1 M2-01 Publish runtime parity matrix
- [x] P1 M1-10 Semgrep workflow hardening
- [x] P1 M2-16 Gateway correlation ID header
- [x] P1 M2-17 OTel GenAI semconv pin + CI drift
- [x] P1 M1-11 Self-metrics latency semantics doc

### P0-Only Execution Order (Crunch Mode)

1. M0-03 — Field-completeness guardrail tests
2. M0-02 — CI smoke benchmark
3. M0-01 — Repeatable 30-min harness
4. M0-05 — Null-field operator doc
5. M1-06 — Error taxonomy hardening
6. M1-01 — `resource.*` enrichment defaults
7. M1-02 — Conversation/turn correlation
8. M1-03 — Trace propagation support
9. M1-08 — `deny` policy stage (**done**: `builtin.deny.all` in `mara-policy`)
10. M1-09 — LLM proxy threat model for non-loopback bind (**done**: `docs/llm-proxy-non-loopback-threat-model.md`, `allow_non_loopback_listen` in `LlmProxyAdapterConfig`)
11. M2-02 — Minimum schema completeness across runtimes (**done**: `schema_completeness_gate.py` + fixtures, CI job `schema-completeness-gate`)

## Capacity View (Suggested)

### This Sprint Capacity

| Owner | Tickets | Notes |
|---|---:|---|
| Runtime | 3 | M0-03, M0-05, M1-01 (done); highest implementation load this sprint. |
| Platform | 2 | M0-01, M0-02; CI + harness reliability focus. |
| Observability | 1 | M0-04 (done): fixture report + CI diff. |
| Security/Policy | 0 | No dedicated ticket in this sprint (can advise on doc/review). |
| Integrations | 0 | No dedicated ticket in this sprint. |

### Next Sprint Capacity

| Owner | Tickets | Notes |
|---|---:|---|
| Runtime | 1 | M1-02 (done earlier). |
| Platform | 0 | M1-03, M1-09, M1-10 done (trace, proxy bind guard, Semgrep + SARIF). |
| Observability | 0 | M1-11 latency histogram doc landed. |
| Security/Policy | 0 | M1-09 landed; M1-07/M1-08 done. |
| Integrations | 0 | M2-01 parity matrix landed. |

### Rebalance Suggestions

- If Runtime gets blocked this sprint, move **M1-01** to next sprint and pull **M1-03** forward.
- If CI work lands early, Platform can co-own **M0-04** automation with Observability.
- Keep **M1-04** and **M1-05** together under Observability; splitting them increases churn.

## Milestone Gates

- **M0 Gate:** Benchmark + CI smoke are stable for one week; no required-field regressions.
- **M1 Gate:** Operators can answer “slowest, most expensive, most error-prone model/runtime” from Mara outputs alone.
- **M1 Hardening Gate (follow-up):** Policy config is honest (`deny` not ignored); Semgrep signal is mandatory (**done**: SARIF upload + failing scan in `security.yml`); non-loopback proxy is explicitly threat-modeled.
- **M2 Gate:** Same questions are answerable across at least three runtimes and one agent framework.
- **M2 Reliability Gate (engineering review):** Sink fan-out does not serialize on slow sinks; readiness reflects adapters/sinks; `/metrics` scrape cost is bounded or documented.
- **M2 Market-parity Gate (telemetry):** Gateway issues a **stable request correlation ID** when traces are absent (M2-16); **semconv version** is pinned and drift-guarded in CI (M2-17); **export path** to eval backends is documented without building eval UI (M2-20); **single-upstream** semantics are explicit in docs (M2-21).

## Weekly Tracking Notes

Use this section for owner updates without editing ticket definitions.

- Week of _YYYY-MM-DD_: _TBD_
