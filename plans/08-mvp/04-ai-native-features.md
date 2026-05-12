# MVP — AI-Native Features

## Executive summary

"AI-native" is the differentiator. A generic log shipper bent into AI duty (Fluent Bit + a regex transform, Vector + a VRL script) covers maybe 30 % of what an AI workload operator actually needs. The other 70 % is the specific knowledge that AI runtimes emit specific shapes, that AI vendors charge per token in specific ways, that AI tools call MCP servers in specific patterns, and that an operator's compliance team has specific obligations about prompt content. This document enumerates the AI-specific behaviour Mara ships in MVP and points to where each lives in the codebase.

The MVP doesn't ship every AI-specific feature in the encyclopedia — only the subset that closes the value-claim loop for the indie-developer persona. The rest stays scaffolded and ships in MVP+1 / MVP+2.

## AI features shipping in MVP

### 1. OpenTelemetry `gen_ai.*` canonical schema

Mara's canonical `Event` type is structured around the OpenTelemetry generative-AI semantic conventions (and the related MCP attribute conventions). Adapters normalise into this shape; sinks consume it. No invented parallel namespace.

**Lives in:** [`crates/mara-schema/src/lib.rs`](../../crates/mara-schema/src/lib.rs) — already done in M1.

**Why it matters for AI:** every observability backend that adopts `gen_ai.*` (Honeycomb, Logfire, Datadog LLM Obs, Grafana stack with appropriate dashboards) can consume Mara's output natively. Operators don't have to re-shape their dashboards when they adopt Mara.

### 2. AI vendor secret redaction

The built-in PII pack redacts Anthropic, OpenAI, GitHub, AWS, GCP, Slack tokens and JWTs in addition to standard PII (email, phone, SSN). For MVP we add explicit detection of:

- `sk-ant-*` (Anthropic API keys).
- `sk-proj-*` and `sk-*` (OpenAI / project keys).
- `xoxb-*` / `xoxa-*` (Slack bot/app tokens).
- `ghp_*`, `gho_*`, `ghs_*`, `ghu_*`, `ghr_*` (GitHub tokens).
- Generic JWT (`eyJ*.eyJ*.<sig>`).
- AWS access keys (`AKIA*`, `ASIA*`).

**Lives in:** [`crates/mara-policy/src/builtin/redact.rs`](../../crates/mara-policy/src/builtin/redact.rs) — already partially done; MVP completes the pack and adds golden tests against synthetic key payloads.

**Why it matters for AI:** the most common leak vector for AI workloads is "developer pasted a secret into a prompt, the prompt ended up in logs." Redacting at the agent boundary, before sink dispatch, is the only architectural defence that doesn't depend on the sink behaving.

### 3. Token-based cost computation

Vendors don't always emit `gen_ai.usage.cost`. They almost always emit token counts. Mara ships a built-in price table for the major models and computes `mara.cost.usd` from `gen_ai.usage.input_tokens × price_in + gen_ai.usage.output_tokens × price_out + cached_tokens × price_cached + reasoning_tokens × price_reasoning` when the cost field is absent.

**Lives in:** new module `crates/mara-policy/src/builtin/cost.rs` (MVP work).

**Why it matters for AI:** every AI workload operator wants real-time cost visibility. Vendors leave a gap; Mara fills it. Price tables are signed and versioned; updates roll out with policy bundles in Option C.

### 4. ZDR-aware capture defaults

Prompt content, completion content, and raw API bodies are NOT captured by default. Capture requires two opt-ins:

- Runtime-side: `OTEL_LOG_USER_PROMPTS=true` (Claude Code) or equivalent.
- Mara-side: `mara.policy.capture_optin = true` per pipeline.

When either is false, content is hashed (`mara.body.prompt_hash`, etc.) and structural metadata is preserved.

**Lives in:** [`crates/mara-runtime-claude-code/src/lib.rs`](../../crates/mara-runtime-claude-code/src/lib.rs) (env-var names defined); enforcement during normalization in MVP-stage OTLP receiver.

**Why it matters for AI:** Zero Data Retention is a vendor promise; Mara turns it into a technical guarantee at the operator's premises. Regulated industries cannot ship Mara without this.

### 5. Runtime preset (Claude Code) end-to-end

`mara setup claude-code` writes a runnable config that, with no further editing, captures Claude Code events when the user sets the standard OTel env vars. Per [`../07-quickstarts/01-claude-code.md`](../07-quickstarts/01-claude-code.md), the user does:

```bash
brew install mara
mara setup claude-code
brew services start mara
export CLAUDE_CODE_ENABLE_TELEMETRY=1
export OTEL_EXPORTER_OTLP_PROTOCOL=http/protobuf
export OTEL_EXPORTER_OTLP_ENDPOINT=http://127.0.0.1:4318
```

…and starts seeing events.

**Lives in:** `mara-cli::setup` (MVP), `mara-runtime-claude-code` (config template), `mara-adapter-otlp` (receiver).

**Why it matters for AI:** the indie developer persona evaporates if first-event-to-dashboard is > 5 minutes. The whole MVP exists for this scripted path.

### 5b. Runtime preset (Ollama) end-to-end via HTTP proxy

`mara setup ollama` reconfigures Ollama to listen on port 11435 (via `launchctl setenv` on macOS or a systemd override on Linux) and tells Mara to bind 11434 as a transparent proxy. AI clients (the `ollama` CLI, Open WebUI, Continue.dev, OpenAI-SDK code) keep pointing at the conventional port; Mara observes every request/response pair and emits canonical events with token counts, latency breakdown, model identity, and tokens-per-second. Local inference is cost-zero but rich in telemetry: per-request `mara.ollama.total_duration_ms`, `mara.ollama.eval_duration_ms`, `mara.ollama.tokens_per_sec`, and `mara.compute.is_local = true`.

**Lives in:** `mara-adapter-llm-proxy` (proxy), `mara-runtime-ollama` (normalizer), `mara-cli::setup` (preset writer). Detailed in [`12-ollama-integration-design.md`](12-ollama-integration-design.md).

**Why it matters for AI:** local LLM observability is the gap nobody covers. Cloud-side observability tools (Datadog, Honeycomb, Langfuse) cannot see Ollama because Ollama traffic never leaves the laptop. Mara on the loopback interface is the only feasible observability path for local inference, and the integration shape (HTTP proxy with response capture) generalizes to OpenAI-compat upstreams in MVP+1.

### 6. MCP attributes preserved

When a Claude Code session uses an MCP tool, the `mcp.*` attributes (server name, version, tool name, transport) survive through the pipeline. MVP doesn't add MCP-specific dashboards or queries; it preserves the attributes so existing OTel-aware sinks can.

**Lives in:** OTLP receive path in `mara-adapter-otlp` (MVP). Already present in `mara-schema::Mcp`.

**Why it matters for AI:** MCP is becoming the standard for AI tool invocation. Observability backends are starting to add MCP-aware views. Mara preserving these attributes day-one means MCP visibility is automatic for anyone with a modern sink.

### 7. Cardinality-bounded Loki sink

Loki sinks naively configured for AI workloads explode in cardinality (prompt content as labels). Mara's Loki sink ships with a strict default: labels are `runtime` and `event_kind` only; everything else is structured metadata (Loki 3.x feature) or in the line body.

**Lives in:** `mara-sink-loki` (MVP work).

**Why it matters for AI:** verbose LLM traffic crushes naive Loki configurations. Operators expect Mara to know how to ship to Loki without inducing a TSDB outage.

### 8. Self-telemetry that doesn't leak

Mara's own metrics (`mara_pipeline_events_total`, `mara_sink_errors_total`, etc.) never include user event content. The contract is explicit: self-telemetry is for operators of Mara, not for the AI workload.

**Lives in:** `mara-core::self_telemetry` (MVP work). Endpoint at `127.0.0.1:9099`.

**Why it matters for AI:** an AI-workload telemetry agent that exfiltrates content via its own metrics labels would be a comedy of errors. We pre-empt the failure mode by contract.

## AI features explicitly NOT in MVP

These are valuable, AI-specific, and shipping in MVP+1 or later. They are not deleted; they're scoped.

- **PHI redaction pack** (`builtin.phi`) — HIPAA-aware. Ships in MVP+1.
- **PCI redaction pack** (`builtin.pci`) — cardholder data. Ships in MVP+1.
- **Locale-specific PII packs** (EU, JP, IN, CN). Ships in MVP+2.
- **Multi-tenant policy selection.** Ships in Option B.
- **Tamper-evident audit log of policy decisions.** Ships in Option C.
- **Cost-aware sampling** (keep all events for sessions over $1, sample 10 % otherwise). Ships in MVP+2.
- **Agent-loop reconstruction** for sub-agent fan-out where trace context drops. Ships in Option C.
- **MCP-aware routing** (route to different sinks based on tool catalog). Ships in Option C.
- **Eval result correlation** (link eval pipeline output back to the source session). Ships in MVP+2.
- **Embedding ingest** (capture embedding requests as a distinct event kind). Ships in MVP+2.
- **WASM bundle hosting** for org-specific redaction packs. Ships in Option C.

## How AI-specific features compose with generic features

MVP delivers a useful product because the AI-specific features compose with the generic-shipper features we already have:

- Pipeline scheduler is generic; AI knowledge lives in the adapters, the policy pack, and the canonical schema.
- File sink is generic; cost-bearing events flow through it unchanged.
- The TOML config is generic; the `redact` policy stage referenced in config is AI-aware.
- The CLI is generic; `mara setup claude-code` is AI-aware.

This separation is deliberate. The generic core means a future contributor can ship a non-AI sink (e.g., an exotic SIEM forwarder) without touching the AI-specific knowledge, and vice versa.

## Sales / marketing implication

The MVP can credibly claim:

- "AI-native": OTel `gen_ai.*` canonical, AI-vendor-aware redaction, token-based cost computation, ZDR-respecting capture, runtime-aware preset.
- "Edge-first": single Rust binary, ≤128 MiB RSS idle, no SaaS account required.
- "Vendor-neutral": OTLP + Loki HTTP at MVP; more sinks in MVP+1.
- "Open": Apache 2.0; CI signs every release.

The MVP cannot yet credibly claim:

- "All six AI runtimes covered." Only Claude Code at MVP.
- "Signed policy bundles." Built-in only.
- "Tamper-evident audit log." Not in MVP.
- "v1.0." Pre-1.0; explicit in `CHANGELOG.md`.

We say what we ship, not what we plan.

## Cross-references

- [`../03-value-proposition/03-unique-value-claims.md`](../03-value-proposition/03-unique-value-claims.md) — the seven value claims.
- [`../04-implementation/04-data-model.md`](../04-implementation/04-data-model.md) — full schema.
- [`../02-gaps/04-policy-and-redaction-gaps.md`](../02-gaps/04-policy-and-redaction-gaps.md) — what redaction is hard.
- [`../01-landscape/04-otel-gen-ai-semconv.md`](../01-landscape/04-otel-gen-ai-semconv.md) — semconv state of the art.
