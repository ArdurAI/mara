# Target Personas

## Executive summary

Mara serves four primary personas with distinct jobs-to-be-done. v1 prioritizes the first two; v2 deepens support for the third; v3 unlocks the fourth via the commercial control plane. Personas are decision drivers — UX, packaging, defaults, and messaging follow from them.

## Persona 1 — Priya, the Platform Engineer at an AI-forward SaaS

**Org context:** mid-sized B2B SaaS, 50–500 engineers, embedding LLM features into the product. Multi-cloud, primarily Kubernetes. Existing observability stack (likely Datadog or the Grafana stack with Loki+Tempo+Mimir, sometimes Elastic).

**Day-to-day pain:**
- Engineers ship LangChain/LlamaIndex/Pydantic-AI apps with one-off OTel instrumentation that varies per service.
- Cost attribution per feature/team is manual via dashboards that lie.
- Multi-LLM-vendor traces don't normalize — OpenAI's spans look different from Anthropic's, which look different from Bedrock's.
- Prompts and completions sometimes end up in production logs in violation of internal policy.

**What success looks like with Mara:**
- One agent per node (DaemonSet) collects OTLP from app code and from on-host AI runtimes engineers use locally.
- Built-in redaction profiles enforce the "no prompts in prod logs" policy at the agent.
- Canonical `gen_ai.*` schema unifies all vendor traffic.
- Ships into existing Datadog or Grafana without changing the backend.

**Decision drivers:** Helm chart quality, Datadog/Grafana integration depth, RBAC + policy bundle signing, k8s operator (later), Apache 2.0 license, SOC 2.

**Anti-patterns:** anything that requires changing the existing observability backend.

## Persona 2 — Rohan, the AI-Native Startup Engineer / Indie Developer

**Org context:** 1–20 engineers, building agent products. Heavy users of Claude Code, Codex, Cursor, Augment, Gemini CLI in their own daily workflow. Cloud is whatever-startup-credits-they-have. Observability stack is whatever they could set up in an afternoon (often Grafana Cloud free tier, sometimes Honeycomb, sometimes just stdout).

**Day-to-day pain:**
- They literally cannot see what their AI tools do across sessions. Claude Code sessions in JSONL files are technically there but uninspectable.
- Costs surprise them at end of month.
- They want to replay a session that produced a great refactor, or share one with the team.
- Their evals tooling and their dev-loop tooling don't share data.

**What success looks like with Mara:**
- `brew install mara`, `mara setup claude-code`, agent runs as a LaunchAgent, immediately sees Claude Code sessions in Grafana / Honeycomb / a local SQLite mirror.
- Cost panel in Grafana shows weekly spend per project.
- Sessions are searchable, replayable, and exportable.

**Decision drivers:** zero-config defaults, single-binary install, macOS Homebrew tap quality, friction-free quickstart per runtime, optional local-only mode (file sink), beautiful CLI.

**Anti-patterns:** anything that requires Kubernetes, anything that requires a hosted account to start, anything that takes more than 5 minutes to first dashboard.

## Persona 3 — Sasha, the Compliance / Security Engineer

**Org context:** regulated industry (finance, healthcare, public sector) or any org pursuing SOC 2 Type II / ISO 27001 / HIPAA / FedRAMP. AI adoption is happening but with high scrutiny. Security and audit teams need provable records of every AI-agent action that touched sensitive data.

**Day-to-day pain:**
- AI tools generate volumes of activity that don't fit existing audit pipelines.
- No tool produces a tamper-evident agent-action trail by default.
- Vendor "ZDR" commitments are policy promises, not technical proofs.
- Prompt content sometimes contains PHI/PCI/PII and there's no agent-level redaction.

**What success looks like with Mara:**
- Tamper-evident append-only audit log of every policy decision and material agent event, with periodic Merkle root exports to a separate sink.
- Built-in PII/PHI/PCI redaction packs with policy-bundle signing.
- SOC 2 control mapping documented, evidence pipeline scriptable.
- ZDR toggles surfaced and enforced agent-side, not just vendor-side.

**Decision drivers:** signed policy bundles, audit log integrity proofs, SBOM + provenance on every release, FIPS-mode crypto availability, EU AI Act + NIST AI RMF alignment docs, vendor-neutral.

**Anti-patterns:** anything that phones home, anything where the agent itself can't pass an audit, anything with a sketchy supply chain.

## Persona 4 — Mira, the AI-Operations Lead at an Enterprise

**Org context:** large enterprise (1000+ engineers) rolling out AI tools to internal developer fleets. Maybe distributing Claude Code or Cursor org-wide. Needs central visibility into agent adoption, usage, cost, and policy compliance.

**Day-to-day pain:**
- Cannot see what tools individual developers are using.
- Cannot enforce org policy at the agent boundary (e.g., "redact customer data from any prompt").
- Cannot answer the CFO's question "what did we spend on AI tools last quarter and where did the value go".
- Cannot prove to InfoSec that AI usage is governed.

**What success looks like with Mara:**
- Edge agent deployed via existing MDM (Jamf/Intune) or build chain.
- Gateway tier aggregates per-team telemetry.
- Hosted control plane (v3) provides org-level dashboards, policy distribution, fleet status, cost attribution.
- Federated identity for policy authoring (Okta/Entra/Google Workspace SSO).

**Decision drivers (v3-era):** MDM-friendly install, gateway scale, policy distribution at scale, SSO/SCIM, cost-allocation, vendor support.

**Anti-patterns:** any feature that requires per-user manual setup.

## Cross-persona constants

- **Apache 2.0 license** — non-negotiable for personas 1, 3, 4.
- **Single binary, no runtime deps** — non-negotiable for persona 2.
- **OTel `gen_ai.*` canonical schema** — non-negotiable for persona 1.
- **Signed releases, SBOM, provenance** — non-negotiable for personas 3 and 4.
- **Beautiful CLI and docs** — non-negotiable for persona 2 and helpful for all.

## Persona prioritization for v1

1. **Rohan** (indie / startup) — primary. Drives quickstart UX, single-binary install, macOS/Linux ergonomics.
2. **Priya** (platform engineer) — primary. Drives Helm chart, OTLP fidelity, k8s DaemonSet quality.
3. **Sasha** (compliance) — secondary in v1, primary in v1.x. Drives SOC 2 mapping, signed bundles, audit log.
4. **Mira** (enterprise lead) — v3 priority. Drives gateway and control-plane design.

This prioritization is reflected in [`../04-implementation/07-phased-milestones.md`](../04-implementation/07-phased-milestones.md).
