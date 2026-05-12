# Data Model

## Executive summary

The Mara canonical event model is built on top of the OpenTelemetry semantic conventions for Generative AI (the `gen_ai.*` namespace) and the OpenTelemetry MCP attribute conventions, extended where necessary with `mara.*` attributes for fields the upstream conventions do not yet cover. The wire format is OTLP-compatible: every canonical event maps cleanly to an OTel log record, span, or metric data point.

The data model is the contract between adapters and sinks. Schema changes go through an ADR.

## Source of truth

- OpenTelemetry Semantic Conventions repository: <https://github.com/open-telemetry/semantic-conventions>, namespace `docs/gen-ai/` and `docs/mcp/`.
- Mara pins a specific commit hash in `crates/mara-schema/semconv.lock`.
- `xtask codegen-semconv` regenerates Rust types from the pinned commit.
- CI fails on drift between the lockfile and the generated Rust.

## Conventions

- Time precision is nanoseconds (`i64`).
- Strings are UTF-8 owned.
- Optional fields use `Option<T>`.
- Attribute bags use a typed `Attributes` struct backed by `IndexMap<KeyName, AttrValue>` for predictable ordering.
- All identifiers use the OTel format: lowercase ASCII with dots as separators.

## Event kinds

Every canonical event has a single `event_kind` value drawn from this closed enum:

- `prompt` — input to a model (system, user, or tool-result-as-input).
- `completion` — output from a model.
- `tool_call` — model-requested invocation of a tool.
- `tool_result` — result returned to the model from a tool.
- `cost` — usage / billing record.
- `error` — failure attributable to the agent loop.
- `system` — agent lifecycle (session start/end, model switch, config change).
- `eval` — eval result attached to a session.
- `feedback` — user feedback (thumbs up/down, rating) attached to a session.

## Core fields (every event)

```rust
pub struct Event {
    // OTel resource attributes (the agent + host + process identity)
    pub resource: Resource,

    // OTel scope attributes (the adapter or library that produced the event)
    pub scope: Scope,

    pub timestamp_ns: i64,                  // event time (UTC)
    pub observed_timestamp_ns: i64,         // when Mara received it

    pub trace_id: Option<TraceId>,          // OTel-compatible 16-byte
    pub span_id: Option<SpanId>,            // OTel-compatible 8-byte
    pub parent_span_id: Option<SpanId>,

    pub event_kind: EventKind,
    pub severity: Severity,                  // OTel SeverityNumber

    pub gen_ai: GenAI,                       // gen_ai.* fields
    pub mcp: Option<Mcp>,                    // mcp.* fields when applicable
    pub mara: Mara,                          // mara.* extensions
    pub attributes: Attributes,              // free-form bag (typed values)
    pub body: Option<EventBody>,             // raw text body if captured (opt-in)
}
```

## `gen_ai.*` fields (subset, aligned with OTel semconv)

- `gen_ai.system` — `"openai"`, `"anthropic"`, `"google"`, `"bedrock"`, `"azure_openai"`, `"vertex_ai"`, `"deepseek"`, `"mistral"`, `"local"`, etc.
- `gen_ai.operation.name` — `"chat"`, `"completion"`, `"embeddings"`, `"image_generation"`, `"speech_to_text"`, `"text_to_speech"`, `"agent_step"`, `"agent_session"`.
- `gen_ai.request.model` — requested model name.
- `gen_ai.response.model` — actual model that served.
- `gen_ai.response.id` — vendor response id.
- `gen_ai.response.finish_reasons` — list.
- `gen_ai.usage.input_tokens`
- `gen_ai.usage.output_tokens`
- `gen_ai.usage.cached_tokens`
- `gen_ai.usage.reasoning_tokens`
- `gen_ai.usage.total_tokens`
- `gen_ai.request.temperature`, `top_p`, `top_k`, `max_tokens`, `presence_penalty`, `frequency_penalty`, `stop_sequences`.
- `gen_ai.tool.name`, `gen_ai.tool.call.id`, `gen_ai.tool.type` (`"function"`, `"retrieval"`, `"code_interpreter"`, `"mcp"`).
- `gen_ai.agent.name`, `gen_ai.agent.id`, `gen_ai.agent.description`.
- `gen_ai.conversation.id`.

Verify against the pinned semconv commit; the above is the May 2026 cohort that is reasonably stable.

## `mcp.*` fields

- `mcp.client.name`, `mcp.client.version`.
- `mcp.server.name`, `mcp.server.version`.
- `mcp.protocol.version`.
- `mcp.tool.name`, `mcp.tool.namespace`.
- `mcp.resource.uri`.
- `mcp.transport` — `"stdio"` | `"http"` | `"sse"` | `"websocket"`.

## `mara.*` extensions

These are Mara additions where `gen_ai.*` is silent. They graduate upstream when accepted.

- `mara.source.adapter` — adapter name that produced the event.
- `mara.source.runtime` — `"claude_code"`, `"codex"`, `"cursor"`, `"kimi"`, `"augment"`, `"gemini"`, `"other"`.
- `mara.source.runtime_version` — runtime version string.
- `mara.session.id` — runtime-local session identifier.
- `mara.turn.id` — turn within session.
- `mara.policy.profile` — applied policy bundle name + version.
- `mara.policy.decisions` — list of `{stage, decision, reason}` records.
- `mara.policy.capture_optin` — bool; whether prompt/body capture was opted in.
- `mara.cost.usd` — computed normalized cost in USD (when computable).
- `mara.cost.source` — `"vendor"` | `"mara_estimated"`.
- `mara.eval.result` — eval pass/fail/score when present.
- `mara.feedback.value` — numeric or categorical feedback.
- `mara.tenant.id` — multi-tenant identifier (optional).
- `mara.compliance.tags` — list of `"hipaa"`, `"pci"`, `"gdpr"`, `"ferpa"`, etc.

## Body capture (opt-in)

When operators opt in (per pipeline + per runtime ZDR toggle):

- `body.prompt.messages[]` — list of `{role, content, attachments?}`.
- `body.completion.choices[]` — list of `{message, finish_reason, logprobs?}`.
- `body.tool_call.arguments` — JSON of arguments.
- `body.tool_result.content` — tool output.
- `body.raw_request` — raw API body (opt-in, separate flag).
- `body.raw_response` — raw API response body (opt-in, separate flag).

When capture is not opted in, the body fields are absent and content-derived hashes are populated:

- `mara.body.prompt_hash` — SHA-256 of canonicalized prompt.
- `mara.body.completion_hash` — SHA-256 of canonicalized completion.
- `mara.body.tool_args_hash` — SHA-256 of canonicalized tool args.

Hashes are still useful for dedup, eval cross-reference, and audit linkage without leaking content.

## Severity mapping

Use the OTel SeverityNumber scale (1–24):

- 1–4: `trace`, useful for fine-grained tool-call entry/exit.
- 5–8: `debug`.
- 9–12: `info` — default for normal completions, tool results.
- 13–16: `warn` — retries, partial failures, content moderation hits.
- 17–20: `error` — agent errors, model errors, sink errors.
- 21–24: `fatal` — agent crash, unrecoverable failure.

## OTLP mapping

Canonical events serialize to OTel signals:

- `event_kind in {prompt, completion, tool_call, tool_result, error, system, eval, feedback}` → OTel **LogRecord** (default) or OTel **Span** event (when span context is present and the operator opts in to span mode).
- `event_kind == "cost"` → OTel **Sum** metric data point with attributes flattened.
- Multi-turn agent sessions → OTel **Span tree** (when span IDs are populated by the adapter or by Mara's normalizer).

This mapping is implemented in `crates/mara-sinks/otlp/` and is the inverse of what `crates/mara-adapters/otlp/` does on the receive side.

## Sink-specific mappings

- **Loki:** structured metadata for stable labels (`runtime`, `session_id`, `event_kind`); body as `line`; high-cardinality fields go into `__structured_metadata__` (Loki 3.x) rather than labels.
- **Splunk HEC:** `event` is the canonical JSON; `fields` includes resource attributes; `sourcetype = "mara:gen_ai"` by default.
- **Elasticsearch:** index template `mara-gen_ai-YYYY.MM.DD`; mapping published with the crate.
- **Object store (Parquet):** column schema published in `crates/mara-schema/parquet_schema.json`; row groups bounded by event count and size.

## Versioning

The schema is versioned with `mara.schema.version` (semver). Major-version bumps require an ADR. Minor-version bumps are additive only. Patch-level bumps cannot change types.

## Round-trip guarantees

For every adapter/sink pair where both speak OTLP, OTel `gen_ai.*` events MUST round-trip through Mara with zero attribute loss. Golden tests in `tests/` enforce this.

For non-OTLP sinks (Loki, Splunk, Parquet), the canonical event is the source and the sink-specific mapping is documented per sink crate. Round-trip is best-effort within the sink's own schema constraints (e.g., Loki labels have cardinality constraints we explicitly trade off).
