# MVP — Risk Register

## Executive summary

Risks specific to the MVP execution: things that could derail the five-week plan, blow the sign-off criteria, or invalidate the persona assumption. Each risk has a trigger, likelihood, impact, owner, and a concrete mitigation. The owner column is intentionally generic role names ("MVP Engineer", "Tech Lead") rather than named people; substitute real names at MVP kickoff.

This document is read at the start of each MVP week and updated when a risk materialises or a new one surfaces.

## Risk taxonomy

Risks fall into five categories. Each section lists the risks, ordered by `Likelihood × Impact`.

## 1. Technical risks

### R-T1 — OTLP proto codegen complexity (Likelihood: Medium · Impact: High)

**Trigger:** the `opentelemetry-proto` crate's generated types don't match the wire shape Claude Code actually emits, OR proto deserialization introduces non-obvious correctness bugs (unknown fields, varint edge cases).

**Mitigation:** use `opentelemetry-proto = "0.27"` as the canonical types (don't hand-roll prost). Capture a real Claude Code OTLP payload during week 1 and pin it as a golden test fixture. If the upstream crate has gaps, file an upstream PR rather than fork.

**Owner:** MVP Engineer.

**Contingency:** if the upstream crate is unworkable, fall back to hand-rolled prost from `opentelemetry-proto/proto/` — adds ~3 days.

### R-T2 — `gen_ai.*` semconv churn during MVP (Likelihood: Medium · Impact: Medium)

**Trigger:** the OTel gen-ai SIG ships a breaking attribute rename during the 5-week window.

**Mitigation:** pin the semconv commit in `crates/mara-schema/semconv.lock` (already designed). Subscribe a maintainer to the OTel semconv repo. If a breaking rename lands, we ship MVP against the pre-rename attribute set and document the migration in `CHANGELOG.md`.

**Owner:** Schema Lead.

**Contingency:** delay the corresponding sink-side mapping; runtimes that emit the new attribute names get a "next release" tag.

### R-T3 — Loki structured-metadata cardinality blow-up (Likelihood: Medium · Impact: Medium)

**Trigger:** initial Loki sink configuration leaks high-cardinality fields into labels by accident; first real user's Grafana account thrashes.

**Mitigation:** ship strict default labels = `{runtime, event_kind}` only. Add a config-validation rule that rejects label sets containing patterns like `*_id` or `*_hash`. Document the failure mode prominently in the quickstart.

**Owner:** Sinks team (MVP: same MVP Engineer).

**Contingency:** emergency sink-disable flag in config; rollback the Loki sink in a patch release.

### R-T4 — `rustls` + `aws-lc-rs` build failure on macOS aarch64 (Likelihood: Low · Impact: Medium)

**Trigger:** `aws-lc-rs` requires CMake and a recent compiler; macOS CI runner image change breaks the build.

**Mitigation:** pin `aws-lc-rs` version. Run the CI matrix on `macos-latest` consistently. Document the required tooling in CONTRIBUTING.

**Owner:** MVP Engineer.

**Contingency:** fall back to `ring` crypto backend; documented in ADR addendum.

### R-T5 — Streaming events fragment trace context (Likelihood: Medium · Impact: Medium)

**Trigger:** Claude Code streaming responses emit one OTLP span event per token instead of a single completion span; Mara's normalizer drops them into separate canonical events; downstream sinks see fragmented traces.

**Mitigation:** detect streaming responses by `gen_ai.response.is_streaming = true` and aggregate token events at the normalizer boundary. Add a test fixture with a recorded streaming session.

**Owner:** MVP Engineer.

**Contingency:** document the fragmentation as a known v0.2.0-alpha limitation; aggregation lands in MVP+1.

### R-T6 — Performance regression mid-week breaks PR gate (Likelihood: Medium · Impact: Low)

**Trigger:** an unrelated refactor in week 3 introduces a 10 % throughput regression that blocks all subsequent PRs.

**Mitigation:** introduce the PR-gate perf check in week 6, not week 1 — early MVP doesn't yet have the harness anyway. Maintain a manual perf baseline check at end of each week.

**Owner:** Core Lead.

**Contingency:** temporarily widen the regression threshold to 10 % while the cause is isolated; revert to 5 % once fixed.

### R-T7 — Ollama proxy is not actually transparent (Likelihood: Medium · Impact: High)

**Trigger:** Ollama clients (the `ollama` CLI, Open WebUI, Continue.dev, OpenAI-SDK code) observe different behaviour when going through Mara than going directly to Ollama. Examples: streaming chunk timing changes that break a UI; HTTP keep-alive semantics shift; the `Host` header forwarded incorrectly.

**Mitigation:** SC-9 (proxy transparency) is explicit. Week 4 includes a checksum-based test: response bytes received by the client when routed through Mara are byte-identical to bytes received when going directly. Streaming chunks must arrive in the same order with bounded jitter (≤ 5 ms median over loopback).

**Owner:** MVP Engineer.

**Contingency:** if a specific Ollama client breaks under Mara, document the workaround (e.g., disable response buffering) and add an explicit fixture-driven regression test for that client.

### R-T8 — Ollama response schema drift across versions (Likelihood: Medium · Impact: Medium)

**Trigger:** an Ollama release between week 1 and the v0.2.0-alpha tag changes the response field set (e.g., renames `eval_count` or adds a new endpoint we don't handle).

**Mitigation:** pin a tested Ollama version range in `crates/mara-runtime-ollama/Cargo.toml` documentation. Subscribe a maintainer to Ollama's GitHub release notifications. Treat unknown response fields as opaque attributes rather than failing.

**Owner:** Integrations Lead.

**Contingency:** restrict the documented quickstart to a pinned Ollama version range; widen as we test newer versions.

### R-T9 — Ollama SSE streaming buffer growth (Likelihood: Low · Impact: Medium)

**Trigger:** a long-running streaming response (e.g., a 30,000-token generation from a 70B model) exceeds the in-memory response-buffer budget; Mara either drops capture or runs out of memory.

**Mitigation:** bounded response-buffer (default 16 MiB) with documented behaviour: forward chunks to client unchanged in real time (no buffering for forwarding); buffer for capture only; on capture-buffer overflow emit a `mara.body.truncated = true` event but never block the proxy path.

**Owner:** MVP Engineer.

**Contingency:** raise the default buffer; expose as configurable; document for users running very large models.

## 2. Scope risks

### R-S1 — Scope creep into Option B during MVP (Likelihood: High · Impact: High)

**Trigger:** "while we're here, let's also add Codex." The most likely way MVP slips its 5-week schedule.

**Mitigation:** [`01-scope-and-decision-criteria.md`](01-scope-and-decision-criteria.md) is explicit and signed off. Every PR title gets reviewed against the in-scope list. New work that doesn't trace to an MVP sign-off criterion goes to a `mvp+1` milestone, not into the current branch.

**Owner:** Tech Lead.

**Contingency:** mid-week scope-creep audit; remove anything that snuck in.

### R-S2 — Apple notarization friction extends week 5 (Likelihood: High · Impact: Medium)

**Trigger:** first time setting up Apple Developer ID + `notarytool` + stapling in the release workflow; first-run gotchas eat 2–3 days.

**Mitigation:** schedule the notarization work in week 4 (not week 5) so week 5 has slack. Test the workflow against a pre-release tag during week 4.

**Owner:** MVP Engineer.

**Contingency:** ship `v0.2.0-alpha` as Linux-only if macOS notarization slips; macOS follows in `v0.2.1-alpha` within 1 week.

### R-S3 — Persona acceptance test reveals a 30-minute install reality (Likelihood: Medium · Impact: High)

**Trigger:** external user runs the quickstart and discovers a missing step / unintuitive command / unclear error message; documented "5 minutes" is empirically 30.

**Mitigation:** internal dogfooding in week 4 should catch most of these. Schedule the external persona test in week 5 with at least 3 candidates; iterate on friction within the week.

**Owner:** MVP Engineer + Product (if separate).

**Contingency:** if the quickstart can't be reduced to 5 minutes, document the actual time in the README; don't ship `v0.2.0-alpha` until SC-1 is honestly green.

## 3. Adoption / signal risks

### R-A1 — Zero external users sign up to test (Likelihood: Medium · Impact: High)

**Trigger:** no Discord / Twitter / mailing-list user volunteers to run the quickstart during week 5.

**Mitigation:** soft-launch in week 4 (post in 2–3 AI engineering communities asking for testers). Have a backup plan: 2–3 ArdurAI contacts who informally agreed to try it. Persona acceptance can happen with one user, ideally more.

**Owner:** Product.

**Contingency:** delay the `v0.2.0-alpha` tag by one week to widen the recruitment window.

### R-A2 — Initial user finds Mara useful but pivots to "I want Codex too" (Likelihood: High · Impact: Low)

**Trigger:** first external user says "great, now add Codex / Cursor / etc."

**Mitigation:** this is the expected reaction. Document it as confirming-the-thesis; defer to MVP+1.

**Owner:** Product.

**Contingency:** none needed; this is success signal.

### R-A3 — Initial user reports OTel env vars are confusing (Likelihood: Medium · Impact: Medium)

**Trigger:** "what is `OTEL_EXPORTER_OTLP_PROTOCOL` and why do I need to set it?"

**Mitigation:** `mara setup claude-code` prints the env-var setup with an example. Quickstart leads with a copy-paste shell block.

**Owner:** MVP Engineer.

**Contingency:** add a `mara setup claude-code --shell zsh` mode that writes the env-var lines to the user's shell profile (with confirmation prompt).

## 4. Operational risks

### R-O1 — CI compute budget exceeded by perf benches (Likelihood: Low · Impact: Low)

**Trigger:** nightly bench runs blow through GitHub Actions free-tier minutes.

**Mitigation:** restrict nightly to one runner; PR-gate bench is short (60s); self-hosted runner an option if it becomes a real cost.

**Owner:** MVP Engineer.

**Contingency:** move nightly bench to a self-hosted runner before incurring overage charges.

### R-O2 — GitHub Container Registry rate-limited during testing (Likelihood: Low · Impact: Low)

**Trigger:** repeated container image push / pull from CI hits rate limits.

**Mitigation:** unlikely at MVP volume. If it happens, switch the bench-image push step to weekly rather than per-tag.

**Owner:** MVP Engineer.

### R-O3 — Dependency security advisory mid-MVP forces upgrade (Likelihood: Medium · Impact: Low)

**Trigger:** RustSec or OSV publishes a high/critical CVE in `tokio` / `hyper` / `rustls` / `prost`.

**Mitigation:** auto-upgrade via Dependabot is configured. PRs land within a day of advisory.

**Owner:** Security Lead (MVP: same engineer).

**Contingency:** carve out a half-day to address; not catastrophic for MVP schedule.

## 5. Compliance / license risks

### R-L1 — A direct dependency relicenses to AGPL during MVP (Likelihood: Low · Impact: High)

**Trigger:** a maintainer of `tokio`, `hyper`, `rustls`, `prost`, or `serde` relicenses upstream (extremely unlikely but the precedent of Grafana / Elastic looms).

**Mitigation:** `cargo deny` license-allowlist gates every PR. Apache 2.0 / MIT / BSD / MPL only. If a transitive dep relicenses, our deny check catches it before merge.

**Owner:** Security Lead.

**Contingency:** pin the pre-relicense version; find an alternative crate; document in an ADR.

### R-L2 — `opentelemetry-proto`'s generated code has a subtle license issue (Likelihood: Low · Impact: Medium)

**Trigger:** generated proto code under a license that conflicts with our Apache 2.0 outputs.

**Mitigation:** verify upstream is Apache 2.0 (it is, as of May 2026). Track upstream license claims in `NOTICE`.

**Owner:** Security Lead.

## Risk-tracking discipline

- Re-read this register at the start of each MVP week's planning.
- Add a row when a risk materialises (so post-mortem learning persists).
- Strike a row when a risk has been mitigated to "negligible" (don't delete — line through, so the rationale is in git history).
- Owner column is real at MVP kickoff (substitute names for the role labels).
- Likelihood × Impact is opinion at MVP start; updated quarterly.

## Cross-references

- [`01-scope-and-decision-criteria.md`](01-scope-and-decision-criteria.md) — why scope discipline matters.
- [`06-mvp-implementation-plan.md`](06-mvp-implementation-plan.md) — the plan being defended.
- [`07-test-and-improve-loop.md`](07-test-and-improve-loop.md) — how regressions are caught early.
- [`../04-implementation/07-phased-milestones.md`](../04-implementation/07-phased-milestones.md) — broader MOS risk view.
