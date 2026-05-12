# MVP — Migration Guides

## Executive summary

Most early Mara adopters will be operators who have already built some kind of AI telemetry pipeline using existing tools — Fluent Bit + custom Lua, Vector + VRL transforms, OpenTelemetry Collector + custom OTTL, LiteLLM + their own observability backend, or hand-rolled scripts that tail Claude Code transcripts into a file. They are not greenfield users; they have working systems that almost solve their problem. Mara's job is to make the migration cost lower than the cost of continuing to maintain their workaround. This document is concrete: side-by-side configurations, migration scripts where useful, what survives the move and what doesn't.

Three migration paths covered for MVP: **Fluent Bit + Lua AI pipeline**, **Vector + VRL AI pipeline**, **OpenTelemetry Collector + OTTL AI pipeline**. A fourth (**LiteLLM proxy + custom observability**) is sketched but deferred because it requires the post-MVP `llm-proxy` adapter generalization.

These migration guides are operational artifacts. The strategic comparison lives in [`17-otel-collector-cookbook.md`](17-otel-collector-cookbook.md) and [`../03-value-proposition/02-feature-matrix.md`](../03-value-proposition/02-feature-matrix.md).

## When to migrate

Reasonable triggers:

- The custom pipeline's Lua / VRL / OTTL is hard to maintain after a personnel change.
- The custom pipeline doesn't survive an AI runtime upgrade (Claude Code 0.42 → 0.46 broke the regex).
- The team wants cost computation, AI-vendor key redaction, and MCP attribute capture without writing them.
- A new AI runtime joins the stack (e.g., adding Ollama to a previously Claude-only setup) and the custom pipeline doesn't generalize.

Unreasonable triggers:

- "Mara is shinier." Wait until a real pain point bites.
- "The current pipeline works but I want to consolidate vendors." Maybe; weigh the migration cost.

## Migration 1 — From Fluent Bit + Lua AI pipeline

### What the existing setup looks like

Fluent Bit configured to tail `~/.claude/projects/*.jsonl`, transform records with a Lua filter that pulls out token counts and redacts secrets, output to Loki via the `loki` output plugin. Configuration file in `/etc/fluent-bit/fluent-bit.conf`:

```ini
[SERVICE]
    flush        1
    daemon       Off
    log_level    info
    parsers_file parsers.conf

[INPUT]
    Name              tail
    Path              ~/.claude/projects/**/*.jsonl
    Parser            json
    Tag               claude_code
    Refresh_Interval  5
    Mem_Buf_Limit     50MB
    Skip_Long_Lines   On

[FILTER]
    Name    lua
    Match   claude_code
    script  redact_and_normalize.lua
    call    transform

[OUTPUT]
    Name    loki
    Match   claude_code
    Host    logs-prod-006.grafana.net
    Port    443
    TLS     On
    HTTP_User <user>
    HTTP_Passwd ${GRAFANA_CLOUD_API_KEY}
    Labels  job=claude_code, runtime=claude_code
```

And `redact_and_normalize.lua`:

```lua
function transform(tag, timestamp, record)
    -- Redact common patterns
    for k, v in pairs(record) do
        if type(v) == "string" then
            v = string.gsub(v, "sk%-ant%-[%w%-_]+", "[anthropic-key]")
            v = string.gsub(v, "sk%-proj%-[%w%-_]+", "[openai-key]")
            v = string.gsub(v, "[%w%.%-_+]+@[%w%.%-]+%.[%w]+", "[email]")
            record[k] = v
        end
    end
    -- Normalize token usage
    if record.gen_ai_usage_input_tokens then
        record["gen_ai.usage.input_tokens"] = record.gen_ai_usage_input_tokens
    end
    return 2, timestamp, record
end
```

### What changes when moving to Mara

Replace the Fluent Bit config with a Mara config:

```toml
schema_version = "1"

[[adapters.otlp]]
name = "claude_code_otlp"
http_listen = "127.0.0.1:4318"

[[adapters.jsonl]]
name = "claude_code_redundant"
globs = ["~/.claude/projects/**/*.jsonl"]
checkpoint_path = "~/.local/state/mara/claude_code"

[[policies.default]]
type = "redact"
pack = "builtin.pii"

[[sinks.loki]]
name = "grafana_cloud"
url = "https://logs-prod-006.grafana.net/loki/api/v1/push"
auth = { type = "basic", username = "<user>", password = "${GRAFANA_CLOUD_API_KEY}" }
labels = ["runtime", "event_kind"]

[[pipelines]]
name = "claude_code"
adapters = ["claude_code_otlp", "claude_code_redundant"]
policy_chain = "default"
sinks = ["grafana_cloud"]
```

And run:

```bash
export CLAUDE_CODE_ENABLE_TELEMETRY=1
export OTEL_EXPORTER_OTLP_PROTOCOL=http/protobuf
export OTEL_EXPORTER_OTLP_ENDPOINT=http://127.0.0.1:4318
```

### What you gain

- The Lua script is gone. Redaction comes from `builtin.pii` (nine patterns including the three you had, plus six more).
- Token-based cost computation, which the old setup didn't have, comes free if you enable the `cost` policy stage.
- OTLP receive in addition to JSONL tail (more reliable; Claude Code structurally emits OTel when telemetry is enabled).
- A clear schema (`gen_ai.*`) instead of the ad-hoc attribute renaming you had.
- Self-telemetry on `:9099` — Fluent Bit has its own metrics but the data shape differs.

### What you lose

- Fluent Bit's vast input plugin set. If you used Fluent Bit for non-AI inputs (host metrics, syslog, k8s pod logs), those don't move to Mara — Mara is AI-specific. Keep Fluent Bit (or OTel Collector) for those.
- Lua flexibility. Mara's built-in policies cover the common case; custom redaction needs `mara-policy-sdk-rust` (a WASM plugin in v1.x) or a feature request.
- Memory budget familiarity. Fluent Bit's `Mem_Buf_Limit` semantics don't have an exact Mara equivalent; Mara's bounded channels + WAL behave differently. Run for a week and re-tune.

### Migration script

A `tools/migrate-from-fluent-bit.sh` script (TBD post-MVP) parses a Fluent Bit config and outputs an equivalent Mara config skeleton. For MVP, manual conversion is straightforward enough.

## Migration 2 — From Vector + VRL AI pipeline

### What the existing setup looks like

Vector tailing `~/.claude/projects/*.jsonl`, transforming with VRL, shipping to OTLP. Configuration file in `vector.yaml`:

```yaml
sources:
  claude_code_tail:
    type: file
    include:
      - "~/.claude/projects/**/*.jsonl"
    encoding:
      codec: ndjson

transforms:
  redact_and_normalize:
    type: remap
    inputs: [claude_code_tail]
    source: |
      # Redact secrets
      .message = replace(string!(.message), r'sk-ant-[A-Za-z0-9\-_]{20,}', "[anthropic-key]")
      .message = replace(string!(.message), r'sk-(?:proj-)?[A-Za-z0-9\-_]{20,}', "[openai-key]")
      .message = replace(string!(.message), r'\b[a-z0-9._%+-]+@[a-z0-9.-]+\.[a-z]{2,}\b', "[email]")

      # Set runtime label
      .runtime = "claude_code"

      # Normalize gen_ai usage fields
      if exists(.gen_ai_usage_input_tokens) {
        ."gen_ai.usage.input_tokens" = .gen_ai_usage_input_tokens
      }

sinks:
  otlp:
    type: opentelemetry
    inputs: [redact_and_normalize]
    endpoint: https://api.honeycomb.io
    tls:
      enabled: true
    encoding:
      codec: json
    request:
      headers:
        x-honeycomb-team: "${HONEYCOMB_API_KEY}"
```

### What changes when moving to Mara

Replace `vector.yaml` with the Mara config from Migration 1 above, swap the Loki sink for an OTLP sink targeting Honeycomb:

```toml
[[sinks.otlp]]
name = "honeycomb"
endpoint = "https://api.honeycomb.io"
protocol = "http"
headers = { "x-honeycomb-team" = "${HONEYCOMB_API_KEY}" }
```

### What you gain

- VRL is gone. You write zero transformation code.
- Mara's redaction pack is more comprehensive than the three patterns you had.
- Native `gen_ai.*` field names; the ad-hoc renaming is unnecessary.
- Single Rust binary; same DNA as Vector but specialized.

### What you lose

- VRL's full programmability. Mara's policy stages are typed primitives, not a general scripting language; for custom logic, you write a WASM policy (v1.x) or submit a built-in primitive PR.
- Vector's huge sink ecosystem. Mara ships 10 sinks at v1.0; Vector has 80+. If you ship to an obscure backend, Vector wins.
- Vector's ability to do non-AI workloads in the same agent. Mara is AI-specific.

### Verdict

Vector users move to Mara when their AI pipeline is the painful part. If their pipeline is mostly host metrics + k8s logs + a sprinkle of AI, they keep Vector.

## Migration 3 — From OpenTelemetry Collector + OTTL AI pipeline

This is covered in depth in [`17-otel-collector-cookbook.md`](17-otel-collector-cookbook.md). Brief version:

### What the existing setup looks like

OTel Collector contrib with the `otlp` receiver, several `attributes` and `transform` processors implementing redaction and cost computation, the `otlphttp` exporter.

### What changes when moving to Mara

Replace the Collector configuration with a Mara configuration. The OTLP receive path is identical (both bind 4318); the difference is everything between receive and emit.

### Decision rule

If the OTel Collector pipeline does **only** AI signals and the OTTL/processor stack is more than ~150 lines, Mara is the cheaper substrate. If it does more than just AI signals, keep the Collector and feed it from Mara via OTLP — Mara becomes the AI-specialist input, Collector remains the generic infrastructure.

## Migration 4 — From LiteLLM proxy + custom observability (post-MVP preview)

LiteLLM acts as a proxy in front of multiple LLM vendors and emits OTel. Users often pair it with a custom Collector pipeline that adds redaction and cost normalization on top of LiteLLM's emission.

Post-MVP: the generalized `mara-adapter-llm-proxy` (built for Ollama at MVP) can also proxy OpenAI / Anthropic / Bedrock directly, removing the LiteLLM dependency for the observability use case. LiteLLM remains valuable for the multi-vendor routing / fallback / caching use cases it specializes in; Mara doesn't try to displace those.

Detailed migration guide deferred until the proxy generalization ships in MVP+2.

## Cross-runtime migration: from "I instrument my Python app" to "I observe the runtime"

A subtle migration: many developers add `openinference-instrumentation-anthropic` or `opentelemetry-instrumentation-openai` to their Python app and ship OTel traces. That works for code they own. It doesn't capture Claude Code on the laptop, Cursor in the IDE, or Ollama via the CLI.

Migration is not "rip out the SDK"; it's "add Mara alongside, so app code and AI-runtime traffic share an observability surface." Both flows go to the same sink; Mara captures the gap the SDK can't see.

## Schema migration

Operators who built their own schema and have months of historical data in it: Mara's canonical schema is OTel `gen_ai.*` aligned. Historical data with custom attribute names continues to work in your existing sink; new data from Mara arrives with `gen_ai.*` names. You either:

- **Soft migration:** keep both schemas live in the sink, rewrite dashboards over time.
- **Hard migration:** use the sink's own renaming rules (Honeycomb derived columns, Datadog log pipelines, Loki promotion to indexed labels) to retrofit old data into the new shape.

We do not provide a Mara-side tool to backfill historical data into the new schema; that's a sink-side concern.

## Common migration pitfalls

- **Forgetting to disable the old pipeline.** Running Mara + Fluent Bit + Vector simultaneously tailing the same files leads to duplicate events. Disable the old pipeline before enabling Mara, even if just for testing.
- **Port conflicts.** Mara binds 4318; if you previously had an OTel Collector on the same port, free the port first.
- **Forgetting the env vars.** OTel emission from runtimes is opt-in; the env var set is the trigger.
- **Cardinality re-checking.** Your old pipeline may have been emitting labels Mara puts in structured metadata (Loki) or attributes (OTLP). Re-check your dashboards.
- **Custom redaction patterns lost.** If your old setup had patterns Mara's built-in pack doesn't have (e.g., your company's internal token format), file a PR to add it to `builtin.pii` or accept it as MVP+1 work.

## Migration support during MVP

For the first 90 days post-launch:

- A dedicated `migration` label on GitHub Issues.
- The maintainer personally reviews migration questions within 3 business days.
- A `MIGRATIONS.md` file in the repo collects published migration walkthroughs from real users (with their permission).

After 90 days, migration support continues as standard community support.

## Cross-references

- [`17-otel-collector-cookbook.md`](17-otel-collector-cookbook.md) — concrete side-by-side configs.
- [`../03-value-proposition/02-feature-matrix.md`](../03-value-proposition/02-feature-matrix.md) — feature-by-feature competitive comparison.
- [`../01-landscape/01-classic-log-shippers.md`](../01-landscape/01-classic-log-shippers.md) — landscape of source tools.
- [`14-launch-and-early-adopter-experience.md`](14-launch-and-early-adopter-experience.md) — first 30 days post-launch.
