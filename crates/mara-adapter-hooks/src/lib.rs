//! Hooks adapter.
//!
//! Accepts JSON over stdio from subprocesses invoked as runtime
//! hooks (e.g., Cursor's hooks, Codex's `notify`).  Optionally
//! exposes an HTTP endpoint for hooks that post over the network.
//! Tier B in the integration-tier model.
//!
//! M0 status: stub.  Implementation lands in M3.

#![doc(html_root_url = "https://docs.rs/mara-adapter-hooks/0.1.0")]
