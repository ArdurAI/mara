# Mara Planning Encyclopedia

This folder is the living knowledge base for Mara — an AI-native log shipper and telemetry stack for AI agents and LLM workloads. It catalogs the competitive landscape, gaps in current tools, Mara's value proposition, implementation requirements, deployment blueprints, evaluation methodology, and per-runtime quickstart designs.

The goal is encyclopedic. Each document is a self-contained reference. Documents cross-link rather than duplicate.

Reading order for new contributors:

1. `00-overview/` — what Mara is, what it isn't, the words we use.
2. `01-landscape/` — the existing ecosystem (classic shippers, observability platforms, AI/LLM tools, OTel gen-ai semconv state, AI runtime telemetry surfaces).
3. `02-gaps/` — what's broken or missing in AI observability today, sourced from practitioners.
4. `03-value-proposition/` — where Mara fits and why it matters.
5. `04-implementation/` — concrete requirements, architecture, data model, milestones.
6. `05-evaluation/` — how we measure success and prove compatibility.
7. `06-deployment-blueprints/` — copy-pasteable patterns per platform.
8. `07-quickstarts/` — per-AI-runtime onboarding.
9. `08-mvp/` — what we're shipping next, why, in what scope, and how we'll know.

## Document index

### 00-overview
- [`01-mission-and-scope.md`](00-overview/01-mission-and-scope.md)
- [`02-non-goals.md`](00-overview/02-non-goals.md)
- [`03-glossary.md`](00-overview/03-glossary.md)

### 01-landscape
- [`01-classic-log-shippers.md`](01-landscape/01-classic-log-shippers.md)
- [`02-observability-platforms.md`](01-landscape/02-observability-platforms.md)
- [`03-ai-llm-observability-tools.md`](01-landscape/03-ai-llm-observability-tools.md)
- [`04-otel-gen-ai-semconv.md`](01-landscape/04-otel-gen-ai-semconv.md)
- [`05-licensing-and-governance.md`](01-landscape/05-licensing-and-governance.md)
- [`06-security-and-compliance.md`](01-landscape/06-security-and-compliance.md)
- [`07-rust-crate-ecosystem.md`](01-landscape/07-rust-crate-ecosystem.md)
- [`08-ai-runtime-telemetry-surfaces.md`](01-landscape/08-ai-runtime-telemetry-surfaces.md)

### 02-gaps
- [`01-operational-pain-points.md`](02-gaps/01-operational-pain-points.md)
- [`02-cost-and-latency-gaps.md`](02-gaps/02-cost-and-latency-gaps.md)
- [`03-agent-loop-observability-gaps.md`](02-gaps/03-agent-loop-observability-gaps.md)
- [`04-policy-and-redaction-gaps.md`](02-gaps/04-policy-and-redaction-gaps.md)
- [`05-schema-fragmentation.md`](02-gaps/05-schema-fragmentation.md)

### 03-value-proposition
- [`01-positioning-statement.md`](03-value-proposition/01-positioning-statement.md)
- [`02-feature-matrix.md`](03-value-proposition/02-feature-matrix.md)
- [`03-unique-value-claims.md`](03-value-proposition/03-unique-value-claims.md)
- [`04-target-personas.md`](03-value-proposition/04-target-personas.md)

### 04-implementation
- [`01-functional-requirements.md`](04-implementation/01-functional-requirements.md)
- [`02-non-functional-requirements.md`](04-implementation/02-non-functional-requirements.md)
- [`03-architecture-blocks.md`](04-implementation/03-architecture-blocks.md)
- [`04-data-model.md`](04-implementation/04-data-model.md)
- [`05-pipeline-topologies.md`](04-implementation/05-pipeline-topologies.md)
- [`06-deployment-patterns-overview.md`](04-implementation/06-deployment-patterns-overview.md)
- [`07-phased-milestones.md`](04-implementation/07-phased-milestones.md)
- [`08-success-metrics.md`](04-implementation/08-success-metrics.md)
- [`09-test-strategy.md`](04-implementation/09-test-strategy.md)

### 05-evaluation
- [`01-benchmark-methodology.md`](05-evaluation/01-benchmark-methodology.md)
- [`02-compatibility-matrix-spec.md`](05-evaluation/02-compatibility-matrix-spec.md)
- [`03-soc2-control-mapping.md`](05-evaluation/03-soc2-control-mapping.md)
- [`04-eu-ai-act-alignment.md`](05-evaluation/04-eu-ai-act-alignment.md)

### 06-deployment-blueprints
- [`01-macos-launchd.md`](06-deployment-blueprints/01-macos-launchd.md)
- [`02-linux-systemd.md`](06-deployment-blueprints/02-linux-systemd.md)
- [`03-windows-service.md`](06-deployment-blueprints/03-windows-service.md)
- [`04-kubernetes-daemonset.md`](06-deployment-blueprints/04-kubernetes-daemonset.md)
- [`05-kubernetes-sidecar.md`](06-deployment-blueprints/05-kubernetes-sidecar.md)
- [`06-serverless-lambda-extension.md`](06-deployment-blueprints/06-serverless-lambda-extension.md)
- [`07-docker-compose.md`](06-deployment-blueprints/07-docker-compose.md)
- [`08-ci-runners.md`](06-deployment-blueprints/08-ci-runners.md)

### 07-quickstarts
- [`01-claude-code.md`](07-quickstarts/01-claude-code.md)
- [`02-codex.md`](07-quickstarts/02-codex.md)
- [`03-cursor.md`](07-quickstarts/03-cursor.md)
- [`04-kimi.md`](07-quickstarts/04-kimi.md)
- [`05-augment.md`](07-quickstarts/05-augment.md)
- [`06-gemini-cli.md`](07-quickstarts/06-gemini-cli.md)
- [`07-ollama.md`](07-quickstarts/07-ollama.md)

### 08-mvp
- [`01-scope-and-decision-criteria.md`](08-mvp/01-scope-and-decision-criteria.md)
- [`02-gap-analysis.md`](08-mvp/02-gap-analysis.md)
- [`03-language-choice.md`](08-mvp/03-language-choice.md)
- [`04-ai-native-features.md`](08-mvp/04-ai-native-features.md)
- [`05-problem-statement.md`](08-mvp/05-problem-statement.md)
- [`06-mvp-implementation-plan.md`](08-mvp/06-mvp-implementation-plan.md)
- [`07-test-and-improve-loop.md`](08-mvp/07-test-and-improve-loop.md)
- [`08-risk-register.md`](08-mvp/08-risk-register.md)
- [`09-differentiation-and-moat.md`](08-mvp/09-differentiation-and-moat.md)
- [`10-owasp-alignment.md`](08-mvp/10-owasp-alignment.md)
- [`11-pre-mvp-user-research.md`](08-mvp/11-pre-mvp-user-research.md)
- [`12-ollama-integration-design.md`](08-mvp/12-ollama-integration-design.md)
- [`13-research-recruiting-and-script.md`](08-mvp/13-research-recruiting-and-script.md)
- [`14-launch-and-early-adopter-experience.md`](08-mvp/14-launch-and-early-adopter-experience.md)
- [`15-mvp-abort-criteria.md`](08-mvp/15-mvp-abort-criteria.md)
- [`16-engineering-budget.md`](08-mvp/16-engineering-budget.md)
- [`17-otel-collector-cookbook.md`](08-mvp/17-otel-collector-cookbook.md)
- [`18-long-term-roadmap.md`](08-mvp/18-long-term-roadmap.md)
- [`19-community-governance.md`](08-mvp/19-community-governance.md)
- [`20-migration-guides.md`](08-mvp/20-migration-guides.md)
- [`21-faq-and-troubleshooting.md`](08-mvp/21-faq-and-troubleshooting.md)

## Authoring conventions

- Every document starts with a one-paragraph executive summary.
- Citations use full URLs, not bare references. Prefer official docs and primary sources.
- When a fact is uncertain or dated, mark it `[verify: <reason>]`.
- Diagrams use Mermaid.
- Code samples use fenced blocks with language tags.
- License is Apache 2.0; contributions follow [`../CONTRIBUTING.md`](../CONTRIBUTING.md).

## Status

This is M0-era research output. Implementation reference documents under `04-implementation/` reflect the canonical MOS plan and are the source of truth for engineering. Landscape and gaps documents are continuously curated.
