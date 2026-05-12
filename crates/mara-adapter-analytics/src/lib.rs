//! Analytics REST adapter.
//!
//! Polls a configured vendor analytics REST API on a schedule with
//! exponential backoff, dedupes via a configurable key, and persists
//! a last-seen cursor for durable resume.  Tier C in the
//! integration-tier model.  Used primarily for Augment Code.
//!
//! M0 status: stub.  Implementation lands in M3.

#![doc(html_root_url = "https://docs.rs/mara-adapter-analytics/0.1.0")]
