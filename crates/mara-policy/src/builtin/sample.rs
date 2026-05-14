//! Head sampler primitive.
//!
//! Probabilistic head sampling: the first decision per event
//! determines whether it is kept.  Uses a simple `Pcg64Mcg` PRNG
//! seeded from a fixed seed so behaviour is reproducible in tests;
//! production deployments seed from the system entropy via the
//! `seed` configuration option.

use std::sync::atomic::{AtomicU64, Ordering};

use async_trait::async_trait;
use mara_core::error::Result;
use mara_core::policy::{Policy, PolicyContext, PolicyOutcome};
use mara_schema::{Event, PolicyDecisionKind};

/// Probabilistic head sampler with rate in `[0.0, 1.0]`.
#[derive(Debug)]
pub struct HeadSampler {
    /// Keep probability.
    rate: f64,
    /// xorshift state for cheap per-event decisions.
    state: AtomicU64,
}

impl HeadSampler {
    /// Construct a sampler with a fixed seed.  Rate is clamped to
    /// `[0.0, 1.0]`.
    #[must_use]
    pub fn with_seed(rate: f64, seed: u64) -> Self {
        let rate = rate.clamp(0.0, 1.0);
        Self { rate, state: AtomicU64::new(if seed == 0 { 0xDEAD_BEEF_CAFE_BABE } else { seed }) }
    }

    /// Construct a sampler with a default seed.
    #[must_use]
    pub fn new(rate: f64) -> Self {
        Self::with_seed(rate, 0x4D41_5241_5345_4544)
    }

    fn keep(&self) -> bool {
        if self.rate >= 1.0 {
            return true;
        }
        if self.rate <= 0.0 {
            return false;
        }
        // xorshift64*: simple, cheap, good enough for sampling.
        let mut x = self.state.load(Ordering::Relaxed);
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.state.store(x, Ordering::Relaxed);
        let frac = (x as f64) / (u64::MAX as f64);
        frac < self.rate
    }
}

#[async_trait]
impl Policy for HeadSampler {
    fn name(&self) -> &str {
        "builtin.sample.head"
    }

    async fn apply(&self, ctx: &PolicyContext, mut event: Event) -> Result<PolicyOutcome> {
        if self.keep() {
            ctx.record_decision(&mut event, PolicyDecisionKind::Sampled, None);
            Ok(PolicyOutcome::pass(event))
        } else {
            Ok(PolicyOutcome::drop(event, "head-sampled out"))
        }
    }
}

#[cfg(test)]
mod tests {
    use mara_schema::EventKind;

    use super::*;

    #[tokio::test]
    async fn rate_one_always_passes() {
        let s = HeadSampler::new(1.0);
        let ev = Event::now(EventKind::Prompt, "t");
        let outcome = s.apply(&PolicyContext::new("s", "default"), ev).await.unwrap();
        assert!(matches!(outcome, PolicyOutcome::Pass(_)));
    }

    #[tokio::test]
    async fn rate_zero_always_drops() {
        let s = HeadSampler::new(0.0);
        let ev = Event::now(EventKind::Prompt, "t");
        let outcome = s.apply(&PolicyContext::new("s", "default"), ev).await.unwrap();
        assert!(matches!(outcome, PolicyOutcome::Drop { .. }));
    }

    #[tokio::test]
    async fn rate_half_distributes_roughly_evenly() {
        let s = HeadSampler::with_seed(0.5, 42);
        let mut pass = 0;
        let mut drop = 0;
        for _ in 0..10_000 {
            let ev = Event::now(EventKind::Prompt, "t");
            let outcome = s.apply(&PolicyContext::new("s", "default"), ev).await.unwrap();
            match outcome {
                PolicyOutcome::Pass(_) => pass += 1,
                PolicyOutcome::Drop { .. } => drop += 1,
                _ => unreachable!("non_exhaustive variants not produced"),
            }
        }
        let pct_pass = (pass as f64) / (pass + drop) as f64;
        assert!((pct_pass - 0.5).abs() < 0.05, "expected ~50% but got {pct_pass:.3}");
    }
}
