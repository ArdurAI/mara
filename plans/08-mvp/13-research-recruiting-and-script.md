## Recruiting + interview script (verbatim)

This is the operational companion to [`11-pre-mvp-user-research.md`](11-pre-mvp-user-research.md). That document defines the methodology; this one is the copy-paste artifacts that get the research executed without further drafting work. Everything here is meant to be used verbatim or with one-line edits.

## Executive summary

The user research depends on four written artifacts: the recruiting posts (per channel), the screener form that filters volunteers down to the right persona, the 45-minute interview script (verbatim questions in interview order), and the thank-you / follow-up email. All four are in this document, ready to send.

## Recruiting messages

### Discord / Slack (AI engineering communities)

Copy-paste verbatim into Anthropic Discord #general, OpenAI dev Discord, Cursor community, LangChain Discord, Cloudflare AI Discord, Latent Space community Slack. Adjust the channel etiquette per community (some require admin permission for posts, some have a #help-wanted or #research channel).

> Hey! I'm working on an open-source observability tool for AI coding assistants — specifically Claude Code, Codex, Cursor Agents, and local LLMs via Ollama. Before I write a line of MVP code, I want to talk to 8–12 developers who use any of those tools daily.
>
> What I'm looking for: 45 minutes of your time over Zoom in the next 2 weeks. I'll ask about your actual recent usage, your current observability stack (or lack thereof), and what friction looks like when AI tools surprise you. I'm NOT going to demo anything or ask you to evaluate my product — I want to learn what your day looks like.
>
> Honorarium: $50 Amazon gift card or equivalent open-source donation to a project you pick.
>
> If you're interested: 2-minute screener at `<screener URL>`. DM me with any questions. Will pick first 12 who fit.
>
> Apache 2.0 OSS, not a sales call. Project: github.com/ArdurAI/mara.

### Reddit (/r/LocalLLaMA, /r/MachineLearning, /r/devops)

Submit as a self-post in /r/LocalLLaMA and /r/devops. /r/MachineLearning may require flair = "Discussion" and a follow-up comment with details. Read each subreddit's posting rules first — some prohibit recruiting; that's fine, skip them.

Title:

> [N] User research: 8-12 indie devs using Claude Code / Ollama, 45min, $50 gift card

Body:

> Working on an open-source observability tool for AI coding assistants and local LLMs. Before writing MVP code, doing user research with people who actually use these tools daily.
>
> Looking for: developers who use Claude Code, Codex, Cursor Agents, OR run Ollama locally (Llama, Mistral, Phi, Gemma, etc.). 1–20 person teams, indie/startup work. 45 minutes over Zoom, sometime in the next 2 weeks.
>
> I'll ask about your real recent usage — what tool you opened yesterday, what surprised you, what you wish you could see. NOT a product demo, NOT a sales call. The output is a research report that informs which features I build.
>
> Honorarium: $50 Amazon gift card or equivalent OSS donation. Apache 2.0 project, ArdurAI/mara on GitHub.
>
> Screener: `<screener URL>`. Comments welcome too.

### Twitter / X

Single tweet, no thread:

> Building an open-source observability tool for Claude Code / Codex / Cursor / Ollama. Looking for 8 indie devs using any of those daily, for a 45-min Zoom interview. $50 gift card. Apache 2.0, no demo, no sales. Screener: `<screener URL>`

If quoted/retweeted, a follow-up thread can describe the project; the recruiting tweet stays tight.

### Hacker News

Wait for the next "Who is hiring / Freelancer / Who wants to be hired" monthly thread — there's an unofficial "Who wants user-research participants" thread that surfaces occasionally; otherwise, do not submit as a Show HN unless the project itself has been launched.

Comment template if a research-recruiting thread exists:

> ArdurAI | User research participants | $50 gift card | Remote (Zoom) | github.com/ArdurAI/mara
>
> Building an AI-native open-source observability tool. Doing 8–12 user interviews before MVP coding starts. Looking for developers using Claude Code, Codex, Cursor Agents, or Ollama daily. 45 min, structured around your actual recent usage. Screener: `<screener URL>`.

### Personal network (Twitter DMs, email)

For warm-network reach:

> Hi — I'm doing user research for an open-source AI observability tool I'm building (Apache 2.0, github.com/ArdurAI/mara). I'm looking for 8–12 devs who use Claude Code, Codex, Cursor, or Ollama daily. 45 min Zoom, $50 gift card, in the next 2 weeks. I'd love to talk to you if it fits, or know if you can introduce me to one or two people. Screener if useful: `<screener URL>`. Thanks!

## Screener form

Hosted at e.g. Tally / Typeform / Google Forms. The URL goes into `<screener URL>` above. Fields:

1. **Name** (free text).
2. **Email** (free text, validated).
3. **City + timezone** (free text). Used for scheduling, not bias.
4. **What's your role?** Multi-select with "Other":
   - Indie / solo developer.
   - Startup engineer (team size 1–20).
   - Engineer at mid-size company (20–200).
   - Engineer at large company (200+).
   - Researcher.
   - Founder / non-IC.
5. **Which AI coding tools do you use AT LEAST WEEKLY?** Multi-select:
   - Claude Code.
   - OpenAI Codex CLI.
   - Cursor (Cmd+K / Agent).
   - Kimi.
   - Augment Code.
   - Gemini CLI.
   - Ollama (local models).
   - Aider.
   - Continue.dev.
   - Cody.
   - GitHub Copilot.
   - Other (free text).
6. **What observability tools do you currently use?** Multi-select:
   - Datadog.
   - Honeycomb.
   - Grafana Cloud / self-hosted Grafana.
   - New Relic.
   - Splunk.
   - Logfire.
   - Langfuse.
   - Phoenix.
   - LangSmith.
   - Helicone.
   - None — I just use stdout / `console.log`.
   - Other (free text).
7. **Have you tried wiring telemetry into Claude Code / Codex / Ollama?** Single choice:
   - Yes, currently set up.
   - Tried, gave up.
   - Wanted to but never got around to it.
   - Never thought about it.
8. **Are you willing to share screen during the call?** Yes / No.
9. **Anything else we should know?** Free text, optional.

Selection logic (manual triage):

- **Must have:** at least one of Claude Code / Codex / Cursor / Ollama at weekly+ frequency.
- **Prefer:** mix of "currently uses observability" and "uses none."
- **Prefer:** mix of "tried telemetry" and "never thought about it."
- **Mix:** geographies, team sizes, seniority.
- **De-duplicate:** at most 2 from any one company.

Reply within 2 business days with either an interview-scheduling link (Cal.com / Calendly with 45-min slots) or a polite decline ("we're full this round; thanks for offering"). All declined respondents get a follow-up offer to be on the early-adopter list for v0.2.0-alpha.

## Consent + recording protocol

At the start of every call, before recording starts:

> Thanks for making time. Before we record: this is recorded for transcription so I can take fewer notes and listen better. The recording stays on my laptop and is deleted within 30 days after the research synthesis. I won't share recordings or transcripts publicly. I might quote you in an internal report — anonymized unless you explicitly say otherwise. If at any point you want me to stop recording, just say so and I'll stop immediately. OK to record?

If they say no: don't record. Take handwritten notes.

If they say yes: start recording, restate consent on the recording for the log, proceed.

Tooling: built-in macOS QuickTime, Zoom's local recording, or otter.ai for live transcription. Don't use Granola, Fireflies, or any third-party AI note-taker without their explicit consent to that specific tool.

## Interview script (verbatim, 45 minutes)

The script is **prompts, not a checklist**. If the conversation goes somewhere interesting, follow it. Cover the four sections in order; budget by clock not by question count.

### Opening (5 min)

1. "Tell me a bit about your work. What do you build?"
2. "How big is your team?"
3. "Walk me through yesterday — when you were coding, which AI tools did you actually touch and for what?"
4. "How long have you been using <Claude Code / Ollama / their primary one>?"

### Current observability behaviour (15 min)

Anchor every question on a specific recent event. If they generalize, gently redirect: "Can you tell me about the last specific time that happened?"

5. "Tell me about the last time you wondered 'what is <Claude Code / agent name> actually doing right now?' What did you do to find out?"
6. "Have you ever been surprised by an AI vendor bill? What happened? How did you investigate?"
7. "Tell me about the last time an AI tool did something you wished you could replay or undo."
8. "Have you ever worried about a secret ending up in an AI prompt? What was the situation?"
9. "Show me where you'd look right now if you wanted to see what Claude Code did in a session that already closed."  *(Ask them to share screen if they consented.)*
10. "For your Ollama use specifically: tell me about the last time you wondered what your local model was doing. Token speed? Memory? Which model was loaded?"

### Tooling stack and friction (10 min)

11. "What observability or monitoring tools are part of your day? Walk me through what you actually look at."
12. "Have you tried any AI-observability product — Langfuse, Phoenix, LangSmith, Helicone, anything? What was the experience?"
13. "Where do AI tools fit into your existing observability — or not?"
14. "Tell me about the last 'this should take 5 minutes' that turned into 5 hours. (Doesn't have to be observability — any dev tool.)"

### Mara-specific dry-run (10 min, ONLY after the above)

15. "I'm going to describe what I'm building. Tell me where it would be useful, useless, or unclear — be brutally honest. *(Show README — `https://github.com/ArdurAI/mara`.)* What's confusing in this description?"
16. *(Show the Claude Code quickstart text from `plans/07-quickstarts/01-claude-code.md` on screen-share or in a paste.)* "Read through this. As you go, tell me what each line means to you and where you'd get stuck."
17. *(If they use Ollama: show the Ollama quickstart, same instructions.)*
18. "If this existed today and worked exactly as described, would you actually install it on your machine this week? Why or why not? What's the friction?"

### Wrap (5 min)

19. "If we ship a working version in 6 weeks, would you be willing to try it for an hour and tell us what broke?" *(Captures the early-adopter pool for week-5 persona acceptance.)*
20. "Is there anyone else I should talk to — friends, colleagues, anyone who fits this profile?"
21. "Anything I should have asked but didn't?"
22. *(Stop recording.)* "Thanks. Gift card incoming via email this week. Where should I send it?"

## Anti-patterns the interviewer must avoid

- **Leading questions.** "Would you use a tool that…" → reword as "Tell me about the last time…"
- **Selling.** Mention Mara only after question 15. Until then, you're a researcher, not a founder.
- **Closed-ended bias.** "Was that frustrating?" → "How did that feel? What did you do next?"
- **Filling silence.** Wait 4 seconds. They'll say something useful.
- **Defending the design.** When they say something dumb about what you're building, write it down. Do not defend.

## Thank-you and follow-up email

Send within 24 hours of the interview:

> Subject: Thanks for the Mara research interview
>
> Hi <first name>,
>
> Thanks again for the time today — really useful conversation. A few follow-ups:
>
> 1. **Gift card.** Amazon $50 gift card on its way to <email>. Should arrive within 24 hours. If you'd rather donate the $50 to an OSS project, reply with which one and I'll do that instead.
> 2. **What you'll see from us.** I'll send a one-page summary of the research synthesis (anonymized) in ~3 weeks, so you can see what we heard collectively and what we're building.
> 3. **Early adopter pool.** You said yes (/no) to trying the MVP in ~6 weeks. I'll be in touch then if so.
> 4. **Quote permission.** If I want to quote anything you said (anonymously by default; with attribution only if you explicitly approve), I'll ask before publishing.
> 5. **Recording disposal.** Recording deleted in 30 days. Transcript deleted in 30 days. No exceptions.
>
> If anything else comes to mind that you wished you'd said — or you stumble across a Twitter thread / GitHub issue / blog post that relates to what we talked about — please send it over.
>
> Thanks again.
>
> <name>
>
> github.com/ArdurAI/mara — Apache 2.0

## Post-research artifact

After all interviews are complete and synthesis is done, write the findings report at `plans/08-mvp/14-research-findings.md` (this is reserved; the file is created post-research, not pre-emptively). Structure per [`11-pre-mvp-user-research.md`](11-pre-mvp-user-research.md) §"Output artifact."

Inform anyone who wants to know — Twitter announcement, Discord update, ADOPTERS-list email — when the findings are public. Transparent research builds the community moat described in [`09-differentiation-and-moat.md`](09-differentiation-and-moat.md).

## Cross-references

- [`11-pre-mvp-user-research.md`](11-pre-mvp-user-research.md) — methodology, decision rules, timeline, budget.
- [`05-problem-statement.md`](05-problem-statement.md) — the persona and pains we're validating.
- *The Mom Test* by Rob Fitzpatrick — methodology source.
