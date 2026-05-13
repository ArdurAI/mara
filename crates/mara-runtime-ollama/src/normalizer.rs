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

        apply_request_fields(&mut ev, request);

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
    ev.resource = Resource {
        service_name: std::env::var("MARA_SERVICE_NAME").ok().filter(|s| !s.is_empty()),
        host_name: hostname::get().ok().and_then(|h| h.into_string().ok()),
        process_pid: Some(std::process::id()),
        source_runtime: Some(SourceRuntime::Ollama),
        ..Default::default()
    };
    ev
}

/// Fills `gen_ai.request` (and client `stream` intent on `gen_ai.response.is_streaming`) from the
/// proxied **client** JSON body. Ollama native uses a nested `options` object; OpenAI-compatible
/// requests often put `temperature`, `max_tokens`, etc. at the top level.
fn apply_request_fields(ev: &mut Event, request: &ProxiedRequest) {
    if request.body_truncated || request.body.is_empty() {
        return;
    }
    let Ok(v) = serde_json::from_slice::<serde_json::Value>(&request.body) else {
        return;
    };
    if let Some(m) = v.get("model").and_then(serde_json::Value::as_str).filter(|s| !s.is_empty()) {
        ev.gen_ai.request.model = Some(m.to_owned());
    }
    if let Some(b) = v.get("stream").and_then(serde_json::Value::as_bool) {
        ev.gen_ai.response.is_streaming = b;
    }
    // Native Ollama: tunables live under `options`. OpenAI-compat: often top-level.
    let opts = v.get("options");
    let pick = |key: &str| -> Option<&serde_json::Value> {
        opts.and_then(|o| o.get(key)).or_else(|| v.get(key))
    };
    if let Some(t) = pick("temperature").and_then(serde_json::Value::as_f64) {
        ev.gen_ai.request.temperature = Some(t);
    }
    if let Some(t) = pick("top_p").and_then(serde_json::Value::as_f64) {
        ev.gen_ai.request.top_p = Some(t);
    }
    if let Some(k) = pick("top_k").and_then(serde_json::Value::as_u64) {
        ev.gen_ai.request.top_k = Some(k.min(u64::from(u32::MAX)) as u32);
    }
    if let Some(n) = pick("num_predict").or_else(|| v.get("max_tokens")).and_then(|x| x.as_u64()) {
        ev.gen_ai.request.max_tokens = Some(n.min(u64::from(u32::MAX)) as u32);
    }
    if let Some(s) = pick("seed").and_then(serde_json::Value::as_i64) {
        ev.gen_ai.request.seed = Some(s);
    }
    if let Some(p) = pick("presence_penalty").and_then(serde_json::Value::as_f64) {
        ev.gen_ai.request.presence_penalty = Some(p);
    }
    if let Some(f) = pick("frequency_penalty").and_then(serde_json::Value::as_f64) {
        ev.gen_ai.request.frequency_penalty = Some(f);
    }
    append_stop_sequences(&mut ev.gen_ai.request.stop_sequences, pick("stop"));
}

fn append_stop_sequences(out: &mut Vec<String>, stop: Option<&serde_json::Value>) {
    let Some(s) = stop else { return };
    match s {
        serde_json::Value::String(t) => {
            if !t.is_empty() {
                out.push(t.clone());
            }
        }
        serde_json::Value::Array(items) => {
            for it in items {
                if let Some(t) = it.as_str().filter(|x| !x.is_empty()) {
                    out.push(t.to_owned());
                }
            }
        }
        _ => {}
    }
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
    if let Some(n) = v.pointer("/usage/total_tokens").and_then(serde_json::Value::as_u64) {
        ev.gen_ai.usage.total_tokens = Some(n);
    }
    if let Some(n) = v
        .pointer("/usage/prompt_tokens_details/cache_read_tokens")
        .or_else(|| v.pointer("/usage/cache_read_input_tokens"))
        .and_then(serde_json::Value::as_u64)
    {
        ev.gen_ai.usage.cached_tokens = Some(n);
    }
    if let Some(model) = v.get("model").and_then(|x| x.as_str()) {
        ev.gen_ai.response.model = Some(model.to_owned());
    }
    if let Some(dr) = v.get("done_reason").and_then(serde_json::Value::as_str) {
        ev.gen_ai.response.finish_reasons = vec![dr.to_owned()];
    } else if let Some(fr) =
        v.pointer("/choices/0/finish_reason").and_then(serde_json::Value::as_str)
    {
        ev.gen_ai.response.finish_reasons = vec![fr.to_owned()];
    }
    if let Some(id) = v.get("id").and_then(serde_json::Value::as_str).filter(|s| !s.is_empty()) {
        ev.gen_ai.response.id = Some(id.to_owned());
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

    if ev.gen_ai.usage.total_tokens.is_none()
        && let (Some(i), Some(o)) = (ev.gen_ai.usage.input_tokens, ev.gen_ai.usage.output_tokens)
    {
        ev.gen_ai.usage.total_tokens = Some(i + o);
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
        assert_eq!(ev.resource.process_pid, Some(std::process::id()));
        assert_eq!(ev.resource.source_runtime, Some(mara_schema::SourceRuntime::Ollama));
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

    #[test]
    fn fills_generate_request_from_client_json_and_response_meta() {
        let n = OllamaNormalizer;
        let req = ProxiedRequest {
            method: "POST".into(),
            path_and_query: "/api/generate".into(),
            headers: vec![],
            body: Bytes::from_static(
                br#"{"model":"gpt-oss:120b-cloud","stream":false,"options":{"temperature":0.7,"top_p":0.9,"top_k":40,"num_predict":256,"stop":["\n\n"]}}"#,
            ),
            body_truncated: false,
        };
        let body = br#"{"model":"gpt-oss:120b","done_reason":"stop","prompt_eval_count":10,"eval_count":3,"total_duration":1000000}"#;
        let resp = ProxiedResponse::from_upstream(200, vec![], Bytes::from_static(body), false);
        let evs = n.normalize("sess-req", &req, &resp);
        let ev = &evs[0];
        assert_eq!(ev.gen_ai.request.model.as_deref(), Some("gpt-oss:120b-cloud"));
        assert_eq!(ev.gen_ai.request.temperature, Some(0.7));
        assert_eq!(ev.gen_ai.request.top_p, Some(0.9));
        assert_eq!(ev.gen_ai.request.top_k, Some(40));
        assert_eq!(ev.gen_ai.request.max_tokens, Some(256));
        assert_eq!(ev.gen_ai.request.stop_sequences, vec!["\n\n"]);
        assert!(!ev.gen_ai.response.is_streaming);
        assert_eq!(ev.gen_ai.usage.total_tokens, Some(13));
        assert_eq!(ev.gen_ai.response.finish_reasons, vec!["stop"]);
        assert_eq!(ev.gen_ai.response.model.as_deref(), Some("gpt-oss:120b"));
    }

    #[test]
    fn fills_openai_compat_request_top_level_tuning() {
        let n = OllamaNormalizer;
        let req = ProxiedRequest {
            method: "POST".into(),
            path_and_query: "/v1/chat/completions".into(),
            headers: vec![],
            body: Bytes::from_static(
                br#"{"model":"qwen2.5","temperature":0.2,"max_tokens":512,"stream":true,"stop":"USER:"}"#,
            ),
            body_truncated: false,
        };
        let body = br#"{"model":"qwen2.5","choices":[{"finish_reason":"length"}],"usage":{"prompt_tokens":1,"completion_tokens":2,"total_tokens":3}}"#;
        let resp = ProxiedResponse::from_upstream(200, vec![], Bytes::from_static(body), false);
        let evs = n.normalize("sess-oai", &req, &resp);
        let ev = &evs[0];
        assert_eq!(ev.gen_ai.request.model.as_deref(), Some("qwen2.5"));
        assert_eq!(ev.gen_ai.request.temperature, Some(0.2));
        assert_eq!(ev.gen_ai.request.max_tokens, Some(512));
        assert!(ev.gen_ai.response.is_streaming);
        assert_eq!(ev.gen_ai.request.stop_sequences, vec!["USER:"]);
        assert_eq!(ev.gen_ai.response.finish_reasons, vec!["length"]);
        assert_eq!(ev.gen_ai.usage.total_tokens, Some(3));
    }

    /// M0 guardrail: `/api/generate` success path must always expose core gen_ai fields for operators.
    #[test]
    fn guardrail_api_generate_requires_operation_usage_and_models() {
        let n = OllamaNormalizer;
        let req = ProxiedRequest {
            method: "POST".into(),
            path_and_query: "/api/generate".into(),
            headers: vec![],
            body: Bytes::from_static(
                br#"{"model":"llama3.2:latest","prompt":"hi","stream":false}"#,
            ),
            body_truncated: false,
        };
        let body = br#"{"model":"llama3.2:latest","response":"ok","done":true,"done_reason":"stop","prompt_eval_count":9,"eval_count":4}"#;
        let resp = ProxiedResponse::from_upstream(200, vec![], Bytes::from_static(body), false);
        let evs = n.normalize("guard-gen", &req, &resp);
        let ev = &evs[0];
        assert_eq!(ev.gen_ai.operation_name.as_deref(), Some("text_completion"));
        assert_eq!(ev.gen_ai.request.model.as_deref(), Some("llama3.2:latest"));
        assert_eq!(ev.gen_ai.response.model.as_deref(), Some("llama3.2:latest"));
        assert_eq!(ev.gen_ai.usage.input_tokens, Some(9));
        assert_eq!(ev.gen_ai.usage.output_tokens, Some(4));
        assert_eq!(ev.gen_ai.usage.total_tokens, Some(13));
        assert_eq!(ev.gen_ai.response.finish_reasons, vec!["stop"]);
    }

    /// M0 guardrail: `/api/chat` success path must always expose core gen_ai fields for operators.
    #[test]
    fn guardrail_api_chat_requires_operation_usage_and_models() {
        let n = OllamaNormalizer;
        let req = ProxiedRequest {
            method: "POST".into(),
            path_and_query: "/api/chat".into(),
            headers: vec![],
            body: Bytes::from_static(
                br#"{"model":"llama3.2:latest","messages":[{"role":"user","content":"hi"}],"stream":false}"#,
            ),
            body_truncated: false,
        };
        let body = br#"{"model":"llama3.2:latest","message":{"role":"assistant","content":"ok"},"done":true,"done_reason":"stop","prompt_eval_count":11,"eval_count":3}"#;
        let resp = ProxiedResponse::from_upstream(200, vec![], Bytes::from_static(body), false);
        let evs = n.normalize("guard-chat", &req, &resp);
        let ev = &evs[0];
        assert_eq!(ev.gen_ai.operation_name.as_deref(), Some("chat"));
        assert!(matches!(ev.event_kind, EventKind::Completion));
        assert_eq!(ev.gen_ai.request.model.as_deref(), Some("llama3.2:latest"));
        assert_eq!(ev.gen_ai.response.model.as_deref(), Some("llama3.2:latest"));
        assert_eq!(ev.gen_ai.usage.input_tokens, Some(11));
        assert_eq!(ev.gen_ai.usage.output_tokens, Some(3));
        assert_eq!(ev.gen_ai.usage.total_tokens, Some(14));
        assert_eq!(ev.gen_ai.response.finish_reasons, vec!["stop"]);
    }
}
