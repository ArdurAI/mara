# MVP — Problem Statement

## Executive summary

The Mara MVP solves one specific problem for one specific person: **an indie or startup developer running Claude Code on their laptop cannot see what their AI agent does, what it costs, or whether it leaks anything.** Existing observability tools (Datadog, Honeycomb, Langfuse, Phoenix) require either an SDK in code you don't own, a proxy in front of a service you didn't write, or an enterprise account they don't have. Generic log shippers (Fluent Bit, Vector) require the user to invent the AI schema, the redaction pack, and the cost-from-tokens math themselves. There is no edge-first, AI-aware, vendor-neutral telemetry agent for AI runtimes today. The MVP fills exactly that gap.

This document expands the problem statement with cited concrete pain, names the primary user, and sets the success bar.

## Who has the problem

**Primary persona for MVP: Rohan, the indie / startup developer.** See [`../03-value-proposition/04-target-personas.md`](../03-value-proposition/04-target-personas.md) for the full sketch. In short:

- 1–20 engineers shipping AI features.
- Heavy daily Claude Code / Codex / Cursor users for cloud-assisted work.
- **Also a heavy Ollama user** for cost-conscious or privacy-conscious tasks: prototyping with local Llama 3 / Mistral / Phi / Gemma; doing eval sweeps without burning vendor budget; offline development on planes and trains.
- Pays for whatever observability service they could set up in an afternoon (Grafana Cloud free tier, Honeycomb, Logfire, sometimes just stdout).
- Cannot answer "how much did our team spend on Claude this week" or "how many tokens per second is my local model actually producing" with anything resembling precision.

Secondary persona (intentionally deferred to Option B): Priya, the platform engineer. Tertiary (Option C): Sasha, the compliance lead.

## The five pains MVP relieves

Each pain is sourced from real practitioner content. See [`../02-gaps/01-operational-pain-points.md`](../02-gaps/01-operational-pain-points.md) and `02-gaps/02-cost-and-latency-gaps.md` for the source list.

### Pain 1 — Local AI sessions are invisible

**What developers say:** "I just used Claude Code for four hours and have no idea what files it edited, what tools it called, or what I paid." Or for the local-model case: "I'm running Llama 3.3 locally and have no idea if it's fast enough for what I'm building." Or: "Ollama is using 12 GB of RAM and I don't know which model loaded what."

**Status quo:** Claude Code transcripts are in `~/.claude/projects/*.jsonl` but uninspectable; no dashboard exists; no replay. Ollama logs at `~/.ollama/logs/server.log` are unstructured text and don't carry token counts; the API responses do but nothing collects them. No SaaS observability product covers either case because the workload never leaves the laptop.

**MVP solution:** `mara setup claude-code` + an OTel-compatible sink renders the cloud-LLM session in any standard observability dashboard within minutes. `mara setup ollama` plus the HTTP proxy adapter renders local-LLM activity with token counts, latency breakdown, and tokens-per-second, in the same dashboard. Both replayable, shareable, searchable.

### Pain 2 — Cost is unobservable until end-of-month

**What developers say:** "I'm pretty sure that agent loop hit me for $40 yesterday but I can't prove it." Vendor consoles aggregate by day at best. Per-feature, per-project, per-session attribution is manual.

**Status quo:** vendor billing dashboards lag, don't break down by client / project / session, and absolutely don't include locally-running CLI tools' costs in the same view as a SaaS app.

**MVP solution:** `mara.cost.usd` per event; aggregated server-side in the operator's chosen sink. Grafana / Honeycomb sum by attribute. Real-time within seconds of the cost-bearing call.

### Pain 3 — Secrets leak into AI traffic

**What developers say:** "I'm paranoid I'm one bad copy-paste from sending a customer's API key into Claude." Real incidents have happened (the Anthropic Claude Code OAuth stealth-hijacking via `~/.claude.json`, etc.).

**Status quo:** vendor side has some scrubbing; observability tools have post-hoc scrub rules. Neither defends against the prompt actually leaving the laptop in cleartext.

**MVP solution:** agent-side regex redaction with explicit detection of common vendor keys (Anthropic, OpenAI, AWS, GCP, GitHub, Slack, JWT). The redacted token is `[anthropic-key]`, preserving the structural evidence without the secret.

### Pain 4 — Multi-vendor traffic doesn't normalize

**What developers say:** "I tried both Claude and GPT and the dashboards look completely different." OpenAI emits one event shape; Anthropic another; Bedrock wraps either; Vertex AI is its own thing.

**Status quo:** every vendor SDK normalizes their own. Cross-vendor comparison requires a custom ETL.

**MVP solution:** OpenTelemetry `gen_ai.*` is the canonical model. Mara accepts OTLP from any vendor's SDK, normalizes attribute names where vendors deviate from semconv, and emits OTLP to any sink. One schema, all vendors. Honeycomb / Logfire / Datadog / Grafana all see the same shape.

### Pain 5 — Vendor lock-in via SDK or proxy

**What developers say:** "Langfuse looks great but it's another account, another SDK, and I can't see anything that doesn't import their library." Helicone needs you to route through their proxy. LangSmith only sees LangChain runs.

**Status quo:** every AI-obs SaaS captures only the traffic that flows through its own surface area. The runtimes Mara cares about (Claude Code, Codex, Cursor) do not flow through any of them.

**MVP solution:** Mara is the input layer. The operator keeps their existing sink (or chooses a new one) without rewriting application code. Apache 2.0; CNCF-track; vendor-neutral.

## Concrete user story for MVP

```
As Rohan, a startup engineer who uses Claude Code AND Ollama on my MacBook,
I want to install Mara, run two setup commands (one per runtime), and within
five minutes see both my cloud Claude Code sessions and my local Ollama
inferences in a single Grafana Cloud Loki or Honeycomb dashboard,
with cost per session computed for Claude Code, tokens-per-second tracked
for Ollama, and any leaked API keys redacted from either runtime before
they leave my machine,
so that I can monitor cloud AI spend, audit my agent's actions, see how
my local models compare for cost and latency, and trust that my secrets
aren't being shipped to a vendor database in cleartext.
```

This story is the MVP's north star. Every MVP feature traces back to enabling one sentence of it. Anything that doesn't trace back is out of scope.

## What we are NOT solving in MVP

These are real problems, just not for this persona at this milestone.

- **"My Kubernetes cluster of 200 nodes runs 30 AI services and I need fleet observability."** → That's Persona 1; Option B handles it.
- **"I need a tamper-evident audit trail for my SOC 2 Type II auditor."** → That's Persona 3; Option C handles it.
- **"I run 10,000 employees who each use Cursor and I need org-wide governance."** → That's Persona 4 / v3; out of v1 entirely.
- **"My LLM is hallucinating; can Mara fix it?"** → No. Mara is observability, not eval / guardrail.
- **"Mara should block prompts that contain PII."** → No. Mara redacts; guardrail tools (Lakera, NeMo Guardrails, Llama Guard) block. We can ingest their decisions.
- **"Mara should be my LLM cost optimizer."** → No. Mara emits cost telemetry. OpenCost / Vantage / etc. optimize.

The Mara non-goals in [`../00-overview/02-non-goals.md`](../00-overview/02-non-goals.md) apply unchanged to MVP.

## How we know MVP solved the problem

Per [`01-scope-and-decision-criteria.md`](01-scope-and-decision-criteria.md) §"Sign-off criteria": all eight sign-off criteria green for two consecutive nightly CI runs. The five pains above are translatable 1:1 to sign-off criteria:

- Pain 1 ↔ SC-1 (five-minute test) + SC-2 (OTLP round-trip).
- Pain 2 ↔ SC-4 (cost computed).
- Pain 3 ↔ SC-3 (redaction works).
- Pain 4 ↔ SC-2 (OTLP round-trip preserves `gen_ai.*`).
- Pain 5 ↔ SC-7 (zero phone-home) + Apache 2.0 release.

If all sign-off criteria pass and the persona-acceptance test (a real developer running through the quickstart) succeeds, the MVP has solved the problem it set out to solve.

## Cross-references

- [`../02-gaps/01-operational-pain-points.md`](../02-gaps/01-operational-pain-points.md) — gaps that span MVP and beyond.
- [`../02-gaps/02-cost-and-latency-gaps.md`](../02-gaps/02-cost-and-latency-gaps.md) — cost specifically.
- [`../02-gaps/04-policy-and-redaction-gaps.md`](../02-gaps/04-policy-and-redaction-gaps.md) — redaction specifically.
- [`../03-value-proposition/01-positioning-statement.md`](../03-value-proposition/01-positioning-statement.md) — where Mara sits relative to competitors.
- [`../00-overview/02-non-goals.md`](../00-overview/02-non-goals.md) — what we never solve.
