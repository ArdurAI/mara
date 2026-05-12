# Quickstart — Codex

## Executive summary

Five-minute setup to capture OpenAI Codex CLI activity into your sink. Codex is a **Tier A** runtime via its `[otel]` config block (gRPC/HTTP OTLP); we additionally tail `~/.codex/history.jsonl` and wire the experimental `notify` hook for turn-completion events.

## Prerequisites

- Codex CLI installed (`codex --version` ≥ 0.x with `[otel]` support).
- Mara installed.
- A sink endpoint.

## Step 1 — Apply the Codex preset

```bash
mara setup codex
```

This generates a Mara config with the OTLP receiver, a JSONL tail on `~/.codex/history.jsonl`, and a `notify`-hook adapter that listens on a Unix socket at `${XDG_STATE_HOME}/mara/codex_notify.sock`.

## Step 2 — Configure your sink

(Same patterns as the Claude Code quickstart.)

## Step 3 — Start Mara

```bash
brew services start mara   # macOS
# or
systemctl --user enable --now mara   # Linux
```

## Step 4 — Configure Codex to emit telemetry

Edit `~/.codex/config.toml`:

```toml
[otel]
exporter = "otlp"
otlp_endpoint = "http://127.0.0.1:4317"
otlp_protocol = "grpc"
log_user_prompt = false        # ZDR-strict default; flip per policy

[notify]
type = "exec"
command = ["mara", "codex-hook"]    # Mara CLI emits to its hooks adapter

[analytics]
enabled = false                # turn off vendor-side phone-home
```

The `[notify]` block tells Codex to invoke `mara codex-hook` on every `agent-turn-complete`. The hook forwards JSON to Mara's running hooks adapter.

## Step 5 — Run Codex

Interactive:

```bash
codex "fix the failing tests in this repo"
```

Or non-interactive (JSON event stream from Codex):

```bash
codex exec --json "refactor the data layer" | tee codex-events.jsonl
```

In the non-interactive mode, Mara captures via OTLP and via the JSONL stream (the stream you piped to `tee` is the same one `history.jsonl` records).

## What gets captured

- `thread.*` and `turn.*` lifecycle events.
- `item.*` events (each turn item: model, tool, etc.).
- Token usage, computed cost.
- Per-turn latency.
- Errors and retries.
- MCP tool activity (when Codex uses MCP).

## Codex `app-server` mode

For experimental local bridging (`codex app-server --listen ws://127.0.0.1:7100`), Mara can also be configured with a WebSocket source adapter (v1.x). v1 captures via OTLP + JSONL + notify-hook, which covers the same content.

## Codex MCP-server mode

If you use `codex mcp-server` to expose Codex as an MCP server to other tools, the MCP traffic is captured as part of Codex's normal `[otel]` emission (per Codex docs). Mara normalizes MCP attributes to `mcp.*` fields.

## ZDR considerations

- `log_user_prompt = false` in `[otel]` is the default. Prompt content is summarized as hashes; full content not exported.
- Mara additionally requires `capture_optin = true` to record prompt body in canonical events.
- `[analytics] enabled = false` turns off Codex's vendor-side analytics phone-home (separate from OTel).

## Verify

```bash
mara test pipeline --name primary --input ~/.codex/history.jsonl --pretty
```

Or watch live:

```bash
mara diag --watch
```

## Common pitfalls

- **`[hooks]` block needs feature flag** (`codex_hooks`). The `[notify]` block does not need a feature flag.
- **`codex exec` not seeing OTel config** if `CODEX_HOME` is non-default. Pin `CODEX_HOME` explicitly when running in CI.
- **`mara codex-hook` not on PATH** when Codex tries to invoke it. Ensure `mara` is in the user's `PATH` or use an absolute path in `[notify]`.

## Reference documents

- Codex advanced config / OTel + hooks: <https://developers.openai.com/codex/config-advanced>.
- Codex non-interactive / JSONL: <https://developers.openai.com/codex/noninteractive>.
- Mara Codex runtime preset: `crates/mara-runtimes/codex/`.
- AI runtime telemetry surfaces (Codex section): [`../01-landscape/08-ai-runtime-telemetry-surfaces.md`](../01-landscape/08-ai-runtime-telemetry-surfaces.md).
