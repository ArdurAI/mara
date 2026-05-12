# EU AI Act and NIST AI RMF Alignment

## Executive summary

The EU AI Act (Regulation (EU) 2024/1689) and NIST AI Risk Management Framework (AI RMF 1.0 + Generative AI profile) are the two most consequential AI regulatory frameworks Mara's users face as of May 2026. Mara is not itself an "AI system" under the AI Act — it's a telemetry tool. But Mara's outputs are often the **evidence** operators need to demonstrate compliance for the AI systems they build or operate. This document explains where Mara fits and what we explicitly guarantee.

## What Mara is, in regulatory terms

- Mara is **infrastructure software**. It does not make AI decisions.
- Mara is **a record-keeping tool**: per AI Act Article 12 and Article 19, providers and deployers of high-risk AI systems must maintain logs that allow tracing the system's operation.
- Mara may be a **building block** in an operator's GPAI obligations (Article 53), risk-management system (Article 9), human oversight (Article 14), and post-market monitoring (Article 72).

## AI Act timeline (May 2026 posture)

- Feb 2025: Articles 1–5 in force (prohibited practices).
- Aug 2025: GPAI obligations (Article 51 onwards) in force, with the Code of Practice as the main soft-law instrument.
- Aug 2026: most other obligations enforceable (high-risk systems, transparency, conformity assessment).
- Aug 2027: full effect for high-risk AI systems embedded in regulated products.

The May 2026 posture means GPAI providers are already under obligation; high-risk operators are preparing for August.

## Mara's value proposition for AI Act compliance

### Article 12 — Record-keeping

> High-risk AI systems shall technically allow for the automatic recording of events ("logs") over the lifetime of the system.

Mara captures the logs. Specifically:

- Session lifecycle and user interactions.
- Prompts, completions, and tool calls (opt-in capture).
- Cost and resource usage.
- Errors and refusals.
- Tamper-evident audit log via M4 deliverables.

Mara's canonical schema aligned with OTel `gen_ai.*` means these logs are portable and inspectable.

### Article 13 — Transparency and provision of information

Operators must inform users about the AI system's capabilities and limitations. Mara doesn't address user-facing transparency directly, but the evidence Mara captures supports the operator's transparency documentation.

### Article 14 — Human oversight

Oversight requires the ability to monitor what an AI system is doing. Mara provides the monitoring substrate (live + historical).

### Article 15 — Accuracy, robustness, cybersecurity

Mara's PII redaction primitives and audit log support the cybersecurity dimension. The accuracy and robustness obligations are on the operator; Mara provides the evidence pipeline.

### Article 19 — Automatically generated logs

> Providers of high-risk AI systems shall keep the logs... for a period appropriate to the intended purpose, of at least six months.

Mara's WAL is short-term; the operator's chosen sink provides the long-term retention. Document this delegation explicitly in operator-facing materials.

### Article 53 / 55 — GPAI obligations

Providers of general-purpose AI models have obligations around training data documentation, copyright compliance, and (for systemic-risk models) extensive evaluation logging. Mara doesn't provide training-data instrumentation, but it covers the **inference-side** evidence stream: every call to a GPAI model can be captured, tagged, and routed.

### Article 72 — Post-market monitoring

> Providers shall establish... a post-market monitoring system that actively and systematically collects, documents and analyses relevant data.

This is essentially Mara's job in production.

## What Mara explicitly does NOT do for AI Act compliance

- Conformity assessment.
- Risk classification of the operator's system.
- CE marking.
- Notified-body interaction.
- Fundamental rights impact assessment (Article 27).
- Training-data documentation.
- Copyright compliance verification.

These remain operator obligations.

## NIST AI RMF mapping

The NIST AI RMF 1.0 has four functions: GOVERN, MAP, MEASURE, MANAGE. NIST's GenAI Profile (NIST AI 600-1) extends with 12 risks specific to generative AI.

- **GOVERN.** Mara contributes to GV.1 (policies & procedures) via signed policy bundles and audit log; GV.4 (risk management) via the captured telemetry feeding the operator's risk function.
- **MAP.** Mara contributes to MAP.5 (impacts on individuals) by capturing the inputs and outputs to inform impact analyses.
- **MEASURE.** Mara is most relevant here. MS.2 (system performance and trustworthiness) is the heart of what Mara enables — latency, cost, accuracy, error-rate trends. MS.4 (effectiveness of measurement) is also addressed via the eval signal class.
- **MANAGE.** MN.1 (risks prioritized and acted upon) leverages Mara's data; MN.4 (incident response) leverages the audit log and alerting fed by Mara.

The GenAI Profile risks (e.g., CBRN content, confabulation, dangerous or violent recommendations, data privacy, environmental impact, harmful bias, human-AI configuration, information integrity, information security, intellectual property, obscene/abusive content, value chain & component integration) all benefit from a telemetry layer that captures evidence per inference.

## ISO/IEC 42001 (AI management system)

ISO/IEC 42001:2023 specifies an AI Management System (AIMS). Mara contributes to:

- Clause 6.1.4 (AI system impact assessment) — input data.
- Clause 8.3 (AI system life-cycle) — operations telemetry.
- Annex A.6.2.6 (AI system performance) — direct evidence.
- Annex A.8.3 (system monitoring and logging) — Mara's direct purpose.

## GDPR + EU AI Act interaction

When prompts or completions contain personal data, GDPR applies independently of the AI Act. Mara's relevant primitives:

- **Data minimization (Article 5(1)(c))**: prompt body capture is opt-in.
- **Storage limitation (Article 5(1)(e))**: WAL retention is bounded; long-term storage is the operator's sink (operator's policy applies).
- **Integrity and confidentiality (Article 5(1)(f))**: TLS in transit, PII redaction at the agent, signed policy bundles.
- **DPIA (Article 35)**: Mara's captured data supports DPIA evidence requirements.

## US state laws

- **Colorado AI Act (effective 2026)** — focus on consumer-facing high-risk AI decisions; Mara's evidence stream supports the documentation obligations.
- **California SB 942 (in effect)** — AI disclosure; operational telemetry supports verification.
- **Other state laws** — pattern is similar; Mara's evidence is operator-agnostic.

## Operator-facing alignment statement

When an operator asks "is Mara EU AI Act compliant?":

- The Mara software itself is infrastructure software, not an AI system, and is not subject to AI Act conformity assessment.
- Mara provides technical capabilities that **support** operators' compliance with Articles 12, 14, 19, and 72 (logging, monitoring, audit trail).
- The operator remains responsible for their own conformity assessment, risk classification, transparency, oversight, and reporting.

This statement is the canonical wording, to be replicated in product collateral.

## Documentation we maintain

- **AI Act capability map** (this section) — what Mara does and doesn't do for AI Act compliance.
- **NIST AI RMF map** (this section).
- **ISO/IEC 42001 control map** — published as `plans/05-evaluation/05-iso-42001-mapping.md` post-v1.
- **ZDR-by-runtime matrix** — already in [`../01-landscape/08-ai-runtime-telemetry-surfaces.md`](../01-landscape/08-ai-runtime-telemetry-surfaces.md).
- **GDPR data-flow doc** — published per request to compliance teams.

## Operator checklist

For an EU operator deploying Mara as part of an AI Act compliance posture:

1. Enable the audit log feature (M4).
2. Pick a sink with ≥ 6-month retention to satisfy Article 19.
3. Enable PII redaction packs.
4. Sign and version-control your policy bundles.
5. Document Mara's role in your AI system's technical documentation (Annex IV) under the logging and monitoring sections.
6. Include Mara in your DPIA's technical safeguards section.
7. Note Mara's ZDR-respecting defaults in your transparency disclosures.

## Update cadence

This document is reviewed:

- On every EU Commission Code of Practice update.
- On every NIST AI RMF profile update.
- On every ISO/IEC 42001 amendment.
- Annually otherwise.
