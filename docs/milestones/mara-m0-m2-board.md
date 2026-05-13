# Mara Milestone Board (M0/M1/M2)

This board converts recent test findings into a tracked execution plan.

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
| M1-03 | P0 | [ ] | Add trace propagation support | Platform | 2026-06-03 | Inbound `traceparent` maps to non-null `trace_id`/`span_id` in emitted events. |
| M1-04 | P1 | [ ] | Implement cost estimator v1 | Observability | 2026-06-05 | `cost_usd` is computed from model usage + pricing map with documented assumptions. |
| M1-05 | P1 | [ ] | Ship derived metrics pack and starter dashboard | Observability | 2026-06-06 | Request rate, tokens, p95 latency, error rate, and cost are queryable out of the box. |
| M1-06 | P0 | [x] | Harden error taxonomy and mappings | Runtime | 2026-05-29 | `ProxyFailureKind` enum + `docs/ollama-proxy-error-taxonomy.md`; synthetic 502 `failure_kind` strings are stable and unit-tested. |
| M1-07 | P1 | [ ] | Add privacy modes (metadata-only/hashed/body-opt-in) | Security/Policy | 2026-06-10 | Policy and config toggles are tested and documented for each mode. |

## M2 — Cross-Runtime + Agent-Native Differentiation (6–12 weeks)

### Milestone Outcome

Mara is clearly differentiated through cross-runtime and agent-level observability.

| Ticket | Priority | Status | Task | Owner | ETA | Acceptance Criteria |
|---|---|---|---|---|---|---|
| M2-01 | P1 | [ ] | Publish runtime parity matrix (Claude/Codex/Kimi/Cursor/Ollama) | Integrations | 2026-06-17 | Matrix includes required fields and completeness score per runtime. |
| M2-02 | P0 | [ ] | Reach minimum schema completeness per runtime | Runtime | 2026-06-24 | At least 3 runtimes meet production threshold (for example >=85% required-field fill-rate). |
| M2-03 | P1 | [ ] | Add agent semantics (`agent_id`, `step_id`, `tool_name`, `tool_outcome`) | Runtime | 2026-06-26 | Agent-context fields appear in end-to-end runs with tests for extraction. |
| M2-04 | P1 | [ ] | Add Hermes/OpenClaw integration guide | Integrations | 2026-06-27 | Guide and sample config produce usable telemetry on a reference workflow. |
| M2-05 | P1 | [ ] | Build agent run summary materializer | Observability | 2026-07-02 | Per-run summary reports steps, tokens, cost, failures, and latency hotspots. |
| M2-06 | P2 | [ ] | Build cross-runtime comparative dashboard | Observability | 2026-07-03 | Single dashboard compares model/runtime latency, cost, and error rates for same workload. |
| M2-07 | P1 | [ ] | Publish adoption quickstarts pack | Integrations | 2026-07-08 | Templates for Ollama-heavy, mixed-runtime, and privacy-first setups pass smoke tests. |

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
- [ ] P0 M1-03 Add trace propagation support
- [ ] P1 M1-04 Implement cost estimator v1
- [ ] P1 M1-05 Ship derived metrics pack and starter dashboard
- [ ] P1 M1-07 Add privacy modes
- [ ] P1 M2-01 Publish runtime parity matrix

### P0-Only Execution Order (Crunch Mode)

1. M0-03 — Field-completeness guardrail tests
2. M0-02 — CI smoke benchmark
3. M0-01 — Repeatable 30-min harness
4. M0-05 — Null-field operator doc
5. M1-06 — Error taxonomy hardening
6. M1-01 — `resource.*` enrichment defaults
7. M1-02 — Conversation/turn correlation
8. M1-03 — Trace propagation support
9. M2-02 — Minimum schema completeness across runtimes

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
| Runtime | 1 | M1-02. |
| Platform | 1 | M1-03. |
| Observability | 2 | M1-04, M1-05; cost + dashboard bundle. |
| Security/Policy | 1 | M1-07. |
| Integrations | 1 | M2-01. |

### Rebalance Suggestions

- If Runtime gets blocked this sprint, move **M1-01** to next sprint and pull **M1-03** forward.
- If CI work lands early, Platform can co-own **M0-04** automation with Observability.
- Keep **M1-04** and **M1-05** together under Observability; splitting them increases churn.

## Milestone Gates

- **M0 Gate:** Benchmark + CI smoke are stable for one week; no required-field regressions.
- **M1 Gate:** Operators can answer “slowest, most expensive, most error-prone model/runtime” from Mara outputs alone.
- **M2 Gate:** Same questions are answerable across at least three runtimes and one agent framework.

## Weekly Tracking Notes

Use this section for owner updates without editing ticket definitions.

- Week of _YYYY-MM-DD_: _TBD_
