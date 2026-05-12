# ADR-0007: TOML primary, YAML alternate, JSON Schema-validated configuration

- **Status:** Accepted
- **Date:** 2026-05-12
- **Authors:** Mara M1 architecture review.

## Context

Operators configure Mara through a file. The file format influences ergonomics, tooling, and error messages. Common choices in the telemetry agent ecosystem are TOML (Fluent Bit's classic format, Cargo's format, idiomatic Rust), YAML (Fluent Bit's recent dual-format support, OTel Collector's, Kubernetes manifests), HCL (HashiCorp / Terraform), and JSON (machine but not human friendly).

## Decision

Mara uses **TOML as the primary configuration format**. **YAML is supported as an alternate** with identical semantics. **JSON Schema** validates both at load time.

- The canonical configuration file is `mara.toml`. The same configuration can equivalently be expressed in `mara.yaml`; the loader detects extension. A directory containing both `mara.toml` and `mara.yaml` is rejected as ambiguous.
- The JSON Schema is generated from the typed Rust configuration types via `schemars` and shipped at `crates/mara-core/schema/mara-config.schema.json` for IDE autocompletion.
- Validation errors include file path, line, and column when the parser supplies them (TOML parser typically does; YAML parser via `serde_yaml` / `yaml-rust2` does as well).
- Environment variable interpolation: `${VAR}` and `${VAR:-default}` are expanded prior to parsing.
- Secret references: `@file:/path/to/secret` and `@vault:path#field` are recognized at runtime by configuration consumers (not at file load time).

## Alternatives considered

- **YAML primary.** Pros: matches Kubernetes / OTel Collector. Cons: classic YAML pitfalls (Norway problem, implicit type coercion, whitespace sensitivity); confusing diagnostics. Reject as primary; support as alternate to ease operator migration.
- **HCL.** Pros: matches Terraform / HashiCorp ecosystem; expressive. Cons: outside the Rust crate ecosystem; another parser dependency; community fragmentation. Rejected.
- **JSON.** Pros: machine native; minimal parsing. Cons: poor human ergonomics (no comments by default); rejected as primary; can still be ingested when the file extension is `.json` if there's demand (post-v1).
- **JSON5 / JSON-with-Comments.** Pros: a bit nicer than JSON. Cons: niche; not worth the extra format. Rejected.
- **Custom config DSL.** Pros: maximum control. Cons: see "rejected" everywhere.

## Consequences

- The workspace pulls `toml`, `serde_yaml` (or `yaml-rust2`), and `schemars` as dependencies.
- The configuration types live in `mara-core::config` and derive `Serialize`, `Deserialize`, `JsonSchema`.
- A generated JSON Schema is shipped per release and published to GitHub Releases alongside the binary; IDE editors with YAML / TOML JSON Schema awareness can consume it.
- `mara validate` is a fast operation (NFR-6.3 ≤100 ms) that loads and validates without starting pipelines.
- Examples in `examples/` cover both TOML and YAML formats and exercise every documented configuration option.
- Schema versioning: the top-level `schema_version` field is required. v1 is `"1"`; future major bumps require a new ADR.

## References

- TOML spec: <https://toml.io>.
- YAML 1.2 spec: <https://yaml.org/spec/1.2/>.
- schemars crate: <https://github.com/GREsau/schemars>.
- [`plans/04-implementation/01-functional-requirements.md`](../../plans/04-implementation/01-functional-requirements.md) FR-1.
- [`plans/04-implementation/02-non-functional-requirements.md`](../../plans/04-implementation/02-non-functional-requirements.md) NFR-6.
