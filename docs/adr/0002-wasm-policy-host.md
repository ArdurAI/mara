# ADR-0002: Use Wasmtime as the WASM host for policy plugins

- **Status:** Accepted
- **Date:** 2026-05-12
- **Authors:** Mara M1 architecture review.

## Context

Mara's policy stage executes redaction, sampling, and routing decisions on every canonical event. Built-in primitives cover the common cases; for everything else operators need to load custom policies. Custom policies must be sandboxed (a buggy policy must not crash or escape the agent), polyglot (operators write in whatever language they prefer), and signable so compliance can demonstrate provenance.

WebAssembly is the obvious fit: sandbox-by-construction, polyglot, mature tooling for signing and distribution.

## Decision

Mara embeds [Wasmtime](https://wasmtime.dev) (Bytecode Alliance) as the WebAssembly host for policy plugins. The host:

- Runs each plugin in an isolated `wasmtime::Store` with no host filesystem or network access.
- Enforces a wall-clock timeout per `apply` invocation (default 50 ms, configurable).
- Limits memory per store (default 64 MiB, configurable).
- Disables JIT when `MemoryDenyWriteExecute=true` is set in the systemd unit by falling back to the Pulley interpreter, or by configuring Cranelift in ahead-of-time mode where supported.
- Exposes a stable `mara_plugin_v1` ABI (described in `mara-policy::abi`).
- Provides `mara-policy-sdk` crates / packages for Rust, TinyGo, and AssemblyScript to wrap the ABI.

Bundles are TARs containing `policy.wasm`, `policy.yaml` manifest, and a `cosign` signature. The host verifies the signature before instantiation.

## Alternatives considered

- **Wasmer.** Pros: comparable performance; commercial backing. Cons: vendor-led (Wasmer Inc.); governance neutrality matters for CNCF alignment. Rejected.
- **Wasmi.** Pros: pure-Rust interpreter; smallest dependency surface; trivially safe in restricted environments. Cons: meaningfully slower than Wasmtime; not Mara's primary host. Decision: ship as a fallback engine selectable by config when `MemoryDenyWriteExecute=true` blocks JIT.
- **Rhai (embedded Rust scripting).** Pros: zero external dependency; great Rust integration. Cons: not polyglot; no first-class signing story; not a standards-track sandbox. Rejected.
- **OPA / Rego natively (without WASM).** Pros: OPA is a known policy language. Cons: Rego is more limited than WASM-from-arbitrary-language; running OPA standalone is heavyweight. Decision: ship OPA / Rego as a built-in WASM module (`opa.wasm`) rather than as a separate engine.
- **CEL (Common Expression Language).** Pros: simple, easy to reason about. Cons: not polyglot; not suitable for complex transforms. Decision: not the default but offer `cel-rust` evaluation as a configuration option for simple decision policies.

## Consequences

- Mara workspace gains `wasmtime` as a direct dependency. Build time and binary size grow modestly (~5 MiB).
- The plugin ABI (`mara_plugin_v1`) is a stable contract that the team commits to maintaining via the same deprecation cycle as public traits (per NFR-8 in the non-functional requirements).
- A `mara-policy-sdk-rust` crate is published; equivalent SDKs for TinyGo and AssemblyScript are roadmapped for v1.x.
- Compliance and audit teams have a clean story: bundles are signed, instantiation is verified, decisions are recorded in the audit log.
- The `wasmtime` upstream's posture on security advisories and supply-chain attestation must be monitored; subscription to RustSec advisories is mandatory.

## References

- Wasmtime: <https://wasmtime.dev>.
- Wasmi: <https://github.com/wasmi-labs/wasmi>.
- WASI subgroups for component model and policy: <https://github.com/WebAssembly/component-model>.
- [`plans/04-implementation/03-architecture-blocks.md`](../../plans/04-implementation/03-architecture-blocks.md) Section "Crate responsibilities — mara-policy".
- [`plans/02-gaps/04-policy-and-redaction-gaps.md`](../../plans/02-gaps/04-policy-and-redaction-gaps.md).
