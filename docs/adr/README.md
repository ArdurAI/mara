# Architecture Decision Records

This folder contains Mara's Architecture Decision Records (ADRs). An ADR captures a material architectural decision, its context, the considered alternatives, the chosen option, and the consequences.

## Format

Each ADR is a numbered Markdown file with the following sections:

1. **Status** — Proposed, Accepted, Deprecated, Superseded.
2. **Context** — what circumstances led to the decision.
3. **Decision** — what we chose.
4. **Alternatives considered** — what else we evaluated and why we rejected them.
5. **Consequences** — what this commits us to.

ADRs are immutable once accepted. Superseded ADRs link forward to the replacement.

## Index

- [`0001-license-apache-2-0.md`](0001-license-apache-2-0.md) — Mara is licensed Apache 2.0.
- [`0002-wasm-policy-host.md`](0002-wasm-policy-host.md) — Wasmtime is the WASM host for policy plugins.
- [`0003-wal-format.md`](0003-wal-format.md) — Segmented append-only WAL with per-sink offsets.
- [`0004-hot-reload.md`](0004-hot-reload.md) — SIGHUP + inotify debounced reload.
- [`0005-async-runtime.md`](0005-async-runtime.md) — Tokio multi-thread.
- [`0006-error-model.md`](0006-error-model.md) — Thiserror-based enums; no anyhow in public APIs.
- [`0007-config-format.md`](0007-config-format.md) — TOML primary, YAML alternate, JSON Schema validated.

## Authoring a new ADR

1. Copy [`_template.md`](_template.md) to `NNNN-title.md` with the next number.
2. Fill out the sections.
3. Open a PR; ADRs require maintainer sign-off.
4. Once merged, link from this index.

## When to write an ADR

- Public API or trait shape changes.
- Cross-crate architectural patterns.
- License or governance changes.
- Build, release, or supply-chain changes.
- Anything that the team would later want to know "why did we do that?"
