//! Built-in policy primitives shipped in the agent binary.
//!
//! These are deliberately small, fast, and dependency-light.
//! Larger or domain-specific policies should live in WASM bundles.

pub mod redact;
pub mod sample;

pub use redact::{Pack, RegexRedactor};
pub use sample::HeadSampler;
