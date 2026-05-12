# MVP — OWASP Alignment

## Executive summary

Two OWASP lists matter for Mara: **OWASP Top 10 for LLM Applications (2025)** because Mara observes AI workloads, and **OWASP Top 10 (Web Application Security, 2021)** because Mara is itself a Rust service with network-facing endpoints (the OTLP receiver, the self-telemetry server, and the future webhook sink). This document maps Mara's MVP design to both lists item by item, distinguishing what we directly mitigate, what we observe but don't block, and what we explicitly leave to other tools. Findings here feed back into the STRIDE threat model in [`../../docs/threat-model.md`](../../docs/threat-model.md) and the SOC 2 control mapping in [`../05-evaluation/03-soc2-control-mapping.md`](../05-evaluation/03-soc2-control-mapping.md).

Sources:

- OWASP Top 10 for LLM Applications 2025: <https://genai.owasp.org/llm-top-10/>.
- OWASP Top 10 2021 (Web): <https://owasp.org/Top10/>.
- OWASP API Security Top 10 2023: <https://owasp.org/API-Security/editions/2023/en/0x11-t10/>.

## Part 1 — OWASP Top 10 for LLM Applications (2025)

Mara is an observability and governance substrate. It does not block prompts at inference time (that's the guardrail category — Lakera Guard, NeMo Guardrails, Llama Guard). It observes, classifies, redacts, and routes. Where Mara matters for each LLM Top 10 item is the *evidence pipeline* — the trail of what happened and why.

### LLM01:2025 — Prompt Injection

**Threat:** adversarial prompts that subvert the model's instructions.

**Mara's role:** observability, not prevention.

- ✅ Captures prompt + completion + tool-call trail when capture is opted in.
- ✅ Preserves trace context so post-incident investigation can reconstruct the agent loop.
- ✅ Audit log of policy decisions provides a tamper-evident record (Option C; for MVP the audit log is in-memory and best-effort).
- ❌ Does NOT detect prompt-injection attempts at inference time. Use Lakera Guard, NeMo Guardrails, Llama Guard, Prompt Security, or Pillar Security and ingest their decisions as canonical events.

**MVP-specific:** if a guardrail tool POSTs decisions to Mara via webhook (post-MVP) or OTLP (MVP), Mara forwards them with the same trace context as the underlying agent loop, enabling backend dashboards to correlate.

### LLM02:2025 — Sensitive Information Disclosure

**Threat:** prompts or completions contain or leak PII/PHI/PCI/credentials/IP.

**Mara's role:** **direct mitigation.** This is Mara's strongest LLM Top 10 fit.

- ✅ Agent-side redaction via `builtin.pii` pack before any sink dispatch (9 patterns in M2, +4 in MVP: Anthropic, OpenAI, Bedrock, GCP SA JSON).
- ✅ ZDR-aware capture defaults (prompts/completions NOT captured by default; require dual opt-in).
- ✅ Hash-only fallback (`mara.body.prompt_hash`) for dedup without leaking content.
- ✅ Audit log of every redaction decision (Option C; in-memory at MVP).
- ❌ Does NOT prevent prompts from being sent to vendor APIs. That's the vendor's contractual boundary (Zero Data Retention).

**MVP-specific test:** synthetic Anthropic / OpenAI / GCP keys, JWTs, US SSNs, emails all redacted in golden tests under `crates/mara-policy/src/builtin/redact.rs`.

### LLM03:2025 — Supply Chain

**Threat:** training data poisoning, compromised model weights, malicious model marketplace artifacts.

**Mara's role:** indirect — we attest *our own* supply chain.

- ✅ Mara releases ship CycloneDX + SPDX SBOMs (M4 release workflow).
- ✅ SLSA Level 2 provenance attestation on every binary, container image, and Helm chart.
- ✅ `cosign` keyless signatures verifiable via Sigstore Rekor transparency log.
- ✅ Container image is distroless and pinned by digest.
- ❌ Does NOT verify or attest LLM model artifacts themselves. That's a model registry concern (Hugging Face Hub provenance, NVIDIA NGC signing, AWS Bedrock model attestations).

**MVP-specific:** the release workflow already produces SBOMs and provenance for `v0.2.0-alpha`.

### LLM04:2025 — Data and Model Poisoning

**Threat:** poisoned training data corrupts model behavior.

**Mara's role:** none. Out of scope.

- ❌ Mara is inference-side observability, not training-time.
- ✅ Mara's telemetry of inference-side anomalies (sudden hallucination rate change, sudden refusal rate change) can be evidence in post-incident analysis if poisoning is suspected.

### LLM05:2025 — Improper Output Handling

**Threat:** application code blindly trusts model output and executes / renders / passes it on.

**Mara's role:** observability only.

- ✅ Captures tool-call arguments and tool-call results in canonical events (subject to ZDR opt-in). Operators see the chain of what the model produced and what the application did with it.
- ❌ Does NOT enforce output sanitization. That's the application's job.

### LLM06:2025 — Excessive Agency

**Threat:** the model has too many tools, too much authority, or too autonomous a loop, and does damage.

**Mara's role:** **direct evidence pipeline.** Strong fit.

- ✅ Every `tool_call` and `tool_result` event captured with `gen_ai.tool.name`, `mcp.tool.name`, `mcp.server.name`, `mcp.tool.namespace`.
- ✅ Multi-turn agent traces reconstructable from `gen_ai.agent.id` + `mara.session.id` + `mara.turn.id`.
- ✅ Cost telemetry (`mara.cost.usd`) lets operators detect runaway agent loops.
- ❌ Does NOT enforce tool allowlists at runtime. That's the agent framework's job (LangChain authorization, Claude Code's MCP allow-list, etc.).

**MVP-specific:** the MCP attribute namespace is preserved in MVP because we instrument the canonical schema for it (`mara-schema::Mcp`). When Claude Code uses an MCP tool, Mara records what server, what version, what tool.

### LLM07:2025 — System Prompt Leakage

**Threat:** the system prompt (intended to be hidden) appears in user-visible output.

**Mara's role:** observability + classification.

- ✅ When body capture is opted in, Mara records system / user / assistant role labels separately. Operators can write detection queries against the sink ("system prompt content appearing in `assistant` role messages").
- ❌ Does NOT block or alert at inference time. Classification policy can flag, sink-side alerting actions.

### LLM08:2025 — Vector and Embedding Weaknesses

**Threat:** poisoned or adversarial embeddings in RAG pipelines.

**Mara's role:** none directly. Adjacent.

- ✅ When retrieval tool calls are emitted (e.g., `gen_ai.tool.type = "retrieval"`), Mara captures the query and result metadata.
- ❌ Does NOT inspect embedding vectors themselves; doesn't compute similarity-based anomaly scores.

### LLM09:2025 — Misinformation

**Threat:** the model produces confidently-wrong output.

**Mara's role:** none at runtime; evidence trail for eval pipelines.

- ✅ When eval pipelines produce results, they can post `event_kind = "eval"` events to Mara, joined to the underlying session by trace context.
- ❌ Does NOT detect hallucinations itself.

### LLM10:2025 — Unbounded Consumption

**Threat:** runaway token / cost / latency on the LLM provider side.

**Mara's role:** **observability and alerting feedstock.** Strong fit.

- ✅ Token usage (`gen_ai.usage.*`) and computed cost (`mara.cost.usd`) per event.
- ✅ Per-session aggregation in the sink (`mara.session.id`).
- ✅ Latency telemetry (`gen_ai.latency.ttft_ms`, `gen_ai.latency.total_ms` when emitted).
- ❌ Does NOT enforce rate limits or budget caps. That's a gateway / proxy job (LiteLLM, Portkey, Helicone in proxy mode).

**MVP-specific:** cost computation is in scope per [`04-ai-native-features.md`](04-ai-native-features.md) §3 and [`06-mvp-implementation-plan.md`](06-mvp-implementation-plan.md) week 3.

## Part 2 — OWASP Top 10 (Web, 2021) — for the Mara agent itself

Mara is a Rust service that:

- Binds an OTLP receiver on `127.0.0.1:4318` (MVP).
- Binds a self-telemetry server on `127.0.0.1:9099` (MVP).
- Initiates outbound TLS to operator-configured sinks.
- Reads files specified by adapter configuration.
- Loads policy bundles from operator-configured locations.

These are network and filesystem attack surfaces. The Web Top 10 still applies.

### A01:2021 — Broken Access Control

**Risk:** unauthorized control of Mara's configuration or runtime.

**Mara's defenses:**

- ✅ Default binds are `127.0.0.1`-only. Non-loopback binds require explicit opt-in.
- ✅ Configuration is filesystem-permission-gated; runs as unprivileged user.
- ✅ mTLS required for any non-loopback bind (post-MVP enforcement; MVP documents the threat).
- ✅ Self-telemetry endpoint exposes only metrics + healthz; no admin write surface.

### A02:2021 — Cryptographic Failures

**Risk:** weak TLS, missing certificate verification, plaintext secrets in transit.

**Mara's defenses:**

- ✅ TLS 1.3 default; TLS 1.2 fallback requires explicit per-sink opt-in.
- ✅ Certificate verification on by default; `insecure_skip_verify` requires explicit per-sink opt-in with startup log warning.
- ✅ `rustls` + `aws-lc-rs` (FIPS-capable backend).
- ✅ Secret references (`@file:`, `@vault:`) resolved lazily; never serialized to disk or logged.
- ❌ FIPS-mode crypto on Windows is post-MVP.

### A03:2021 — Injection

**Risk:** malformed OTLP / JSONL / config triggers code execution or denial-of-service.

**Mara's defenses:**

- ✅ All parsing through `serde` + `prost`; no `eval` / `exec` surface.
- ✅ `regex` crate (linear-time guarantee; no ReDoS).
- ✅ `unsafe_code = forbid` workspace lint blocks unsafe Rust.
- ✅ Config parsed to typed Rust structs; no shell interpolation.
- ✅ Bounded message sizes on OTLP receiver (tonic / hyper config).
- ✅ Bounded JSONL line size (default 10 MiB; documented).

### A04:2021 — Insecure Design

**Risk:** the architecture itself is flawed.

**Mara's defenses:**

- ✅ STRIDE threat model at `docs/threat-model.md`.
- ✅ Capability-scoped adapters (a JSONL adapter configured to read `~/.claude/projects/` cannot read elsewhere).
- ✅ Bounded queues + back-pressure prevent exhaustion attacks.
- ✅ ZDR-aware defaults prevent accidental capture.

### A05:2021 — Security Misconfiguration

**Risk:** insecure defaults; verbose error messages; unnecessary features.

**Mara's defenses:**

- ✅ Defaults: loopback bind, no phone-home, capture off, TLS verify on.
- ✅ Error messages use placeholders for secrets.
- ✅ Distroless container image (no shell, no package manager).
- ✅ `mara setup` produces a working config without exposing security surface.
- ✅ Configuration validation (`mara validate`) catches common misconfigurations before runtime.

### A06:2021 — Vulnerable and Outdated Components

**Risk:** known CVEs in dependencies.

**Mara's defenses:**

- ✅ `cargo audit` (RustSec advisories) on every PR and nightly.
- ✅ `cargo deny` (license + advisory + bans + sources) on every PR.
- ✅ OSV scanner workflow on every PR.
- ✅ Trivy filesystem + container image scans on every PR.
- ✅ Dependabot weekly updates, auto-merge for patch-level safe updates.
- ✅ Time-to-fix SLA: ≤ 7 days for High/Critical, ≤ 30 days for Medium.

### A07:2021 — Identification and Authentication Failures

**Risk:** weak auth on Mara's surfaces.

**Mara's defenses:**

- ✅ Loopback bind avoids the question at MVP.
- ✅ mTLS supported on all network sinks and the OTLP receiver when non-loopback.
- ❌ No OIDC integration at MVP (post-MVP for v3 control plane).

### A08:2021 — Software and Data Integrity Failures

**Risk:** untrusted code / data accepted as trusted.

**Mara's defenses:**

- ✅ Releases signed with `cosign` keyless via Sigstore.
- ✅ SLSA Level 2 build provenance attestations.
- ✅ SBOM (CycloneDX + SPDX) published per release.
- ✅ Policy bundles `cosign`-verified before load (Option C).
- ✅ WAL records CRC-checked (Option C; in-memory at MVP).
- ✅ Container images pinned by digest, not tag, in Helm chart and docs.

### A09:2021 — Security Logging and Monitoring Failures

**Risk:** insufficient logging to detect attacks.

**Mara's defenses:**

- ✅ Mara is itself a logging tool; eats its own dogfood.
- ✅ Self-telemetry metrics include security-relevant counters (`mara_sink_auth_failures_total`, `mara_policy_traps_total`, `mara_config_reload_failures_total`).
- ✅ Audit log of policy decisions (Option C; in-memory MVP).
- ✅ STRIDE threat model documents detection points.

### A10:2021 — Server-Side Request Forgery (SSRF)

**Risk:** an adapter or sink can be coerced into making attacker-controlled requests.

**Mara's defenses:**

- ✅ Sink endpoints are operator-configured (never derived from incoming event content).
- ⚠️ Analytics REST adapter (post-MVP for Augment) polls a fixed endpoint configured by the operator; not derived from event content.
- ⚠️ Webhook sink (post-MVP) sends to operator-configured endpoint; payload is canonical event, not attacker-controlled.
- ✅ No follow-redirects-blindly behaviour; `reqwest` default with explicit redirect policy.

## Part 3 — OWASP API Security Top 10 (2023) — for the OTLP receiver

The OTLP HTTP receiver is an API. The OWASP API Top 10 applies briefly:

- **API01: Broken Object Level Authorization** — N/A. OTLP has no object model in this sense; we don't authorize per-record.
- **API02: Broken Authentication** — addressed by loopback default + mTLS for non-loopback.
- **API03: Broken Object Property Level Authorization** — N/A.
- **API04: Unrestricted Resource Consumption** — bounded message sizes, bounded queue depths, back-pressure.
- **API05: Broken Function Level Authorization** — N/A.
- **API06: Unrestricted Access to Sensitive Business Flows** — N/A.
- **API07: Server-Side Request Forgery** — see A10 above.
- **API08: Security Misconfiguration** — see A05 above.
- **API09: Improper Inventory Management** — single API surface, documented in code.
- **API10: Unsafe Consumption of APIs** — Mara is a consumer of OTLP; we validate strictly via prost.

## MVP-specific OWASP checklist

To be passed before tagging `v0.2.0-alpha`:

- [ ] All MVP sinks default to TLS 1.3 + cert verify on.
- [ ] OTLP receiver defaults to `127.0.0.1:4318`; non-loopback bind emits a startup warning.
- [ ] Self-telemetry endpoint defaults to `127.0.0.1:9099`; exposes no admin write surface.
- [ ] Secret references in config never appear in logs or metrics (regression test).
- [ ] `cargo audit`, `cargo deny`, OSV scanner, Trivy fs/image all green on the release commit.
- [ ] SBOM (CycloneDX + SPDX), SLSA L2 provenance, `cosign` signatures published with the release.
- [ ] LLM06 (Excessive Agency) — Mara's MCP attribute coverage verified by integration test.
- [ ] LLM02 (Sensitive Information Disclosure) — Anthropic / OpenAI / GitHub key redaction verified by integration test.
- [ ] LLM10 (Unbounded Consumption) — cost telemetry verified by integration test.
- [ ] STRIDE threat model reviewed and dated.

## Cross-references

- [`../../docs/threat-model.md`](../../docs/threat-model.md) — STRIDE.
- [`../05-evaluation/03-soc2-control-mapping.md`](../05-evaluation/03-soc2-control-mapping.md) — SOC 2 control mapping.
- [`../05-evaluation/04-eu-ai-act-alignment.md`](../05-evaluation/04-eu-ai-act-alignment.md) — EU AI Act + NIST AI RMF.
- [`../01-landscape/06-security-and-compliance.md`](../01-landscape/06-security-and-compliance.md) — broader security landscape.
- [`../02-gaps/04-policy-and-redaction-gaps.md`](../02-gaps/04-policy-and-redaction-gaps.md) — redaction-specific gaps.
- [`../../SECURITY.md`](../../SECURITY.md) — vulnerability reporting policy.
