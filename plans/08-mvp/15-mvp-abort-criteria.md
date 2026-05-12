# MVP — Abort and Pivot Criteria

## Executive summary

Every project should know in advance the conditions under which it stops. Not stops a feature, stops the whole project. Without those triggers written down before the code is shipped, abort decisions become emotional, late, and expensive. This document defines the specific triggers that would cause us to pause MVP work, pivot scope, or kill the project entirely. Each trigger is observable, time-bound, and reviewed at three checkpoints: after user research, at MVP week 3 (mid-build), and 30 days post-launch.

The honest premise: most ambitious projects deserve to die. Mara might be one of them. Defining "when we stop" in advance keeps us honest later.

## Three abort/pivot tiers

### Tier 1 — Pause MVP, rescope

Triggered by signals that say "we're building the wrong thing" but the underlying mission is still valid. Response: pause coding, run a 1-week re-planning sprint, restart with revised scope. Acceptable outcome.

### Tier 2 — Major pivot

Triggered by signals that say "the mission is partly right but the shape is wrong." Response: 2-4 week scope rewrite, possibly a new persona, possibly a different deployment shape (e.g., not edge-first; SaaS-first). Significant work loss but the project continues.

### Tier 3 — Kill the project

Triggered by signals that say "the problem we're solving doesn't have enough customers, or solutions already exist that are good enough." Response: archive the repo with a final retrospective post; release the planning encyclopedia as a public artifact for anyone else to learn from.

## Checkpoint 1 — After pre-MVP user research (week 0)

User research completes at the end of week -1 / start of week 0 of MVP coding. Synthesis happens in 3 working days. Findings drive the first abort/pivot decision.

### Tier 1 triggers — Pause and rescope (1 week)

- **Specific MVP features are wrong.** E.g., research reveals "nobody cares about cost tracking; latency is what matters" or "Loki sink is wrong, everyone uses Datadog." Pause; rewrite [`01-scope-and-decision-criteria.md`](01-scope-and-decision-criteria.md); restart.
- **Quickstart steps are user-hostile.** E.g., "OTel env vars are a deal-breaker; users won't set them manually." Rewrite quickstart + `mara setup`; restart.
- **Persona refinement.** E.g., research reveals indie devs aren't the right primary; platform engineers at 50-person companies are. Re-scope around Persona 1.

### Tier 2 triggers — Major pivot

- **Wrong deployment shape.** E.g., research reveals "everyone wants a proxy in front of every LLM call; an edge agent that scrapes things is wrong." Pivot to inference-proxy positioning.
- **Wrong language.** Unlikely given research is about user need, but theoretically: if research reveals "we want to embed Mara as a library in our app, not run it as a daemon," that changes language and shape.

### Tier 3 triggers — Kill

- **Nobody in the persona uses any of the target runtimes.** Recruiting yielded zero valid participants after multiple weeks of trying.
- **The pain we hypothesized is not real.** Across 8+ interviews, nobody describes any pain matching our problem statement. They're happy with how things are.
- **Existing tools cover 95% of the need.** Across 8+ interviews, participants say "I tried Mara's positioning; Datadog LLM Obs / Langfuse / OTel Collector handles all of it." We have no seat.

### What we do not consider an abort signal at this checkpoint

- "Half the participants didn't know what OTel is." Expected; we educate.
- "Participants suggested features we didn't think of." Expected; that's why we're researching.
- "Participants pushed back on Apache 2.0." Unlikely but unimpressive; license is non-negotiable.
- "Participants want it now, not in 6 weeks." Expected; we ship when we ship.

## Checkpoint 2 — MVP week 3 (mid-build)

Halfway through the 6-week build. The OTLP receiver + sender + Loki sink are done; the proxy adapter and Ollama integration are starting.

### Tier 1 triggers

- **A single MVP feature is consuming 50%+ over budget.** E.g., the proxy adapter is taking 10 days instead of 5. Defer to MVP+1; ship with one runtime not two.
- **Performance baseline is way off.** E.g., week-3 smoke is 2k EPS at 1 GiB RSS instead of the 10k EPS / 384 MiB target. Pause feature work; spend 1 week on perf; resume.

### Tier 2 triggers

- **The proxy adapter design has a fundamental flaw.** E.g., transparent SSE forwarding without buffer-then-emit doesn't work reliably with Ollama's actual streaming semantics. Major rework of [`12-ollama-integration-design.md`](12-ollama-integration-design.md); +2 weeks to schedule.
- **An external user from the research pool says "I tried the WIP build and the whole approach is wrong."** Take the feedback seriously; reconsider scope.

### Tier 3 triggers

- **The engineer building the MVP burns out, leaves, or signals "this is a year of work, not 5 weeks."** No replacement available, no path forward. Pause indefinitely.
- **A competitor ships exactly what we're building under a license we can't beat.** Specifically: OpenTelemetry Collector contrib ships a `gen-ai/runtime-preset` processor that does 90% of Mara's curated knowledge. We have nothing left to add. Archive.

## Checkpoint 3 — 30 days post-launch

v0.2.0-alpha has been shipped for 30 days.

### Tier 1 triggers

- **Specific friction patterns in issues.** E.g., 10 issues all complain about the same setup step. Fix the setup step in v0.2.1; not an abort.
- **Specific runtimes work, one consistently breaks.** Patch + better tests.

### Tier 2 triggers

- **Users adopt a different sink than we built for.** E.g., everyone wants Splunk HEC, nobody uses Loki. Reprioritize MVP+1 to ship Splunk HEC first.
- **The proxy approach for Ollama gets repeated negative feedback.** "Why isn't this an OpenAI-compat sidecar?" Reconsider; possibly re-architect Ollama capture.

### Tier 3 triggers (any single one is enough to consider killing)

- **Zero external GitHub stars beyond the maintainer + research participants after 30 days.** No reach.
- **Zero external issues with reproducers in 30 days.** Nobody is using it carefully.
- **Zero ADOPTERS.md PRs in 30 days.** Nobody trusts it for production.
- **Zero external contributors in 60 days.** Nobody believes in it.
- **30-day Homebrew install count under 100.** Distribution failure.
- **The maintainer / ArdurAI cannot dedicate continued engineering time.** Resource starvation. Pause and announce.

## What "kill" actually means

Killing Mara doesn't mean burning the repo. It means:

1. **Tag a final release** documenting current state honestly.
2. **Write a retrospective post** ("we built Mara, here's what we learned, here's why we stopped"). Publish to the GitHub release, the Mara blog if it exists, and HN.
3. **Archive the repo** on GitHub with a clear `ARCHIVED.md` at the top of the README pointing readers to alternative projects (OTel Collector, Vector, whatever fits their need).
4. **Donate the encyclopedia.** The 60+ planning documents under `plans/` become a public artifact. The OTel community in particular may find the gen-ai integration intel useful; offer the runtime-surface research as a CC BY 4.0 contribution.
5. **Honor the maintainer commitments.** Anyone who depended on us (research participants, ADOPTERS, external contributors) gets a personal note explaining what's happening.

The point: kill the project gracefully, leave value for whoever comes next, do not pretend things are fine while letting them rot.

## What "pivot" actually means

Pivoting is harder than killing because it requires admitting that what we built isn't what's needed. Concrete pivot examples:

- **From edge agent to gateway-first.** If research shows everyone wants Mara as a remote service rather than a local binary, abandon edge-first; recast as a hosted product. Massive rewrite. Honest assessment: probably not viable for ArdurAI's current resources.
- **From AI-specialist shipper to AI-policy gateway.** If users say "we don't need observability; we need redaction-at-egress," recast Mara as a guardrail / proxy product. Reasonable pivot; reuses much of the codebase.
- **From multi-runtime shipper to single-runtime power tool.** If only Claude Code resonates, abandon Codex/Cursor/Kimi/Augment/Gemini/Ollama and become "the Claude Code observability tool." Easier scope; narrower seat.

Pivots require: explicit go/no-go decision in writing, ADR documenting the change, re-baseline of all planning docs, communication to research participants and early adopters.

## Reviewing this document

This document is read at each of the three checkpoints. After every read it gets updated: thresholds adjusted based on what we now know, new triggers added based on patterns we didn't anticipate, dead triggers struck through (not deleted, so the historical record is preserved).

If we never read this document, that's a bad sign — it means we're avoiding the question of whether we should stop.

## Cross-references

- [`11-pre-mvp-user-research.md`](11-pre-mvp-user-research.md) — checkpoint 1 input.
- [`06-mvp-implementation-plan.md`](06-mvp-implementation-plan.md) — checkpoint 2 source of truth.
- [`14-launch-and-early-adopter-experience.md`](14-launch-and-early-adopter-experience.md) — checkpoint 3 signals.
- [`08-risk-register.md`](08-risk-register.md) — risks; this doc is what we do when risks materialize.
- [`09-differentiation-and-moat.md`](09-differentiation-and-moat.md) §"What happens if the moat erodes anyway" — strategic response to losing differentiation.
