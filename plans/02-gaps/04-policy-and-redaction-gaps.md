# Policy and Redaction Gaps

## Executive summary

PII, PHI, PCI, and secrets routinely end up in AI prompt and completion content. Existing observability tools handle this with server-side scrub rules that fire after the data has already left the operator's premises. The right architecture redacts at the agent boundary, before any sink dispatch, with signed and auditable policy. This document catalogs how today's tools fall short and how Mara's policy-as-code primitive — WASM-sandboxed plugins, signed bundles, built-in primitives — closes the gap.

## Gap 1 — Server-side scrubbing happens too late

Most SaaS observability tools (Datadog, Splunk, New Relic, Honeycomb) accept the raw event and then apply scrubbing rules on ingestion. By the time the rule fires:

- Data has left the operator's network.
- Data has crossed a vendor's TLS boundary.
- Data may have been logged in transit.
- The vendor has it cached for some time.

**Mara approach:** redaction is a pipeline stage **before** the sink. The agent never emits unredacted prompts to a sink unless the operator explicitly opted in.

## Gap 2 — Regex coverage is incomplete and ad-hoc

Vendors publish regex packs but they're inconsistent. Coverage varies:

- Email: standard.
- US SSN: usually present.
- EU IBAN: sometimes.
- API keys (AWS, GCP, Stripe, GitHub, OpenAI, Anthropic, Slack): inconsistent.
- Credit cards (Luhn check): sometimes.
- Non-ASCII names and addresses: rare.

**Mara approach:** built-in `builtin.pii` pack covers a curated set of regexes and Luhn validators. `builtin.phi` covers HIPAA categories. `builtin.pci` covers cardholder data. Packs versioned and signed. Operators can extend with their own packs.

## Gap 3 — Redaction destroys searchability and audit utility

Replacing `john.doe@example.com` with `[email]` works for hiding, but if you later need to investigate "did user X interact with this feature," the redacted text is useless.

**Mara approach:** two redaction modes:

- `replace` — `[email]` placeholder.
- `tokenize` — deterministic format-preserving token like `[email:7a8f3b]` where the suffix is a HMAC-of-content with a per-tenant key. Same content yields same token; cross-event correlation possible without revealing content.

Operators choose per policy.

## Gap 4 — Regex DoS (ReDoS)

Bad regex patterns can run in exponential time on adversarial inputs. A malicious prompt could be crafted to exhaust CPU on the redaction stage.

**Mara approach:** all default regexes in built-in packs are validated against the `regex-syntax` linear-time guarantee (Mara uses `regex` crate which is linear-time by construction; non-backtracking). Custom packs go through the same validator at policy load. WASM-based policies are time-budgeted (default 50ms per event, configurable).

## Gap 5 — Policy bundles are not signed by default

Most observability tools allow user-supplied scrub rules but don't sign them, don't version them, and don't provide an audit trail of changes.

**Mara approach:** policy bundles are TARs containing rules + manifest + signature. `cosign` keyless verification on load. Unsigned bundles require `--allow-unsigned-policy` flag with a startup warning.

## Gap 6 — Tenant-scoped policies are rare

A SaaS handles tenants with very different sensitivity profiles. A simple "redact emails" rule is too coarse — tenant A may opt for full capture; tenant B may need PHI redaction; tenant C may require complete prompt suppression.

**Mara approach:** per-tenant policy selection (v1 via static configuration; v2 via dynamic gateway distribution).

## Gap 7 — Audit log of redaction decisions is rare

Compliance teams ask: "show me every time we redacted a credit-card number in February." Most tools can't answer.

**Mara approach:** when audit logging is enabled, every policy decision (redact, deny, sample, classify, route) generates an audit event with: timestamp, policy rule id, attribute that matched, hash of original value, action taken. Audit events are append-only and Merkle-rooted.

## Gap 8 — Redaction of structured data

Redacting from JSON / Parquet / protobuf is harder than from plain strings. The redactor must understand the structure to avoid corruption.

**Mara approach:** redactors run over the canonical schema's typed fields. String fields get regex; nested JSON in `attributes.*` gets path-aware redaction via `jsonpath-rust`; binary fields are out of scope (don't put binary in attributes).

## Gap 9 — Cross-language WASM policies have an ABI gap

WASM lets you author policies in Rust, Go, AssemblyScript, Python (via Pyodide), JavaScript (via Javy), Zig, and others. But the ABI for "receive an event, return a decision" is not standardized — every host invents its own.

**Mara approach:** publish a `mara-policy-sdk` for Rust, Go, and TypeScript that wraps the host ABI. Document the ABI for any language with a WASM toolchain.

## Gap 10 — Performance budget for the policy stage

Heavy policy chains can become the bottleneck. A 50µs regex stage times 50,000 EPS = 2.5 CPU-seconds per second. Bad regex on a single core kills the SLO.

**Mara approach:**

- All built-in primitives benchmarked with `criterion`; per-event budget published.
- WASM policy stages are sandboxed and time-budgeted.
- Compiled regex packs use `aho-corasick` for multi-pattern matching where literal matching suffices.
- The policy chain runs in parallel across pipelines.

## Gap 11 — False positives

A regex matching SSN-like patterns can also match a normal 9-digit ID. Over-redaction destroys utility; under-redaction defeats the purpose.

**Mara approach:**

- Built-in regexes are conservative (high precision, lower recall by default).
- "Strict" packs are available for higher-stakes industries (PHI, PCI).
- The `tokenize` mode allows operator review post-hoc by deterministic mapping back, with proper key management.

## Gap 12 — Internationalization

PII formats are locale-specific. US SSN regex doesn't help for an EU-only deployment. Address formats vary wildly.

**Mara approach:** built-in packs include locale-specific extensions:

- `builtin.pii.us`
- `builtin.pii.eu`
- `builtin.pii.jp`
- `builtin.pii.in`
- `builtin.pii.cn`

Operators select per their deployment. v1 ships US, EU, UK as default; others as community contributions.

## Gap 13 — Sensitive data in attribute keys

Most redactors operate on values. But attribute keys themselves can be sensitive — e.g., `attributes."user.john.doe@example.com.permission"` is a key that contains an email.

**Mara approach:** key-redaction policy primitive available; not enabled by default but documented.

## Gap 14 — Tool-call argument redaction

Tool calls have structured arguments (JSON). A "send_email" tool's `to` argument may be PII. Redaction needs JSON-path awareness.

**Mara approach:** `gen_ai.tool.call.arguments` is treated as opaque JSON; policy primitives include `redact-jsonpath` that operates on it.

## Gap 15 — "Soft" redaction for evaluations

Evaluations sometimes need to know prompts contained PII (to check the model's behavior) without storing the actual PII. Pure replacement loses that information.

**Mara approach:** classifier policy primitive marks events with `mara.pii.kinds = ["email", "phone"]` without removing or replacing the content (when capture is opted in). When capture is off, the classifier still records the kinds, providing the eval signal without storing content.

## Implementation primitives in Mara v1

- `redact-regex` — built-in compiled regex pack.
- `redact-jsonpath` — JSONPath-aware redaction in structured attributes.
- `redact-luhn` — Luhn-validated card number detection.
- `redact-hash` — replace with HMAC token (deterministic with per-tenant key).
- `redact-classify` — annotate with PII kinds without removing.
- `policy-deny` — drop the event entirely.
- `policy-sample` — keep N% of matching events.
- `policy-rate-limit` — token bucket per category.
- `policy-route` — fan out matching events to a different sink.
- `policy-wasm` — load and apply a WASM module.

## Reference implementations and prior art

- **Presidio (Microsoft):** open-source PII detection + redaction. <https://github.com/microsoft/presidio>. Worth integrating as an optional WASM-wrapped backend for sophisticated NER-based detection.
- **detect-secrets (Yelp):** secret detection patterns. <https://github.com/Yelp/detect-secrets>. Source of regex patterns for `builtin.secrets`.
- **Gitleaks rules:** secret detection. <https://github.com/gitleaks/gitleaks>.
- **OPA / Rego:** policy language. <https://www.openpolicyagent.org>. Embedded as a built-in WASM module.
- **Open Cyber Patterns / NIST regex packs:** authoritative patterns for various categories.

## What Mara explicitly does not provide

- Deep NER-based PII detection in the agent's core (heavy ML); available via WASM Presidio integration if the operator needs it.
- Content classification beyond PII (NSFW, toxicity, etc.); use guardrail tools (Llama Guard, NeMo Guardrails) and ingest their results as events.
- DLP-style behavior detection across time series; that's a SIEM concern.
- Encryption at rest beyond what the WAL provides (CRC, not encryption). Disk encryption is the operator's OS/cloud concern.

## Open questions

- Should Mara ship a "redact-everything-by-default" mode for first-time users to prevent accidental leaks? Likely yes, with an explicit "I want capture" opt-in to disable.
- Should we add a "trial run" mode that logs redaction decisions without applying them, for policy authors to validate? Yes, as `--policy-dry-run` v1.x.
- Centralized vs decentralized policy bundle distribution — v1 file-based, v2 gateway-pushed, v3 control-plane-managed.
