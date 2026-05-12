# MVP — Pre-MVP User Research

## Executive summary

Before we write a line of MVP code, we talk to people who actually live in Persona 2's shoes — indie / startup developers running Claude Code, Codex, Cursor, or Ollama on their own machines daily. The MVP plan in [`06-mvp-implementation-plan.md`](06-mvp-implementation-plan.md) is built on our internal model of what they need; that model is at best 70 % right and at worst 30 % right. Spending two weeks talking to eight to twelve real users before week 1 of coding is the cheapest way to find out which 30 % is wrong. This document specifies who we talk to, what we ask, how we synthesize, and what we change in the MVP plan based on what we hear.

Total cost: ≈ 2 weeks elapsed, ≈ 30 hours of research effort, $400–$800 in honorariums. Saves potentially weeks of building the wrong thing.

## Why we're doing this

The risks we're spending two weeks to retire:

- **R-A1 (Adoption risk):** zero external users sign up to test the MVP because nobody actually wants what we're shipping. Catching this before week 1 saves five weeks of build.
- **R-S3 (Scope risk):** the persona acceptance test reveals at week 5 that the "5-minute setup" is actually 30 minutes because our model of the user is wrong about which steps are obvious.
- **R-A3 (Adoption risk):** initial users report friction we should have anticipated (OTel env vars are confusing, sink credentials are hard, "what's a Prometheus scraper" etc.).

The two-week investment is also a marketing dividend: every person we interview becomes a candidate for the week-5 persona acceptance test and the v0.2.0-alpha launch. Eight interviews → eight potential first users.

## Who we talk to

Target sample: **8–12 developers**, mix of:

- **5–7 indie / startup engineers** (1–20 person teams) using Claude Code, Codex, or Cursor daily for real work.
- **2–3 developers running local LLMs via Ollama** for cost / privacy / latency reasons. Often the same person as above; bonus signal when they overlap.
- **1–2 platform engineers at slightly larger companies** (50–200 people) — secondary persona, useful as a sanity check on what we're NOT optimising for at MVP.

We deliberately do not interview:

- Enterprise platform leads at 1000+ engineer companies (that's Persona 4 / v3 territory).
- AI researchers at frontier labs (different needs; not our user).
- Compliance / security leads (that's Persona 3 / Option C).

## How we recruit

In rough order of expected yield:

1. **AI engineering Discord servers and Slack communities.** Anthropic Discord, OpenAI dev Discord, Cursor community, LangChain Discord, Cloudflare AI Discord, Latent Space community. Public ask: "Working on an open-source observability tool for Claude Code / Codex / Cursor / Ollama. Looking for 8 indie devs to do 45-minute Zoom interviews. $50 Amazon gift card or equivalent OSS donation. DM me." Target: 4–6 recruits.
2. **/r/LocalLLaMA and /r/MachineLearning** on Reddit. Same post template. Target: 2–3 recruits.
3. **Twitter / X.** Personal networks + tagged thread. Target: 1–2 recruits.
4. **Hacker News "Who wants to be interviewed" thread** if one is open during the window.
5. **ArdurAI's own network** — colleagues' friends. Last resort because of bias risk; use only if 1–4 fall short.

Mix of geographies and experience levels deliberately; do not over-recruit Bay Area senior engineers.

## What we ask (interview script)

45 minutes, semi-structured. Recorded with consent, transcribed.

### Opening (5 min)

- Tell me about your work. What do you build?
- What AI tools are part of your daily workflow? How long for each?
- Walk me through yesterday: which AI tools did you actually use and for what?

### Current observability behaviour (15 min) — The Mom Test, no "would you use X?"

- Tell me about the last time you wondered "what is Claude Code actually doing right now?" What did you do to find out?
- Tell me about the last time you got a surprising bill from an AI vendor. What did you do?
- Has an AI tool ever done something you wished you could replay or undo? What happened?
- Tell me about the last time you worried about secrets ending up in an AI prompt.
- Show me where you'd look if you wanted to see what Claude Code did in a session that closed.
- For Ollama users: tell me about the last time you wanted to know what your local model was doing. CPU? memory? tokens per second? cost-equivalent?

### Tooling stack and friction (10 min)

- What observability / monitoring tools do you use day-to-day? (Honeycomb, Datadog, Grafana, just stdout, nothing?)
- Have you tried any AI-observability product (Langfuse, Phoenix, LangSmith, Helicone)? What worked, what didn't?
- Where do AI tools fit (or not) in your existing observability?
- Tell me about your last "5-minute setup" that turned into a 5-hour setup.

### Hypothetical (5 min) — only after the above, never first

- I'm going to describe what I'm building and you tell me where it would be useful, useless, or unclear. [Show README.] What's confusing? What's missing? What's irrelevant?

### Mara-specific dry-run (5 min)

- Show them the proposed quickstart text from `plans/07-quickstarts/01-claude-code.md`. Ask them to read it aloud and tell you what each line means.
- Where do they get stuck? What words don't they recognize? What steps feel unnecessary?

### Wrap (5 min)

- If we ship this in 6 weeks, would you be willing to try it for an hour and tell us what broke? (Builds the early-adopter pool.)
- Anyone else we should talk to?
- Anything we should have asked but didn't?

## Anti-patterns to avoid (The Mom Test)

- ❌ Never ask "would you pay for this?" — they'll say yes to be polite.
- ❌ Never ask "is this a good idea?" — they'll say yes.
- ❌ Never describe Mara's features and ask for opinions — leads them.
- ❌ Never start with the hypothetical — anchor on real recent behaviour first.
- ✅ Ask about specific recent events ("last time," "yesterday," "show me").
- ✅ Listen for emotion words (frustrating, scary, surprising). Probe those.
- ✅ Take notes on what they SHOW you, not what they SAY they do.

Reference: Rob Fitzpatrick, *The Mom Test*, 2013. We're not original.

## Synthesis methodology

After interviews are done:

1. **Affinity mapping.** Transcribe; cluster quotes by theme on a Miro board (or sticky notes). Look for clusters of three or more independent mentions.
2. **Jobs-to-Be-Done framing.** For each cluster, formulate as "When I'm <situation>, I want to <motivation>, so I can <expected outcome>." Compare to our Persona 2 user story in [`05-problem-statement.md`](05-problem-statement.md) §"Concrete user story." Note divergences.
3. **Failure-mode catalogue.** Every "5-minute setup that turned into 5 hours" story is a failure mode for our quickstart to avoid. Catalogue them.
4. **Top 5 surprises.** Things we believed before research that turned out to be wrong. Each surprise either:
   - Adjusts an MVP scope item (in/out of [`01-scope-and-decision-criteria.md`](01-scope-and-decision-criteria.md)).
   - Adds an AI-native feature to [`04-ai-native-features.md`](04-ai-native-features.md).
   - Adds a risk to [`08-risk-register.md`](08-risk-register.md).
   - Adds a no-goal to [`../00-overview/02-non-goals.md`](../00-overview/02-non-goals.md).

## Decision rules

What the research could tell us, and what we'd change in response:

- **"Nobody actually wants to capture Claude Code telemetry; the JSONL files are good enough."** → MVP pivot. Possibly kill the project or reframe completely. (Likelihood: low; we'd be surprised, but worth knowing.)
- **"5-minute setup is unrealistic; users expect 30 minutes for any new dev tool."** → Reduce SC-1 stringency; pivot quickstart messaging.
- **"OTel env vars are a deal-breaker; users want a one-line install with auto-detection."** → Add scope: `mara setup claude-code` writes to the user's shell profile too. Adds 1–2 days.
- **"Loki is the wrong sink; everyone uses Datadog / Honeycomb / Logfire."** → Reprioritize MVP sinks. OTLP-out covers Honeycomb / Logfire / Datadog already; Loki may be deferred.
- **"Cost tracking is the killer feature, redaction is nice-to-have."** → Already aligned with our plan. Confirm and ship.
- **"Ollama users want energy / GPU-utilization telemetry more than tokens."** → New AI-native feature: `mara.compute.*` extensions for local inference. Defer to MVP+1 or add to MVP.
- **"Persona doesn't exist; nobody we recruited fits the profile."** → Major problem. Either our persona is wrong, our recruiting channels are wrong, or both. Pause and re-plan.

## Timeline

Two-week window, in parallel with the rest of MVP planning so coding can start week 3.

- **Week -2:** post recruiting messages; schedule 8–12 interviews.
- **Week -1:** conduct interviews (1–2 per day with buffer); transcribe ASAP.
- **End of Week -1:** affinity mapping session; write the research report.
- **Day 1 of Week 0:** team reads the report; agrees on which MVP plan docs need updating.
- **Day 2–3 of Week 0:** update the affected plan docs; sign off the revised MVP scope.
- **Week 1:** MVP coding begins per revised plan.

If the research surfaces a fundamental rethink ("our persona doesn't exist"), the MVP slips by up to 4 weeks while we re-plan. That's the trade.

## Output artifact

A single Markdown report at `plans/08-mvp/12-research-findings.md` (created post-interviews; not pre-emptively). Structure:

- Who we talked to (anonymized: role, company size, AI tools used, geography).
- Top 5 surprises (things we got wrong).
- Top 5 confirmations (things we got right).
- Jobs-to-be-done catalogue.
- Failure-mode catalogue for the quickstart.
- Recommended changes to: scope, AI-native features, problem statement, implementation plan, risk register.
- Excerpts (anonymized quotes that support the synthesis).

The report is the artifact; the conversations are the value.

## Budget

| Item | Cost |
|---|---|
| Honorariums: 12 × $50 Amazon gift cards | $600 |
| Transcription (otter.ai, ~12 hours audio) | $50–$100 |
| Researcher time (one person, 30 hours over 2 weeks) | Internal |
| Total cash | ≈ $700 |

## Cross-references

- [`01-scope-and-decision-criteria.md`](01-scope-and-decision-criteria.md) — what the research could change.
- [`05-problem-statement.md`](05-problem-statement.md) — the user-story we're validating.
- [`08-risk-register.md`](08-risk-register.md) — R-A1, R-A3, R-S3.
- [`07-test-and-improve-loop.md`](07-test-and-improve-loop.md) — the test loop the research feeds into.
- *The Mom Test* by Rob Fitzpatrick (2013) — methodology source.
