# SOC 2 Control Mapping

## Executive summary

This document maps Mara's design and operational practices to SOC 2 Trust Services Criteria (TSC), with concrete evidence pointers for each control. SOC 2 Type I (control design) is targeted at M5 of v1; Type II (operating effectiveness over a 6–12 month observation window) is post-v1.

Scope of this mapping: the Mara agent (and gateway in v2) as a product; the ArdurAI org running the project; the hosted control plane (v3) gets its own additional mapping later.

This document is a **draft** until reviewed by a qualified auditor. It is published as an engineering reference, not a compliance attestation.

## TSC categories covered

- **Common Criteria (CC)** — required for every SOC 2 audit, comprising CC1–CC9.
- **Availability** — relevant when the hosted control plane (v3) ships; not in v1 scope.
- **Processing Integrity** — relevant for Mara's data-handling claims (no data loss SLOs).
- **Confidentiality** — relevant for PII redaction and audit log.
- **Privacy** — relevant for GDPR/CCPA alignment.

For Type I in v1, we map CC1–CC9 + Processing Integrity + Confidentiality.

## CC1 — Control Environment

- **CC1.1 Integrity and ethical values.** Code of conduct ([`../../CODE_OF_CONDUCT.md`](../../CODE_OF_CONDUCT.md)); contributor DCO requirement.
- **CC1.2 Board independence.** ArdurAI corporate governance (separate document outside repo).
- **CC1.3 Organizational structure.** Maintainer roster in `MAINTAINERS.md` (post-v1).
- **CC1.4 Commitment to competence.** Job descriptions for security and engineering roles; documented onboarding.
- **CC1.5 Accountability.** Per-file `CODEOWNERS` (post-v1); per-area maintainer assignments.

## CC2 — Communication and Information

- **CC2.1 Quality of information.** Documentation set (this `plans/` folder + `docs/adr/`); CHANGELOG per release; release notes.
- **CC2.2 Internal communications.** Issue tracker, RFCs in `docs/rfc/`, regular maintainer syncs.
- **CC2.3 External communications.** Public roadmap, public release notes, public security advisories, security@ardurai.dev for embargoed reports.

## CC3 — Risk Assessment

- **CC3.1 Specifies objectives.** Success metrics ([`../04-implementation/08-success-metrics.md`](../04-implementation/08-success-metrics.md)); MOS milestones.
- **CC3.2 Identifies risks.** Risk register (in the MOS plan; expanded in M1 ADRs).
- **CC3.3 Considers fraud potential.** Supply-chain threat model (M4 STRIDE document).
- **CC3.4 Identifies changes.** Architecture Decision Records (`docs/adr/`).

## CC4 — Monitoring Activities

- **CC4.1 Ongoing evaluations.** CI gates (perf, security, license); nightly soak; quarterly retros.
- **CC4.2 Communicates deficiencies.** Public issue triage; security advisories per `SECURITY.md`.

## CC5 — Control Activities

- **CC5.1 Selects and develops control activities.** Coding standards (`rustfmt.toml`, `clippy.toml`); CI policy.
- **CC5.2 Technology general controls.** Branch protection, required reviews, signed commits, CODEOWNERS.
- **CC5.3 Policy and procedure deployment.** Documented release process; documented incident response.

## CC6 — Logical and Physical Access

### Logical access to the codebase

- **CC6.1 Logical access controls.** GitHub org permissions, SSO required for maintainers, 2FA mandatory.
- **CC6.2 Authentication.** GitHub SSO; signed commits encouraged (gitsign or GPG).
- **CC6.3 Authorization.** Branch protection on `main`; required reviews; merge by maintainers only.

### Logical access to runtime artifacts

- **CC6.4 Restrict access to data.** The agent runs as an unprivileged user; file-tail capability scoped per adapter; secrets via env or `@file:` references with file-mode checks.
- **CC6.5 Logical access changes.** Configuration changes are version-controlled when policy bundles are used; bundle signing required.
- **CC6.6 Logical and physical access.** Hosted control plane (v3) only — not in v1 scope.
- **CC6.7 Physical access.** N/A for the agent product; relevant for v3 hosted control plane.
- **CC6.8 Mobile devices and removable media.** N/A.

## CC7 — System Operations

- **CC7.1 Detects and prevents anomalies.** Self-telemetry exposes pipeline metrics; anomaly detection is the operator's choice of backend.
- **CC7.2 Monitors system components.** `mara_self_*` metrics on `:9099/metrics`; `/healthz`.
- **CC7.3 Evaluates security events.** Audit log (M4) — tamper-evident.
- **CC7.4 Responds to security events.** Incident response procedure documented in `SECURITY.md`; coordinated disclosure window.
- **CC7.5 Recovers from security events.** Backup/restore patterns for WAL; rollback via release tagging.

## CC8 — Change Management

- **CC8.1 Manages changes.** All changes via PR; CI gates; required reviews; ADRs for material changes; SemVer with deprecation policy (NFR-8).

## CC9 — Risk Mitigation

- **CC9.1 Risk identification.** Risk register maintained per release.
- **CC9.2 Vendor risk.** Dependency policy via `cargo deny`; transitive license audit; SBOM published per release.

## Additional Criteria: Processing Integrity

- **PI1.1 Definition of processing integrity.** Canonical schema documented in [`../04-implementation/04-data-model.md`](../04-implementation/04-data-model.md).
- **PI1.2 Processing complete and accurate.** Round-trip tests; golden-file tests; coverage report.
- **PI1.3 Processing authorized.** Policy chain runs before every sink dispatch.
- **PI1.4 Processing timely.** Latency SLOs (NFR-1.2).
- **PI1.5 Stores complete and accurate.** WAL integrity checks; CRC32 per record.

## Additional Criteria: Confidentiality

- **C1.1 Identifies confidential data.** PII / PHI / PCI categorization in policy packs.
- **C1.2 Disposes of confidential data.** WAL bounded retention; explicit drop on policy deny; audit log records deny events.

## Evidence pipeline

Each SOC 2 audit requires evidence of operating effectiveness. v1 ships these evidence streams:

- **CI logs**: every PR run is retained 90 days; release-pipeline runs retained 2 years.
- **Signed releases**: every release tag has SBOM, provenance, cosign signature, retained in GitHub Releases.
- **Audit log exports**: when the audit log is enabled, periodic Merkle root exports go to an operator-chosen sink; ArdurAI hosted-instance Merkle roots will go to the org's auditor-shared bucket (v3).
- **Access review**: GitHub org access reviewed quarterly; documented in `docs/access-review-<YYYY-QN>.md`.
- **Incident logs**: postmortems published for any operator-visible incident.

## Gap analysis vs SOC 2 Type II readiness

- **Type I** (design only) achievable at M5 with the controls above.
- **Type II** (operating effectiveness over time) requires:
  - 6–12 months of evidence accumulation.
  - A formal information security policy document (currently scattered across `SECURITY.md`, `CODE_OF_CONDUCT.md`, this mapping).
  - A documented incident response runbook with on-call rotation.
  - A documented vendor management process for third-party services (GitHub, OCI registry, CI providers).
  - Annual penetration test (planned for v1.x).
  - Annual security training attestations for maintainers.

These items are scoped for the v1.x → v2.0 window.

## Auditor handoff package

When Mara engages an auditor, ArdurAI provides:

- This document.
- The MOS plan and ADRs.
- The threat model.
- Recent CI logs.
- Recent release artifacts with signatures and SBOMs.
- Access review records.
- Incident logs (or attestation of none).

The auditor maps observed evidence to TSC and produces the report.

## Relationship to other compliance frameworks

- **ISO 27001:2022** — overlaps ≈ 70% with SOC 2 CC. A separate Annex A control mapping is drafted post-v1.
- **HIPAA / PCI-DSS** — orthogonal; relevant when an operator deploys Mara in a regulated workflow. Mara provides the technical primitives (redaction, audit log); the operator's deployment provides the rest.
- **EU AI Act / NIST AI RMF** — covered in [`04-eu-ai-act-alignment.md`](04-eu-ai-act-alignment.md).
- **FedRAMP** — out of v1 scope; would require government-cloud hosting + StateRAMP/FedRAMP-Moderate baseline.

## Maintenance

This document is reviewed at each major release and updated as controls evolve. The version that ships with each release is preserved at `plans/05-evaluation/03-soc2-control-mapping-<version>.md` for historical reference.
