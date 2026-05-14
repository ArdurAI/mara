//! Mara canonical event schema.
//!
//! The types in this crate are the contract between adapters and
//! sinks.  Field names and structure follow OpenTelemetry's
//! `gen_ai.*` and `mcp.*` semantic conventions wherever possible.
//! Mara-specific extensions live under [`MaraExtensions`].
//!
//! ## Stability
//!
//! In M1 the surface is published as stable enough to build
//! against, but typed as `#[non_exhaustive]` so additive evolution
//! does not break consumers.  Major-version bumps are gated by ADR
//! per the deprecation policy described in
//! `plans/04-implementation/02-non-functional-requirements.md`.
//!
//! ## Codegen
//!
//! Long-term this crate is generated from a pinned commit of
//! `open-telemetry/semantic-conventions` via `xtask codegen-semconv`.
//! M1 ships hand-written equivalents; semconv codegen lands in
//! M1 follow-up work.  Per-field documentation comes from the
//! semconv `brief:` fields when codegen lands; until then field
//! names align with the semconv attribute keys.

#![doc(html_root_url = "https://docs.rs/mara-schema/0.1.0")]
// The schema types are 1:1 with OTel semconv attributes; per-field
// docs are intentionally deferred until codegen consumes the brief:
// fields from semconv YAML.  Removing this allow is a sequenced
// task in M1 follow-up work.
#![allow(missing_docs)]

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

/// Schema version emitted by this build.
pub const SCHEMA_VERSION: &str = "0.1.0";

/// Pinned commit hash of `open-telemetry/semantic-conventions` that
/// this build aligns with.  Kept in sync with `docs/semconv.lock`; CI enforces match.
pub const SEMCONV_COMMIT: &str = "aec6e9d3e86754683dab7c707655d69d953b2768";

/// A canonical Mara event.
///
/// One event represents a single observation from an AI runtime:
/// a prompt, completion, tool call, tool result, cost record,
/// error, system lifecycle event, eval result, or feedback signal.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Event {
    /// OTel resource attributes identifying the source process.
    pub resource: Resource,

    /// OTel instrumentation scope describing the producing adapter.
    pub scope: Scope,

    /// Wall-clock event time, nanoseconds since the Unix epoch.
    pub timestamp_ns: i64,

    /// When Mara first observed the event.  Independent of
    /// `timestamp_ns` because adapters may ingest historical data.
    pub observed_timestamp_ns: i64,

    /// W3C-compatible 128-bit trace identifier, when present.
    pub trace_id: Option<TraceId>,
    /// W3C-compatible 64-bit span identifier, when present.
    pub span_id: Option<SpanId>,
    /// Parent span identifier within the same trace, when present.
    pub parent_span_id: Option<SpanId>,

    /// High-level classification of the event.
    pub event_kind: EventKind,
    /// OTel SeverityNumber (1–24).  Defaults to `Severity::INFO` (9).
    pub severity: Severity,

    /// OpenTelemetry `gen_ai.*` attributes.
    pub gen_ai: GenAi,

    /// MCP attributes, when the event concerns Model Context Protocol traffic.
    pub mcp: Option<Mcp>,

    /// Mara-specific extensions under the `mara.*` namespace.
    pub mara: MaraExtensions,

    /// Free-form attribute bag for fields outside the structured
    /// namespaces above.  Preserved verbatim from the source.
    pub attributes: BTreeMap<String, AttrValue>,

    /// Optional raw event body (prompt content, completion content,
    /// tool arguments) when capture is opted in.  See
    /// `mara.policy.capture_optin` for the gate.
    pub body: Option<EventBody>,
}

/// OTel resource attributes identifying the producing process.
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct Resource {
    /// `service.name`
    pub service_name: Option<String>,
    /// `service.version`
    pub service_version: Option<String>,
    /// `host.name`
    pub host_name: Option<String>,
    /// `process.pid`
    pub process_pid: Option<u32>,
    /// `mara.source.runtime` — which AI runtime emitted the source data.
    pub source_runtime: Option<SourceRuntime>,
    /// Additional resource attributes that don't fit the typed slots.
    pub extra: BTreeMap<String, AttrValue>,
}

/// OTel instrumentation scope.
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct Scope {
    /// Instrumentation library name (e.g., `"mara-adapter-otlp"`).
    pub name: String,
    /// Instrumentation library version.
    pub version: Option<String>,
}

/// High-level classification of every canonical Mara event.
#[derive(Copy, Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum EventKind {
    /// Model received input (system, user, or tool-result-as-input prompt).
    Prompt,
    /// Model produced output.
    Completion,
    /// Model requested a tool invocation.
    ToolCall,
    /// Tool returned a result to the model.
    ToolResult,
    /// Cost / usage record.
    Cost,
    /// Failure attributable to the agent loop.
    Error,
    /// Lifecycle event (session start/end, model switch, config change).
    System,
    /// Eval pipeline produced a result attached to a session.
    Eval,
    /// User feedback (thumbs up/down, rating) on a session.
    Feedback,
}

/// OTel SeverityNumber 1..=24.
#[derive(Copy, Clone, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Severity(pub u8);

impl Severity {
    /// `TRACE` family (1..=4).
    pub const TRACE: Self = Self(1);
    /// `DEBUG` family (5..=8).
    pub const DEBUG: Self = Self(5);
    /// `INFO` family (9..=12).
    pub const INFO: Self = Self(9);
    /// `WARN` family (13..=16).
    pub const WARN: Self = Self(13);
    /// `ERROR` family (17..=20).
    pub const ERROR: Self = Self(17);
    /// `FATAL` family (21..=24).
    pub const FATAL: Self = Self(21);
}

impl Default for Severity {
    fn default() -> Self {
        Self::INFO
    }
}

/// AI runtime that produced the source data.
#[derive(Copy, Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum SourceRuntime {
    /// Anthropic Claude Code.
    ClaudeCode,
    /// OpenAI Codex CLI / desktop.
    Codex,
    /// Cursor Agents (IDE + CLI).
    Cursor,
    /// Moonshot Kimi.
    Kimi,
    /// Augment Code.
    Augment,
    /// Google Gemini CLI.
    Gemini,
    /// Ollama local inference server (via Mara HTTP proxy).
    Ollama,
    /// Any other runtime captured generically.
    Other,
}

/// OpenTelemetry `gen_ai.*` attributes.
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct GenAi {
    /// `gen_ai.system` — provider identifier.
    pub system: Option<String>,
    /// `gen_ai.operation.name` — chat, completion, embeddings, ...
    pub operation_name: Option<String>,
    /// Request attributes.
    pub request: GenAiRequest,
    /// Response attributes.
    pub response: GenAiResponse,
    /// Usage / token-accounting attributes.
    pub usage: GenAiUsage,
    /// Tool-call attributes, when relevant.
    pub tool: Option<GenAiTool>,
    /// Agent attributes, when relevant.
    pub agent: Option<GenAiAgent>,
    /// `gen_ai.conversation.id`.
    pub conversation_id: Option<String>,
}

/// Request-side attributes of a `gen_ai` event.
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct GenAiRequest {
    pub model: Option<String>,
    pub temperature: Option<f64>,
    pub top_p: Option<f64>,
    pub top_k: Option<u32>,
    pub max_tokens: Option<u32>,
    pub presence_penalty: Option<f64>,
    pub frequency_penalty: Option<f64>,
    pub seed: Option<i64>,
    pub stop_sequences: Vec<String>,
}

/// Response-side attributes of a `gen_ai` event.
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct GenAiResponse {
    pub model: Option<String>,
    pub id: Option<String>,
    pub finish_reasons: Vec<String>,
    /// `gen_ai.response.is_streaming`.
    pub is_streaming: bool,
}

/// Usage / token-accounting attributes.
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct GenAiUsage {
    pub input_tokens: Option<u64>,
    pub output_tokens: Option<u64>,
    pub cached_tokens: Option<u64>,
    pub reasoning_tokens: Option<u64>,
    pub total_tokens: Option<u64>,
}

/// Tool-call attributes.
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct GenAiTool {
    pub name: Option<String>,
    pub call_id: Option<String>,
    pub r#type: Option<ToolType>,
}

/// Agent attributes.
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct GenAiAgent {
    pub id: Option<String>,
    pub name: Option<String>,
    pub description: Option<String>,
}

/// Tool type as defined by `gen_ai.tool.type`.
#[derive(Copy, Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum ToolType {
    Function,
    Retrieval,
    CodeInterpreter,
    Mcp,
}

/// MCP attributes attached to events that traverse MCP.
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct Mcp {
    pub client_name: Option<String>,
    pub client_version: Option<String>,
    pub server_name: Option<String>,
    pub server_version: Option<String>,
    pub protocol_version: Option<String>,
    pub tool_name: Option<String>,
    pub tool_namespace: Option<String>,
    pub resource_uri: Option<String>,
    pub transport: Option<McpTransport>,
}

/// MCP transport flavours.
#[derive(Copy, Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum McpTransport {
    Stdio,
    Http,
    Sse,
    Websocket,
}

/// Mara-specific extensions under the `mara.*` namespace.
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct MaraExtensions {
    /// Identifier of the adapter that produced the event.
    pub source_adapter: Option<String>,
    /// Originating runtime version, as reported by the source.
    pub source_runtime_version: Option<String>,
    /// Runtime-local session identifier.
    pub session_id: Option<String>,
    /// Turn within session.
    pub turn_id: Option<String>,
    /// Applied policy profile name and version.
    pub policy_profile: Option<String>,
    /// Whether prompt / raw body capture was opted in for this event.
    pub policy_capture_optin: bool,
    /// Policy decisions recorded by stages in the chain.
    pub policy_decisions: Vec<PolicyDecisionRecord>,
    /// Normalized cost in USD.
    pub cost_usd: Option<f64>,
    /// Where the cost number came from: vendor-emitted vs Mara-computed.
    pub cost_source: Option<CostSource>,
    /// Confidence in [`Self::cost_usd`] when estimation inputs are partial.
    pub cost_confidence: Option<CostConfidence>,
    /// Stable per-request id from the LLM gateway (`x-mara-request-id`) for log/trace correlation.
    pub request_id: Option<String>,
    /// Agent or orchestration run identifier when the client supplies one (M2-03).
    pub agent_id: Option<String>,
    /// Logical step index or name within an agent run (M2-03).
    pub step_id: Option<String>,
    /// Tool invoked in an agent step (M2-03); distinct from [`Mcp::tool_name`] when MCP is also set.
    pub tool_name: Option<String>,
    /// Outcome label for a tool step (`success`, `error`, …) when supplied by the client (M2-03).
    pub tool_outcome: Option<String>,
    /// Optional tenant identifier for multi-tenant capture.
    pub tenant_id: Option<String>,
    /// Compliance tags (`hipaa`, `pci`, `gdpr`, ...).
    pub compliance_tags: Vec<String>,
    /// Pre-redaction hashes when raw body capture is suppressed.
    pub body_hashes: BodyHashes,
}

/// Where the normalized cost came from.
#[derive(Copy, Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum CostSource {
    /// The upstream vendor emitted the cost directly.
    Vendor,
    /// Mara computed the cost from token counts × price table.
    MaraEstimated,
}

/// Honesty tier for `mara.cost_usd` when inputs are incomplete (M2-11).
#[derive(Copy, Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum CostConfidence {
    /// Full request/response and usage suitable for chargeback-grade estimates.
    High,
    /// Estimates usable for trending; some fields were defaulted or inferred.
    Medium,
    /// Missing usage, truncation, or disabled pricing — treat cost as indicative only.
    Low,
}

/// SHA-256 hashes of body content when raw capture is suppressed.
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct BodyHashes {
    pub prompt: Option<String>,
    pub completion: Option<String>,
    pub tool_args: Option<String>,
}

/// Policy-decision record produced by a stage in the policy chain.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[non_exhaustive]
pub struct PolicyDecisionRecord {
    /// Stage name (matches the policy chain configuration entry).
    pub stage: String,
    /// What the stage decided.
    pub decision: PolicyDecisionKind,
    /// Optional human-readable reason (e.g., `"matched email regex"`).
    pub reason: Option<String>,
}

impl PolicyDecisionRecord {
    /// Construct a new policy decision record.
    #[must_use]
    pub fn new(
        stage: impl Into<String>,
        decision: PolicyDecisionKind,
        reason: Option<String>,
    ) -> Self {
        Self { stage: stage.into(), decision, reason }
    }
}

/// What a policy stage decided.
#[derive(Copy, Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum PolicyDecisionKind {
    Passthrough,
    Redacted,
    Allowed,
    Denied,
    Sampled,
    RateLimited,
    Transformed,
    Classified,
    Routed,
}

/// Raw event body — populated only when capture is opted in.
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct EventBody {
    pub prompt: Option<PromptBody>,
    pub completion: Option<CompletionBody>,
    pub tool_call: Option<ToolCallBody>,
    pub tool_result: Option<ToolResultBody>,
    pub raw_request: Option<String>,
    pub raw_response: Option<String>,
}

/// Captured prompt messages.
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct PromptBody {
    pub messages: Vec<Message>,
}

/// Captured completion choices.
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct CompletionBody {
    pub choices: Vec<CompletionChoice>,
}

/// A single message in a prompt body.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Message {
    pub role: String,
    pub content: String,
}

/// A single completion choice.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CompletionChoice {
    pub message: Message,
    pub finish_reason: Option<String>,
}

/// Captured tool call payload.
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct ToolCallBody {
    /// JSON-encoded tool arguments.
    pub arguments_json: Option<String>,
}

/// Captured tool result payload.
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct ToolResultBody {
    /// JSON-encoded tool output (free-form).
    pub content_json: Option<String>,
    /// `true` when the result was truncated before reaching the model.
    pub truncated: bool,
}

/// W3C-compatible 128-bit trace identifier (16 bytes).
#[derive(Copy, Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct TraceId(pub [u8; 16]);

/// W3C-compatible 64-bit span identifier (8 bytes).
#[derive(Copy, Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct SpanId(pub [u8; 8]);

/// Typed attribute value, mirroring OTel's AnyValue.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
#[non_exhaustive]
pub enum AttrValue {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    String(String),
    Bytes(Vec<u8>),
    Array(Vec<AttrValue>),
    Map(BTreeMap<String, AttrValue>),
}

impl Event {
    /// Construct a minimally-populated event with the current
    /// observed time and a defaulted body.  Suitable as the
    /// starting point for adapters to enrich.
    #[must_use]
    pub fn now(event_kind: EventKind, scope_name: impl Into<String>) -> Self {
        let now_ns = OffsetDateTime::now_utc().unix_timestamp_nanos() as i64;
        Self {
            resource: Resource::default(),
            scope: Scope { name: scope_name.into(), version: None },
            timestamp_ns: now_ns,
            observed_timestamp_ns: now_ns,
            trace_id: None,
            span_id: None,
            parent_span_id: None,
            event_kind,
            severity: Severity::default(),
            gen_ai: GenAi::default(),
            mcp: None,
            mara: MaraExtensions::default(),
            attributes: BTreeMap::new(),
            body: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schema_version_is_non_empty() {
        assert!(!SCHEMA_VERSION.is_empty());
    }

    #[test]
    fn event_constructs_with_defaults() {
        let ev = Event::now(EventKind::Prompt, "test-scope");
        assert!(matches!(ev.event_kind, EventKind::Prompt));
        assert_eq!(ev.scope.name, "test-scope");
        assert!(ev.body.is_none());
        assert_eq!(ev.severity, Severity::INFO);
    }

    #[test]
    fn event_roundtrips_through_json() {
        let ev = Event::now(EventKind::Completion, "rt-test");
        let json = serde_json::to_string(&ev).expect("encode");
        let back: Event = serde_json::from_str(&json).expect("decode");
        assert!(matches!(back.event_kind, EventKind::Completion));
        assert_eq!(back.scope.name, "rt-test");
    }

    #[test]
    fn severity_constants_in_otel_ranges() {
        assert_eq!(Severity::TRACE.0, 1);
        assert_eq!(Severity::DEBUG.0, 5);
        assert_eq!(Severity::INFO.0, 9);
        assert_eq!(Severity::WARN.0, 13);
        assert_eq!(Severity::ERROR.0, 17);
        assert_eq!(Severity::FATAL.0, 21);
    }
}
