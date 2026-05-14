//! W3C `traceparent` propagation for proxied HTTP exchanges (M1-03).

use mara_core::Event;
use mara_schema::{SpanId, TraceId};

use crate::exchange::ProxiedRequest;

/// If the inbound request carries a valid `traceparent` header, set `trace_id` and `span_id`
/// on `ev` to the decoded trace and span identifiers from that header.
pub fn apply_traceparent_from_request(ev: &mut Event, request: &ProxiedRequest) {
    let Some(raw) = header_value_trimmed(request, "traceparent") else {
        return;
    };
    if let Some((tid, sid)) = parse_traceparent(&raw) {
        ev.trace_id = Some(tid);
        ev.span_id = Some(sid);
    }
}

fn header_value_trimmed(request: &ProxiedRequest, name: &str) -> Option<String> {
    for (k, val) in &request.headers {
        if k.as_str().eq_ignore_ascii_case(name) {
            let t = val.trim();
            if !t.is_empty() {
                return Some(t.to_owned());
            }
        }
    }
    None
}

/// Parse `traceparent` per W3C Trace Context: `version-traceid-spanid-flags`.
///
/// Returns `None` if the value is malformed or uses an all-zero trace id.
#[must_use]
pub(crate) fn parse_traceparent(value: &str) -> Option<(TraceId, SpanId)> {
    let mut parts = value.trim().split('-');
    let version = parts.next()?;
    let trace_hex = parts.next()?;
    let span_hex = parts.next()?;
    let flags = parts.next()?;
    if parts.next().is_some() {
        return None;
    }
    if version.len() != 2 || flags.len() != 2 {
        return None;
    }
    if trace_hex.len() != 32 || span_hex.len() != 16 {
        return None;
    }
    if !trace_hex.bytes().all(|b| b.is_ascii_hexdigit())
        || !span_hex.bytes().all(|b| b.is_ascii_hexdigit())
    {
        return None;
    }
    let trace_bytes = decode_hex_fixed::<16>(trace_hex)?;
    let span_bytes = decode_hex_fixed::<8>(span_hex)?;
    if trace_bytes == [0u8; 16] {
        return None;
    }
    Some((TraceId(trace_bytes), SpanId(span_bytes)))
}

fn decode_hex_fixed<const N: usize>(s: &str) -> Option<[u8; N]> {
    if s.len() != N * 2 {
        return None;
    }
    let mut out = [0u8; N];
    let b = s.as_bytes();
    for i in 0..N {
        let hi = decode_hex_nibble(b[i * 2])?;
        let lo = decode_hex_nibble(b[i * 2 + 1])?;
        out[i] = (hi << 4) | lo;
    }
    Some(out)
}

fn decode_hex_nibble(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_w3c_example_traceparent() {
        let s = "00-0af7651916cd43dd8448eb211c80319c-b7ad6b7169203331-01";
        let (tid, sid) = parse_traceparent(s).expect("valid");
        assert_eq!(
            tid.0,
            [
                0x0a, 0xf7, 0x65, 0x19, 0x16, 0xcd, 0x43, 0xdd, 0x84, 0x48, 0xeb, 0x21, 0x1c, 0x80,
                0x31, 0x9c
            ]
        );
        assert_eq!(sid.0, [0xb7, 0xad, 0x6b, 0x71, 0x69, 0x20, 0x33, 0x31]);
    }

    #[test]
    fn rejects_all_zero_trace_id() {
        assert!(
            parse_traceparent("00-00000000000000000000000000000000-0000000000000000-00").is_none()
        );
    }

    #[test]
    fn rejects_bad_segment_count() {
        assert!(parse_traceparent("00-abc").is_none());
    }

    #[test]
    fn rejects_flags_not_two_chars() {
        assert!(parse_traceparent("00-0af7651916cd43dd8448eb211c80319c-b7ad6b7169203331-1").is_none());
    }

    #[test]
    fn rejects_invalid_hex_in_trace_id() {
        assert!(parse_traceparent("00-0af7651916cd43dd8448eb211c80319g-b7ad6b7169203331-01").is_none());
    }
}
