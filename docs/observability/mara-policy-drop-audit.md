# Policy drop audit events (optional)

When a policy stage returns **`PolicyOutcome::Drop`**, the original canonical event is **not** delivered to sinks by default (kill-switch and ZDR friendly).

Some operators still need **visibility** that traffic was blocked (for dashboards, SIEM joins, or chargeback disputes) without re-exporting prompts or completions.

## Enabling audit export

Per pipeline in `mara.toml`:

```toml
[[pipelines]]
name = "default"
adapters = ["…"]
sinks = ["…"]
policy_chain = "…"
audit_policy_drops = true   # default: false (omit or set false)
```

When `audit_policy_drops` is **true**, every policy **drop** causes a single additional **`EventKind::System`** event to be fanned out to the pipeline’s sinks:

- **Scope** name: `mara.policy.audit`
- **No `body`** and **no `gen_ai` usage** payload
- **Copied** from the dropped event (when present): `trace_id`, `span_id`, `parent_span_id`, `resource`, `severity`, `mara.request_id`, `mara.session_id`, `mara.turn_id`, `mara.tenant_id`, `mara.policy_profile`, `mara.policy_decisions`, `mara.source_adapter`
- **Attributes** (bounded strings):
  - `mara.policy_audit.kind` = `drop`
  - `mara.policy_audit.pipeline` = pipeline name
  - `mara.policy_audit.chain_reason` = drop reason (truncated to 512 Unicode scalars)
  - `mara.policy_audit.base_event_kind` = snake-case style slug of the original `event_kind` (`prompt`, `completion`, …)

`mara.policy_capture_optin` on the audit event is forced **false** so downstream redaction paths treat it as non–user-content.

## Self-metrics

Audit events increment **`mara_pipeline_events_delivered_total`** (one delivery per audit). They do **not** increment GenAI completion / token / cost counters.

## Threat and privacy notes

- **Default off** — explicit opt-in avoids surprising data leaving a deny or sampling stage.
- Audit events can still carry **policy decision metadata** and correlation IDs; treat sinks and OTLP endpoints like any other regulated telemetry path.
- This is **not** a full tamper-evident audit log; for that, see the product roadmap (WAL / signed exports).
