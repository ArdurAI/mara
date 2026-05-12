//! Policy stage contract and policy chain runtime.
//!
//! Policy stages decide what happens to a canonical event before
//! it reaches sinks: pass it through, redact a field, drop it,
//! sample it, rate-limit it, classify it, or route it.  Each stage
//! is invoked once per event.  A [`PolicyChain`] runs a sequence
//! of stages in order; if any stage drops the event the remaining
//! stages are skipped.

use std::sync::Arc;

use async_trait::async_trait;
use mara_schema::{Event, PolicyDecisionKind, PolicyDecisionRecord};

use crate::error::Result;
use crate::health::Health;

/// Outcome of applying a policy stage to an event.
///
/// The Event variant is intentionally not boxed; the policy chain
/// is by-value mutation in the common path and the per-stage
/// allocation cost would dominate.  M2 may revisit this if the
/// chain is restructured.
#[derive(Debug)]
#[non_exhaustive]
#[allow(
    clippy::large_enum_variant,
    reason = "Event-by-value is intentional in the policy hot path."
)]
pub enum PolicyOutcome {
    /// Continue to the next stage with the (possibly mutated) event.
    Pass(Event),
    /// Drop the event entirely; remaining stages are skipped and
    /// no sink dispatch occurs.  Reason is recorded in the audit log.
    Drop {
        /// Operator-facing reason for the drop.
        reason: String,
    },
    /// Route the event onto an alternate channel.  Use sparingly;
    /// most fan-out is configured at pipeline level, not per stage.
    Route {
        /// Mutated event.
        event: Box<Event>,
        /// Logical channel identifier.
        channel: String,
    },
}

impl PolicyOutcome {
    /// Convenience constructor for a simple pass-through.
    #[must_use]
    pub fn pass(event: Event) -> Self {
        Self::Pass(event)
    }

    /// Convenience constructor for a drop with a reason.
    #[must_use]
    pub fn drop(reason: impl Into<String>) -> Self {
        Self::Drop { reason: reason.into() }
    }
}

/// Context passed to a policy stage.
#[derive(Debug)]
#[non_exhaustive]
pub struct PolicyContext {
    /// Configured stage name (matches the policy chain config).
    pub stage_name: String,
    /// Active policy profile.
    pub profile: String,
}

impl PolicyContext {
    /// Construct a new context.
    #[must_use]
    pub fn new(stage_name: impl Into<String>, profile: impl Into<String>) -> Self {
        Self { stage_name: stage_name.into(), profile: profile.into() }
    }

    /// Record a decision on the event's `mara.policy.decisions` list.
    pub fn record_decision(
        &self,
        event: &mut Event,
        decision: PolicyDecisionKind,
        reason: Option<String>,
    ) {
        event.mara.policy_decisions.push(PolicyDecisionRecord::new(
            self.stage_name.clone(),
            decision,
            reason,
        ));
    }
}

/// A single stage in a Mara policy chain.
///
/// Implementations must be `Send + Sync` and concrete invocations
/// must be cheap; heavy work is encouraged to be deferred or
/// budgeted by the host.
#[async_trait]
pub trait Policy: Send + Sync {
    /// Stable identifier of this policy implementation (e.g., `"redact-regex"`).
    fn name(&self) -> &str;

    /// Apply the policy to a single event.
    async fn apply(&self, ctx: &PolicyContext, event: Event) -> Result<PolicyOutcome>;

    /// Report current health.  Default returns a healthy report.
    fn health(&self) -> Health {
        Health::healthy()
    }
}

#[cfg(test)]
mod tests {
    use mara_schema::EventKind;

    use super::*;

    #[test]
    fn pass_outcome_carries_event() {
        let ev = Event::now(EventKind::Prompt, "test");
        let outcome = PolicyOutcome::pass(ev);
        match outcome {
            PolicyOutcome::Pass(_) => {}
            _ => panic!("expected pass"),
        }
    }

    #[test]
    fn drop_outcome_carries_reason() {
        let outcome = PolicyOutcome::drop("test reason");
        match outcome {
            PolicyOutcome::Drop { reason } => assert_eq!(reason, "test reason"),
            _ => panic!("expected drop"),
        }
    }

    #[test]
    fn context_records_decision_onto_event() {
        let ctx = PolicyContext::new("stage-1", "profile-default");
        let mut ev = Event::now(EventKind::Prompt, "test");
        ctx.record_decision(&mut ev, PolicyDecisionKind::Redacted, Some("matched email".into()));
        assert_eq!(ev.mara.policy_decisions.len(), 1);
        assert_eq!(ev.mara.policy_decisions[0].stage, "stage-1");
        assert_eq!(ev.mara.policy_decisions[0].decision, PolicyDecisionKind::Redacted);
    }

    /// A simple test policy that always returns `Pass`.
    struct AlwaysPass;

    #[async_trait]
    impl Policy for AlwaysPass {
        fn name(&self) -> &str {
            "always-pass"
        }
        async fn apply(&self, _ctx: &PolicyContext, ev: Event) -> Result<PolicyOutcome> {
            Ok(PolicyOutcome::pass(ev))
        }
    }

    /// A simple test policy that always returns `Drop`.
    struct AlwaysDrop;

    #[async_trait]
    impl Policy for AlwaysDrop {
        fn name(&self) -> &str {
            "always-drop"
        }
        async fn apply(&self, _ctx: &PolicyContext, _ev: Event) -> Result<PolicyOutcome> {
            Ok(PolicyOutcome::drop("test drop"))
        }
    }

    #[tokio::test]
    async fn chain_runs_all_pass_stages() {
        let chain = PolicyChain::new(
            "test",
            vec![Arc::new(AlwaysPass) as Arc<dyn Policy>, Arc::new(AlwaysPass) as Arc<dyn Policy>],
        );
        let ev = Event::now(EventKind::Prompt, "test");
        let outcome = chain.run(ev).await.expect("chain ran");
        assert!(matches!(outcome, ChainOutcome::Deliver(_)));
    }

    #[tokio::test]
    async fn chain_short_circuits_on_drop() {
        let chain = PolicyChain::new(
            "test",
            vec![Arc::new(AlwaysDrop) as Arc<dyn Policy>, Arc::new(AlwaysPass) as Arc<dyn Policy>],
        );
        let ev = Event::now(EventKind::Prompt, "test");
        let outcome = chain.run(ev).await.expect("chain ran");
        match outcome {
            ChainOutcome::Drop(reason) => assert_eq!(reason, "test drop"),
            ChainOutcome::Deliver(_) => panic!("expected drop"),
        }
    }
}

/// An ordered chain of policy stages.
#[derive(Clone)]
pub struct PolicyChain {
    profile: String,
    stages: Vec<Arc<dyn Policy>>,
}

impl PolicyChain {
    /// Construct a new chain from an ordered list of policies.
    #[must_use]
    pub fn new(profile: impl Into<String>, stages: Vec<Arc<dyn Policy>>) -> Self {
        Self { profile: profile.into(), stages }
    }

    /// Profile name associated with this chain.
    #[must_use]
    pub fn profile(&self) -> &str {
        &self.profile
    }

    /// Run the chain on a single event.
    pub async fn run(&self, mut ev: Event) -> Result<ChainOutcome> {
        for stage in &self.stages {
            let ctx = PolicyContext::new(stage.name().to_owned(), self.profile.clone());
            match stage.apply(&ctx, ev).await? {
                PolicyOutcome::Pass(next) => {
                    ev = next;
                }
                PolicyOutcome::Drop { reason } => {
                    return Ok(ChainOutcome::Drop(reason));
                }
                PolicyOutcome::Route { event, .. } => {
                    // Routing in M2 collapses to plain delivery; full
                    // routing surface lives at pipeline level in v2.
                    return Ok(ChainOutcome::Deliver(*event));
                }
            }
        }
        Ok(ChainOutcome::Deliver(ev))
    }
}

/// The terminal outcome of running an event through a chain.
#[derive(Debug)]
#[non_exhaustive]
#[allow(clippy::large_enum_variant, reason = "Mirrors PolicyOutcome size tradeoff.")]
pub enum ChainOutcome {
    /// The event survives the chain and should be dispatched to sinks.
    Deliver(Event),
    /// The event was dropped by a stage.
    Drop(String),
}
