# MVP — Long-Term Roadmap (MVP through v3)

## Executive summary

The MVP is the first six weeks. What comes after, in what order, on what cadence, and tied to what outside signals — is what this document defines. The MVP is not the destination; it is the first proof point. Beyond it lies a sequence of progressively wider scopes (more runtimes, more sinks, a gateway tier, a hosted control plane) and progressively deeper guarantees (WAL durability, signed policy bundles, SOC 2 Type II, CNCF graduation). Most of these are already mentioned in passing across the encyclopedia; this document pulls them into one timeline so a reader, contributor, or investor sees the full arc.

The dates are intentions, not commitments. They flex with the adoption signals defined in [`14-launch-and-early-adopter-experience.md`](14-launch-and-early-adopter-experience.md) and the abort criteria in [`15-mvp-abort-criteria.md`](15-mvp-abort-criteria.md).

## Cadence and versioning

- **Alpha (`0.2.x-alpha`):** MVP era. Breaking changes can land in any release. Six-week iteration loops.
- **Beta (`0.3.x` through `0.9.x`):** widening scope from MVP. Each release adds one or two runtimes or sinks. Quarterly minor releases. Monthly patch releases.
- **`1.0.0`:** the canonical v1 release. All six original target runtimes + Ollama active. Full sink set. WAL durability. Signed policy bundles. SOC 2 Type I attestation. CNCF Sandbox accepted. Apache 2.0 finalized as the trademark + license bundle.
- **`1.x`:** stability era. Quarterly minor releases. No breaking changes except via deprecation-then-removal cycle.
- **`2.0`:** the gateway tier (`mara-gateway` binary) GA. Multi-tenant policy distribution. Gateway-pushed config. Existing v1 deployments upgrade with no breaking config changes.
- **`3.0`:** ArdurAI-hosted control plane GA (commercial product layered over the Apache 2.0 OSS). Fleet management, SSO/SCIM, signed policy marketplace.

## v0.2.0-alpha — MVP (week 6, June 2026)

What ships, per [`01-scope-and-decision-criteria.md`](01-scope-and-decision-criteria.md):

- Claude Code OTLP receive (Tier A).
- Ollama HTTP proxy (Proxy tier).
- OTLP HTTP sender + Loki HTTP push sink.
- Built-in PII redaction (9 patterns + 4 AI-vendor key patterns).
- Token-based cost computation for one cloud-vendor model family + zero-cost local-inference flagging.
- Self-telemetry endpoint + `mara diag`.
- macOS + Linux packaging (Homebrew, deb, rpm, OCI image, Helm chart).
- Quickstarts for both runtimes.

## v0.3.0-alpha — MVP+1 (target: 6 weeks after MVP, July-August 2026)

- gRPC OTLP receiver on `:4317` (alongside HTTP).
- Codex runtime preset activated (Tier A, same shape as Claude Code).
- Gemini CLI runtime preset activated (Tier A).
- Splunk HEC sink.
- Generic webhook sink.
- Bedrock + GCP service-account JSON redaction patterns finalized.
- Hooks adapter (`mara-adapter-hooks`) implemented; Cursor preset activated.
- Windows packaging.
- `mara test pipeline` and `mara dlq` subcommands working.
- Streaming-aware Ollama capture with per-token-rate histograms.

Acceptance: 3 of 7 runtimes at Tier A, 1 at proxy, 1 at Tier B, 2 still scaffolded. Six of ten sinks shipping.

## v0.4.0-alpha — MVP+2 (target: 12 weeks after MVP, September 2026)

- Kimi runtime preset activated (Tier B JSONL tail).
- Elasticsearch / OpenSearch bulk sink.
- Object store sink (S3 / GCS / Azure Blob with JSONL and Parquet formats).
- Kafka sink.
- PHI redaction pack (`builtin.phi`).
- PCI redaction pack (`builtin.pci`).
- Locale-specific PII packs (EU, UK, JP, IN).
- Augment Code analytics REST adapter implemented; preset activated (Tier C best-effort).
- Generic OpenAI-compat upstream support via the `llm-proxy` adapter (OpenAI direct, Anthropic direct, Together, Fireworks, etc.).

Acceptance: 7 of 7 target runtimes active at tier-appropriate fidelity. 10 of 10 sinks shipping.

## v0.5.0-rc.1 → v1.0.0 (target: 24 weeks after MVP, late 2026)

Hardening release window. No new runtimes; no new sinks. Focus on:

- **Segmented append-only WAL** per [ADR-0003](../../docs/adr/0003-wal-format.md). Per-sink offsets. ≤ 1 s data loss on SIGKILL. WAL replay perf gate in CI.
- **WASM-hosted policy bundles** per [ADR-0002](../../docs/adr/0002-wasm-policy-host.md). `cosign`-verified at load.
- **Tamper-evident audit log** with periodic Merkle root export.
- **`gen_ai.usage.cost.usd`** upstream contribution to OTel semconv, plus `mara.cost.usd` graduation if accepted.
- **Performance bench harness** wired into CI with the 1-hour 50k EPS gate.
- **SOC 2 Type I audit** with a vendor (Vanta / Drata / TrustCloud / Lacework).
- **CNCF Sandbox application** submitted.
- **`docs.ardurai.dev`** documentation site live (mdBook).
- **Trademark policy** published.

Versioning at this point: each release in `0.5.x → 0.9.x` is a release candidate iteration. `1.0.0` ships when every M5 sign-off criterion from the original MOS plan is honestly green and at least 10 external production users are on `ADOPTERS.md`.

## v1.x — Stability and Plugin ABI (2027)

- **`mara_plugin_v1` C ABI** stabilized. Third-party adapter and sink crates can ship independently as dynamic libraries.
- **Policy marketplace** (signed bundles distributed via OCI registry).
- **`mara compose`** subcommand for managing per-pipeline configurations declaratively.
- **`mara-runtime-*` contributions from external maintainers.** Target: at least 2 external runtime presets (e.g., for a tool ArdurAI hasn't focused on).
- **More sinks** as community demand surfaces: ClickHouse, Honeycomb-native (vs OTLP), Sentry, Logfire-native, custom enterprise SIEMs.
- **SLSA Level 3** build provenance.

Quarterly minor releases, monthly patches, no breaking changes except via the deprecation cycle in [NFR-8](../04-implementation/02-non-functional-requirements.md#nfr-8--backward-compatibility).

## v2.0 — Gateway Tier (mid-2027)

The `mara-gateway` binary, sketched in M0/M1 as a stub, ships GA. Reuses the `mara-core` crate. Adds:

- Aggregation of telemetry from many edge agents.
- Gateway-pushed configuration: edge agents can pull config + signed policy bundles from a gateway endpoint rather than reading from local disk.
- Multi-tenant policy selection: per-tenant policy bundles, per-tenant sinks.
- Per-tenant rate limits and quotas.
- High-availability gateway clustering (Raft or gossip-based leader election, TBD in v2 design RFC).

Customers running `mara` at edge in v1.x continue running v1.x; v2 is opt-in. The v2 gateway is itself Apache 2.0; the v3 control plane (proprietary) is what is layered on top.

## v3.0 — Hosted Control Plane (2028)

ArdurAI commercial product: the hosted control plane. Layered over v2's OSS gateway. Adds:

- **Web UI** for fleet visibility, policy authoring, audit-log review.
- **SSO / SCIM** for org-wide identity.
- **Hosted policy bundle distribution** with signed updates.
- **Cross-tenant analytics** for ArdurAI fleet operators (e.g., aggregate AI spend trends by industry).
- **Managed gateway operation** for customers who don't want to host the gateway themselves.

The Apache 2.0 OSS continues unchanged. The control plane is the commercial product; it does not gate any OSS feature.

This is when ArdurAI's commercial model becomes durable. The OSS is the strategic gift; the hosted product is the business.

## Cross-cutting themes (apply across all versions)

- **OpenTelemetry alignment.** Mara contributes to OTel semconv when `mara.*` extensions prove their worth. We track upstream changes in `crates/mara-schema/semconv.lock` and re-vendor quarterly.
- **CNCF track.** Sandbox at v1.0; Incubating within 18 months of acceptance; Graduated when the metrics warrant (project longevity, adoption, governance).
- **Security posture.** SOC 2 Type I at v1.0, Type II by end of 1.x. ISO 27001 Annex A control mapping by v2.0.
- **Open governance.** v1.0 ships with at least one non-ArdurAI maintainer. v2.0 ships with a formal RFC process and a steering committee.

## What we are NOT planning to do

- **Build a UI in the OSS core.** No web UI in `mara-core`, `mara-gateway`, or any first-party Mara crate. UI is hosted product (v3) or third-party (Grafana, Honeycomb, Datadog).
- **Build inference proxies.** Mara observes, it does not gateway inference. LiteLLM / Portkey / OpenRouter cover that.
- **Support every AI runtime.** Six target runtimes + Ollama is enough for v1.0. Community contributions can extend to more.
- **Compete with the OTel Collector.** When OTel Collector grows native AI semantics that approach Mara's curated knowledge, we graduate into a Collector distribution rather than fight.
- **Ship a CLI for non-edge use cases.** `mara` is the edge agent CLI. `mara-gateway` is the gateway CLI. Other modes are out of scope.

## Roadmap maintenance

This document is reviewed at every major release. Items move from future-tense to past-tense; abandoned items get struck-through (not deleted) with a date and reason. New items get added when adoption signals surface them.

A truncated "what's next" version of this roadmap lives in `README.md` so the casual reader sees the arc without diving into planning.

## Cross-references

- [`01-scope-and-decision-criteria.md`](01-scope-and-decision-criteria.md) — MVP scope.
- [`06-mvp-implementation-plan.md`](06-mvp-implementation-plan.md) — MVP week-by-week.
- [`14-launch-and-early-adopter-experience.md`](14-launch-and-early-adopter-experience.md) — adoption signals that flex this roadmap.
- [`15-mvp-abort-criteria.md`](15-mvp-abort-criteria.md) — when this roadmap is invalidated.
- [`../04-implementation/07-phased-milestones.md`](../04-implementation/07-phased-milestones.md) — original MOS plan.
- [`19-community-governance.md`](19-community-governance.md) — how community plays into v1+.
