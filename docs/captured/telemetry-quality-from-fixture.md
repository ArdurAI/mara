# Telemetry quality (fill-rate)

- **Source:** `ollama-proxy-smoke-sample.jsonl`
- **Rows:** 3

| Field | Present | Fill rate |
|-------|--------:|----------:|
| `resource.service_name` | 0 / 3 | 0.0% |
| `resource.service_version` | 0 / 3 | 0.0% |
| `resource.host_name` | 3 / 3 | 100.0% |
| `resource.process_pid` | 3 / 3 | 100.0% |
| `gen_ai.operation_name` | 2 / 3 | 66.7% |
| `gen_ai.conversation_id` | 0 / 3 | 0.0% |
| `gen_ai.request.model` | 2 / 3 | 66.7% |
| `gen_ai.response.model` | 2 / 3 | 66.7% |
| `gen_ai.usage.input_tokens` | 2 / 3 | 66.7% |
| `gen_ai.usage.output_tokens` | 2 / 3 | 66.7% |
| `mara.session_id` | 3 / 3 | 100.0% |
| `mara.turn_id` | 0 / 3 | 0.0% |

*Present* means non-null, non-empty string, or any number/bool/list/dict with content.
