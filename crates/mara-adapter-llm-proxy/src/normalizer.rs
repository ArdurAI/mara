//! Runtime-supplied translation from captured HTTP to canonical events.

use mara_core::Event;

use crate::exchange::{ProxiedRequest, ProxiedResponse};

/// Translates a proxied request/response pair into zero or more Mara
/// [`Event`] values.
pub trait UpstreamNormalizer: Send + Sync {
    /// Produce events for one completed exchange.
    fn normalize(
        &self,
        session_id: &str,
        request: &ProxiedRequest,
        response: &ProxiedResponse,
    ) -> Vec<Event>;
}

/// Debug / generic normalizer: emits a single [`EventKind::System`]
/// record with HTTP metadata and body sizes (no JSON parsing).
#[derive(Clone, Copy, Debug, Default)]
pub struct PassthroughNormalizer;

impl UpstreamNormalizer for PassthroughNormalizer {
    fn normalize(
        &self,
        session_id: &str,
        request: &ProxiedRequest,
        response: &ProxiedResponse,
    ) -> Vec<Event> {
        use mara_schema::{AttrValue, EventKind, Severity};

        let mut ev = Event::now(EventKind::System, "mara-adapter-llm-proxy");
        if response.failure_kind.is_some() || !(200..300).contains(&response.status) {
            ev.event_kind = EventKind::Error;
            ev.severity = Severity::ERROR;
        }
        ev.mara.session_id = Some(session_id.to_owned());
        ev.attributes.insert("http.method".into(), AttrValue::String(request.method.clone()));
        ev.attributes.insert(
            "http.path_and_query".into(),
            AttrValue::String(request.path_and_query.clone()),
        );
        ev.attributes.insert("http.status_code".into(), AttrValue::Int(i64::from(response.status)));
        if let Some(ref fk) = response.failure_kind {
            ev.attributes.insert("mara.proxy.failure_kind".into(), AttrValue::String(fk.clone()));
        }
        if let Some(us) = response.upstream_status {
            ev.attributes
                .insert("mara.proxy.upstream_status".into(), AttrValue::Int(i64::from(us)));
        }
        if response.stream_cut_short {
            ev.attributes.insert("mara.proxy.stream_cut_short".into(), AttrValue::Bool(true));
        }
        ev.attributes.insert(
            "mara.proxy.request_bytes".into(),
            AttrValue::Int(i64::try_from(request.body.len()).unwrap_or(i64::MAX)),
        );
        ev.attributes.insert(
            "mara.proxy.response_bytes".into(),
            AttrValue::Int(i64::try_from(response.body.len()).unwrap_or(i64::MAX)),
        );
        ev.attributes
            .insert("mara.proxy.request_truncated".into(), AttrValue::Bool(request.body_truncated));
        ev.attributes.insert(
            "mara.proxy.response_truncated".into(),
            AttrValue::Bool(response.body_truncated),
        );
        vec![ev]
    }
}
