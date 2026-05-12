# Schema Fragmentation in AI Observability

## Executive summary

Every vendor, every framework, and every observability tool has its own way of representing a "model call." This document catalogs the major schemas in use as of May 2026, where they diverge, and what Mara does to provide one canonical representation without inventing yet another. Mara's answer is to align with OpenTelemetry `gen_ai.*` semantic conventions and to map other schemas in or out as needed.

## The current zoo

### OpenTelemetry `gen_ai.*` semconv (OTel-led)

- **Status:** Development.
- **Governance:** OpenTelemetry Specification working group + gen-ai SIG.
- **Adoption:** Honeycomb, Grafana Cloud, Datadog (partial), New Relic (partial); first-party in Claude Code, Codex `[otel]`, Gemini CLI.
- **Strengths:** vendor-neutral, OTel-ecosystem compatible, growing fast.
- **Weaknesses:** still Development; breaking changes possible; MCP conventions just landed.
- **Authoritative source:** <https://opentelemetry.io/docs/specs/semconv/gen-ai/>.

### OpenInference (Arize-led)

- **Status:** v1 stable.
- **Governance:** Arize-led, community contributors.
- **Adoption:** Phoenix (native), Arize AX, several SDKs (`openinference-instrumentation-*`).
- **Strengths:** stable, comprehensive, eval-first orientation.
- **Weaknesses:** parallel to OTel `gen_ai.*`; not officially under OTel governance.
- **Authoritative source:** <https://github.com/Arize-ai/openinference>.

### Datadog LLM Observability schema

- **Status:** product-defined.
- **Governance:** Datadog.
- **Adoption:** Datadog only.
- **Strengths:** integrated with Datadog's existing APM model.
- **Weaknesses:** proprietary; exports to OTel possible via Datadog's standard exporter.

### Langfuse schema

- **Status:** product-defined, OSS.
- **Governance:** Langfuse Inc.
- **Adoption:** Langfuse hosted + self-hosted.
- **Strengths:** session-oriented, eval-friendly, RAG-friendly.
- **Weaknesses:** proprietary shape; OTel ingestion is supported but mapping is opinionated.

### LangSmith schema

- **Status:** product-defined.
- **Governance:** LangChain Inc.
- **Adoption:** LangSmith hosted, LangChain SDK auto-instrumentation.
- **Strengths:** rich tree structure, run-tree model.
- **Weaknesses:** proprietary; OTel export available but lossy.

### Helicone schema

- **Status:** product-defined.
- **Governance:** Helicone Inc.
- **Adoption:** Helicone proxy and SaaS.
- **Strengths:** proxy-derived, captures full request/response naturally.
- **Weaknesses:** proxy-shape doesn't fit agent shapes well.

### Vendor-direct schemas

- **OpenAI**: ChatCompletion / Responses API shape.
- **Anthropic**: Messages API shape.
- **Google Vertex AI**: GenerateContent shape.
- **AWS Bedrock**: InvokeModel + Converse API shapes.
- **Mistral / DeepSeek / Cohere / Groq / Together**: variants of OpenAI-compatible.

Each is a JSON shape that observability tools either parse or wrap.

### MCP attribute schema (recent)

- **Status:** Development in OTel semconv.
- **Governance:** OTel + MCP working group collaboration.
- **Adoption:** few implementations as of May 2026.
- **Strengths:** standardizes tool-call attribution.
- **Weaknesses:** new; few vendors emit it.
- **Authoritative source:** <https://github.com/open-telemetry/semantic-conventions/tree/main/docs/gen-ai> (MCP section).

## Where they diverge

### Token naming

- OTel: `gen_ai.usage.input_tokens`, `gen_ai.usage.output_tokens`.
- Anthropic raw: `input_tokens`, `output_tokens`.
- Google raw: `promptTokenCount`, `candidatesTokenCount`.
- OpenAI raw: `prompt_tokens`, `completion_tokens`.
- LangSmith: `prompt_tokens`, `completion_tokens` (matches OpenAI).
- OpenInference: `llm.token_count.prompt`, `llm.token_count.completion`.

Mapping is essential.

### Span / event hierarchy

- OTel: spans with attributes.
- OpenInference: spans with `openinference.span.kind`.
- Datadog LLM Obs: `dd_llm_obs.span_kind` plus standard spans.
- LangSmith: "runs" form a tree with `run_type`.
- Langfuse: "observations" form a session.

The names differ but the trees encode similar structure. Mara's canonical schema uses OTel spans and adds a `mara.event_kind` discriminator for non-span events (cost, error, system).

### Cost representation

- OTel: usage tokens + a separate cost metric.
- OpenInference: `llm.token_count.completion.cost`.
- Datadog: a `llm_obs.cost` attribute.
- Langfuse: an `usage.usd` field.

Mara's canonical: `gen_ai.usage.*` for tokens + `mara.cost.usd` for currency-normalized cost.

### Tool call representation

- OTel: `gen_ai.tool.call.*` and recent MCP attributes.
- OpenInference: `tool.*` namespace + `openinference.span.kind = "TOOL"`.
- LangSmith: `tool_calls` array on the run.

Mapping is mostly mechanical.

### Streaming token semantics

- OTel: a single span; cumulative `gen_ai.usage.*` at completion.
- OpenInference: same.
- LangSmith: each chunk emits a sub-span.
- Datadog: span events for chunks.

Mara's canonical: single span, cumulative usage, streaming flag.

## Mara's mapping strategy

### Inbound (adapters)

For each adapter, document the source schema and the mapping rule into canonical:

- OTLP receiver (`gen_ai.*`-aware): pass through; rename any deprecated keys.
- JSONL adapter (Claude Code transcripts): runtime-specific mapping in the runtime preset.
- JSONL adapter (Codex history): runtime-specific.
- Hooks adapter (Cursor): runtime-specific.
- Analytics REST (Augment): runtime-specific.
- OpenInference OTLP: map `openinference.*` → `gen_ai.*` where equivalent; preserve `openinference.*` unchanged for fields without equivalent.

### Outbound (sinks)

For each sink, document the canonical → wire mapping:

- OTLP sink: canonical → OTLP `gen_ai.*` directly.
- Loki sink: labels = `runtime`, `event_kind`; structured metadata for high-cardinality.
- Splunk sink: `event` is the canonical JSON; sourcetype `mara:gen_ai`; fields list documented.
- Elasticsearch sink: index template `mara-gen_ai-YYYY.MM.DD`; mapping published.
- Object store sink: Parquet schema published; columnar layout for analytics.

### Round-trip guarantees

Where a sink supports the canonical schema natively (OTLP), round-trip is lossless and tested.

Where a sink imposes structural changes (Loki labels, Parquet columns), round-trip is best-effort within sink constraints; the canonical event is the source of truth, and the sink-specific mapping is documented per crate.

## Why not invent a Mara-original schema?

- Existing standards have momentum (`gen_ai.*` is in CNCF orbit; OpenInference has Arize behind it).
- Operators want portability — what they capture today should be usable in tomorrow's backend.
- The OTel semconv working group is the venue for resolving disagreements; Mara contributes there.

The `mara.*` namespace exists precisely so we can add what's missing without forking the standards.

## What Mara contributes back

When `mara.*` extensions prove their worth (`mara.session.id`, `mara.turn.id`, `mara.cost.usd`, `mara.policy.decisions`), we propose them upstream to the OTel gen-ai SIG. Successful proposals graduate into `gen_ai.*` or a sibling namespace; the `mara.*` extension is then deprecated with a documented migration window.

Tracked upstream PRs and proposals are listed in `docs/upstream-contributions.md`.

## Migration considerations for adopters

Adopters with existing Datadog/Langfuse/LangSmith data who switch to Mara:

- Mara can ingest OTLP that existing tools export; previous data isn't lost.
- Mara emits OTLP to those tools; existing dashboards continue to work.
- For shape-specific dashboards (e.g., Datadog LLM Obs), an extra mapping layer at the sink (a sink-specific transform plugin) may be needed.

## Future state

We expect by end of 2026:

- OTel `gen_ai.*` reaches `Stable` status for the core attributes.
- `mcp.*` semconv stabilizes alongside.
- OpenInference and OTel align further or merge.
- Vendor-direct schemas remain but auto-instrumentation libraries normalize them to OTel.

Mara's strategy survives any of those outcomes because canonical = OTel + `mara.*`, with mappers in/out for anything else.

## Open questions

- Who owns the `mara.*` namespace registry? Initially this document; long-term, a public schema repo.
- How aggressively do we deprecate `mara.*` fields after they graduate upstream? Recommended: announce immediately on graduation, deprecate in next major, remove in major+1.
- Should we publish a `gen_ai.*`-compliance scorecard for sink backends? Likely yes; that's a v1.x project.
