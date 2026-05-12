# Quickstart — Kimi (Moonshot)

## Executive summary

Kimi CLI is a **Tier B** runtime. The official OTel exporter is in flux as of May 2026, so the v1 default integration is JSONL tail of `~/.kimi/logs/kimi.log` plus opportunistic ingest of `kimi export` ZIPs and (optionally) the `stream-json` print mode. When the OTel exporter stabilizes upstream, Mara will graduate Kimi to Tier A.

## Prerequisites

- Kimi CLI installed (`kimi --version`).
- Mara installed.
- A sink endpoint.

## Step 1 — Apply the Kimi preset

```bash
mara setup kimi
```

This writes a Mara config with:

- A JSONL tail on `~/.kimi/logs/kimi.log` (when debug mode is enabled in Kimi).
- A directory watcher on `~/.kimi/exports/` to ingest `kimi export` ZIPs.
- An optional `stream-json` parser for piped Kimi output.

## Step 2 — Configure your sink

(Same patterns as other quickstarts.)

## Step 3 — Start Mara

```bash
brew services start mara
```

## Step 4 — Enable Kimi's debug logging

Kimi only writes to `~/.kimi/logs/kimi.log` when the `--debug` flag is set or the relevant config option is enabled. Recommend enabling for any session you want captured:

```bash
alias kimi='kimi --debug'    # or add to ~/.zshrc
```

For the `stream-json` print mode capture (alternative — useful when you don't want the full debug log):

```bash
kimi --print-format stream-json "your prompt" | mara stream-json --runtime kimi
```

The `mara stream-json` subcommand reads JSONL from stdin and forwards to the running Mara agent. Useful for piped/wrapped invocations.

## Step 5 — Use Kimi as usual

```bash
kimi "summarize the README"
```

Or batch:

```bash
kimi export --session <id> --out ~/.kimi/exports/
# Mara's directory watcher picks up the ZIP and ingests context.jsonl + wire.jsonl
```

## What gets captured

From the debug log:

- Session lifecycle (with timestamps).
- Tool calls and results.
- Errors and warnings.
- Model usage when emitted (verbose mode).

From `kimi export` ZIPs:

- Full session context (post-hoc).
- Wire-level message history.

From `stream-json` print mode:

- Per-token streaming if you instrumented the pipe.
- Final completion + tool calls.

## Known gaps

- **Token usage**: implementation-dependent; not always emitted to the debug log. **PARTIAL** in compatibility matrix.
- **Cost**: Mara computes from token counts × Moonshot price table when token usage is present; otherwise **PARTIAL/GAP**.
- **OTel native**: not yet stable; tracked as upstream-dependent. When stable, Kimi becomes Tier A.

## ZDR considerations

- Moonshot-hosted traffic (when using cloud API mode) is subject to Moonshot's data-handling policy.
- For on-device / local-model mode, no vendor egress; Mara captures purely local activity.
- `capture_optin` in Mara policy controls whether prompt body is recorded in canonical events.

## Verify

```bash
mara diag
mara test pipeline --name primary --input ~/.kimi/logs/kimi.log --pretty
```

## Common pitfalls

- **`--debug` not set** — no debug log produced; Mara has nothing to tail. Default to setting it via alias or shell function.
- **Log rotation**: Kimi rotates `kimi.log` aggressively under heavy use; Mara's JSONL adapter handles rotation transparently.
- **Encoding**: Kimi's CLI may emit Chinese or other non-Latin characters; Mara is UTF-8 throughout. If you see mojibake, file an issue with a sample.
- **`kimi web` / `kimi acp` / `kimi wire` modes** are not integration transports for telemetry; Mara does not connect to those endpoints directly in v1.

## Upgrade path

When Moonshot ships stable OTel export for Kimi, the runtime preset will add an OTLP receive adapter and `mara setup kimi` will reconfigure automatically (with a one-line CHANGELOG entry). Existing JSONL-only captures continue working as redundant signal.

## Reference documents

- Kimi command reference: <https://moonshotai.github.io/kimi-cli/en/reference/kimi-command.html>.
- Kimi CLI index: <https://moonshotai.github.io/kimi-cli/>.
- Mara Kimi runtime preset: `crates/mara-runtimes/kimi/`.
- AI runtime telemetry surfaces (Kimi section): [`../01-landscape/08-ai-runtime-telemetry-surfaces.md`](../01-landscape/08-ai-runtime-telemetry-surfaces.md).
