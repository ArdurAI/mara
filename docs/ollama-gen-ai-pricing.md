# Ollama proxy: `gen_ai` cost estimates (`mara.cost_usd`)

Mara can fill **`mara.cost_usd`** on Ollama LLM-proxy completion events using configured **USD per 1M tokens** (input vs output). This is an **estimate**, not vendor billing data.

## Configuration

Under **`[server.gen_ai_pricing]`** in `mara.toml`:

| Field | Meaning |
| --- | --- |
| `estimate_enabled` | When `false` (default), `mara.cost_usd` is set to **`0.0`** and `mara.cost_source` stays **`mara_estimated`** (legacy placeholder). |
| `default_input_per_million_usd` / `default_output_per_million_usd` | Rates used when no prefix row matches (defaults **0.25** / **1.0** if omitted). |
| `[[server.gen_ai_pricing.models]]` | Optional rows: `prefix`, `input_per_million_usd`, `output_per_million_usd`. The **longest** `starts_with` match on the effective model id wins. |

Effective model id: **`gen_ai.request.model`**, else **`gen_ai.response.model`**, else empty string (defaults apply).

## Formula

`cost_usd = (input_tokens / 1_000_000) * r_in + (output_tokens / 1_000_000) * r_out`

Missing token fields are treated as **0**. If both are zero, cost is **0**.

## Assumptions and limits

- Token counts come from Ollama / OpenAI-compat response bodies only; no tokenizer replay.
- Single flat rate per direction; no cache-discount modeling beyond whatever the upstream puts in usage fields.
- Prefix table is operator-maintained; wrong prefixes fall back to defaults.
- **Not** a substitute for provider invoices or internal chargeback systems.
