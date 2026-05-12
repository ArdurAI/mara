//! `mara-gateway` — v2 aggregator placeholder.
//!
//! v1 ships this as a stub so the workspace stays cohesive.  The
//! gateway tier's real implementation arrives in v2 per the MOS
//! plan (see `plans/04-implementation/07-phased-milestones.md`).

fn main() {
    println!(
        "mara-gateway v{} — placeholder for v2 (core {})",
        env!("CARGO_PKG_VERSION"),
        mara_core::version()
    );
}
