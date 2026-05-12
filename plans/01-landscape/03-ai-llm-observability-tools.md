# AI / LLM / Agent Observability Tools

## Executive summary

This is the most important landscape category for Mara's positioning, because it contains the tools operators most often consider as alternatives or complements. As of May 2026, the AI/LLM observability market is fragmenting along three axes: (1) hosted SaaS dashboards with SDK auto-instrumentation, (2) open-source frameworks for tracing inside application code, (3) inference proxies that capture from the request path. None of them are agent-on-the-host tools; none of them target Claude Code / Codex / Cursor / Kimi / Augment as runtimes whose telemetry needs to be captured without owning the code. Mara fills that gap and complements every tool in the list as a feeder.

## Tools by category

### Open-source frameworks for app-code instrumentation

#### Langfuse

- **Description:** open-source LLM engineering platform — tracing, evaluations, prompt management.
- **Form factor:** self-hosted server + client SDKs (Python, JS, Go).
- **Signals captured:** prompts, completions, tool calls, costs, latencies, evals, scores, user feedback.
- **Integration:** SDK with framework wrappers (LangChain, LlamaIndex, OpenAI, Anthropic).
- **Data destination:** Langfuse server (self-host or SaaS).
- **License:** MIT (core); commercial SaaS at langfuse.com.
- **OTel `gen_ai.*` alignment:** partial; native OTel export from SDKs available.
- **Governance / PII:** server-side scrubbing, role-based access.
- **Mara relationship:** complementary. Mara can ship to Langfuse via OTLP. Mara captures from runtimes Langfuse SDKs don't reach.
- **Docs:** <https://langfuse.com/docs>.

#### Arize Phoenix + OpenInference

- **Description:** open-source LLM observability with a strong evals focus. OpenInference is the semconv layer Arize promotes; widely adopted in non-OTel SDKs.
- **Form factor:** self-hosted notebook/web app + client SDKs.
- **Signals captured:** spans, evals, RAG-specific signals (retrieval, query, response), feedback.
- **Integration:** SDK + OTLP receive (it's an OTLP-compatible backend).
- **Data destination:** Phoenix server (self-host) or Arize AX (commercial).
- **License:** Elastic License v2 (Phoenix); commercial for Arize AX.
- **OTel `gen_ai.*` alignment:** parallel namespace (`openinference.*`); active discussions with OTel about convergence.
- **Mara relationship:** complementary. Mara → Phoenix via OTLP works. Mara maps `gen_ai.*` ↔ `openinference.*`.
- **Docs:** <https://docs.arize.com/phoenix>, <https://github.com/Arize-ai/openinference>.

#### Traceloop / OpenLLMetry

- **Description:** auto-instrumentation library and observability service. OpenLLMetry is the OSS instrumentation layer that emits OTel-compatible spans.
- **Form factor:** SDK + hosted dashboard.
- **Signals:** OTel-compatible.
- **License:** Apache 2.0 (OpenLLMetry SDK); commercial dashboard.
- **OTel `gen_ai.*` alignment:** native.
- **Mara relationship:** complementary. Mara can ingest OpenLLMetry-emitted OTLP unchanged.
- **Docs:** <https://traceloop.com>, <https://github.com/traceloop/openllmetry>.

#### Inspect AI (UK AISI)

- **Description:** evaluation framework from UK AI Safety Institute.
- **Form factor:** library + CLI.
- **Signals:** eval-focused, not general telemetry.
- **License:** MIT.
- **Mara relationship:** complementary. Mara captures live agent activity; Inspect runs offline evals. Eval results can be ingested back into Mara via webhook.

### Hosted SaaS dashboards (proprietary)

#### LangSmith (LangChain Inc.)

- **Form factor:** hosted SaaS; SDK auto-instruments LangChain.
- **Signals:** prompts, completions, costs, latencies, evals, runs as trees.
- **License:** proprietary.
- **OTel alignment:** OTel export supported.
- **Mara relationship:** complementary; Mara ships to LangSmith via OTLP.
- **Docs:** <https://docs.smith.langchain.com>.

#### W&B Weave

- **Description:** Weights & Biases' LLM observability product.
- **Form factor:** hosted SaaS + Python SDK.
- **Signals:** traces, evals, prompts, completions.
- **License:** proprietary SaaS; open-source SDK.
- **OTel alignment:** partial.
- **Mara relationship:** complementary via OTLP / webhook.
- **Docs:** <https://docs.wandb.ai/guides/weave>.

#### Braintrust

- **Description:** evals + observability for LLM developers.
- **Form factor:** hosted SaaS + SDK.
- **Signals:** evals + traces.
- **License:** proprietary SaaS; open-source SDK.
- **Mara relationship:** complementary; Mara → Braintrust via SDK forwarding or OTLP.
- **Docs:** <https://www.braintrust.dev>.

#### Galileo

- **Description:** Galileo Observe + Evaluate.
- **Form factor:** hosted SaaS.
- **Signals:** observation, eval, RAG-specific.
- **License:** proprietary.
- **Mara relationship:** sink candidate.
- **Docs:** <https://www.galileo.ai>.

#### Comet Opik

- **Description:** Comet's LLM observability + evaluation product.
- **Form factor:** SaaS + open-source SDK.
- **License:** Apache 2.0 (Opik OSS); commercial Comet platform.
- **Mara relationship:** complementary.
- **Docs:** <https://www.comet.com/site/products/opik/>.

#### Patronus AI

- **Description:** LLM evaluation focus; observability adjacent.
- **Form factor:** hosted SaaS.
- **License:** proprietary.

#### LangWatch

- **Description:** LLM monitoring focused on European market.
- **Form factor:** SaaS + self-host option.
- **License:** dual license.

#### Confident AI / DeepEval

- **Description:** evaluation framework + hosted product.
- **License:** Apache 2.0 (DeepEval OSS); commercial Confident AI.
- **Mara relationship:** Mara emits events that DeepEval can consume for eval pipelines.
- **Docs:** <https://www.confident-ai.com>.

#### Logfire (Pydantic)

- **Description:** OTel-native observability product; especially good for Python.
- **License:** proprietary SaaS; open-source SDK.
- **OTel alignment:** native; first-class `gen_ai.*`.
- **Mara relationship:** complementary; OTLP sink works.
- **Docs:** <https://pydantic.dev/logfire>.

### Inference proxies / gateways

#### Helicone

- **Description:** proxy-based LLM observability + cost tracking. Optional self-host.
- **Form factor:** HTTP proxy (configure `OPENAI_API_BASE` to Helicone URL) + SaaS dashboard.
- **License:** Apache 2.0 (open source); commercial SaaS.
- **OTel alignment:** OTel export available.
- **Mara relationship:** orthogonal — Helicone sits in the request path; Mara sits in the agent path. Often used together.
- **Docs:** <https://docs.helicone.ai>.

#### LiteLLM (BerriAI)

- **Description:** Python library + standalone proxy for multi-vendor LLM calls; cost tracking + observability.
- **Form factor:** library + proxy server.
- **License:** MIT.
- **OTel alignment:** OTel exporter available.
- **Mara relationship:** orthogonal; LiteLLM proxy can emit OTLP to Mara.
- **Docs:** <https://docs.litellm.ai>.

#### Portkey AI Gateway

- **Description:** AI gateway for multi-vendor routing, fallback, caching, observability.
- **Form factor:** proxy + SaaS dashboard.
- **License:** proprietary SaaS; OSS gateway component (Apache 2.0).
- **Mara relationship:** orthogonal.
- **Docs:** <https://portkey.ai>.

### Major-vendor first-party observability

#### OpenAI Tracing / Dashboard

- OpenAI's developer console has request logs and basic analytics. Not exportable as a structured stream.
- **Mara relationship:** N/A — Mara doesn't talk to vendor consoles.

#### Anthropic Console observability

- Similar to OpenAI's: in-console view, no structured export.

#### Google Vertex AI experiments + logging

- Vertex AI has experiment logging integrated with Cloud Logging.
- **Mara relationship:** Cloud Logging is a sink (via OTel Collector → Cloud Logging exporter).

### Major-vendor general observability with LLM features

- **Datadog LLM Observability** — covered in [`02-observability-platforms.md`](02-observability-platforms.md).
- **New Relic AI Monitoring** — same.
- **Honeycomb AI features** — same.

### Guardrail products (governance-adjacent, not pure observability)

- **Lakera Guard, Pillar Security, Prompt Security, NVIDIA NeMo Guardrails, Meta Llama Guard** — runtime guardrails that decide whether a prompt or completion is allowed.
- **Mara relationship:** Mara can ingest guardrail decisions as canonical events and route them to audit logs / SIEM. Mara is not itself a guardrail.

## Where the LLM-obs market converges

Trends as of May 2026:

1. **OpenTelemetry `gen_ai.*` is the convergence point** — every serious player now exports or accepts OTel `gen_ai.*` events, even when they have their own native schema underneath.
2. **Evals + observability merge** — the line between "trace your prod traffic" and "run an eval on captured traffic" is fading.
3. **Sentry-style pricing models** (per-event with quotas) replace per-GB models for LLM traffic.
4. **Self-hosted re-emerges** — privacy and cost drive teams to self-host (Langfuse, Phoenix, Signoz, Hyperdx, Highlight).
5. **Cost is the killer feature** — every product leads with cost dashboards because operators are bleeding money on token bills.

## Gaps Mara uniquely fills

1. **Telemetry from AI runtimes whose code you don't own** — Claude Code, Codex, Cursor, Kimi, Augment, Gemini CLI.
2. **Edge-first, single-binary deployment** — no SaaS account required; no SDK to embed.
3. **WASM-sandboxed policy at the agent boundary** — redaction and routing before sink dispatch.
4. **Vendor-neutral by license and architecture** — Apache 2.0; any sink; CNCF-track governance.
5. **First-class MCP observability** — `mcp.*` semconv is first-class in our canonical schema.
6. **Audit-log-grade evidence pipeline** — tamper-evident, Merkle-rooted.

## Competitor vs complement vs both

For each tool above:

- **Competitor:** none, strictly. Mara has a different category seat.
- **Complementary sink:** Langfuse, Phoenix, LangSmith, Honeycomb, Datadog LLM Obs, New Relic AI, W&B Weave, Braintrust, Galileo, Opik, Logfire, Signoz, Hyperdx, Highlight, Helicone (when used as a sink, not as a proxy), Portkey.
- **Complementary orthogonal (request-path):** LiteLLM, Helicone proxy mode, Portkey, OpenRouter.
- **Complementary downstream consumer:** Inspect AI, DeepEval, Confident AI, Patronus, LangWatch.
- **Guardrail (orthogonal):** Lakera, Pillar, Prompt Security, NeMo Guardrails, Llama Guard.

Mara's value proposition is to deepen and unify the input layer that all of these tools depend on.

## References

- Langfuse: <https://langfuse.com>.
- Arize Phoenix: <https://phoenix.arize.com>, OpenInference spec: <https://github.com/Arize-ai/openinference>.
- LangSmith: <https://www.langchain.com/langsmith>.
- W&B Weave: <https://wandb.ai/site/solutions/weave>.
- Braintrust: <https://www.braintrust.dev>.
- Helicone: <https://helicone.ai>.
- Traceloop OpenLLMetry: <https://github.com/traceloop/openllmetry>.
- LiteLLM: <https://docs.litellm.ai>.
- Portkey AI: <https://portkey.ai>.
- Inspect AI: <https://github.com/UKGovernmentBEIS/inspect_ai>.
- DeepEval: <https://docs.confident-ai.com>.
- Galileo: <https://www.galileo.ai>.
- Logfire (Pydantic): <https://pydantic.dev/logfire>.
- Comet Opik: <https://github.com/comet-ml/opik>.
