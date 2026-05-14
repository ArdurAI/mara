//! Unconditional deny policy stage (M1-08).
//!
//! Drops every event that reaches this stage. Use for kill-switch chains
//! or testing; place earlier in the chain if later stages should not run.

use async_trait::async_trait;
use mara_core::error::Result;
use mara_core::policy::{Policy, PolicyContext, PolicyOutcome};
use mara_schema::{Event, PolicyDecisionKind};

/// Drops all events with a fixed operator-facing reason.
#[derive(Debug, Clone)]
pub struct DenyAll {
    reason: String,
}

impl DenyAll {
    /// Build a deny stage. Empty or missing `reason` defaults to `policy:deny`.
    #[must_use]
    pub fn new(reason: Option<String>) -> Self {
        let reason = reason
            .map(|s| s.trim().to_owned())
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| "policy:deny".into());
        Self { reason }
    }
}

#[async_trait]
impl Policy for DenyAll {
    fn name(&self) -> &str {
        "builtin.deny.all"
    }

    async fn apply(&self, ctx: &PolicyContext, mut event: Event) -> Result<PolicyOutcome> {
        ctx.record_decision(&mut event, PolicyDecisionKind::Denied, Some(self.reason.clone()));
        Ok(PolicyOutcome::drop(event, self.reason.clone()))
    }
}

#[cfg(test)]
mod tests {
    use mara_core::policy::{ChainOutcome, PolicyChain};
    use mara_schema::EventKind;
    use std::sync::Arc;

    use super::*;

    #[tokio::test]
    async fn deny_drops_with_configured_reason() {
        let p = DenyAll::new(Some("blocked by policy".into()));
        let ctx = PolicyContext::new("deny", "default");
        let ev = Event::now(EventKind::Prompt, "t");
        let out = p.apply(&ctx, ev).await.expect("apply");
        match out {
            PolicyOutcome::Drop { reason, .. } => assert_eq!(reason, "blocked by policy"),
            _ => panic!("expected drop"),
        }
    }

    #[tokio::test]
    async fn deny_in_chain_drops() {
        let chain = PolicyChain::new(
            "deny-chain",
            vec![Arc::new(DenyAll::new(Some("nope".into()))) as Arc<dyn Policy>],
        );
        let ev = Event::now(EventKind::Prompt, "x");
        let out = chain.run(ev).await.expect("run");
        match out {
            ChainOutcome::Drop { reason, .. } => assert_eq!(reason, "nope"),
            ChainOutcome::Deliver(_) => panic!("expected drop"),
            _ => panic!("unexpected chain outcome"),
        }
    }
}
