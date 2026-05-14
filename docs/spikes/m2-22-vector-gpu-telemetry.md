# Spike: vector DB / GPU telemetry (M2-22)

## Question

Should Mara grow **OpenLIT-style** breadth (vector DB health, GPU counters) as first-class adapters, or stay focused on LLM/proxy + OTLP ingestion?

## Findings (time-boxed)

- Vector-store metrics are highly vendor-specific (Milvus, Qdrant, pgvector) and change release-to-release; a dedicated scraper or OTel receiver per vendor fits better than bloating `llm-proxy`.
- GPU telemetry is already well-served by DCGM / node exporters; duplicating that inside Mara adds operational overlap.
- Mara’s differentiation remains **canonical GenAI events + policy + multi-runtime**; breadth signals should attach via **OTLP adapters** or sidecar exporters, not the HTTP LLM proxy hot path.

## Recommendation

Defer a native “vector adapter” until a concrete customer requires unified **LLM + retrieval** traces in one schema. Until then, document sidecar export (Prometheus textfile, OTel Collector `prometheus` receiver) and keep Mara’s core surface small.
