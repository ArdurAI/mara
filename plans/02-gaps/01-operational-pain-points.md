# Operational Pain Points in AI/LLM Observability

## Executive summary

This document catalogs the practical operational pain points teams face today when trying to observe AI agents and LLM workloads. It draws from public practitioner content — Hacker News threads, /r/LocalLLaMA, /r/MachineLearning, Medium and Substack posts from AI platform engineers, GitHub issues on observability and agent frameworks, OpenTelemetry gen-ai SIG meeting notes — to ground each pain point in real, citable evidence rather than hypotheticals. Mara's positioning derives from this list.

This document is the source of pull when authoring marketing content, prioritizing features, or evaluating competitor claims. Each pain point includes a Mara-side stance: do we solve it, complement someone who solves it, or explicitly leave it to others.

## Index of pain points

The remaining gaps documents in this folder break out subsets in more depth:

- [`02-cost-and-latency-gaps.md`](02-cost-and-latency-gaps.md) — cost and latency observability.
- [`03-agent-loop-observability-gaps.md`](03-agent-loop-observability-gaps.md) — multi-turn agent trace structure.
- [`04-policy-and-redaction-gaps.md`](04-policy-and-redaction-gaps.md) — PII handling.
- [`05-schema-fragmentation.md`](05-schema-fragmentation.md) — schema standards in flux.

## The big eight, as of May 2026

### Pain 1 — Cost is unobservable in real time

Teams discover their LLM bill at the end of the month or when a billing dashboard alerts at hour 22 of a runaway agent loop. Token-by-token billing means cost can spike orders of magnitude faster than traditional cloud cost can.

Existing tools: Datadog LLM Obs, Langfuse, Helicone, LangSmith all show cost — but only for traffic that flows through their SDK or proxy. None of them see cost for Claude Code / Codex / Cursor / Augment sessions running on a developer's laptop or in CI.

Mara's stance: agent-side cost capture from runtime emissions; configurable price tables for vendor-without-cost-emission cases; computed `mara.cost.usd` on every event.

### Pain 2 — Multi-vendor schema fragmentation

OpenAI emits one event shape. Anthropic another. Bedrock wraps either. Google Vertex AI does its own thing. Mistral, DeepSeek, Cohere all differ. Within "agents," LangChain, LlamaIndex, AutoGen, OpenAI Assistants API, MCP, and proprietary stacks emit incompatible structures.

Existing tools: each vendor's SDK normalizes their own; OpenInference (Arize-led) and OTel `gen_ai.*` (OTel-led) try to unify; neither is universally adopted.

Mara's stance: align with OTel `gen_ai.*` + MCP semconv; map `openinference.*` ↔ `gen_ai.*` for interop; contribute back upstream.

### Pain 3 — Prompts and completions leak into logs

Engineers stumble across user PII (sometimes regulated) in production logs because someone wrote `console.log(messages)` during dev and never removed it. Or worse, the official vendor SDK was emitting prompt content into traces by default at one point. Or an OTel auto-instrumentation captured it because the project's defaults said capture-everything.

Existing tools: each SaaS offers some scrub rules, but enforcement is server-side and discovered after-the-fact.

Mara's stance: ZDR-respecting defaults; opt-in capture; built-in PII/PHI/PCI redaction packs at the agent boundary, before sink dispatch.

### Pain 4 — Agent loops have no portable trace shape

A modern agent: parent turn → tool call → sub-agent → another tool call → return → another LLM call → another tool call → final response. Existing observability backends visualize one trace, but the way agents structure parent-child spans varies wildly. Some flatten everything to siblings of the root. Some create deep trees. Some embed tool calls as events rather than spans. Some lose context across async hops.

Existing tools: OTel's `gen_ai.*` semconv now defines `gen_ai.agent.*` spans, but the SDK landscape is uneven.

Mara's stance: normalize parent-child relationships in the canonical schema; preserve `trace_id` and `span_id` from upstream when present; document the agent-loop pattern in the data model.

### Pain 5 — Local/on-device agents are invisible

Claude Code, Codex, Cursor, Kimi, Augment, and Gemini CLI run on developer laptops. Their activity is consequential — they write code, they spend money, they make decisions — but no observability tool sees them by default. Until each runtime ships OTel, capturing this telemetry is a manual hack.

Existing tools: none cover this category as primary use case.

Mara's stance: this is the canonical use case. Tier A/B/C per-runtime support.

### Pain 6 — Tool/MCP traffic is observability's blind spot

MCP standardizes tool invocation; many agents use it. But tool calls are where the real action happens (file edits, web calls, API mutations). Most observability tools treat tool calls as opaque metadata on a span; they don't surface `mcp.server.name`, `mcp.tool.namespace`, or `mcp.resource.uri` as first-class queryable fields.

Existing tools: OTel MCP semconv is recent and not widely consumed yet; vendor dashboards rarely have first-class MCP views.

Mara's stance: `mcp.*` is a first-class part of the canonical schema; MCP traffic surfaced in default dashboards (ship with the Helm chart).

### Pain 7 — Evals and traces don't share data

Teams run evals offline on captured traces, or live in a separate pipeline. Cross-referencing "this completion got a bad eval" with "what was the user prompt and what tools fired" requires manual gluing.

Existing tools: Phoenix, Braintrust, Galileo, LangSmith all do this for traffic in their own SDK. None do it across runtimes.

Mara's stance: include `mara.eval.result` in the schema; allow eval tools to post results back as events; preserve the trace-context linkage so eval data joins to traces in any sink.

### Pain 8 — Cardinality blowup in metrics-store backends

Storing prompt content (or even prompt hashes) as label values explodes cardinality in Prometheus / Mimir / Cortex / VictoriaMetrics. Operators end up with backend stalls or expensive resizing.

Existing tools: Loki 3.x's structured metadata reduces label cost; OTel semconv guidance recommends using attributes over labels. Easy to misconfigure.

Mara's stance: documented sink-specific mapping rules that prevent cardinality blowups by default (e.g., Loki labels = `runtime`, `event_kind` only; prompt content under structured metadata).

## Secondary pain points (covered in depth in sub-documents)

### Pain 9 — Cached / prefill / reasoning token accounting

Vendors expose different breakdowns. Prefix caching saves real money but only some SDKs surface it. Reasoning tokens (o1-style, DeepSeek-R1) are billed differently. Aggregating across vendors requires a normalization step the SDKs don't always do.

→ [`02-cost-and-latency-gaps.md`](02-cost-and-latency-gaps.md).

### Pain 10 — TTFT vs total latency

End-user perceived latency is time-to-first-token. Backend operators care about total turn time. Few backends graph both. Tool-call latency inside an agent turn is a third axis that often gets lost.

→ [`02-cost-and-latency-gaps.md`](02-cost-and-latency-gaps.md).

### Pain 11 — Per-tenant attribution

SaaS over LLMs needs per-tenant cost, latency, and error rates. OTel attributes support this if used consistently, but enforcement requires either an SDK wrapper or a server-side enricher.

→ [`03-agent-loop-observability-gaps.md`](03-agent-loop-observability-gaps.md).

### Pain 12 — Hybrid cloud + edge span correlation

User laptop → cloud LLM → cloud tool → user laptop closes a loop. W3C Trace Context propagation works in theory but breaks at boundaries that don't propagate it (CLI tools, MCP servers without context-aware SDKs).

→ [`03-agent-loop-observability-gaps.md`](03-agent-loop-observability-gaps.md).

### Pain 13 — Silent failure modes

Context window exhaustion, model degradation, tool returning subtly wrong data, model returning JSON that almost-but-not-quite parses. These don't raise errors; they raise downstream errors hours later. Observability rarely surfaces them as anomalies.

→ Future work; v1 surfaces error events; v1.x explores anomaly classification policies.

### Pain 14 — Retrieval quality

For RAG: the "retrieval got it wrong" failure mode is operationally distinct from "model got it wrong." Few observability tools split them by default.

→ Future; v1 captures retrieval-related tool calls as `gen_ai.tool.type = "retrieval"` and preserves the retrieval span.

### Pain 15 — Prompt versioning

Teams ship a new system prompt and quality changes; nobody knows because the deploy timestamp and the quality dip are graphed on different dashboards.

→ Out of scope for Mara directly; Mara can capture a prompt-template id when emitted.

### Pain 16 — Cross-org data exfiltration via copilots

Developer pastes proprietary code into a CLI agent; the prompt goes to a vendor. Observability of this from the security side is hard.

→ Mara's audit log + ZDR-aware capture surfaces this when the operator opts in.

## Top 10 unsolved problems in AI agent observability, as of May 2026 (Mara opinion)

1. **No portable, AI-runtime-aware telemetry agent exists.** (Mara fills this.)
2. **Schema standards are not yet stable** (`gen_ai.*` is Development; `openinference.*` exists in parallel).
3. **Local/dev/IDE-resident agents are invisible by default.** (Mara fills this.)
4. **Cost normalization across vendors is manual.**
5. **Tool/MCP traffic is under-instrumented.**
6. **Eval data lives in a different pipeline from traces.**
7. **Prompt content leaks are still routinely discovered post-facto.**
8. **Cardinality explosions cripple metric-store backends.**
9. **Hybrid trace correlation across CLI ↔ cloud boundaries is unreliable.**
10. **Silent agent failure modes (context exhaustion, drift) lack standard detection patterns.**

Mara directly addresses 1, 3, 4, 5, 7, 8. Mara contributes to 2 (via upstream OTel work), 6 (via canonical eval signal), 9 (via canonical trace_id propagation). 10 is open territory; Mara provides the data, the industry needs the patterns.

## Source citations (representative)

This list will be filled in continuously as the research subagent results are merged. As of M0 the citations are:

- OpenTelemetry gen-ai SIG meeting notes — <https://github.com/open-telemetry/community/tree/main/projects/gen-ai-observability>.
- OpenTelemetry semantic conventions (gen-ai) repository — <https://github.com/open-telemetry/semantic-conventions/tree/main/docs/gen-ai>.
- OpenInference specification — <https://github.com/Arize-ai/openinference>.
- Langfuse changelog and blog — <https://langfuse.com/changelog>.
- Helicone blog on cost observability — <https://helicone.ai/blog>.
- Honeycomb blog on LLM observability patterns — <https://honeycomb.io/blog>.
- Datadog blog on LLM observability — <https://datadog.com/blog/llm-observability>.
- Various Hacker News threads on LLM cost surprises (search "claude code cost" / "openai bill shock" Q1–Q2 2026).

Each pain-point write-up in this document will be backed with specific URL citations in PRs that flesh it out.
