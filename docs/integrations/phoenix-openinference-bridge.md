# Phoenix / OpenInference bridge (M2-18)

Mara emits **canonical JSON events** and OTLP logs with `gen_ai.*`, `mara.*`, and `http.*` style attributes (see `crates/mara-sink-otlp/src/encode.rs`). Arize Phoenix and OpenInference tooling typically expect OTLP traces/logs with GenAI semantic conventions.

## Practical mapping

| Mara / OTLP log field | Phoenix / Otel GenAI expectation |
|-----------------------|----------------------------------|
| `gen_ai.operation.name` (typed `gen_ai`) | `gen_ai.operation.name` |
| `gen_ai.usage.*` | token attributes |
| `mara.request_id` | correlate runs when `trace_id` absent |
| `mara.cost.usd` + `mara.cost.confidence` | cost overlays in dashboards |

## Golden sample

Use `examples/mara-phoenix-bridge.toml` with a file sink, run a short workload through the LLM proxy, then import `events.jsonl` into your collector → Phoenix. Adjust attribute translation in the collector if your Phoenix version expects different key casing.

No in-product Phoenix UI is bundled with Mara—**export-only** positioning (see M2-20).
