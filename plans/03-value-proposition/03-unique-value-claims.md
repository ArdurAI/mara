# Unique Value Claims

## Executive summary

Mara's value is the conjunction of seven properties that no single existing tool offers together. Each claim below is testable, defensible, and tracked in `evaluation/` for ongoing verification.

## Claim 1 — Runtime-aware, not generic

**Statement:** Mara ships first-party presets for six AI runtimes (Claude Code, Codex, Cursor Agents, Kimi, Augment Code, Gemini) on day one of v1.

**Why it matters:** No other shipper has runtime-specific knowledge of `~/.claude/projects/*.jsonl` semantics, Codex `hooks.json` event schemas, Cursor's hooks-over-stdio JSON shape, Kimi's `~/.kimi/logs/kimi.log` patterns, Gemini's `gen_ai.*` env-var-driven OTLP, or Augment's analytics REST shape. Operators get telemetry that's already meaningful, not raw bytes that need post-processing.

**Test of the claim:** the compatibility matrix in [`../05-evaluation/02-compatibility-matrix-spec.md`](../05-evaluation/02-compatibility-matrix-spec.md) shows pass/fail per runtime per signal. v1 must hit tier-appropriate pass on all six.

## Claim 2 — Aligned with OpenTelemetry `gen_ai.*`, not yet-another-schema

**Statement:** Mara's canonical model is the OTel `gen_ai.*` semantic conventions plus `mara.*` extensions for gaps. Mara contributes findings back to the OTel semconv working group.

**Why it matters:** Every vendor today has its own schema (Datadog LLM Obs, Arize OpenInference, Langfuse, Helicone, etc.). The OTel `gen_ai.*` namespace is the only neutral standard, and it's where the industry is converging — but it's still Development status, so Mara's adoption helps push it toward stable while preventing operator lock-in.

**Test of the claim:** semconv-drift CI check; documented diff between the pinned `gen_ai.*` version and `mara.*` extensions; quarterly upstream contribution log.

## Claim 3 — Edge-first, single-binary, no runtime dependencies

**Statement:** Mara is a single statically-linked Rust binary that runs on macOS, Linux, and Windows. No JVM, no Python, no Node, no Docker required at runtime.

**Why it matters:** Indie devs install via `brew install mara` and have telemetry in five minutes. Platform teams ship one binary across heterogeneous fleets. Compliance teams audit one binary, not a stack of interpreters and dependencies. Footprint stays under 128 MB RSS idle and under 512 MB at sustained 50k events/sec SLO load.

**Test of the claim:** CI matrix builds release artifacts for macOS (universal2), Linux (amd64 + arm64, glibc + musl), Windows (amd64). Memory and CPU SLO gates run on every PR.

## Claim 4 — Policy as code, sandboxed, signed

**Statement:** Policy lives in WASM modules that are sandboxed by `wasmtime`, distributed as signed bundles, and verified via `cosign` before load.

**Why it matters:** Compliance and security teams need provable enforcement, not configuration that anyone with shell access can mutate. Polyglot policy (any language to WASM) means teams use the tools they know. Sandboxing means a buggy policy can't crash the agent.

**Test of the claim:** policy-bundle integration test suite; signature-verification regression test; fault-injection test (broken WASM module must not bring down the pipeline).

## Claim 5 — Zero phone-home, opt-in capture, ZDR-respecting

**Statement:** Mara emits no telemetry to ArdurAI by default. Prompt and raw-API-body capture is opt-in per pipeline. Each runtime's ZDR toggle is honored agent-side.

**Why it matters:** Enterprise procurement teams reject tools that exfiltrate metadata silently. Developer trust is eroded the moment a tool secretly sends data home. Vendor ZDR commitments are policy promises; agent-side enforcement turns them into technical guarantees.

**Test of the claim:** network egress test on a brand-new install with no configured sinks shows zero outbound traffic. Capture-toggle tests verify opt-in only, with `mara.policy.capture_optin = true` and `OTEL_LOG_USER_PROMPTS=false` honored. Documented per-runtime ZDR matrix.

## Claim 6 — Vendor-neutral, multi-sink, open-source under Apache 2.0

**Statement:** Mara is Apache 2.0 with no proprietary core. Any sink is a plugin. v1 ships ten sinks; the plugin ABI is stable enough that external sink plugins can ship independently.

**Why it matters:** Operators who chose Datadog, Splunk, Grafana, or Honeycomb don't have to also choose Mara's backend. The commercial path for ArdurAI is the gateway and control plane, not the shipper itself.

**Test of the claim:** at least one external (non-ArdurAI) sink contribution merged before v1.0. License audit by `cargo-deny` on every PR. CNCF Sandbox application drafted before v1.0.

## Claim 7 — Supply chain hardened by default

**Statement:** Every release ships with CycloneDX and SPDX SBOMs, SLSA Level 2 provenance, `cosign` keyless signatures, and a published security policy. v2 targets SLSA Level 3.

**Why it matters:** A telemetry agent has root-adjacent privileges and reads sensitive content. Supply-chain compromise of the agent is a worst-case scenario. Modern teams require attestations, not promises.

**Test of the claim:** GitHub release page shows SBOM + provenance + signature artifacts for every release tag. `cosign verify-attestation` succeeds against published policy. Trivy + Grype scans pass with zero high/critical CVEs.

## Comparison summary

The conjunction of the seven claims is the value. Most competing tools meet some of these, none meets all of them as of May 2026:

- Fluent Bit / Vector / OTel Collector: claims 3, 6, 7 — but not 1, 2, 4, 5 (no AI runtime knowledge, no AI schema, no AI policy, no ZDR-aware capture defaults).
- Langfuse / Phoenix / LangSmith / Helicone: claims 1 (their own SDK surface, not runtimes you don't own), and partial 2 — but not 3 (SaaS or hosted), often not 5 (SDK collects everything by default), not 6 (vendor-coupled backend).
- Datadog LLM Obs / Splunk / Honeycomb AI: claim 2 partially, claim 7 fully — but not 1 (no agent for AI runtimes), 3 (SaaS), 5 (vendor decides), 6 (proprietary backend coupling).
- Vector + custom Lua/VRL transforms: can be bent to approximate claim 1 and 4 — but the work is custom, unmaintained, and unsigned.

## Falsifiability

If, by v1.0, any of these claims cannot be substantiated by published artifacts or test results, the claim is removed from this document and the gap is recorded in [`../02-gaps/`](../02-gaps/) instead. Marketing follows reality.
