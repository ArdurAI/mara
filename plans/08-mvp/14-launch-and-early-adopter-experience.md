# MVP — Launch and Early-Adopter Experience

## Executive summary

Once `v0.2.0-alpha` is tagged, Mara enters a 30-day window where its survival depends on what happens after the binary ships, not before. This document covers the launch posture (where and how we announce), the first 30-day support model (where users go when things break, what response times we hold ourselves to), the human-run version of the SC-1 acceptance test (recruiting early adopters from the research pool), and a documentation-site decision because operators landing on a GitHub Markdown tree from a Google search is friction we should remove.

The document is operational — it tells the maintainer team what to do on day 0, day 1, day 7, and day 30.

## Launch posture for v0.2.0-alpha

### What we are announcing

A clearly-pre-1.0 alpha that does two things well: captures Claude Code via OTLP and Ollama via HTTP proxy. We are not announcing v1.0. We are not announcing six runtimes. We are announcing the two paths the MVP delivers, plus the architectural foundation that gets us to the others.

### What we are NOT announcing

- That we replace Datadog / Honeycomb / Langfuse / Phoenix / Helicone. We complement those.
- That we're production-ready. We're alpha.
- That we have SOC 2 / EU AI Act compliance certifications. We have control mappings; not audits.
- That every AI runtime works. Two do; the rest are scaffolded.

The CHANGELOG section for v0.2.0-alpha enumerates working features and known gaps, in that order. The README's "Status" section is updated to say "alpha; two of seven target runtimes supported; pre-1.0."

### Announcement channels (day 0)

In rough order of expected signal:

1. **A GitHub release post** at `https://github.com/ArdurAI/mara/releases/tag/v0.2.0-alpha`. Auto-generated release notes plus a 200-word hand-written summary at the top. The hand-written summary leads with "we built X; here's what works and what doesn't."
2. **Hacker News Show HN** at `https://news.ycombinator.com/submit`. Title: `Show HN: Mara – Apache 2.0 telemetry agent for Claude Code and Ollama`. Body: 3 short paragraphs (what it does, why a new tool, what's next). Be on-thread for 24 hours answering replies. Do not vote-manipulate.
3. **A Twitter / X thread** from the project's account (`@MaraTelemetry` or `@ArdurAI` depending on what's set up by launch). 5-7 tweet thread covering: the problem, the two runtimes, the architecture, the quickstart, the limitations, what's next. Pin the thread.
4. **The AI engineering Discords / Slacks** where we did user research, in `#showcase` or equivalent channels with explicit permission. Reference the research participants who were invited as early testers.
5. **Personal LinkedIn from the maintainer.** One post, 300 words, framed around "I learned this from user research; we built X." Avoid the "excited to announce" template.
6. **Cross-post to /r/LocalLLaMA** (for the Ollama angle) and **/r/devops** (for the observability angle). Read the rules of each subreddit first.

### What we deliberately don't do at launch

- We do not pay for boosts or ads.
- We do not press-release. Mara is too small.
- We do not announce on the OpenTelemetry blog (we'll earn that with a real semconv contribution, not a launch post).
- We do not submit to TLDR newsletter / Hacker Newsletter / etc. They'll find us if it's worth finding.
- We do not chase a Product Hunt launch. Wrong audience for an OSS infra tool.

### The 24-hour launch checklist

Day 0:

- [ ] Tag pushed; release workflow succeeded; signed artifacts visible.
- [ ] Homebrew tap updated; `brew install ardurai/mara/mara` works.
- [ ] Linux deb/rpm visible in the repo.
- [ ] Container image pulled and run end-to-end from a clean Docker.
- [ ] README updated; CHANGELOG accurate; compat matrix accurate.
- [ ] Both quickstarts (Claude Code, Ollama) verified to work copy-paste on macOS + Linux from fresh VMs.
- [ ] `mara version` shows correct version; `mara validate` accepts the example config.
- [ ] GitHub release post published.
- [ ] Show HN submitted (mid-morning Pacific time on a Tuesday-Thursday for best timing).
- [ ] Twitter thread posted; pinned.
- [ ] Discord/Slack posts published in the communities where permission was secured.
- [ ] Maintainer is online + responsive for the next 8 hours.

## Support model for the first 30 days

### Where users go

- **Bugs / issues / feature requests:** GitHub Issues at `https://github.com/ArdurAI/mara/issues`. Templates for bug reports and feature requests live in `.github/ISSUE_TEMPLATE/`.
- **Conversation / how-do-I questions:** GitHub Discussions at `https://github.com/ArdurAI/mara/discussions`. Categories: Q&A, Show & Tell, Ideas, General.
- **Security reports:** `security@ardurai.dev` per [`../../SECURITY.md`](../../SECURITY.md). Never public.
- **Chat (early adopters / contributors):** the maintainer's chosen Discord server, channel `#mara`. Invite link is published in the README. Not the primary support channel; just where the community can co-locate.

We deliberately don't ship with Slack, paid support, on-call rotation, or SLAs. Alpha software gets best-effort response from the maintainer.

### Response SLAs (best effort, not contractual)

- **Triage** (first maintainer response on a new issue): ≤ 3 business days during the 30-day window. Stretch to ≤ 1 business day for Security issues.
- **High / Critical bug fix:** patch release within 7 days of a confirmed reproducer.
- **PR review:** ≤ 5 business days for first review.

These numbers are aggressive on purpose. Alpha software with slow maintainer response is dead alpha software. After the 30-day window, the cadence relaxes to weekly triage.

### Issue templates

`.github/ISSUE_TEMPLATE/bug-report.md` and `.github/ISSUE_TEMPLATE/feature-request.md` are populated during M5; verify they ask for:

- `mara version` output.
- OS + version.
- AI runtime + version.
- Config (with secrets redacted).
- Steps to reproduce.
- Expected vs actual.
- `mara diag` output.

## Persona acceptance test playbook

Sign-off criterion SC-1 — "5 minutes from install to first event" — is half automated (the scripted test in `tests/quickstart_*.rs`) and half human (a real person running the steps on their real machine without coaching). This section is the human half.

### Recruiting the test participants

From the user-research pool (per [`13-research-recruiting-and-script.md`](13-research-recruiting-and-script.md)), pick **3 participants** who said yes to "would you try the MVP in 6 weeks":

- **One Claude Code user** without Ollama.
- **One Ollama user** without Claude Code.
- **One user of both** runtimes.

If the research pool is dry, recruit from Discord / community channels with a similar offer.

### The test protocol

For each participant, over Zoom (or async if they prefer):

1. Verify they have a fresh machine state — no Mara installed, default Claude Code / Ollama setup.
2. Send them the URL of the relevant quickstart (`07-quickstarts/01-claude-code.md` or `07-ollama.md`).
3. Start a stopwatch when they start reading. Stop it when they see their first event in their chosen sink.
4. Watch silently. Do not coach. If they ask a question, redirect with "What would you do if I weren't here?"
5. Take notes on every place they pause, ask, or skip.
6. After the test, debrief: "What was confusing? What was missing? What would have helped?"

### Pass criterion

- **For each participant**: ≤ 5 minutes from start to first event.
- **Aggregate across all 3**: at least 2 of 3 pass on the first try; the 3rd may need 1 follow-up clarification but no doc rewrites.

### Failure modes and responses

- **The test takes 15 minutes.** Quickstart needs rewriting. Identify the friction (probably env vars or sink credentials) and rewrite. Re-test with a fresh participant.
- **The test fails because of a bug.** Patch ASAP; re-run.
- **The test passes but the user immediately asks "now what?"** That's a docs gap, not a product gap. Add a "next steps" section to the quickstart pointing at dashboards, the runbook, or the runtime-preset reference.

## Documentation site decision

The plans / docs / quickstart material currently lives in GitHub Markdown. For MVP, this is fine — the audience can navigate a repo. For early-adopter friction it's suboptimal — Google indexes the rendered Markdown poorly and "search docs" is awkward.

### Decision

- **MVP through v0.2.0-alpha:** stay GitHub Markdown. No docs site.
- **v0.2.0 (drop the `-alpha`):** stand up a docs site rendered with mdBook or Docusaurus, hosted at `https://docs.ardurai.dev`. Choice of tool deferred; mdBook is the lighter option and matches the Rust ecosystem (rustc docs, cargo book). Docusaurus is heavier but better for search and theming.
- **v1.0 onward:** the docs site is the canonical documentation surface; `plans/` becomes an engineering-only artifact in the repo.

### Rationale

A docs site needs daily curation. Pre-MVP and during MVP we don't have the engineering bandwidth. Render the markdown the rest of the world has been reading since 2010. Optimize the README to deep-link to the right `plans/` doc; that's enough for an alpha audience.

### Tooling for v0.2.0

Recommend mdBook because:

- Maintained by the Rust project (`rust-lang/mdBook`).
- Single binary, fast builds, simple config.
- Native GitHub Pages deploy.
- Search out of the box (Elasticlunr).
- Matches the Rust-y aesthetic of an Apache 2.0 Rust project.

If we later want richer features (live previews, versioning, MDX), migrate to Docusaurus. Don't over-invest in Hugo / Jekyll / Sphinx; they don't fit the codebase culture.

## First 30 days: success and pivot signals

What we look for, and what each signal triggers.

### Positive signals

- **≥ 50 GitHub stars in week 1.** Vanity-but-useful; minimum reach signal.
- **≥ 3 issues opened by users who aren't the research participants.** Means people are using it.
- **≥ 1 issue with a thoughtful reproducer and a fix attached.** Means people are using it carefully.
- **≥ 2 ADOPTERS.md PRs.** Means people are willing to be cited.
- **≥ 1 external contributor PR merged.** Means the project is genuinely open.

### Pivot signals (any one triggers a planning conversation)

- **No external issue activity in 14 days post-launch.** Either the product is invisible or it doesn't solve a real problem. Investigate.
- **Pattern of "I tried; couldn't get it to work" issues.** Quickstart broken; emergency-fix.
- **Pattern of "this would be useful if it also did X."** X is consistent across users. Consider expediting X.
- **Specific runtime works, the other doesn't, repeatedly.** That runtime's adapter is broken; focus.

### Kill signals (project-level)

- **30 days post-launch: zero external contributors, zero ADOPTERS, zero new issues with reproducers.** The project is dead-on-arrival. Stop spending engineering time; either pivot scope to a different persona or archive.
- **The user-research findings synthesis report (see [`11-pre-mvp-user-research.md`](11-pre-mvp-user-research.md)) revealed that the persona we're targeting doesn't actually want this.** Stop and reconsider before any MVP code is written; covered in pre-MVP planning, not post-launch.

## Cross-references

- [`11-pre-mvp-user-research.md`](11-pre-mvp-user-research.md) — pre-MVP research that feeds the early-adopter pool.
- [`13-research-recruiting-and-script.md`](13-research-recruiting-and-script.md) — recruiting + interview artifacts.
- [`15-mvp-abort-criteria.md`](15-mvp-abort-criteria.md) — when do we stop entirely.
- [`07-test-and-improve-loop.md`](07-test-and-improve-loop.md) — ongoing test cadence.
- [`../../docs/runbook.md`](../../docs/runbook.md) — operator-facing runbook.
