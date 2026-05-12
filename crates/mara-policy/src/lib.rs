//! Mara policy stage.
//!
//! Hosts WASM-sandboxed policy plugins and ships built-in primitives
//! (redact, allow, deny, sample, rate-limit, transform, classify,
//! route).  Policy bundles are TARs verifiable via `cosign`.
//!
//! M2 ships the built-in redact and sample primitives.  WASM
//! plugin hosting and signed bundles land in M2 follow-up work.

#![doc(html_root_url = "https://docs.rs/mara-policy/0.1.0")]

pub mod builtin;

pub use mara_core::policy::{Policy, PolicyChain, PolicyContext, PolicyOutcome};
