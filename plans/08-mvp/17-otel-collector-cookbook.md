# MVP — OpenTelemetry Collector Cookbook

## Executive summary

The fairest competitive question Mara has to answer is: "Why not just use the OpenTelemetry Collector contrib distribution?" This document answers concretely. For each MVP capability, it shows the OTel Collector configuration you'd write to do the same thing, with line counts, processor stack, operational notes, and known gaps. The conclusion is not that Collector can't do these things — it almost always can — but that Mara ships them curated, with AI-runtime knowledge baked into a 5-minute setup, and with documentation, presets, and tests for the AI-specific cases. The cookbook makes the trade-off explicit so operators can choose with eyes open.

The exercise also serves as the moat test described in [`09-differentiation-and-moat.md`](09-differentiation-and-moat.md): if Collector-with-config-only ever reaches parity, Mara's value evaporates and we should graduate into a Collector distribution. Until then, the gap is real and measurable.

## Methodology

For each Mara capability, this document shows:

1. **What Mara does** — terse summary.
2. **What it looks like in `mara.toml`** — the actual Mara config the operator writes.
3. **What it looks like in OpenTelemetry Collector contrib's `config.yaml`** — the equivalent operator configuration, using only upstream processors and exporters.
4. **Effort difference** — lines of config, conceptual knowledge required, install steps.
5. **Where Collector wins** — honest acknowledgement of capabilities Collector covers better.
6. **Where Mara wins** — what Mara delivers that Collector cannot, or cannot without significant operator-side work.

Collector version assumed: contrib `v0.150.0` (March 2026). Mara version: target `v0.2.0-alpha`.

## Capability 1 — Receive Claude Code OTLP HTTP and forward to Honeycomb

### What Mara does

Binds an OTLP HTTP receiver on `127.0.0.1:4318`, normalizes against `gen_ai.*` semantic conventions, applies the built-in PII redaction pack, computes `mara.cost.usd` from token counts, ships to Honeycomb via OTLP HTTP.

### Mara config

```toml
schema_version = "1"

[[adapters.otlp]]
name = "ingest"
http_listen = "127.0.0.1:4318"

[[policies.default]]
type = "redact"
pack = "builtin.pii"

[[sinks.otlp]]
name = "honeycomb"
endpoint = "https://api.honeycomb.io"
protocol = "http"
headers = { "x-honeycomb-team" = "${HONEYCOMB_API_KEY}" }

[[pipelines]]
name = "primary"
adapters = ["ingest"]
policy_chain = "default"
sinks = ["honeycomb"]
```

Plus: `mara setup claude-code` writes this automatically and emits the env-var instructions the user needs.

**Lines of operator-written config:** 0 (after `mara setup claude-code`); 18 lines if hand-written.

### OpenTelemetry Collector contrib equivalent

```yaml
receivers:
  otlp:
    protocols:
      http:
        endpoint: 127.0.0.1:4318

processors:
  attributes/redact_email:
    actions:
      - key: gen_ai.prompt.0.content
        action: update
        pattern: '\b[a-z0-9._%+-]+@[a-z0-9.-]+\.[a-z]{2,}\b'
        value: '[email]'
  attributes/redact_anthropic_key:
    actions:
      - key: gen_ai.prompt.0.content
        action: update
        pattern: '\bsk-ant-[A-Za-z0-9\-_]{20,}\b'
        value: '[anthropic-key]'
  attributes/redact_openai_key:
    actions:
      - key: gen_ai.prompt.0.content
        action: update
        pattern: '\bsk-(?:proj-)?[A-Za-z0-9\-_]{20,}\b'
        value: '[openai-key]'
  # ... and 6 more attributes processors for each pattern, each scoped to each prompt/completion field path you care about
  transform/compute_cost:
    log_statements:
      - context: log
        statements:
          - set(attributes["mara.cost.usd"], attributes["gen_ai.usage.input_tokens"] * 0.000003 + attributes["gen_ai.usage.output_tokens"] * 0.000015) where attributes["gen_ai.request.model"] == "claude-sonnet-4-5-20250929"
          # ... and one statement per model you want to price

exporters:
  otlphttp/honeycomb:
    endpoint: https://api.honeycomb.io
    headers:
      x-honeycomb-team: ${env:HONEYCOMB_API_KEY}

service:
  pipelines:
    logs/primary:
      receivers: [otlp]
      processors: [attributes/redact_email, attributes/redact_anthropic_key, attributes/redact_openai_key, transform/compute_cost]
      exporters: [otlphttp/honeycomb]
```

**Lines of operator-written config:** ~50 lines for a minimal version; **~150 lines** for a redaction pack as complete as Mara's nine built-in patterns, scoped to all the field paths where prompt content can appear (`gen_ai.prompt.0.content`, `gen_ai.prompt.1.content`, `gen_ai.completion.0.content`, free-form `body`, etc.).

Plus: the operator needs to know:

- Which `gen_ai.*` field paths Claude Code emits prompt content at.
- The pricing for each Anthropic model and how to keep it updated.
- The OTTL syntax for the `transform` processor.
- That `attributes` processor regex needs to be scoped per field-path; there is no "regex all string attributes" mode.
- That capture is gated by Claude Code env vars; Collector cannot enforce this on the receiving side.

### Effort difference

- **Operator config:** Mara 0–18 lines vs Collector ~150 lines.
- **Knowledge required:** Mara: which sink. Collector: OTTL, attribute paths, model pricing, regex engineering, processor ordering.
- **First-event-to-dashboard time:** Mara ≤ 5 minutes (sign-off criterion). Collector: half a day to a day for someone proficient; a week for a first-timer.
- **Maintenance:** Mara updates redaction packs and price tables via `mara` releases. Collector: operator hand-maintains.

### Where Collector wins

- **Generality.** Collector handles non-AI signals (host metrics, app traces) in the same pipeline. Mara doesn't.
- **Connector / fanout patterns** (e.g., the `routing` connector) are richer than Mara's pipeline model.
- **Sampling.** Collector's `tail_sampling` processor is more sophisticated than Mara's `HeadSampler`.

### Where Mara wins

- **AI knowledge baked in.** Operator doesn't write redaction regex; we ship the pack.
- **Cost computation baked in.** Operator doesn't write per-model pricing OTTL; we ship the table.
- **Quickstart.** `mara setup claude-code` is one command vs hand-writing ~150 lines of YAML.
- **ZDR awareness.** Mara documents which runtime env vars to set to gate prompt capture; Collector treats this as the operator's problem.

## Capability 2 — Observe Ollama via HTTP proxy

### What Mara does

Binds `127.0.0.1:11434`, forwards to Ollama on `:11435`, captures both request and response bodies for every endpoint (native `/api/*` and OpenAI-compat `/v1/*`), normalizes the response telemetry fields (`prompt_eval_count`, `eval_count`, `*_duration`) into `gen_ai.usage.*` and `mara.ollama.*`, emits `mara.cost.usd = 0` and `mara.compute.is_local = true`.

### Mara config

```toml
[[adapters.llm_proxy]]
name = "ollama"
listen = "127.0.0.1:11434"
upstream = "127.0.0.1:11435"
runtime = "ollama"

[[pipelines]]
name = "ollama_pipeline"
adapters = ["ollama"]
policy_chain = "default"
sinks = ["honeycomb"]
```

Plus: `mara setup ollama` writes this automatically and emits the `OLLAMA_HOST=127.0.0.1:11435` reconfig instructions.

**Lines of operator-written config:** 0 (after `mara setup ollama`); 8 lines if hand-written.

### OpenTelemetry Collector contrib equivalent

**Doesn't exist.** Collector contrib has no HTTP-proxy receiver in its receiver set as of `v0.150.0`. The closest options:

1. **Wrap Ollama with a separate proxy like Caddy or nginx, configured to log responses, and tail the log via Collector's `filelog` receiver.** Requires: a separate proxy server, log format design that captures JSON request and response bodies, a parser (`json_parser` operator) that knows Ollama's response shape, OTTL transformations to map `prompt_eval_count` → `gen_ai.usage.input_tokens`, and the entire `mara.ollama.*` extension namespace populated manually. **Estimated**: 80-150 lines of nginx config + 120-200 lines of Collector config + a JSON-logging strategy that doesn't blow nginx config out.
2. **Run LiteLLM in front of Ollama as a translation proxy, with LiteLLM's OTel exporter pointed at Collector.** Adds a process, adds a Python runtime, adds a dependency on LiteLLM's mapping fidelity to `gen_ai.*`. **Estimated**: LiteLLM install + 30 lines of LiteLLM config + 20 lines of Collector config.
3. **Write a custom Collector receiver in Go.** Not realistic for an operator; this is a Collector-development task, not an operator task. Estimated: 1-2 weeks of Go development by someone who knows the Collector contrib codebase.

### Effort difference

For an operator who wants Ollama observability and is unwilling to add LiteLLM to their dependency tree:

- **Mara:** one command, zero config.
- **Collector + nginx + filelog:** half a day of config writing + ongoing maintenance of the nginx-log-to-canonical pipeline.

For an operator who is willing to run LiteLLM:

- **Mara:** one command.
- **LiteLLM + Collector:** 1-2 hours of setup; ongoing maintenance of LiteLLM's pricing tables (which they actually do maintain, well).

### Where Collector wins

- **LiteLLM is a real and well-maintained option.** Operators already running LiteLLM as their gateway will find this easier than adding Mara.
- **Operators who need full OpenAI-compat translation** (calling multiple LLM vendors via one SDK) benefit from LiteLLM's translation layer in a way Mara doesn't replicate.

### Where Mara wins

- **No additional process or runtime.** Mara is one Rust binary. LiteLLM adds Python + LiteLLM + (typically) a Postgres for state.
- **Lower latency floor.** Mara on the loopback adds ~50 μs; LiteLLM adds Python interpreter overhead and a network hop.
- **First-class `mara.compute.is_local` and `mara.ollama.*` extensions.** LiteLLM treats Ollama as one of many vendors; Mara surfaces local-inference-specific telemetry deliberately.

## Capability 3 — Self-telemetry and `mara diag`

### What Mara does

Exposes `127.0.0.1:9099/metrics` and `/healthz` for the agent itself. `mara diag` reads from those endpoints (if running) or inspects state on disk.

### Mara config

Default — no operator action.

### OTel Collector equivalent

The Collector's `telemetry` configuration block enables its own internal metrics on the Prometheus exposition port:

```yaml
service:
  telemetry:
    metrics:
      level: detailed
      address: 127.0.0.1:8888
```

**Lines of config:** 5.

No equivalent of `mara diag` exists; operators query the Prometheus endpoint with `curl` or `promtool query`.

### Where Mara wins

- **Single-command diagnostic.** `mara diag` is human-readable out of the box; the Collector requires the operator to interpret Prometheus output.

### Where Collector wins

- **Mature exposition.** The Collector's internal metrics catalog is broader than Mara's MVP set.

## Aggregate comparison

For the MVP scope specifically:

| Capability | Mara LoC (operator) | Collector LoC (operator) | Mara setup time | Collector setup time |
|---|---|---|---|---|
| Receive Claude Code OTLP → Honeycomb with PII redaction + cost | 0 (post-setup) | ~150 | ≤ 5 min | half a day |
| Ollama observability (proxy) | 0 (post-setup) | ~200 (filelog approach) or 50 (LiteLLM approach) | ≤ 5 min | half a day / 1-2 hours |
| Self-telemetry + diag | 0 | 5 | 0 | 0 |
| **Total** | **0** | **~205 — ~355** | **≤ 10 min** | **~1 day** |

For an operator whose job is "ship AI observability for our Claude Code + Ollama setup this week," Mara cuts the work from a day of config-writing to 10 minutes.

For an operator whose job is "set up our team's entire OTel pipeline including app traces, host metrics, and AI signals," Collector remains the right tool — and Mara feeds into it via the OTLP sink without conflict.

## When the operator should pick Collector instead

Honest recommendation for when Mara is the wrong choice:

- **You already run an OTel Collector and have a strong internal OTTL / OPA / regex authoring practice.** Adding Mara is an extra dependency you don't need.
- **You need to observe LLM traffic from inside your own application code where you can drop a Python / JS / Go OTel SDK.** Use the SDK + Collector; you don't need Mara as the input layer.
- **You're optimizing for a single, opinionated stack** (e.g., pure Datadog with Datadog Agent everywhere). Stay on the vendor's path.
- **Your AI workload isn't on local-binary AI runtimes at all** — pure Bedrock or pure OpenAI SaaS. Mara's gap is "telemetry from AI tools whose code you don't own." If you do own the code, your SDK + Collector covers it.

We document these scenarios in the README so operators self-select. Mara claiming universal applicability would be dishonest.

## When the operator should pick Mara

- **You use Claude Code, Codex, Cursor, Kimi, Augment, Gemini CLI, or Ollama and want observability for activity that doesn't go through your application code.**
- **You want a 5-minute setup rather than a 1-day OTel Collector configuration project.**
- **You want AI-vendor-aware redaction (Anthropic / OpenAI / GCP / AWS keys) without writing the regex yourself.**
- **You want token-based cost computation without maintaining a price table.**
- **You want an edge-first single Rust binary with no JVM, no Python, no Node runtime cost.**

These are the value claims; the cookbook above is their evidence.

## Maintenance commitment

This cookbook is regenerated when:

- OpenTelemetry Collector contrib ships a new processor relevant to the comparison.
- Mara ships a feature that closes or widens a gap.
- A user reports that Collector now does something the cookbook says it can't.

Outdated comparisons are dishonest, so the cookbook stays current or it gets removed.

## Cross-references

- [`09-differentiation-and-moat.md`](09-differentiation-and-moat.md) — the strategic framing this cookbook concretely evidences.
- [`03-language-choice.md`](03-language-choice.md) — why Mara is Rust and Collector is Go.
- [`../01-landscape/01-classic-log-shippers.md`](../01-landscape/01-classic-log-shippers.md) — broader shipper landscape including Collector.
- [`../03-value-proposition/02-feature-matrix.md`](../03-value-proposition/02-feature-matrix.md) — feature-by-feature comparison.
