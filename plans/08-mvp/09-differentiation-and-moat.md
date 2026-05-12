# MVP — Differentiation and Moat

## Executive summary

Mara is **not a fork**. We share zero source code with Fluent Bit, OpenTelemetry Collector, Vector, Logstash, or any other shipper. We share *patterns* — input → policy → output, async pipelines, plugin-style adapters — with the entire log-shipper category, because those patterns are correct and well-understood. The architectural delta from "another fork" is the AI-specialist curation we bake into the defaults: runtime presets, vendor-key redaction packs, token-cost computation, MCP-aware schema, ZDR-respecting capture, and the willingness to write `mara setup claude-code` as a first-class CLI command. This document is the honest answer to the question "could someone just use OTel Collector instead?"

## Are we a fork?

No. Three reasons that's a fair claim:

1. **No vendored source.** Every line of code under `crates/` is original. The `NOTICE` file lists inspirations at the pattern level; nothing more.
2. **No protocol clone.** We consume and emit OpenTelemetry Protocol; we do not invent a parallel wire format. See [`plans/01-landscape/04-otel-gen-ai-semconv.md`](../01-landscape/04-otel-gen-ai-semconv.md).
3. **Different scope.** Fluent Bit / Vector / OTel Collector are general-purpose. Mara is intentionally narrow: AI runtime telemetry. The crate organization (`mara-runtime-*` per AI runtime), the canonical schema (`gen_ai.*` typed in Rust), and the policy primitives (vendor-key redaction, token-cost) reflect that narrowness.

## What we share with existing tools (and admit freely)

The competitive analysis is easier when we're honest about what's commodity vs what's actually ours.

- **Pipeline shape:** adapters → policy chain → sinks. Same as OTel Collector. Same as Vector. Same as Fluent Bit. We're not inventing this and shouldn't pretend to.
- **Async runtime:** `tokio`. Same as Vector.
- **OTLP protocol:** identical to OTel Collector.
- **Configuration as data:** TOML/YAML, hot-reloadable. Same as everyone else in the category.
- **Edge-first single binary:** same as Fluent Bit, Vector, OTel Collector agent mode.
- **Apache 2.0 license + DCO:** same as Fluent Bit, OTel Collector, Grafana Alloy.

If those were the whole product, we'd be a fork (or worse, a worse-engineered re-implementation).

## What is actually ours

These are not capabilities other tools couldn't add. They're capabilities other tools don't ship out of the box, where "out of the box" is the difference between five minutes and five days.

### 1. Six AI runtime presets shipping in source

`crates/mara-runtime-claude-code/`, `crates/mara-runtime-codex/`, `crates/mara-runtime-cursor/`, `crates/mara-runtime-kimi/`, `crates/mara-runtime-augment/`, `crates/mara-runtime-gemini/`. Each knows:

- The runtime's transcript location (per-OS).
- The runtime's OTel env vars or hooks API.
- The runtime's ZDR toggle names.
- A working default configuration template.

In OTel Collector: you'd hand-write a `receiver`, a series of `processor`s, an `exporter`, and figure out the runtime's quirks yourself. In Vector: same picture with VRL. In Fluent Bit: same with Lua. In Mara: `mara setup claude-code`.

This is curated AI knowledge, codified as configuration. It is the only thing other tools cannot trivially replicate by adding a generic component.

### 2. AI-vendor secret detection as a first-class redaction primitive

`crates/mara-policy/src/builtin/redact.rs::Pack::builtin_pii()` ships nine patterns including `anthropic-key` (`sk-ant-*`), `openai-key` (`sk-proj-*`, `sk-*`), `github-token` (`ghp_*`, `gho_*`, `ghs_*`, `ghu_*`, `ghr_*`), AWS access keys, Slack tokens, JWT. MVP adds Bedrock and GCP service-account JSON.

OTel Collector ships an `attributes` processor that can apply regex; you'd have to write the regex yourself. Fluent Bit has filters; same story. Mara has the curated set, signed via cosign in Option C.

### 3. Token-based cost computation as a built-in policy

`crates/mara-policy/src/builtin/cost.rs` (MVP work) carries a price table for Claude / GPT / Bedrock / Vertex AI and emits `mara.cost.usd` when token usage is present but vendor cost is absent. The price table is signed and updated as a separate bundle.

No general-purpose shipper has this. Inference proxies like LiteLLM / Helicone / Portkey do it, but they sit in the request path, which means they can't see CLI-local AI runtimes.

### 4. MCP semantic conventions first-class

The `mara-schema::Mcp` struct (`mcp.client.*`, `mcp.server.*`, `mcp.tool.*`, `mcp.transport`) is wired through every adapter and sink. When the OTel MCP semconv stabilises, we're already aligned. When it churns, we update the one struct.

OTel Collector contrib will catch up here; it always does. The gap is six to eighteen months.

### 5. ZDR-aware capture defaults

Capture of prompt content and raw API bodies is gated by *two* opt-ins: the runtime's own toggle (`OTEL_LOG_USER_PROMPTS=true` for Claude Code, `log_user_prompt = true` for Codex, `logPrompts: true` for Gemini) *and* Mara's `mara.policy.capture_optin = true`. Either being false suppresses capture.

No generic shipper has the runtime knowledge to map its own toggle to a vendor toggle. This is the kind of detail that makes the five-minute setup possible.

### 6. Single Rust binary with the *AI* scope

Vector is Rust and is single-binary. We are not displacing Vector. Vector is generic; we are AI-specific. A Vector instance configured for AI ingestion takes a hundred lines of VRL, a custom redaction pack maintained by you, and a price table you wrote yourself. A Mara instance configured for AI ingestion is `mara setup claude-code`.

## The moat, honestly assessed

### Strong moat (held in MVP through v1.x)

- **Curated AI knowledge under Apache 2.0.** Vendors releasing AI runtimes don't write Mara-compatible configs; we do. The community contributes runtime-preset crates back upstream. Each new runtime is a Mara crate, not user configuration.
- **Persona-fit UX.** Persona 2 (indie developer) chooses Mara because it works in five minutes. OTel Collector takes an afternoon. Vector takes a day.
- **License + governance posture.** Apache 2.0 + CNCF Sandbox track is a real differentiator vs Langfuse (MIT, vendor-led), Phoenix (Elastic License v2), Helicone (Apache, vendor-led).

### Weak moat (could erode in 6–18 months)

- **Schema alignment.** OTel `gen_ai.*` is the standard everyone is converging on; Mara doesn't own it.
- **Pipeline architecture.** Pattern-level, not code-level. Anyone can re-implement.
- **Redaction primitives.** A motivated OTel Collector contrib processor could ship most of what Mara's `builtin.pii` pack does in a quarter.

### What MVP does to defend the moat

1. **Run faster than upstream.** Ship a new runtime preset every release; six runtimes in v1.0 is harder for OTel Collector contrib to match than one or two.
2. **Curate visibly.** Maintain `plans/01-landscape/08-ai-runtime-telemetry-surfaces.md` as living research the community reads. Be the encyclopedia, not just the binary.
3. **Be small enough to be opinionated.** Mara's 24 crates × 100 files << OTel Collector contrib's complexity. Smaller scope = faster decisions = better UX for the narrow seat.
4. **Win on developer ergonomics.** `mara setup`, `mara diag`, `mara test pipeline`, a Homebrew tap, beautiful errors. Generic tools optimize for generality; Mara optimizes for "I want to use Claude Code locally and see my sessions."
5. **Open ArdurAI's runtime intel.** When we figure out (the hard way) that Cursor truncates hook payloads at 16 KiB and chunks beyond that, document it publicly in `plans/01-landscape/08-...md`. The encyclopedia is the moat.

### What happens if the moat erodes anyway

We've over-invested in curation and under-invested in lock-in. If OTel Collector contrib catches up:

- Mara graduates into an OTel Collector distribution flavor.
- The curated knowledge migrates upstream as a processor.
- ArdurAI's commercial gateway (v2/v3) is the actual product moat; the OSS edge agent becomes a feeder.

This is fine. The OSS edge agent is a strategic gift to the community, not the entire business.

## Concrete moat tests for MVP

1. **The five-minute test (SC-1).** If Mara takes longer to set up than OTel Collector, the moat is gone — we're a worse Collector.
2. **The "OTel Collector contrib equivalent" challenge.** Periodically (every release), reproduce Mara's Claude Code pipeline using only upstream OTel Collector contrib processors. Time it. Compare lines-of-configuration. The gap is our moat in concrete units.
3. **Community runtime contributions.** If outside contributors are willing to write `mara-runtime-<thing>` crates (and we get the API ergonomic enough that they can), we're winning. Target: ≥2 external runtime presets in v1.x.

## How we talk about it

- **Honest:** "Mara is OpenTelemetry-aligned, edge-deployed, and specialized for AI runtimes. We don't replace OTel Collector; we're the AI-specialist sibling."
- **Avoid:** "Mara is faster than Fluent Bit / Vector / OTel Collector" — we have no data showing this and probably never will.
- **Avoid:** "Mara replaces Langfuse / Phoenix / LangSmith" — they're sinks, we feed them.
- **Avoid:** "Mara is a fork of [anything]" — we aren't. It's an original implementation.

## Cross-references

- [`../03-value-proposition/01-positioning-statement.md`](../03-value-proposition/01-positioning-statement.md) — full positioning for v1.0.
- [`../03-value-proposition/02-feature-matrix.md`](../03-value-proposition/02-feature-matrix.md) — feature-by-feature comparison.
- [`../03-value-proposition/03-unique-value-claims.md`](../03-value-proposition/03-unique-value-claims.md) — testable claims.
- [`../01-landscape/01-classic-log-shippers.md`](../01-landscape/01-classic-log-shippers.md) — generic shipper landscape.
- [`../01-landscape/03-ai-llm-observability-tools.md`](../01-landscape/03-ai-llm-observability-tools.md) — AI-obs landscape.
- [`04-ai-native-features.md`](04-ai-native-features.md) — what specific AI features ship in MVP.
