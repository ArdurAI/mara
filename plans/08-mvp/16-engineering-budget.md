# MVP — Engineering Budget

## Executive summary

What the 6-week MVP costs ArdurAI in engineer time, cash, and ongoing operational expense. The numbers below are working estimates intended for internal expectation-setting; they are not contractual. Add 25-50% margin if the engineer is part-time or new to Rust async. The intent is so that when someone asks "what does it cost to ship Mara MVP," there is one document that says so concretely.

Total at the low end: ≈ **$8k cash + 6 engineer-weeks**. At the high end: ≈ **$15k cash + 9 engineer-weeks**. Almost all of the cash is signing certificates and external persona honorariums; the engineering time is the dominant resource.

## Engineering time

One engineer, focused full-time, on the dedicated MVP branch:

| Phase | Calendar weeks | Engineer-weeks |
|---|---|---|
| Pre-MVP user research (parallel to other planning) | 2 | 0.75 (interviews + synthesis) |
| Week 1 — OTLP HTTP receiver | 1 | 1.0 |
| Week 2 — OTLP HTTP sender | 1 | 1.0 |
| Week 3 — Loki sink + AI redactor + cost compute | 1 | 1.0 |
| Week 4 — HTTP proxy adapter | 1 | 1.0 |
| Week 5 — Ollama runtime + setup + self-telemetry + diag | 1 | 1.0 |
| Week 6 — Quickstart hardening + bench + release + persona acceptance | 1 | 1.0 |
| **Total engineering** | **8 weeks elapsed** | **6.75 engineer-weeks focused** |

If split across two engineers (parallelism in weeks 1-2 vs 4-5), elapsed time drops to ~5 weeks. Recommend single engineer for MVP; coordination overhead with two would exceed gains at this scale.

## Cash costs

### One-time

| Item | Estimated cost | Notes |
|---|---|---|
| Apple Developer Program membership | $99 / year | Required for macOS notarization. Already needed if ArdurAI ships any macOS product. |
| Windows EV code-signing certificate | $400 / year | Deferred to MVP+2 when Windows is added. |
| Domain (`ardurai.dev` or similar) | $20 / year | Likely already owned by ArdurAI. |
| Sigstore / Rekor usage | $0 | Keyless signing via GitHub OIDC; no per-signature cost. |
| User research honorariums (12 × $50) | $600 | Per [`11-pre-mvp-user-research.md`](11-pre-mvp-user-research.md). |
| Persona acceptance test honorariums (3 × $50) | $150 | Per [`14-launch-and-early-adopter-experience.md`](14-launch-and-early-adopter-experience.md). |
| Otter.ai or equivalent transcription | $20 / month × 1 month | One month is enough for the research window. |
| **One-time total** | **≈ $890** | Apple cert is a yearly recurring; first-year cash is ~$890. |

### Recurring (per month, post-launch)

| Item | Estimated cost | Notes |
|---|---|---|
| GitHub Actions CI compute | $0 — $200 / month | Likely $0 within free-tier for an alpha-stage repo. Adds up if nightly bench moves to self-hosted. |
| GitHub Container Registry storage | $0 — $50 / month | Free for public images at MVP scale. |
| Self-hosted runner for nightly bench (optional) | $50 — $150 / month | Hetzner / Vultr / DigitalOcean instance for the 1-hour bench. |
| Apple Developer Program | $99 / 12 = $8.25 / month | Amortized. |
| Domain renewal | $20 / 12 = $1.67 / month | Amortized. |
| Email forwarding (`security@`, `conduct@`, `oss@`) | $0 — $5 / month | Cloudflare Email Routing is free if domain is on Cloudflare. |
| **Recurring total** | **≈ $0 — $400 / month** | Mostly depends on whether nightly bench needs dedicated hardware. |

For the first 90 days post-launch the realistic operational cost is **< $200 / month** total.

## Headcount and skills

For the MVP build:

- **1 senior Rust engineer.** Async-first, has shipped network services before. Comfortable with `tokio`, `hyper`, `prost`, `wasmtime`. Can write `criterion` benches. Can read OpenTelemetry specs and protobuf definitions.
- **0.25 product / research lead.** Conducts the 12 user interviews and the 3 persona acceptance tests. Writes the synthesis report. Can be the same person as the maintainer if context-switching is acceptable.
- **0.1 security / supply-chain lead.** Reviews the cargo-deny configuration, ensures release workflow attestations succeed, owns `SECURITY.md`. Can be the same engineer with a hat-swap.
- **0 dedicated DevRel.** Avoid hiring DevRel until v1.0+; alpha-stage projects need maintainer-built community, not delegated.

For post-MVP iteration (months 2-6):

- **0.5 engineer continuous.** Triages issues, ships patches, builds MVP+1 features (gRPC OTLP, Splunk HEC, Cursor hooks, Windows packaging).
- **0.1 community / docs.** Reviews PRs, updates `docs/compat-matrix.md`, manages Discord.

## What we don't pay for at MVP

- **Marketing.** No paid acquisition. Hacker News, Twitter, Reddit, Discord, personal networks only.
- **Hosted services we don't need yet.** No Vercel, no Netlify, no docs site at MVP. No Statuspage. No PagerDuty.
- **Legal review beyond the license.** Apache 2.0 is well-trodden; no per-PR legal review.
- **Third-party SaaS development tools.** GitHub free tier, GitHub Actions free tier, GitHub Container Registry free tier. Add paid features when scale demands.
- **Trademark filing.** Defer until v1.0 ships and there's something worth defending.

## Comparison points

For sanity-checking the budget:

- **OpenTelemetry Collector contrib** has dozens of maintainers and a CNCF foundation backing. Mara's MVP is a single-engineer effort scoped to a specific niche.
- **Vector** was 4 engineers for 18 months to reach 1.0. We are not building Vector; we are building a narrow-scope AI-specialist alternative.
- **Fluent Bit** is a multi-year multi-maintainer project under the Linux Foundation. We are not Fluent Bit either.

Mara MVP's scope is deliberately narrower than any of those at their own MVPs. The budget reflects the narrow scope.

## Approvals

For internal ArdurAI use:

- [ ] Engineering lead approves the 6.75-engineer-week commitment.
- [ ] Finance approves the ≈$890 first-year cash spend.
- [ ] Maintainer / community lead approves the response SLA commitment from [`14-launch-and-early-adopter-experience.md`](14-launch-and-early-adopter-experience.md).
- [ ] Security lead approves the release-workflow attestations and acknowledges the 7-day high/critical fix SLA from [`../../SECURITY.md`](../../SECURITY.md).

## Cross-references

- [`06-mvp-implementation-plan.md`](06-mvp-implementation-plan.md) — what the engineering time is being spent on.
- [`11-pre-mvp-user-research.md`](11-pre-mvp-user-research.md) — research costs.
- [`14-launch-and-early-adopter-experience.md`](14-launch-and-early-adopter-experience.md) — launch and ongoing support model.
- [`15-mvp-abort-criteria.md`](15-mvp-abort-criteria.md) — when we stop spending this budget.
