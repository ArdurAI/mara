# OpenTelemetry `gen_ai.*` Semantic Conventions

## Executive summary

The OpenTelemetry Semantic Conventions for Generative AI (`gen_ai.*` namespace) are the closest thing to an industry-standard schema for LLM and agent observability. They are CNCF-governed via the OTel project. As of May 2026, the conventions are in `Development` status — not yet stable but actively used by major vendors and stable enough to align Mara against. This document covers the convention's structure, current status, vendor adoption, the related MCP namespace, the experimental-stability opt-in mechanism, and how Mara consumes them.

## Project organization

- **Repository:** <https://github.com/open-telemetry/semantic-conventions>.
- **gen-ai docs:** <https://github.com/open-telemetry/semantic-conventions/tree/main/docs/gen-ai>.
- **MCP docs:** in the same repo under `docs/gen-ai/` (MCP file).
- **Specification site:** <https://opentelemetry.io/docs/specs/semconv/gen-ai/>.
- **SIG:** OpenTelemetry GenAI Observability SIG; meets bi-weekly; meeting notes in OTel community repo.

## Stability lifecycle

OTel semantic conventions use a stability ladder: `Experimental` → `Development` → `Stable`. The `gen_ai.*` namespace is currently `Development`.

Migration path for breaking changes is via the `OTEL_SEMCONV_STABILITY_OPT_IN` environment variable:

- Default behavior emits the most-recently-stable variant.
- Setting `gen_ai_latest_experimental` opts in to the latest in-flight attribute names.
- A common comma-separated form allows opting into multiple namespaces' latest experiments.

This mechanism lets SDKs and Collectors transition gradually as the namespace evolves.

## Attribute taxonomy

### `gen_ai.system`

Identifies the model provider / vendor. Values include: `openai`, `azure_openai`, `anthropic`, `bedrock`, `vertex_ai`, `gemini`, `cohere`, `mistral_ai`, `groq`, `deepseek`, `xai`, `together_ai`, `perplexity`, `ollama`, `vllm`, `tgi`, etc. Mara preserves whatever the upstream emits.

### `gen_ai.operation.name`

Identifies the kind of operation. Common values: `chat`, `text_completion`, `embeddings`, `image_generation`, `speech_to_text`, `text_to_speech`, `agent_step`, `agent_session`, `agent_run`.

### `gen_ai.request.*`

- `gen_ai.request.model` — requested model identifier.
- `gen_ai.request.temperature`, `gen_ai.request.top_p`, `gen_ai.request.top_k`.
- `gen_ai.request.max_tokens`.
- `gen_ai.request.presence_penalty`, `gen_ai.request.frequency_penalty`.
- `gen_ai.request.stop_sequences`.
- `gen_ai.request.seed`.

### `gen_ai.response.*`

- `gen_ai.response.id` — vendor-side response id.
- `gen_ai.response.model` — actual model that responded (may differ from request when vendor routes).
- `gen_ai.response.finish_reasons` — array of stop reasons.

### `gen_ai.usage.*`

- `gen_ai.usage.input_tokens`.
- `gen_ai.usage.output_tokens`.
- `gen_ai.usage.cached_tokens` — input tokens served from cache.
- `gen_ai.usage.reasoning_tokens` — reasoning-model internal tokens.
- `gen_ai.usage.total_tokens`.

### `gen_ai.tool.*`

- `gen_ai.tool.name`.
- `gen_ai.tool.call.id`.
- `gen_ai.tool.type` — `function`, `retrieval`, `code_interpreter`, `mcp`.
- `gen_ai.tool.call.arguments` — when capture is opted in.

### `gen_ai.agent.*`

- `gen_ai.agent.id`.
- `gen_ai.agent.name`.
- `gen_ai.agent.description`.

### `gen_ai.conversation.id`

Per-conversation identifier; useful for grouping multi-turn sessions.

### `gen_ai.token.*` (event-level, when capture is opted in)

Individual token-level events; high cardinality; usually disabled.

## Metric definitions

`gen_ai.*` defines a set of metric instruments:

- `gen_ai.client.token.usage` — token counts (histogram).
- `gen_ai.client.operation.duration` — call duration (histogram).
- `gen_ai.server.request.duration` — server-side duration when measured.

These flow as OTLP metrics and are aggregatable in Prometheus or any OTel metrics backend.

## Span structure for agent loops

The `gen_ai.*` semconv defines:

- A span per model call.
- A span per tool call.
- A span per agent step / session for hierarchical structure.

Parent-child relationships allow visualization of agent trees in any OTel-aware backend (Tempo, Jaeger, Zipkin, Honeycomb, Datadog, etc.).

## MCP (Model Context Protocol) attributes

The MCP namespace was added to OTel semconv as MCP gained adoption. As of May 2026:

- `mcp.client.name`, `mcp.client.version`.
- `mcp.server.name`, `mcp.server.version`.
- `mcp.protocol.version`.
- `mcp.tool.name`, `mcp.tool.namespace`.
- `mcp.resource.uri`.
- `mcp.transport` — `stdio` | `http` | `sse` | `websocket`.

Status: `Development`. Mara aligns and contributes.

## Vendor adoption

- **Honeycomb:** first-party `gen_ai.*` support in queries and dashboards.
- **Grafana Cloud (Tempo / Loki / Mimir):** ingests via OTLP; queries via attribute filters.
- **Datadog LLM Observability:** ingests OTLP including `gen_ai.*` attributes; UI translates to Datadog's product schema.
- **New Relic AI Monitoring:** similar to Datadog.
- **Signoz / Hyperdx:** native OTel-aligned, including `gen_ai.*`.
- **Logfire (Pydantic):** native.
- **Claude Code, Codex CLI, Gemini CLI:** first-party OTLP exporters that emit `gen_ai.*`.
- **OpenInference / Phoenix:** parallel namespace; mapping required.

## Adoption status (as observed)

- **`gen_ai.system`, `gen_ai.request.model`, `gen_ai.usage.*`:** widely emitted; stable in practice.
- **`gen_ai.response.*`:** mostly stable.
- **`gen_ai.tool.*`:** newer, less universal.
- **`gen_ai.agent.*`:** newest, sparse adoption.
- **`mcp.*`:** brand new, few emitters.

## How Mara consumes the conventions

### Codegen

The `mara-schema` crate's `xtask codegen-semconv` script:

1. Reads `crates/mara-schema/semconv.lock` (commit hash + repo URL).
2. Clones the semconv repo at that commit.
3. Parses the YAML conventions.
4. Generates Rust types into `crates/mara-schema/src/generated/`.

Generated code is committed to the repo for review.

### Drift CI check

A scheduled CI job re-runs codegen and compares output against committed. Drift opens a PR for review.

### Stability opt-in

Mara honors `OTEL_SEMCONV_STABILITY_OPT_IN`. Default emits the most recently stable variant per the pinned semconv version. With `gen_ai_latest_experimental`, Mara emits the latest in-flight variant.

### `mara.*` namespace

For attributes Mara needs but the upstream doesn't yet provide (e.g., `mara.session.id`, `mara.cost.usd`), we use a clearly-namespaced extension. Proposals to graduate these upstream are tracked in `docs/upstream-contributions.md`.

## Concrete example: a Claude Code turn

Canonical event for a single Claude Code turn that called a tool and produced a completion:

```json
{
  "resource": {
    "service.name": "claude-code",
    "host.name": "macbook-pro",
    "mara.source.runtime": "claude_code",
    "mara.source.runtime_version": "0.43.1"
  },
  "scope": { "name": "mara-adapter-otlp" },
  "timestamp_ns": 1715541600123456789,
  "trace_id": "abcdef0123456789abcdef0123456789",
  "span_id": "0123456789abcdef",
  "parent_span_id": null,
  "event_kind": "completion",
  "severity": 9,
  "gen_ai": {
    "system": "anthropic",
    "operation": { "name": "chat" },
    "request": {
      "model": "claude-sonnet-4-5-20250929",
      "temperature": 0.0,
      "max_tokens": 4096
    },
    "response": {
      "model": "claude-sonnet-4-5-20250929",
      "id": "msg_01ABC...",
      "finish_reasons": ["end_turn"]
    },
    "usage": {
      "input_tokens": 1024,
      "output_tokens": 768,
      "cached_tokens": 256,
      "total_tokens": 1792
    },
    "conversation": { "id": "claude-session-xyz" }
  },
  "mcp": null,
  "mara": {
    "session": { "id": "claude-session-xyz" },
    "turn": { "id": "turn-42" },
    "cost": { "usd": 0.0234, "source": "mara_computed" },
    "policy": {
      "profile": "default@1.2.3",
      "capture_optin": false,
      "decisions": [
        { "stage": "redact", "decision": "passthrough" }
      ]
    },
    "body": {
      "prompt_hash": "sha256:9b8c...",
      "completion_hash": "sha256:a3f1..."
    }
  },
  "attributes": { },
  "body": null
}
```

When capture is opted in, `body.prompt.messages` and `body.completion.choices` are populated; otherwise just hashes.

## Limitations of the conventions (as of May 2026)

- No standard for `cost` in USD — Mara fills with `mara.cost.usd`.
- No standard for `tenant.id` — Mara fills with `mara.tenant.id`.
- No standard for `session.id` separate from `gen_ai.conversation.id` — Mara provides `mara.session.id` for runtime-local sessions distinct from logical conversations.
- No standard for `body` field of an event log (the OTel LogRecord `body` is free-form) — Mara documents its body shape under `mara.body.*`.
- Streaming partial-emit semantics are under-specified — Mara treats streams as single spans with cumulative usage.

## Tracking upstream

- We pin a specific semconv commit in `crates/mara-schema/semconv.lock`.
- Quarterly reviews of upstream stability and changes.
- When a `mara.*` extension graduates upstream, deprecate the `mara.*` form in the next minor release; remove in the major after.

## Why we don't just emit OTLP raw

Mara's value comes from:

- Adapter-side normalization (file tail, hooks, REST) into OTLP-shaped events.
- Policy stage before sink.
- WAL for durability.
- Multi-sink fan-out.

We could not get these from emitting raw OTLP from the runtime to the user's backend. Mara's existence is justified by the policy + adapter + buffering layers around OTel; the semconv is the lingua franca.

## Reference contributions tracker

- `mara.cost.usd` → proposed for `gen_ai.usage.cost.usd` (status: under discussion in gen-ai SIG).
- `mara.session.id` → proposed for inclusion as `gen_ai.session.id` distinct from `conversation.id` (status: draft).
- `mara.policy.decisions` → audit-log-shaped attribute, lower priority for upstream.
- MCP transport additional values → contributed to MCP doc.

## References

- OTel semconv repository: <https://github.com/open-telemetry/semantic-conventions>.
- OTel gen-ai docs (rendered): <https://opentelemetry.io/docs/specs/semconv/gen-ai/>.
- OTel gen-ai SIG meeting notes: <https://github.com/open-telemetry/community/tree/main/projects/gen-ai-observability>.
- Stability opt-in environment variable spec: <https://opentelemetry.io/docs/specs/otel/configuration/sdk-environment-variables/#general-sdk-configuration>.
