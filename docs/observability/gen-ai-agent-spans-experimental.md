# GenAI agent spans (experimental OTel alignment)

OpenTelemetry GenAI semantic conventions are evolving. **Agent-level spans** (`gen_ai.agent.*` and related) may be marked **experimental** in a given semconv release.

## `OTEL_SEMCONV_STABILITY_OPT_IN`

OTel language SDKs commonly honor `OTEL_SEMCONV_STABILITY_OPT_IN` (e.g. value `gen_ai`) to emit attributes that are not yet stable. Mara’s schema (`mara-schema`) can carry agent-related fields (`gen_ai.agent`, `mara.*` agent helpers) when upstream OTLP or normalizers populate them.

**Mara guidance**

1. Pin semconv via `docs/semconv.lock` and the repo drift checks (M2-17).
2. When integrating an exporter/SDK that supports agent spans, set `OTEL_SEMCONV_STABILITY_OPT_IN` per vendor docs and verify emitted payloads against a scratch pipeline + `mara validate`.
3. Treat experimental fields as **optional** in dashboards: cardinality and naming may change on semconv bumps.

No separate Mara feature flag is required beyond what your OTLP exporter already exposes.
