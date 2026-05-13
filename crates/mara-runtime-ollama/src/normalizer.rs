//! Ollama-specific [`UpstreamNormalizer`](mara_adapter_llm_proxy::UpstreamNormalizer).

use mara_adapter_llm_proxy::{ProxiedRequest, ProxiedResponse, UpstreamNormalizer};
use mara_core::Event;
use mara_schema::{AttrValue, EventKind, Resource, Severity, SourceRuntime};

/// Maps proxied Ollama HTTP exchanges to canonical Mara events.
#[derive(Clone, Copy, Debug, Default)]
pub struct OllamaNormalizer;

impl UpstreamNormalizer for OllamaNormalizer {
    fn normalize(
        &self,
        session_id: &str,
        request: &ProxiedRequest,
        response: &ProxiedResponse,
    ) -> Vec<Event> {
        if !(200..300).contains(&response.status) {
            let mut ev = base_event(session_id);
            ev.event_kind = EventKind::Error;
            ev.severity = Severity::ERROR;
            ev.attributes
                .insert("http.status_code".into(), AttrValue::Int(i64::from(response.status)));
            if let Some(ref fk) = response.failure_kind {
                ev.attributes
                    .insert("mara.proxy.failure_kind".into(), AttrValue::String(fk.clone()));
            }
            if let Some(us) = response.upstream_status {
                ev.attributes
                    .insert("mara.proxy.upstream_status".into(), AttrValue::Int(i64::from(us)));
            }
            return vec![ev];
        }

        let mut ev = base_event(session_id);
        ev.gen_ai.system = Some("ollama".into());
        if response.stream_cut_short {
            ev.attributes.insert("mara.ollama.partial".into(), AttrValue::Bool(true));
        }

        let path = request.path_and_query.as_str();
        if path.contains("/api/chat") || path.contains("/v1/chat/completions") {
            ev.gen_ai.operation_name = Some("chat".into());
        } else if path.contains("/api/generate") || path.contains("/v1/completions") {
            ev.gen_ai.operation_name = Some("text_completion".into());
        } else if path.contains("embed") {
            ev.gen_ai.operation_name = Some("embeddings".into());
        }

        if let Ok(v) = serde_json::from_slice::<serde_json::Value>(&response.body) {
            apply_json_fields(&mut ev, &v);
            let openai_has_choices = v
                .get("choices")
                .and_then(serde_json::Value::as_array)
                .is_some_and(|a| !a.is_empty());
            if ev.gen_ai.operation_name.as_deref() == Some("chat")
                && (ev.gen_ai.usage.output_tokens.is_some()
                    || v.get("message").is_some()
                    || openai_has_choices)
            {
                ev.event_kind = EventKind::Completion;
            }
        }

        vec![ev]
    }
}

fn base_event(session_id: &str) -> Event {
    let mut ev = Event::now(EventKind::System, "mara-runtime-ollama");
    ev.mara.session_id = Some(session_id.to_owned());
    ev.resource = Resource { source_runtime: Some(SourceRuntime::Ollama), ..Default::default() };
    ev
}

fn apply_json_fields(ev: &mut Event, v: &serde_json::Value) {
    if let Some(n) = v.get("prompt_eval_count").and_then(serde_json::Value::as_u64) {
        ev.gen_ai.usage.input_tokens = Some(n);
    }
    if let Some(n) = v.get("eval_count").and_then(serde_json::Value::as_u64) {
        ev.gen_ai.usage.output_tokens = Some(n);
    }
    if ev.gen_ai.usage.input_tokens.is_none()
        && let Some(n) = v.pointer("/usage/prompt_tokens").and_then(serde_json::Value::as_u64)
    {
        ev.gen_ai.usage.input_tokens = Some(n);
    }
    if ev.gen_ai.usage.output_tokens.is_none()
        && let Some(n) = v.pointer("/usage/completion_tokens").and_then(serde_json::Value::as_u64)
    {
        ev.gen_ai.usage.output_tokens = Some(n);
    }
    if let Some(model) = v.get("model").and_then(|x| x.as_str()) {
        ev.gen_ai.response.model = Some(model.to_owned());
    }
    if let Some(ns) = v.get("total_duration").and_then(serde_json::Value::as_u64) {
        let ms = (ns as f64) / 1_000_000.0;
        ev.attributes.insert("mara.ollama.total_duration_ms".into(), AttrValue::Float(ms));
    }
    if let Some(ns) = v.get("load_duration").and_then(serde_json::Value::as_u64) {
        let ms = (ns as f64) / 1_000_000.0;
        ev.attributes.insert("mara.ollama.load_duration_ms".into(), AttrValue::Float(ms));
    }
    if let Some(ns) = v.get("prompt_eval_duration").and_then(serde_json::Value::as_u64) {
        let ms = (ns as f64) / 1_000_000.0;
        ev.attributes.insert("mara.ollama.prompt_eval_duration_ms".into(), AttrValue::Float(ms));
    }
    if let Some(ns) = v.get("eval_duration").and_then(serde_json::Value::as_u64) {
        let ms = (ns as f64) / 1_000_000.0;
        ev.attributes.insert("mara.ollama.eval_duration_ms".into(), AttrValue::Float(ms));
    }
    if let (Some(ec), Some(ed_ns)) = (
        v.get("eval_count").and_then(serde_json::Value::as_u64),
        v.get("eval_duration").and_then(serde_json::Value::as_u64),
    ) {
        let ed_sec = ed_ns as f64 / 1_000_000_000.0;
        if ed_sec > 0.0 {
            let tps = ec as f64 / ed_sec;
            ev.attributes.insert("mara.ollama.tokens_per_sec".into(), AttrValue::Float(tps));
        }
    }

    ev.mara.cost_usd = Some(0.0);
    ev.mara.cost_source = Some(mara_schema::CostSource::MaraEstimated);
    ev.attributes.insert("mara.compute.is_local".into(), AttrValue::Bool(true));
}

#[cfg(test)]
mod tests {
    use bytes::Bytes;
    use mara_schema::AttrValue;

    use super::*;

    #[test]
    fn parses_native_chat_counters() {
        let n = OllamaNormalizer;
        let req = ProxiedRequest {
            method: "POST".into(),
            path_and_query: "/api/chat".into(),
            headers: vec![],
            body: Bytes::new(),
            body_truncated: false,
        };
        let body = br#"{"model":"llama3.2","eval_count":16,"prompt_eval_count":128,"total_duration":5000000}"#;
        let resp = ProxiedResponse::from_upstream(200, vec![], Bytes::from_static(body), false);
        let evs = n.normalize("sess-1", &req, &resp);
        assert_eq!(evs.len(), 1);
        let ev = &evs[0];
        assert_eq!(ev.gen_ai.usage.input_tokens, Some(128));
        assert_eq!(ev.gen_ai.usage.output_tokens, Some(16));
        assert_eq!(ev.gen_ai.response.model.as_deref(), Some("llama3.2"));
        assert!(matches!(ev.event_kind, EventKind::Completion));
    }

    #[test]
    fn proxy_synthetic_502_records_failure_kind() {
        let n = OllamaNormalizer;
        let req = ProxiedRequest {
            method: "POST".into(),
            path_and_query: "/api/chat".into(),
            headers: vec![],
            body: Bytes::new(),
            body_truncated: false,
        };
        let resp = ProxiedResponse {
            status: 502,
            headers: vec![],
            body: Bytes::from_static(b"nope"),
            body_truncated: false,
            failure_kind: Some("upstream_transport".into()),
            upstream_status: None,
            stream_cut_short: false,
        };
        let evs = n.normalize("s", &req, &resp);
        assert_eq!(evs.len(), 1);
        let ev = &evs[0];
        assert!(matches!(ev.event_kind, EventKind::Error));
        assert_eq!(
            ev.attributes.get("mara.proxy.failure_kind"),
            Some(&AttrValue::String("upstream_transport".into()))
        );
    }

    #[test]
    fn upstream_503_records_http_status() {
        let n = OllamaNormalizer;
        let req = ProxiedRequest {
            method: "POST".into(),
            path_and_query: "/v1/chat/completions".into(),
            headers: vec![],
            body: Bytes::new(),
            body_truncated: false,
        };
        let resp = ProxiedResponse::from_upstream(
            503,
            vec![],
            Bytes::from_static(br#"{"err":"x"}"#),
            false,
        );
        let evs = n.normalize("sess-503", &req, &resp);
        assert_eq!(evs.len(), 1);
        let ev = &evs[0];
        assert!(matches!(ev.event_kind, EventKind::Error));
        assert_eq!(ev.attributes.get("http.status_code"), Some(&AttrValue::Int(503)));
    }

    #[test]
    fn parses_openai_compat_chat_usage() {
        let n = OllamaNormalizer;
        let req = ProxiedRequest {
            method: "POST".into(),
            path_and_query: "/v1/chat/completions".into(),
            headers: vec![],
            body: Bytes::new(),
            body_truncated: false,
        };
        let body = br#"{"model":"qwen2.5","choices":[{"index":0,"message":{"role":"assistant","content":"ok"},"finish_reason":"stop"}],"usage":{"prompt_tokens":3,"completion_tokens":7,"total_tokens":10}}"#;
        let resp = ProxiedResponse::from_upstream(200, vec![], Bytes::from_static(body), false);
        let evs = n.normalize("sess-2", &req, &resp);
        assert_eq!(evs.len(), 1);
        let ev = &evs[0];
        assert_eq!(ev.gen_ai.usage.input_tokens, Some(3));
        assert_eq!(ev.gen_ai.usage.output_tokens, Some(7));
        assert_eq!(ev.gen_ai.response.model.as_deref(), Some("qwen2.5"));
        assert!(matches!(ev.event_kind, EventKind::Completion));
    }
}
