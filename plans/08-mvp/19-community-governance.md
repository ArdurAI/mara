# MVP — Community Governance

## Executive summary

Mara needs to survive its first external contributor, its first runtime preset PR from someone outside ArdurAI, and eventually its first external maintainer. The MVP is a single-maintainer project; v1.0 should not be. This document defines the governance model from where we stand (one maintainer, no external contributors) to where we need to be (formal RFC process, multi-maintainer, CNCF Sandbox-compatible) and the steps to get there. Most of this is lightweight by design — heavyweight governance kills small projects faster than no governance.

Specifically: at MVP launch we publish a minimal `GOVERNANCE.md` and `MAINTAINERS.md`. At v1.0 we have a documented RFC process. At v2.0 we have a steering committee. None of these requires hiring anyone or signing a CLA.

## Phased governance

### MVP era (one maintainer, BDFL-light)

- One maintainer: the MVP engineer (ArdurAI-employed).
- All commits go through PRs with maintainer review. Even maintainer's own PRs require self-review against the PR template checklist before merging.
- Material decisions captured as ADRs under `docs/adr/`. ADRs are not voted on; the maintainer writes them and asks for community comment for ≥ 7 days before accepting.
- No CLA. DCO sign-off on commits is sufficient.
- Issue triage cadence: per SLA in [`14-launch-and-early-adopter-experience.md`](14-launch-and-early-adopter-experience.md).
- No steering committee, no foundation board, no governance theater.

This is enough until the first external maintainer joins.

### v0.5-v1.0 era (small maintainer team)

When the project has ≥ 2 external contributors with merged PRs and one of them is ready to take on triage / review responsibilities:

- Add `MAINTAINERS.md` listing the maintainer team with areas of responsibility (Core, Schema, Adapters, Sinks, Policy, Security, Docs).
- Decision-making moves to a Lazy Consensus model: a maintainer proposes a change; if no objection within 7 days, the change lands. Material decisions still get ADRs.
- A formal RFC process documented (see below). RFCs are required for: new public traits, breaking config changes, new ADR-worthy decisions.
- Conflict resolution: simple majority of named maintainers. Ties broken by the BDFL (initial maintainer / project lead).

This is the model CNCF Sandbox expects.

### v1.0+ era (formal steering)

When the project has ≥ 5 maintainers across ≥ 3 organizations:

- A Steering Committee of 3–5 elected maintainers handles strategic decisions (license changes, governance changes, code-of-conduct enforcement, security disclosure policy).
- The Steering Committee meets monthly; minutes published.
- Major version bumps require Steering Committee approval.
- Day-to-day decisions stay with area maintainers.

This is the model CNCF Incubating expects.

### v2.0+ era (foundation-track)

If the project is CNCF Incubating or Graduated, governance follows CNCF norms: TOC oversight, project-of-projects model possible, neutral trademark holder.

## RFC process (introduced at v0.5 era)

For material changes that aren't appropriate for an ADR alone:

1. Author creates a PR under `docs/rfc/NNNN-title.md` using the template `docs/rfc/_template.md`.
2. RFC is open for comment for ≥ 14 days.
3. The owning area maintainer (or the Steering Committee for cross-cutting RFCs) decides: accepted, rejected, withdrawn, or revised. Decision is recorded in the RFC's "Status" field.
4. Accepted RFCs trigger implementation work. The implementation PR references the RFC number.
5. Rejected RFCs are kept in `docs/rfc/` with status `Rejected` and a rationale.

When to use an RFC vs an ADR:

- **ADR:** a decision affecting code architecture (which crate, which trait, which dependency). Documented post-hoc as needed.
- **RFC:** a decision affecting users (config schema, sink API, runtime preset behaviour) or contributors (process changes). Proposed before implementation; community comment expected.

When to use neither: small changes, bug fixes, documentation tweaks. Most PRs.

## Code of conduct enforcement

[`../../CODE_OF_CONDUCT.md`](../../CODE_OF_CONDUCT.md) is the policy. Enforcement:

- Reports to `conduct@ardurai.dev` (during MVP; an org-neutral address post-CNCF-track).
- During MVP era, the single maintainer is also the code-of-conduct reviewer.
- During v1.0+ era, a 2-person Conduct Committee reviews; the Steering Committee handles appeals.
- Actions follow the Contributor Covenant Enforcement Ladder: Correction → Warning → Temporary Ban → Permanent Ban.
- All enforcement actions are documented privately. Publicly: only "X violated CoC, Y action taken" with anonymized parties unless they self-disclose.

## Maintainer adding / removing

### Adding

A contributor becomes a maintainer when:

1. They've authored ≥ 5 merged PRs (any size).
2. They've reviewed ≥ 10 PRs by others substantively.
3. An existing maintainer nominates them in a PR adding them to `MAINTAINERS.md`.
4. No existing maintainer objects within 7 days.

Maintainers have:

- Write access to the repo.
- Merge authority for their area of responsibility.
- A vote in Steering Committee elections (v1.0+).
- The expectation of triage / review participation.

### Emeritus

A maintainer who is inactive for 6 months is moved to emeritus status in `MAINTAINERS.md`. They retain credit and the ability to return; they lose write access until they re-engage. This is not a punishment; it's clarity.

### Removal

A maintainer can be removed by:

1. Resignation (any time, no reason needed).
2. CoC violation (handled by the Conduct Committee + Steering Committee).
3. Three-month consecutive absence with no response to outreach (auto-emeritus).

## Decision-making mechanics

Lazy Consensus is the default: a maintainer proposes a change; if no objection within 7 days, it lands.

When consensus fails:

- For technical decisions: vote among area maintainers; simple majority.
- For strategic decisions (v1.0+): Steering Committee vote.
- For breaking changes: explicit ADR + RFC required.

When the vote is tied: BDFL casts the deciding vote.

When the BDFL is conflicted (e.g., a decision affects ArdurAI's commercial product): BDFL recuses; Steering Committee votes; simple majority decides.

## Trademark and IP

- **Apache 2.0 license** on all code. Non-negotiable.
- **DCO** (Developer Certificate of Origin) sign-off on all commits via `git commit -s`. Verified by a CI check.
- **No CLA.** The DCO is sufficient legal posture and removes friction for contributors. We do not collect copyright assignments.
- **Mara trademark** owned by ArdurAI (during MVP). Trademark policy published at v1.0 modeled on [CNCF projects](https://www.cncf.io/trademarks/). Forks may not use "Mara" or the project logo without permission; modified versions must rename. Derived non-confusing names are permitted (e.g., "MaraStack-extra" is fine; "Mara Pro" is not).
- **Donation to a foundation.** Considered when the project graduates CNCF Sandbox. Trademark would transfer to the foundation; Apache 2.0 license is unchanged.

## Conflict of interest

Maintainers employed by competing organizations are welcome. Conflicts of interest are managed by:

- Disclosure: maintainers list their employer + funding in `MAINTAINERS.md`.
- Recusal: a maintainer recuses from decisions where their employer has a financial interest (e.g., a Mara-vs-Vector technical evaluation if the maintainer works at Datadog).
- Public discussion: all material decisions happen in public forums (GitHub PRs, public RFCs, public Discord). No private deals.

ArdurAI specifically: ArdurAI employees are maintainers because they wrote the project; they should not have a structural majority on the Steering Committee at v1.0+ (specifically: ≤ 50% of seats).

## Communication channels

- **Public:** GitHub (Issues, Discussions, PRs), Discord `#mara`, Twitter / Mastodon.
- **Maintainer-private:** a `#mara-maintainers` Discord channel for triage coordination.
- **Security-private:** `security@ardurai.dev` per `SECURITY.md`.
- **Code of conduct:** `conduct@ardurai.dev`.

We deliberately do not use private Slack workspaces, Signal groups, or any unauditable channels for project decisions.

## When governance breaks

If governance is itself the problem (e.g., a maintainer feels unsafe, decisions feel railroaded, the BDFL is unresponsive):

1. Email the Steering Committee (v1.0+) or `oss@ardurai.dev` (MVP era).
2. Public airing on a private Discord channel before going to GitHub Issues, to avoid drama spirals.
3. If unresolvable: fork. Apache 2.0 + DCO + a trademark policy that allows non-confusing forks means the project can survive its maintainers.

The right of fork is a feature, not a failure mode.

## Cross-references

- [`../../CODE_OF_CONDUCT.md`](../../CODE_OF_CONDUCT.md) — community behaviour.
- [`../../CONTRIBUTING.md`](../../CONTRIBUTING.md) — contributor workflow.
- [`../../SECURITY.md`](../../SECURITY.md) — security reporting.
- [`../../docs/adr/README.md`](../../docs/adr/README.md) — ADR process.
- [`18-long-term-roadmap.md`](18-long-term-roadmap.md) — when each governance phase activates.
- [`../01-landscape/05-licensing-and-governance.md`](../01-landscape/05-licensing-and-governance.md) — broader OSS governance landscape.
