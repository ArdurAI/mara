//! Encode canonical Mara [`Event`] values into OTLP log protobuf.

use mara_core::Event;
use mara_schema::{
    AttrValue, CostConfidence, CostSource, EventBody, EventKind, GenAi, MaraExtensions, Mcp,
    Resource, Scope, SourceRuntime, SpanId, ToolType, TraceId,
};
use opentelemetry_proto::tonic::collector::logs::v1::ExportLogsServiceRequest;
use opentelemetry_proto::tonic::common::v1::any_value::Value as AnyValueInner;
use opentelemetry_proto::tonic::common::v1::{AnyValue, InstrumentationScope, KeyValue};
use opentelemetry_proto::tonic::logs::v1::{LogRecord, ResourceLogs, ScopeLogs};
use opentelemetry_proto::tonic::resource::v1::Resource as ProtoResource;

/// Build an OTLP `ExportLogsServiceRequest` carrying one
/// [`ResourceLogs`] block per input event (simple, correct grouping
/// for mixed resources).
#[must_use]
pub fn events_to_export_request(events: &[Event]) -> ExportLogsServiceRequest {
    ExportLogsServiceRequest { resource_logs: events.iter().map(event_to_resource_logs).collect() }
}

fn event_to_resource_logs(ev: &Event) -> ResourceLogs {
    ResourceLogs {
        resource: Some(resource_to_proto(&ev.resource)),
        scope_logs: vec![ScopeLogs {
            scope: Some(scope_to_proto(&ev.scope)),
            log_records: vec![event_to_log_record(ev)],
            schema_url: String::new(),
        }],
        schema_url: String::new(),
    }
}

fn resource_to_proto(r: &Resource) -> ProtoResource {
    let mut attributes = Vec::new();
    if let Some(ref s) = r.service_name {
        attributes.push(str_kv("service.name", s));
    }
    if let Some(ref s) = r.service_version {
        attributes.push(str_kv("service.version", s));
    }
    if let Some(ref s) = r.host_name {
        attributes.push(str_kv("host.name", s));
    }
    if let Some(pid) = r.process_pid {
        attributes.push(int_kv("process.pid", i64::from(pid)));
    }
    if let Some(sr) = r.source_runtime {
        attributes.push(str_kv("mara.source.runtime", source_runtime_to_str(sr)));
    }
    for (k, v) in &r.extra {
        attributes.push(attr_kv(k, v));
    }
    ProtoResource { attributes, dropped_attributes_count: 0 }
}

fn scope_to_proto(s: &Scope) -> InstrumentationScope {
    InstrumentationScope {
        name: s.name.clone(),
        version: s.version.clone().unwrap_or_default(),
        attributes: vec![],
        dropped_attributes_count: 0,
    }
}

fn event_to_log_record(ev: &Event) -> LogRecord {
    let mut attributes = Vec::new();
    append_gen_ai_attrs(&mut attributes, &ev.gen_ai);
    if let Some(ref mcp) = ev.mcp {
        append_mcp_attrs(&mut attributes, mcp);
    }
    append_mara_attrs(&mut attributes, &ev.mara);
    attributes.push(str_kv("mara.event_kind", event_kind_to_str(ev.event_kind)));

    for (k, v) in &ev.attributes {
        attributes.push(attr_kv(k, v));
    }

    if let Some(ref p) = ev.parent_span_id {
        attributes.push(str_kv("mara.parent_span_id", &hex_span(p)));
    }

    let time_unix_nano = ns_to_u64(ev.timestamp_ns);
    let observed_time_unix_nano = ns_to_u64(ev.observed_timestamp_ns);

    LogRecord {
        time_unix_nano,
        observed_time_unix_nano,
        severity_number: i32::from(ev.severity.0),
        severity_text: String::new(),
        body: body_to_proto(ev.body.as_ref()),
        attributes,
        dropped_attributes_count: 0,
        flags: 0,
        trace_id: trace_id_bytes(ev.trace_id),
        span_id: span_id_bytes(ev.span_id),
    }
}

fn trace_id_bytes(t: Option<TraceId>) -> Vec<u8> {
    t.map(|t| t.0.to_vec()).unwrap_or_default()
}

fn span_id_bytes(s: Option<SpanId>) -> Vec<u8> {
    s.map(|s| s.0.to_vec()).unwrap_or_default()
}

fn hex_span(s: &SpanId) -> String {
    s.0.iter().map(|b| format!("{b:02x}")).collect()
}

fn ns_to_u64(ns: i64) -> u64 {
    u64::try_from(ns).unwrap_or(0)
}

fn body_to_proto(body: Option<&EventBody>) -> Option<AnyValue> {
    let b = body?;
    let json = serde_json::to_string(b).ok()?;
    Some(AnyValue { value: Some(AnyValueInner::StringValue(json)) })
}

fn append_gen_ai_attrs(out: &mut Vec<KeyValue>, g: &GenAi) {
    if let Some(ref s) = g.system {
        out.push(str_kv("gen_ai.system", s));
    }
    if let Some(ref s) = g.operation_name {
        out.push(str_kv("gen_ai.operation.name", s));
    }
    if let Some(ref s) = g.conversation_id {
        out.push(str_kv("gen_ai.conversation.id", s));
    }
    if let Some(ref s) = g.request.model {
        out.push(str_kv("gen_ai.request.model", s));
    }
    if let Some(v) = g.request.temperature {
        out.push(double_kv("gen_ai.request.temperature", v));
    }
    if let Some(v) = g.request.top_p {
        out.push(double_kv("gen_ai.request.top_p", v));
    }
    if let Some(v) = g.request.max_tokens {
        out.push(int_kv("gen_ai.request.max_tokens", i64::from(v)));
    }
    if let Some(ref s) = g.response.model {
        out.push(str_kv("gen_ai.response.model", s));
    }
    if let Some(ref s) = g.response.id {
        out.push(str_kv("gen_ai.response.id", s));
    }
    out.push(bool_kv("gen_ai.response.is_streaming", g.response.is_streaming));
    if let Some(v) = g.usage.input_tokens {
        out.push(int_kv("gen_ai.usage.input_tokens", i64::try_from(v).unwrap_or(i64::MAX)));
    }
    if let Some(v) = g.usage.output_tokens {
        out.push(int_kv("gen_ai.usage.output_tokens", i64::try_from(v).unwrap_or(i64::MAX)));
    }
    if let Some(v) = g.usage.cached_tokens {
        out.push(int_kv("gen_ai.usage.cached_tokens", i64::try_from(v).unwrap_or(i64::MAX)));
    }
    if let Some(v) = g.usage.reasoning_tokens {
        out.push(int_kv("gen_ai.usage.reasoning_tokens", i64::try_from(v).unwrap_or(i64::MAX)));
    }
    if let Some(v) = g.usage.total_tokens {
        out.push(int_kv("gen_ai.usage.total_tokens", i64::try_from(v).unwrap_or(i64::MAX)));
    }
    if let Some(ref tool) = g.tool {
        if let Some(ref n) = tool.name {
            out.push(str_kv("gen_ai.tool.name", n));
        }
        if let Some(ref id) = tool.call_id {
            out.push(str_kv("gen_ai.tool.call_id", id));
        }
        if let Some(t) = tool.r#type {
            out.push(str_kv("gen_ai.tool.type", tool_type_to_str(t)));
        }
    }
    if let Some(ref agent) = g.agent {
        if let Some(ref s) = agent.id {
            out.push(str_kv("gen_ai.agent.id", s));
        }
        if let Some(ref s) = agent.name {
            out.push(str_kv("gen_ai.agent.name", s));
        }
        if let Some(ref s) = agent.description {
            out.push(str_kv("gen_ai.agent.description", s));
        }
    }
}

fn append_mcp_attrs(out: &mut Vec<KeyValue>, m: &Mcp) {
    if let Some(ref s) = m.client_name {
        out.push(str_kv("mcp.client.name", s));
    }
    if let Some(ref s) = m.client_version {
        out.push(str_kv("mcp.client.version", s));
    }
    if let Some(ref s) = m.server_name {
        out.push(str_kv("mcp.server.name", s));
    }
    if let Some(ref s) = m.server_version {
        out.push(str_kv("mcp.server.version", s));
    }
    if let Some(ref s) = m.protocol_version {
        out.push(str_kv("mcp.protocol.version", s));
    }
    if let Some(ref s) = m.tool_name {
        out.push(str_kv("mcp.tool.name", s));
    }
    if let Some(ref s) = m.tool_namespace {
        out.push(str_kv("mcp.tool.namespace", s));
    }
    if let Some(ref s) = m.resource_uri {
        out.push(str_kv("mcp.resource.uri", s));
    }
    if let Some(t) = m.transport {
        out.push(str_kv("mcp.transport", mcp_transport_to_str(t)));
    }
}

fn append_mara_attrs(out: &mut Vec<KeyValue>, m: &MaraExtensions) {
    if let Some(ref s) = m.source_adapter {
        out.push(str_kv("mara.source.adapter", s));
    }
    if let Some(ref s) = m.source_runtime_version {
        out.push(str_kv("mara.source.runtime_version", s));
    }
    if let Some(ref s) = m.session_id {
        out.push(str_kv("mara.session.id", s));
    }
    if let Some(ref s) = m.request_id {
        out.push(str_kv("mara.request_id", s));
    }
    if let Some(ref s) = m.agent_id {
        out.push(str_kv("mara.agent.id", s));
    }
    if let Some(ref s) = m.step_id {
        out.push(str_kv("mara.agent.step_id", s));
    }
    if let Some(ref s) = m.tool_name {
        out.push(str_kv("mara.agent.tool_name", s));
    }
    if let Some(ref s) = m.tool_outcome {
        out.push(str_kv("mara.agent.tool_outcome", s));
    }
    if let Some(ref s) = m.turn_id {
        out.push(str_kv("mara.turn.id", s));
    }
    if let Some(ref s) = m.tenant_id {
        out.push(str_kv("mara.tenant.id", s));
    }
    if let Some(ref s) = m.policy_profile {
        out.push(str_kv("mara.policy.profile", s));
    }
    out.push(bool_kv("mara.policy.capture_optin", m.policy_capture_optin));
    if let Some(v) = m.cost_usd {
        out.push(double_kv("mara.cost.usd", v));
    }
    if let Some(cs) = m.cost_source {
        out.push(str_kv("mara.cost.source", cost_source_to_str(cs)));
    }
    if let Some(cc) = m.cost_confidence {
        out.push(str_kv("mara.cost.confidence", cost_confidence_to_str(cc)));
    }
    if !m.compliance_tags.is_empty() {
        let vals: Vec<AnyValue> = m
            .compliance_tags
            .iter()
            .map(|s| AnyValue { value: Some(AnyValueInner::StringValue(s.clone())) })
            .collect();
        out.push(KeyValue {
            key: "mara.compliance_tags".to_owned(),
            value: Some(AnyValue {
                value: Some(AnyValueInner::ArrayValue(
                    opentelemetry_proto::tonic::common::v1::ArrayValue { values: vals },
                )),
            }),
        });
    }
    if !m.policy_decisions.is_empty()
        && let Ok(json) = serde_json::to_string(&m.policy_decisions)
    {
        out.push(str_kv("mara.policy_decisions_json", &json));
    }
    if (m.body_hashes.prompt.is_some()
        || m.body_hashes.completion.is_some()
        || m.body_hashes.tool_args.is_some())
        && let Ok(json) = serde_json::to_string(&m.body_hashes)
    {
        out.push(str_kv("mara.body_hashes_json", &json));
    }
}

fn attr_kv(key: &str, v: &AttrValue) -> KeyValue {
    KeyValue { key: key.to_owned(), value: Some(attr_to_any(v)) }
}

fn attr_to_any(v: &AttrValue) -> AnyValue {
    match v {
        AttrValue::Null => AnyValue { value: Some(AnyValueInner::StringValue(String::new())) },
        AttrValue::Bool(b) => AnyValue { value: Some(AnyValueInner::BoolValue(*b)) },
        AttrValue::Int(i) => AnyValue { value: Some(AnyValueInner::IntValue(*i)) },
        AttrValue::Float(f) => AnyValue { value: Some(AnyValueInner::DoubleValue(*f)) },
        AttrValue::String(s) => AnyValue { value: Some(AnyValueInner::StringValue(s.clone())) },
        AttrValue::Bytes(b) => AnyValue { value: Some(AnyValueInner::BytesValue(b.clone())) },
        AttrValue::Array(items) => {
            let values: Vec<AnyValue> = items.iter().map(attr_to_any).collect();
            AnyValue {
                value: Some(AnyValueInner::ArrayValue(
                    opentelemetry_proto::tonic::common::v1::ArrayValue { values },
                )),
            }
        }
        AttrValue::Map(map) => {
            let values: Vec<KeyValue> = map
                .iter()
                .map(|(k, vv)| KeyValue { key: k.clone(), value: Some(attr_to_any(vv)) })
                .collect();
            AnyValue {
                value: Some(AnyValueInner::KvlistValue(
                    opentelemetry_proto::tonic::common::v1::KeyValueList { values },
                )),
            }
        }
        _ => AnyValue { value: Some(AnyValueInner::StringValue(format!("{v:?}"))) },
    }
}

fn str_kv(key: &str, value: &str) -> KeyValue {
    KeyValue {
        key: key.to_owned(),
        value: Some(AnyValue { value: Some(AnyValueInner::StringValue(value.to_owned())) }),
    }
}

fn int_kv(key: &str, value: i64) -> KeyValue {
    KeyValue {
        key: key.to_owned(),
        value: Some(AnyValue { value: Some(AnyValueInner::IntValue(value)) }),
    }
}

fn double_kv(key: &str, value: f64) -> KeyValue {
    KeyValue {
        key: key.to_owned(),
        value: Some(AnyValue { value: Some(AnyValueInner::DoubleValue(value)) }),
    }
}

fn bool_kv(key: &str, value: bool) -> KeyValue {
    KeyValue {
        key: key.to_owned(),
        value: Some(AnyValue { value: Some(AnyValueInner::BoolValue(value)) }),
    }
}

fn source_runtime_to_str(r: SourceRuntime) -> &'static str {
    match r {
        SourceRuntime::ClaudeCode => "claude_code",
        SourceRuntime::Codex => "codex",
        SourceRuntime::Cursor => "cursor",
        SourceRuntime::Kimi => "kimi",
        SourceRuntime::Augment => "augment",
        SourceRuntime::Gemini => "gemini",
        SourceRuntime::Ollama => "ollama",
        SourceRuntime::Other => "other",
        _ => "other",
    }
}

fn event_kind_to_str(k: EventKind) -> &'static str {
    match k {
        EventKind::Prompt => "prompt",
        EventKind::Completion => "completion",
        EventKind::ToolCall => "tool_call",
        EventKind::ToolResult => "tool_result",
        EventKind::Cost => "cost",
        EventKind::Error => "error",
        EventKind::System => "system",
        EventKind::Eval => "eval",
        EventKind::Feedback => "feedback",
        _ => "system",
    }
}

fn tool_type_to_str(t: ToolType) -> &'static str {
    match t {
        ToolType::Function => "function",
        ToolType::Retrieval => "retrieval",
        ToolType::CodeInterpreter => "code_interpreter",
        ToolType::Mcp => "mcp",
        _ => "function",
    }
}

fn mcp_transport_to_str(t: mara_schema::McpTransport) -> &'static str {
    match t {
        mara_schema::McpTransport::Stdio => "stdio",
        mara_schema::McpTransport::Http => "http",
        mara_schema::McpTransport::Sse => "sse",
        mara_schema::McpTransport::Websocket => "websocket",
        _ => "stdio",
    }
}

fn cost_confidence_to_str(c: CostConfidence) -> &'static str {
    match c {
        CostConfidence::High => "high",
        CostConfidence::Medium => "medium",
        CostConfidence::Low => "low",
        _ => "medium",
    }
}

fn cost_source_to_str(c: CostSource) -> &'static str {
    match c {
        CostSource::Vendor => "vendor",
        CostSource::MaraEstimated => "mara_estimated",
        _ => "vendor",
    }
}

#[cfg(test)]
mod tests {
    use mara_core::Event;
    use mara_schema::Severity;

    use super::*;

    #[test]
    fn export_request_encodes_non_empty() {
        let ev = Event {
            resource: Resource { service_name: Some("svc".into()), ..Default::default() },
            scope: Scope { name: "scope".into(), version: Some("1".into()) },
            timestamp_ns: 1_234,
            observed_timestamp_ns: 1_234,
            trace_id: None,
            span_id: None,
            parent_span_id: None,
            event_kind: EventKind::System,
            severity: Severity::INFO,
            gen_ai: GenAi::default(),
            mcp: None,
            mara: MaraExtensions::default(),
            attributes: Default::default(),
            body: None,
        };
        let req = events_to_export_request(std::slice::from_ref(&ev));
        assert_eq!(req.resource_logs.len(), 1);
        let rl = &req.resource_logs[0];
        assert!(rl.resource.is_some());
        assert_eq!(rl.scope_logs.len(), 1);
        assert_eq!(rl.scope_logs[0].log_records.len(), 1);
    }
}
