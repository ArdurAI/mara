//! Privacy capture modes for optional `Event.body` (M1-07).

use async_trait::async_trait;
use mara_core::error::Result;
use mara_core::policy::{Policy, PolicyContext, PolicyOutcome};
use mara_schema::{Event, EventBody, PolicyDecisionKind};
use sha2::{Digest, Sha256};

use mara_core::config::PrivacyCaptureMode;

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    digest.iter().map(|b| format!("{b:02x}")).collect()
}

fn hash_json<T: serde::Serialize>(v: &T) -> Option<String> {
    serde_json::to_vec(v).ok().map(|b| sha256_hex(&b))
}

fn apply_hashes_from_body(body: &EventBody, hashes: &mut mara_schema::BodyHashes) {
    if let Some(ref p) = body.prompt {
        hashes.prompt = hash_json(p).or_else(|| Some(sha256_hex(b"")));
    }
    if let Some(ref c) = body.completion {
        hashes.completion = hash_json(c).or_else(|| Some(sha256_hex(b"")));
    }
    if let Some(ref t) = body.tool_call
        && let Some(ref args) = t.arguments_json
    {
        hashes.tool_args = Some(sha256_hex(args.as_bytes()));
    }
    if let Some(ref tr) = body.tool_result
        && hashes.tool_args.is_none()
        && let Some(ref cj) = tr.content_json
    {
        hashes.tool_args = Some(sha256_hex(cj.as_bytes()));
    }
    if let Some(ref rr) = body.raw_request
        && hashes.prompt.is_none()
    {
        hashes.prompt = Some(sha256_hex(rr.as_bytes()));
    }
    if let Some(ref rr) = body.raw_response
        && hashes.completion.is_none()
    {
        hashes.completion = Some(sha256_hex(rr.as_bytes()));
    }
}

fn clear_body(event: &mut Event) {
    event.body = None;
}

fn clear_hashes(event: &mut Event) {
    event.mara.body_hashes = mara_schema::BodyHashes::default();
}

/// Enforces `metadata_only`, `hashed_bodies`, or `body_opt_in` handling for captured bodies.
#[derive(Debug, Clone, Copy)]
pub struct PrivacyFilter {
    mode: PrivacyCaptureMode,
}

impl PrivacyFilter {
    /// Build a filter for the given [`PrivacyCaptureMode`].
    #[must_use]
    pub const fn new(mode: PrivacyCaptureMode) -> Self {
        Self { mode }
    }
}

#[async_trait]
impl Policy for PrivacyFilter {
    fn name(&self) -> &str {
        "builtin.privacy.capture"
    }

    async fn apply(&self, ctx: &PolicyContext, mut event: Event) -> Result<PolicyOutcome> {
        let reason = match self.mode {
            PrivacyCaptureMode::MetadataOnly => {
                clear_body(&mut event);
                clear_hashes(&mut event);
                "privacy:metadata_only"
            }
            PrivacyCaptureMode::HashedBodies => {
                if let Some(ref body) = event.body {
                    apply_hashes_from_body(body, &mut event.mara.body_hashes);
                }
                clear_body(&mut event);
                "privacy:hashed_bodies"
            }
            PrivacyCaptureMode::BodyOptIn => {
                if event.mara.policy_capture_optin {
                    ctx.record_decision(&mut event, PolicyDecisionKind::Allowed, Some("privacy:body_opt_in:kept".into()));
                    return Ok(PolicyOutcome::pass(event));
                }
                clear_body(&mut event);
                clear_hashes(&mut event);
                "privacy:body_opt_in:stripped"
            }
        };

        ctx.record_decision(
            &mut event,
            PolicyDecisionKind::Transformed,
            Some(reason.to_owned()),
        );
        Ok(PolicyOutcome::pass(event))
    }
}

#[cfg(test)]
mod tests {
    use mara_schema::{CompletionBody, CompletionChoice, EventKind, Message, PromptBody};

    use super::*;

    fn event_with_body() -> Event {
        let mut ev = Event::now(EventKind::Prompt, "t");
        ev.body = Some(EventBody {
            prompt: Some(PromptBody {
                messages: vec![Message {
                    role: "user".into(),
                    content: "secret-prompt".into(),
                }],
            }),
            completion: Some(CompletionBody {
                choices: vec![CompletionChoice {
                    message: Message {
                        role: "assistant".into(),
                        content: "secret-out".into(),
                    },
                    finish_reason: None,
                }],
            }),
            ..Default::default()
        });
        ev
    }

    #[tokio::test]
    async fn metadata_only_strips_body_and_hashes() {
        let mut ev = event_with_body();
        ev.mara.body_hashes.prompt = Some("old".into());
        let p = PrivacyFilter::new(PrivacyCaptureMode::MetadataOnly);
        let ctx = PolicyContext::new("privacy", "default");
        let PolicyOutcome::Pass(out) = p.apply(&ctx, ev).await.expect("apply") else {
            panic!("expected pass");
        };
        assert!(out.body.is_none());
        assert!(out.mara.body_hashes.prompt.is_none());
    }

    #[tokio::test]
    async fn hashed_bodies_fills_hashes_and_strips_body() {
        let ev = event_with_body();
        let p = PrivacyFilter::new(PrivacyCaptureMode::HashedBodies);
        let ctx = PolicyContext::new("privacy", "default");
        let PolicyOutcome::Pass(out) = p.apply(&ctx, ev).await.expect("apply") else {
            panic!("expected pass");
        };
        assert!(out.body.is_none());
        assert!(out.mara.body_hashes.prompt.as_deref().is_some_and(|s| s.len() == 64));
        assert!(out.mara.body_hashes.completion.as_deref().is_some_and(|s| s.len() == 64));
    }

    #[tokio::test]
    async fn body_opt_in_keeps_when_flag_set() {
        let mut ev = event_with_body();
        ev.mara.policy_capture_optin = true;
        let p = PrivacyFilter::new(PrivacyCaptureMode::BodyOptIn);
        let ctx = PolicyContext::new("privacy", "default");
        let PolicyOutcome::Pass(out) = p.apply(&ctx, ev).await.expect("apply") else {
            panic!("expected pass");
        };
        assert!(out.body.is_some());
    }

    #[tokio::test]
    async fn body_opt_in_strips_when_flag_clear() {
        let ev = event_with_body();
        let p = PrivacyFilter::new(PrivacyCaptureMode::BodyOptIn);
        let ctx = PolicyContext::new("privacy", "default");
        let PolicyOutcome::Pass(out) = p.apply(&ctx, ev).await.expect("apply") else {
            panic!("expected pass");
        };
        assert!(out.body.is_none());
    }
}
