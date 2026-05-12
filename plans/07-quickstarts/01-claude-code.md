# Quickstart — Claude Code

## Executive summary

Five-minute setup that captures Claude Code session telemetry into your chosen sink. Claude Code is a **Tier A** runtime: it ships a first-party OpenTelemetry exporter, so Mara primarily receives OTLP. We also configure JSONL tail of `~/.claude/projects/*.jsonl` as a redundant signal that works even when OTel is disabled (e.g., ZDR-strict environments).

## Prerequisites

- macOS, Linux, or Windows.
- Claude Code installed (`claude --version` ≥ 0.40).
- Mara installed (`mara --version` ≥ 1.0).
- A sink endpoint to receive events. For local testing, use the Docker Compose stack in [`../06-deployment-blueprints/07-docker-compose.md`](../06-deployment-blueprints/07-docker-compose.md).

## Step 1 — Apply the Claude Code preset

```bash
mara setup claude-code
```

This writes a sensible default to `~/Library/Application Support/mara/mara.toml` (macOS), `~/.config/mara/mara.toml` (Linux), or `%APPDATA%\Mara\mara.toml` (Windows).

The generated config has two adapters and a permissive sink (file rotation to disk). Open it and configure the sink for your environment.

## Step 2 — Configure your sink

Edit the generated config. Example for shipping to Grafana Cloud Loki:

```toml
[[sinks.loki]]
name = "grafana_cloud"
url = "https://logs-prod-006.grafana.net/loki/api/v1/push"
auth = { type = "basic", username = "<your user id>", password = "${GRAFANA_CLOUD_API_KEY}" }
labels = ["runtime", "event_kind"]
```

For Honeycomb via OTLP-HTTP:

```toml
[[sinks.otlp]]
name = "honeycomb"
endpoint = "https://api.honeycomb.io"
protocol = "http"
headers = { "x-honeycomb-team" = "${HONEYCOMB_API_KEY}" }
```

For an OTel Collector you already operate:

```toml
[[sinks.otlp]]
name = "my_collector"
endpoint = "http://otel-collector.internal:4317"
protocol = "grpc"
```

## Step 3 — Start Mara

macOS:

```bash
brew services start mara
```

Linux (per-user):

```bash
systemctl --user enable --now mara
```

Windows:

```powershell
Start-Service mara
```

Or simply `mara run` in a terminal for one-off testing.

## Step 4 — Enable Claude Code's OTLP export

Add to your shell profile (`~/.zshrc`, `~/.bashrc`, or PowerShell profile):

```bash
export CLAUDE_CODE_ENABLE_TELEMETRY=1
export OTEL_EXPORTER_OTLP_ENDPOINT=http://127.0.0.1:4317
export OTEL_EXPORTER_OTLP_PROTOCOL=grpc

# ZDR-aware capture toggles. Default off; opt in per your policy.
# export OTEL_LOG_USER_PROMPTS=true
# export OTEL_LOG_TOOL_DETAILS=true
# export OTEL_LOG_RAW_API_BODIES=true
```

Restart your shell or `source` the profile.

## Step 5 — Run a Claude Code session

```bash
claude "list this repo's commits"
```

Within a few seconds you should see events in your sink. Inspect Mara's view:

```bash
mara diag
# Or:
curl -s http://127.0.0.1:9099/metrics | grep mara_pipeline_events_total
```

## What gets captured

With OTel enabled (Tier A):

- Session start/end (`event_kind=system`).
- Per-turn prompts and completions (subject to `OTEL_LOG_USER_PROMPTS`).
- Tool calls and results (subject to `OTEL_LOG_TOOL_DETAILS`).
- Token usage and computed cost.
- Per-call latency and TTFT.
- Errors.
- MCP traffic when Claude Code uses MCP tools.

With JSONL tail active (Tier B redundant):

- Same content as OTLP, derived from session transcripts.
- Useful when OTel is disabled or for replay of old sessions.

## ZDR considerations

- Default config does **not** capture prompt or raw API body content. Hashes are stored instead, so dedup and audit still work.
- To enable prompt capture, set both:
  - `OTEL_LOG_USER_PROMPTS=true` (Claude Code-side opt-in), and
  - `capture_optin = true` in the relevant pipeline policy (Mara-side opt-in).
- Both gates must agree; either being false suppresses capture.

## Verify the compatibility-matrix promises

Run the smoke fixture:

```bash
mara test pipeline --name primary --input ~/.claude/projects/<a_session>.jsonl --pretty
```

You should see canonical events with `gen_ai.system="anthropic"`, `gen_ai.request.model`, `gen_ai.usage.*`, and `mara.source.runtime="claude_code"`. Each tool call should map to `gen_ai.tool.name` and the matching MCP `mcp.*` attributes when applicable.

## Common pitfalls

- **`OTEL_EXPORTER_OTLP_ENDPOINT` already set** for another tool. Use `claude --otel-endpoint http://127.0.0.1:4317` per-invocation, or use Claude Code-specific env vars where supported.
- **`CLAUDE_CODE_ENABLE_TELEMETRY=1` not set** — silent no-op for OTLP path. JSONL tail still works.
- **Permission denied on `~/.claude/projects/`** — Mara is running as a different user. Use per-user installation (LaunchAgent / systemd `--user`).
- **Sink credentials in shell history** — use a secrets file referenced as `@file:/etc/mara/secrets.toml` in config.

## Self-telemetry

- Metrics: `http://127.0.0.1:9099/metrics`.
- Health: `http://127.0.0.1:9099/healthz`.
- Mara self-logs go to stdout/stderr (or journald / Console.app / Event Log depending on OS).

## Reference documents

- Anthropic Claude Code observability: <https://code.claude.com/docs/en/agent-sdk/observability>.
- Mara Claude Code runtime preset: `crates/mara-runtimes/claude_code/`.
- Compatibility matrix row: [`../05-evaluation/02-compatibility-matrix-spec.md`](../05-evaluation/02-compatibility-matrix-spec.md).
- AI runtime telemetry surfaces (more detail on Claude Code): [`../01-landscape/08-ai-runtime-telemetry-surfaces.md`](../01-landscape/08-ai-runtime-telemetry-surfaces.md).
