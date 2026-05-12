# Quickstart — Gemini CLI

## Executive summary

`gemini-cli` is the easiest of the six runtimes to integrate. It is **Tier A**: it ships first-class OpenTelemetry support via `.gemini/settings.json` plus `GEMINI_TELEMETRY_*` env vars, emitting both `gen_ai.*` attributes and a rich `gemini_cli.*` namespace. Mara's preset configures Gemini to point at Mara's local OTLP receiver and normalizes the events.

## Prerequisites

- `gemini-cli` installed (`gemini --version`).
- Mara installed.
- A sink endpoint.

## Step 1 — Apply the Gemini preset

```bash
mara setup gemini
```

This generates a Mara config with the OTLP receiver listening on `127.0.0.1:4317`. It also writes a default `.gemini/settings.json` snippet you can merge into your config.

## Step 2 — Configure your sink

(Same patterns as other quickstarts.)

## Step 3 — Start Mara

```bash
brew services start mara
```

## Step 4 — Configure Gemini's telemetry

Edit your Gemini settings file (`~/.gemini/settings.json` or per-project `.gemini/settings.json`):

```json
{
  "telemetry": {
    "enabled": true,
    "target": "otlp",
    "otlpEndpoint": "http://127.0.0.1:4317",
    "otlpProtocol": "grpc",
    "logPrompts": false
  }
}
```

Or via env vars:

```bash
export GEMINI_TELEMETRY_ENABLED=true
export GEMINI_TELEMETRY_OTLP_ENDPOINT=http://127.0.0.1:4317
export GEMINI_TELEMETRY_TRACES_ENABLED=true
# export GEMINI_TELEMETRY_LOG_PROMPTS=true   # opt-in only
```

## Step 5 — Use `gemini-cli`

```bash
gemini "explain this Cargo.toml" -p Cargo.toml
```

Events arrive at Mara's OTLP receiver, normalize, and ship to your sink.

## What gets captured

- Session lifecycle (`session.start`, `session.end`).
- Per-turn prompts and completions (subject to `logPrompts`).
- Tool calls and results — `gemini-cli` exposes these in `gen_ai.tool.*` attributes.
- Token usage — `gen_ai.usage.input_tokens`, `gen_ai.usage.output_tokens`.
- Cost — when `gemini-cli` includes it or Mara computes from a price table.
- Per-turn latency.
- Errors.

The full set of `gemini_cli.*` attributes (per Gemini docs) is preserved under the `attributes.*` bag and surfaced via the OTLP receiver.

## File-based telemetry alternative

If you prefer file output instead of OTLP:

```json
{
  "telemetry": {
    "enabled": true,
    "target": "file",
    "outfile": "${HOME}/.gemini/telemetry.log"
  }
}
```

Then point Mara at the file:

```toml
[[adapters.jsonl]]
name = "gemini_outfile"
globs = ["${HOME}/.gemini/telemetry.log*"]
checkpoint_path = "${XDG_STATE_HOME}/mara/checkpoints/gemini"
```

OTLP is simpler and lower-latency; the file path is useful for offline or air-gapped setups.

## ZDR considerations

- `logPrompts: false` keeps prompt content out of telemetry. Default for Mara's preset.
- To capture prompts: set `logPrompts: true` in Gemini config **and** `capture_optin = true` in Mara policy.
- Mara honors `OTEL_LOG_USER_PROMPTS` if Gemini reads it (verify against your Gemini version).

## Verify

```bash
mara diag
curl -s http://127.0.0.1:9099/metrics | grep gemini
```

Run a Gemini command and watch Mara's metrics tick up:

```bash
gemini "what's 2 + 2?"
sleep 2
curl -s http://127.0.0.1:9099/metrics | grep mara_pipeline_events_total
```

## Common pitfalls

- **Telemetry disabled by default** — Gemini ships with telemetry off; the most common reason no events appear is that the user didn't enable it.
- **Different settings file precedence** — project `.gemini/settings.json` overrides home; check which one applies.
- **`logPrompts` with prompts containing secrets** — pair with Mara's PII pack which redacts API keys and tokens before sink dispatch.
- **OTLP gRPC vs HTTP** — `otlpProtocol: "http"` works against `127.0.0.1:4318` instead of `4317`.

## Use case beyond CLI

Gemini API consumers (apps that call Gemini's HTTP API directly) typically use the OTel SDK for their language. Mara receives that OTLP the same way it receives Gemini CLI's OTLP — the preset's OTLP receiver doesn't care which client emitted the event.

## Reference documents

- Gemini CLI telemetry: <https://geminicli.com/docs/cli/telemetry/>.
- Gemini CLI telemetry (GitHub): <https://github.com/google-gemini/gemini-cli/blob/main/docs/cli/telemetry.md>.
- Mara Gemini runtime preset: `crates/mara-runtimes/gemini/`.
- AI runtime telemetry surfaces (Gemini section): [`../01-landscape/08-ai-runtime-telemetry-surfaces.md`](../01-landscape/08-ai-runtime-telemetry-surfaces.md).
