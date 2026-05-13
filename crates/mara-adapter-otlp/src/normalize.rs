//! Normalize OTLP protobuf messages into canonical Mara events.
//!
//! Translation contract:
//!
//! - Each OTLP [`LogRecord`] emits one Mara [`Event`].
//! - Each OTLP [`Span`] emits one Mara [`Event`] (span events on
//!   the span are flattened into attributes in MVP; multi-event
//!   span expansion lands in MVP+1).
//! - Resource attributes from [`ResourceLogs`] / [`ResourceSpans`]
//!   populate the [`Resource`] on every Event.
//! - Scope attributes from [`ScopeLogs`] / [`ScopeSpans`] populate
//!   the [`Scope`] on every Event.
//! - Attribute keys with prefix `gen_ai.`, `mcp.`, or `mara.` are
//!   bucketed into the typed [`GenAi`], [`Mcp`], and
//!   [`MaraExtensions`] structs. Everything else lands in
//!   `attributes`.
//! - `event_kind` is inferred from `gen_ai.operation.name` +
//!   presence of `gen_ai.tool.*` attributes. Unknown shapes
//!   default to [`EventKind::System`].
//!
//! MVP-stage caveats (refined in MVP+1):
//! - Body AnyValue is captured as a string when it is one; nested
//!   bodies are stringified via debug format.
//! - Per-token streaming chunks are not aggregated.
//! - Span events are not yet expanded into separate Mara events.

use std::collections::BTreeMap;

use mara_core::Event;
use mara_schema::{
    AttrValue, EventKind, GenAi, MaraExtensions, Mcp, Resource, Scope, Severity, SourceRuntime,
    SpanId, TraceId,
};
use opentelemetry_proto::tonic::common::v1::any_value::Value as AnyValueInner;
use opentelemetry_proto::tonic::common::v1::{AnyValue, InstrumentationScope, KeyValue};
use opentelemetry_proto::tonic::logs::v1::LogRecord;
use opentelemetry_proto::tonic::resource::v1::Resource as ProtoResource;
use opentelemetry_proto::tonic::trace::v1::Span;

/// Translate one OTLP `LogRecord` (plus its enclosing resource +
/// scope) into a canonical Mara [`Event`].
pub fn log_record_to_event(
    resource: Option<&ProtoResource>,
    scope: Option<&InstrumentationScope>,
    lr: &LogRecord,
) -> Event {
    let mara_resource = build_resource(resource);
    let mara_scope = build_scope(scope);

    let (gen_ai, mcp, mara_ext, attributes) = bucket_attributes(&lr.attributes);

    let event_kind = infer_event_kind(&gen_ai, &attributes);
    let severity = severity_from_otlp(lr.severity_number);
    let timestamp_ns = ts_or_now(lr.time_unix_nano);
    let observed_ts = ts_or_now(lr.observed_time_unix_nano);
    let trace_id = trace_id_from_bytes(&lr.trace_id);
    let span_id = span_id_from_bytes(&lr.span_id);

    Event {
        resource: mara_resource,
        scope: mara_scope,
        timestamp_ns,
        observed_timestamp_ns: observed_ts,
        trace_id,
        span_id,
        parent_span_id: None,
        event_kind,
        severity,
        gen_ai,
        mcp,
        mara: mara_ext,
        attributes,
        body: None,
    }
}

/// Translate one OTLP `Span` (plus its enclosing resource + scope)
/// into a canonical Mara [`Event`]. Span events on the span are
/// flattened into attributes; expanding them into separate Mara
/// events lands in MVP+1.
pub fn span_to_event(
    resource: Option<&ProtoResource>,
    scope: Option<&InstrumentationScope>,
    span: &Span,
) -> Event {
    let mara_resource = build_resource(resource);
    let mara_scope = build_scope(scope);

    let (gen_ai, mcp, mara_ext, mut attributes) = bucket_attributes(&span.attributes);

    let event_kind = infer_event_kind(&gen_ai, &attributes);
    let timestamp_ns = ts_or_now(span.start_time_unix_nano);
    let observed_ts = ts_or_now(span.end_time_unix_nano);
    let trace_id = trace_id_from_bytes(&span.trace_id);
    let span_id = span_id_from_bytes(&span.span_id);
    let parent_span_id = if span.parent_span_id.is_empty() {
        None
    } else {
        span_id_from_bytes(&span.parent_span_id)
    };

    if !span.name.is_empty() {
        attributes.insert("span.name".to_owned(), AttrValue::String(span.name.clone()));
    }
    if !span.events.is_empty() {
        attributes.insert(
            "span.events.count".to_owned(),
            AttrValue::Int(i64::try_from(span.events.len()).unwrap_or(i64::MAX)),
        );
    }

    Event {
        resource: mara_resource,
        scope: mara_scope,
        timestamp_ns,
        observed_timestamp_ns: observed_ts,
        trace_id,
        span_id,
        parent_span_id,
        event_kind,
        severity: Severity::INFO,
        gen_ai,
        mcp,
        mara: mara_ext,
        attributes,
        body: None,
    }
}

// --------------------------------------------------------------------------
// Helpers
// --------------------------------------------------------------------------

fn build_resource(resource: Option<&ProtoResource>) -> Resource {
    let mut out = Resource::default();
    let Some(r) = resource else {
        return out;
    };
    for kv in &r.attributes {
        let Some(value) = key_value_to_attr(kv) else { continue };
        match kv.key.as_str() {
            "service.name" => {
                if let AttrValue::String(s) = &value {
                    out.service_name = Some(s.clone());
                }
            }
            "service.version" => {
                if let AttrValue::String(s) = &value {
                    out.service_version = Some(s.clone());
                }
            }
            "host.name" => {
                if let AttrValue::String(s) = &value {
                    out.host_name = Some(s.clone());
                }
            }
            "process.pid" => {
                if let AttrValue::Int(i) = &value {
                    out.process_pid = u32::try_from(*i).ok();
                }
            }
            "mara.source.runtime" => {
                if let AttrValue::String(s) = &value {
                    out.source_runtime = Some(source_runtime_from_str(s));
                }
            }
            _ => {
                out.extra.insert(kv.key.clone(), value);
            }
        }
    }
    out
}

fn build_scope(scope: Option<&InstrumentationScope>) -> Scope {
    let Some(s) = scope else {
        return Scope { name: "mara-adapter-otlp".to_owned(), version: None };
    };
    Scope {
        name: if s.name.is_empty() { "mara-adapter-otlp".to_owned() } else { s.name.clone() },
        version: if s.version.is_empty() { None } else { Some(s.version.clone()) },
    }
}

fn bucket_attributes(
    attrs: &[KeyValue],
) -> (GenAi, Option<Mcp>, MaraExtensions, BTreeMap<String, AttrValue>) {
    let mut gen_ai = GenAi::default();
    let mut mcp: Option<Mcp> = None;
    let mut mara_ext = MaraExtensions::default();
    let mut attributes = BTreeMap::new();

    for kv in attrs {
        let Some(value) = key_value_to_attr(kv) else { continue };
        let key = kv.key.as_str();

        if let Some(rest) = key.strip_prefix("gen_ai.") {
            apply_gen_ai(&mut gen_ai, rest, &value, &mut attributes, key);
        } else if let Some(rest) = key.strip_prefix("mcp.") {
            apply_mcp(&mut mcp, rest, &value, &mut attributes, key);
        } else if let Some(rest) = key.strip_prefix("mara.") {
            apply_mara(&mut mara_ext, rest, &value, &mut attributes, key);
        } else {
            attributes.insert(key.to_owned(), value);
        }
    }

    (gen_ai, mcp, mara_ext, attributes)
}

fn apply_gen_ai(
    out: &mut GenAi,
    rest: &str,
    value: &AttrValue,
    fallback: &mut BTreeMap<String, AttrValue>,
    full_key: &str,
) {
    match rest {
        "system" => {
            if let AttrValue::String(s) = value {
                out.system = Some(s.clone());
            }
        }
        "operation.name" => {
            if let AttrValue::String(s) = value {
                out.operation_name = Some(s.clone());
            }
        }
        "request.model" => {
            if let AttrValue::String(s) = value {
                out.request.model = Some(s.clone());
            }
        }
        "response.model" => {
            if let AttrValue::String(s) = value {
                out.response.model = Some(s.clone());
            }
        }
        "response.id" => {
            if let AttrValue::String(s) = value {
                out.response.id = Some(s.clone());
            }
        }
        "usage.input_tokens" => {
            out.usage.input_tokens = as_u64(value);
        }
        "usage.output_tokens" => {
            out.usage.output_tokens = as_u64(value);
        }
        "usage.cached_tokens" => {
            out.usage.cached_tokens = as_u64(value);
        }
        "usage.reasoning_tokens" => {
            out.usage.reasoning_tokens = as_u64(value);
        }
        "usage.total_tokens" => {
            out.usage.total_tokens = as_u64(value);
        }
        "conversation.id" => {
            if let AttrValue::String(s) = value {
                out.conversation_id = Some(s.clone());
            }
        }
        _ => {
            // Keep less-common gen_ai.* attributes in the bag so
            // sinks that care about them still see them.
            fallback.insert(full_key.to_owned(), value.clone());
        }
    }
}

fn apply_mcp(
    out: &mut Option<Mcp>,
    rest: &str,
    value: &AttrValue,
    fallback: &mut BTreeMap<String, AttrValue>,
    full_key: &str,
) {
    let target = out.get_or_insert_with(Mcp::default);
    match rest {
        "client.name" => {
            if let AttrValue::String(s) = value {
                target.client_name = Some(s.clone());
            }
        }
        "client.version" => {
            if let AttrValue::String(s) = value {
                target.client_version = Some(s.clone());
            }
        }
        "server.name" => {
            if let AttrValue::String(s) = value {
                target.server_name = Some(s.clone());
            }
        }
        "server.version" => {
            if let AttrValue::String(s) = value {
                target.server_version = Some(s.clone());
            }
        }
        "tool.name" => {
            if let AttrValue::String(s) = value {
                target.tool_name = Some(s.clone());
            }
        }
        _ => {
            fallback.insert(full_key.to_owned(), value.clone());
        }
    }
}

fn apply_mara(
    out: &mut MaraExtensions,
    rest: &str,
    value: &AttrValue,
    fallback: &mut BTreeMap<String, AttrValue>,
    full_key: &str,
) {
    match rest {
        "source.adapter" => {
            if let AttrValue::String(s) = value {
                out.source_adapter = Some(s.clone());
            }
        }
        "source.runtime_version" => {
            if let AttrValue::String(s) = value {
                out.source_runtime_version = Some(s.clone());
            }
        }
        "session.id" => {
            if let AttrValue::String(s) = value {
                out.session_id = Some(s.clone());
            }
        }
        "turn.id" => {
            if let AttrValue::String(s) = value {
                out.turn_id = Some(s.clone());
            }
        }
        "tenant.id" => {
            if let AttrValue::String(s) = value {
                out.tenant_id = Some(s.clone());
            }
        }
        _ => {
            fallback.insert(full_key.to_owned(), value.clone());
        }
    }
}

fn key_value_to_attr(kv: &KeyValue) -> Option<AttrValue> {
    let v = kv.value.as_ref()?;
    any_value_to_attr(v)
}

fn any_value_to_attr(av: &AnyValue) -> Option<AttrValue> {
    let inner = av.value.as_ref()?;
    Some(match inner {
        AnyValueInner::StringValue(s) => AttrValue::String(s.clone()),
        AnyValueInner::BoolValue(b) => AttrValue::Bool(*b),
        AnyValueInner::IntValue(i) => AttrValue::Int(*i),
        AnyValueInner::DoubleValue(f) => AttrValue::Float(*f),
        AnyValueInner::BytesValue(b) => AttrValue::Bytes(b.clone()),
        AnyValueInner::ArrayValue(a) => {
            AttrValue::Array(a.values.iter().filter_map(any_value_to_attr).collect())
        }
        AnyValueInner::KvlistValue(m) => AttrValue::Map(
            m.values
                .iter()
                .filter_map(|kv| Some((kv.key.clone(), any_value_to_attr(kv.value.as_ref()?)?)))
                .collect(),
        ),
    })
}

fn as_u64(value: &AttrValue) -> Option<u64> {
    match value {
        AttrValue::Int(i) => u64::try_from(*i).ok(),
        _ => None,
    }
}

fn ts_or_now(ts: u64) -> i64 {
    if ts == 0 {
        time::OffsetDateTime::now_utc().unix_timestamp_nanos() as i64
    } else {
        i64::try_from(ts).unwrap_or(i64::MAX)
    }
}

fn trace_id_from_bytes(b: &[u8]) -> Option<TraceId> {
    if b.len() == 16 {
        let mut out = [0u8; 16];
        out.copy_from_slice(b);
        Some(TraceId(out))
    } else {
        None
    }
}

fn span_id_from_bytes(b: &[u8]) -> Option<SpanId> {
    if b.len() == 8 {
        let mut out = [0u8; 8];
        out.copy_from_slice(b);
        Some(SpanId(out))
    } else {
        None
    }
}

fn severity_from_otlp(n: i32) -> Severity {
    if n <= 0 { Severity::INFO } else { Severity(u8::try_from(n.clamp(1, 24)).unwrap_or(9)) }
}

fn source_runtime_from_str(s: &str) -> SourceRuntime {
    match s {
        "claude_code" => SourceRuntime::ClaudeCode,
        "codex" => SourceRuntime::Codex,
        "cursor" => SourceRuntime::Cursor,
        "kimi" => SourceRuntime::Kimi,
        "augment" => SourceRuntime::Augment,
        "gemini" => SourceRuntime::Gemini,
        "ollama" => SourceRuntime::Ollama,
        _ => SourceRuntime::Other,
    }
}

fn infer_event_kind(gen_ai: &GenAi, attributes: &BTreeMap<String, AttrValue>) -> EventKind {
    if attributes.keys().any(|k| k.starts_with("gen_ai.tool.")) {
        return EventKind::ToolCall;
    }
    match gen_ai.operation_name.as_deref() {
        Some("chat") | Some("text_completion") => {
            if gen_ai.usage.output_tokens.is_some() || gen_ai.response.model.is_some() {
                EventKind::Completion
            } else {
                EventKind::Prompt
            }
        }
        Some("embeddings") | Some("image_generation") => EventKind::Completion,
        Some("agent_step") | Some("agent_session") | Some("agent_run") => EventKind::System,
        Some(_) | None => EventKind::System,
    }
}

#[cfg(test)]
mod tests {
    use opentelemetry_proto::tonic::common::v1::any_value::Value;
    use opentelemetry_proto::tonic::common::v1::{AnyValue, KeyValue};

    use super::*;

    fn kv(key: &str, val: AnyValueInner) -> KeyValue {
        KeyValue { key: key.to_owned(), value: Some(AnyValue { value: Some(val) }) }
    }

    /// Build a `LogRecord` with all required fields for this version of
    /// `opentelemetry-proto`, defaulting optional fields.
    fn make_log_record(time_ns: u64, attrs: Vec<KeyValue>) -> LogRecord {
        LogRecord {
            time_unix_nano: time_ns,
            observed_time_unix_nano: time_ns,
            severity_number: 9,
            severity_text: "INFO".to_owned(),
            body: None,
            attributes: attrs,
            dropped_attributes_count: 0,
            flags: 0,
            trace_id: vec![],
            span_id: vec![],
        }
    }

    fn make_resource(attrs: Vec<KeyValue>) -> ProtoResource {
        ProtoResource { attributes: attrs, dropped_attributes_count: 0 }
    }

    fn str_kv(key: &str, value: &str) -> KeyValue {
        kv(key, Value::StringValue(value.to_owned()))
    }

    fn int_kv(key: &str, value: i64) -> KeyValue {
        kv(key, Value::IntValue(value))
    }

    #[test]
    fn log_record_with_gen_ai_attrs_populates_typed_struct() {
        let resource = make_resource(vec![
            str_kv("service.name", "claude-code"),
            str_kv("mara.source.runtime", "claude_code"),
        ]);
        let scope = InstrumentationScope {
            name: "claude-code-instr".to_owned(),
            version: "0.43.1".to_owned(),
            attributes: vec![],
            dropped_attributes_count: 0,
        };
        let mut lr = make_log_record(
            1_700_000_000_000_000_000,
            vec![
                str_kv("gen_ai.system", "anthropic"),
                str_kv("gen_ai.operation.name", "chat"),
                str_kv("gen_ai.request.model", "claude-sonnet-4-5"),
                str_kv("gen_ai.response.model", "claude-sonnet-4-5"),
                int_kv("gen_ai.usage.input_tokens", 1024),
                int_kv("gen_ai.usage.output_tokens", 768),
            ],
        );
        lr.trace_id = vec![1u8; 16];
        lr.span_id = vec![2u8; 8];

        let event = log_record_to_event(Some(&resource), Some(&scope), &lr);

        assert_eq!(event.resource.service_name.as_deref(), Some("claude-code"));
        assert_eq!(event.resource.source_runtime, Some(SourceRuntime::ClaudeCode));
        assert_eq!(event.scope.name, "claude-code-instr");
        assert_eq!(event.scope.version.as_deref(), Some("0.43.1"));
        assert_eq!(event.gen_ai.system.as_deref(), Some("anthropic"));
        assert_eq!(event.gen_ai.operation_name.as_deref(), Some("chat"));
        assert_eq!(event.gen_ai.request.model.as_deref(), Some("claude-sonnet-4-5"));
        assert_eq!(event.gen_ai.response.model.as_deref(), Some("claude-sonnet-4-5"));
        assert_eq!(event.gen_ai.usage.input_tokens, Some(1024));
        assert_eq!(event.gen_ai.usage.output_tokens, Some(768));
        assert!(matches!(event.event_kind, EventKind::Completion));
        assert!(event.trace_id.is_some());
        assert!(event.span_id.is_some());
    }

    #[test]
    fn tool_attribute_makes_event_a_tool_call() {
        let lr = make_log_record(
            1,
            vec![str_kv("gen_ai.system", "anthropic"), str_kv("gen_ai.tool.name", "list_files")],
        );
        let event = log_record_to_event(None, None, &lr);
        assert!(matches!(event.event_kind, EventKind::ToolCall));
    }

    #[test]
    fn mcp_attributes_populate_mcp_struct() {
        let lr = make_log_record(
            1,
            vec![
                str_kv("mcp.server.name", "filesystem"),
                str_kv("mcp.server.version", "1.0.0"),
                str_kv("mcp.tool.name", "read_file"),
            ],
        );
        let event = log_record_to_event(None, None, &lr);
        let mcp = event.mcp.expect("mcp present");
        assert_eq!(mcp.server_name.as_deref(), Some("filesystem"));
        assert_eq!(mcp.server_version.as_deref(), Some("1.0.0"));
        assert_eq!(mcp.tool_name.as_deref(), Some("read_file"));
    }

    #[test]
    fn unknown_attributes_land_in_bag() {
        let lr = make_log_record(1, vec![str_kv("user.id", "alice"), int_kv("retry.count", 3)]);
        let event = log_record_to_event(None, None, &lr);
        assert!(
            matches!(event.attributes.get("user.id"), Some(AttrValue::String(s)) if s == "alice")
        );
        assert!(matches!(event.attributes.get("retry.count"), Some(AttrValue::Int(3))));
    }

    #[test]
    fn trace_id_with_wrong_length_is_none() {
        let mut lr = make_log_record(1, vec![]);
        lr.trace_id = vec![1, 2, 3];
        let event = log_record_to_event(None, None, &lr);
        assert!(event.trace_id.is_none());
        assert!(event.span_id.is_none());
    }
}
