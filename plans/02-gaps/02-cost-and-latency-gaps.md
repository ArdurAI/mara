# Cost and Latency Observability Gaps

## Executive summary

Cost and latency are the two metrics every AI workload operator wants. As of May 2026, neither is easy to observe correctly across vendors, runtimes, or workload patterns. This document catalogs the specific gaps — vendor-by-vendor token accounting differences, prefix/reasoning/cached token complexities, TTFT vs total latency, tool-call latency, queue time, retry chains — and documents what Mara does about each.

## Cost gaps

### Gap 1.1 — Token-accounting nomenclature is inconsistent

- **OpenAI:** `prompt_tokens`, `completion_tokens`, `cached_tokens` (prefix-cached input), `reasoning_tokens` (o1/o3 family). Total = sum.
- **Anthropic:** `input_tokens`, `output_tokens`, `cache_creation_input_tokens`, `cache_read_input_tokens`. Total = sum; cache_read is billed at 10% of normal input.
- **Google Vertex AI / Gemini:** `promptTokenCount`, `candidatesTokenCount`, `totalTokenCount`. Cached input via context caching API has its own accounting.
- **Mistral:** `prompt_tokens`, `completion_tokens`, `total_tokens`.
- **DeepSeek:** `prompt_tokens`, `completion_tokens`, `prompt_cache_hit_tokens`, `prompt_cache_miss_tokens`, `reasoning_tokens` for R1.
- **AWS Bedrock:** wraps the underlying model's structure; `inputTokenCount`, `outputTokenCount`, `cacheReadInputTokenCount`, `cacheWriteInputTokenCount`.
- **Azure OpenAI:** mostly OpenAI-shape with subtle differences in some preview APIs.

**Mara approach:** the canonical schema uses OTel `gen_ai.usage.*` keys: `input_tokens`, `output_tokens`, `cached_tokens`, `reasoning_tokens`, plus `mara.usage.cache_creation_tokens` extension. Adapter-side normalizers handle the per-vendor translation.

### Gap 1.2 — Cost per token varies by model, region, and tier

Vendors update prices regularly. Pricing across regions can vary (Azure regional pricing, Vertex AI multi-region). Volume tiers, enterprise contracts, prompt-caching discounts, batch-API discounts (50% off for OpenAI Batch API) all affect realized cost.

**Mara approach:** a price-table feature in policy configuration:

```toml
[[policies.cost_compute]]
type = "compute_cost"
table = "@file:/etc/mara/price-tables/2026-05.toml"
output_attribute = "mara.cost.usd"
```

The price table is versioned and signed; updates roll out with policy bundle releases. When the vendor already emits cost (Bedrock per-invocation cost, OpenAI batch-API cost summaries), Mara prefers vendor cost and records `mara.cost.source = "vendor"`.

### Gap 1.3 — Cached tokens are usually invisible in dashboards

Prompt caching can drop cost 90% for repeated prompts. But dashboards graph `input_tokens` as a single line, mixing cached and uncached, so the saving is invisible. Operators miss optimization opportunities.

**Mara approach:** canonical schema preserves `gen_ai.usage.cached_tokens` separately. Default Grafana dashboard breaks cost into uncached vs cached components.

### Gap 1.4 — Cost per user / team / feature / request is manual

Vendor SDKs emit a span per call. Mapping that to "this Slack feature cost the team $X today" requires custom attributes that aren't always set, plus a backend that can sum by attribute.

**Mara approach:** documented `mara.tenant.id`, `mara.feature`, `mara.project` extensions; Helm chart's default sample dashboard breaks cost by these attributes when present.

### Gap 1.5 — Local agents have no vendor-emitted cost

When you run Claude Code or Cursor on your laptop, the vendor's API bill is paid directly by the developer or by org SSO. Local agents emit usage but not cost. Aggregating spend across many developers requires multi-account billing visibility.

**Mara approach:** Mara computes cost agent-side from usage × price table; aggregation happens in the operator's chosen sink. This is one of Mara's clear differentiators.

### Gap 1.6 — Batch and async APIs delay cost visibility

OpenAI Batch, Anthropic Message Batches return cost when the batch completes (up to 24h later). Most observability tools lose the link between batch submission and batch completion.

**Mara approach:** batch lifecycle events carry `gen_ai.operation.name = "batch_*"`; cost is updated by a `cost` event when the batch completes. Operator's sink joins by batch id.

### Gap 1.7 — Multi-vendor models with the same name

"claude-3-5-sonnet" might run on Anthropic Direct, Bedrock, Vertex AI, or via a router like LiteLLM. Cost varies. The model identifier alone doesn't disambiguate.

**Mara approach:** canonical schema includes both `gen_ai.system` (e.g., `"anthropic"`, `"bedrock"`, `"vertex_ai"`) and `gen_ai.request.model`; price tables key on the pair.

## Latency gaps

### Gap 2.1 — TTFT vs total latency

Time-to-first-token (TTFT) is user-perceived latency. Total turn time is backend-observed latency. Streaming responses change the meaning of "latency" entirely.

**Mara approach:** canonical schema has `gen_ai.latency.ttft_ms` and `gen_ai.latency.total_ms`; adapters populate both when the source provides them.

### Gap 2.2 — Inter-token latency / streaming throughput

For streaming responses, the cadence of tokens matters. A model that emits 100 tokens in 1s feels different from one that emits 100 tokens in 10s. Few observability tools measure tokens-per-second.

**Mara approach:** `mara.latency.tokens_per_sec` extension when computable. Histograms in default dashboards.

### Gap 2.3 — Queue time / rate limit waits

Vendors throttle. SDKs retry with backoff. The user-perceived wait can include minutes of queue time invisible to most spans.

**Mara approach:** retry chains emit one event per retry, plus a final event with `mara.latency.queue_ms` = sum of retry waits. Compatibility-matrix-dependent.

### Gap 2.4 — Tool-call latency inside agent turns

A "turn" can be 30s because the model itself was slow, or because a tool was slow, or because three tools fired sequentially. Without proper span structure, you can't tell.

**Mara approach:** preserve parent-child span structure when emitted; build the agent-loop view from canonical events.

### Gap 2.5 — Cold start of self-hosted inference

vLLM / SGLang / TGI / Ollama have cold-start latency on first request after spin-up. Per-request latency hides this unless you graph p99 of "first request after cold start."

**Mara approach:** `mara.inference.cold_start = true` attribute when detected via heuristics. Operators add an aggregation in the sink.

### Gap 2.6 — Provider-side latency variability

Vendor APIs have multi-second p99 latency tails that vary by region and time of day. Operators discover this only via outage tickets unless they graph vendor-side latency separately.

**Mara approach:** vendor-side latency captured as `gen_ai.latency.vendor_ms`; client-side total latency captured as `gen_ai.latency.client_ms`. Difference is network + client overhead.

### Gap 2.7 — Multi-modal call latency

Vision and audio calls have different latency profiles from text. Few tools split.

**Mara approach:** `gen_ai.operation.name` plus `gen_ai.request.modality` (extension) for splitting.

## Cross-cutting observations

### Aggregation usually happens in the wrong place

Vendor SDKs sometimes pre-aggregate before emitting (e.g., LangChain's `BatchSpanProcessor` defaults). Loss of per-call detail.

**Mara approach:** documented SDK configuration recommendations in each runtime's quickstart that preserve detail.

### "Cost" attribute is sometimes string, sometimes number

JSON cost-as-string in some vendors' webhooks. Floating-point precision issues across systems.

**Mara approach:** canonical schema types `mara.cost.usd` as `f64`; coerces from string-of-number on ingest with a `mara.cost.coerced` flag if conversion was needed.

### Currency

USD assumed throughout v1. For non-USD billing (Yandex, some Asia-Pacific contracts), v1.x adds `gen_ai.cost.currency`.

## Reference implementations to study

- **LiteLLM cost tracker:** open source, normalizes pricing across many vendors. <https://github.com/BerriAI/litellm>.
- **Helicone cost dashboard:** good example of TTFT + total + cached split. <https://docs.helicone.ai/use-cases/cost-tracking>.
- **OpenLLMetry cost spans:** OTel-compatible cost attributes. <https://github.com/traceloop/openllmetry>.
- **OpenTelemetry gen-ai semconv usage metrics:** <https://github.com/open-telemetry/semantic-conventions/blob/main/docs/gen-ai/gen-ai-metrics.md>.

## What Mara won't do

- Mara is not a billing system; it doesn't produce invoices.
- Mara is not a forecasting tool; cost prediction is the operator's analytics layer.
- Mara doesn't enforce budgets at inference time; that's a guardrail/proxy concern (LiteLLM, Portkey, Helicone).

## Test fixtures

The `tests/external/` folder contains recorded sessions with redacted prompts but full cost / latency attribute payloads. These exercise the canonical-schema normalization across vendor surface area.

## Open questions

- How much of vendor-specific pricing logic do we want in the codebase vs. in policy bundles? Current direction: minimal in code, rich in bundles.
- Should Mara emit a derived "estimated cost if no caching" metric to make caching ROI visible? TBD as v1.x feature.
- How do we keep price tables fresh? Plan: a community-maintained `mara-prices` repo, signed releases, bundle distribution via OCI registry.
