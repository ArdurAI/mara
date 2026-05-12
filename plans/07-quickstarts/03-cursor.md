# Quickstart — Cursor Agents

## Executive summary

Cursor is the trickiest of the six runtimes: it ships **no transcript file and no OTLP exporter** as of May 2026. Its only first-party programmatic observability surface is the **Hooks** mechanism (JSON over stdio). Mara configures Cursor to invoke `mara cursor-hook` as its hook handler, which forwards events into Mara's hooks adapter over a local Unix socket.

This is Mara's **Tier B** integration.

## Prerequisites

- Cursor installed (any recent version with Hooks support).
- Mara installed.
- A sink endpoint.

## Step 1 — Apply the Cursor preset

```bash
mara setup cursor
```

This generates a Mara config with a hooks adapter listening on a Unix socket at `${XDG_STATE_HOME}/mara/cursor_hooks.sock`.

## Step 2 — Configure your sink

(Same patterns as the Claude Code quickstart.)

## Step 3 — Start Mara

```bash
brew services start mara   # macOS
```

## Step 4 — Wire Cursor's hooks to Mara

Cursor's hook configuration lives in your Cursor settings (per-workspace or per-user) under the `cursor.hooks` JSON. Add Mara as a hook handler for Agent and Cmd+K events:

```json
{
  "cursor.hooks": {
    "Agent": {
      "PostMessage":  { "command": "mara cursor-hook --event agent.post_message" },
      "PreToolCall":  { "command": "mara cursor-hook --event agent.pre_tool_call" },
      "PostToolCall": { "command": "mara cursor-hook --event agent.post_tool_call" }
    },
    "Cmd+K": {
      "PostEdit": { "command": "mara cursor-hook --event cmdk.post_edit" }
    }
  }
}
```

`mara cursor-hook` reads the JSON payload from stdin, attaches the `--event` label, and forwards it to the Mara agent's local socket.

## Step 5 — Use Cursor as usual

Open a project, use Cmd+K or the agent panel. Mara captures every hook firing.

## What gets captured

Mapped from Cursor's hook events:

- Agent message lifecycle (user message in, assistant message out).
- Tool call request and result.
- Cmd+K edit events.
- Session correlation via Cursor's session id (when included in payload).

## Known gaps

Cursor's enterprise audit log **explicitly excludes** agent responses and code content. The Hooks payload does include some of this content depending on event, but it is not exhaustive.

Specifically, as of May 2026:

- Token usage: not exposed by Cursor in hooks. **GAP** in compatibility matrix.
- Cost: not exposed. **GAP**.
- Model used: typically exposed in agent metadata. **PARTIAL**.
- Tool calls: exposed. **PASS**.
- Prompt/completion content: exposed via the hook payload, subject to Cursor's payload truncation. **PARTIAL**.

These gaps are documented in the compatibility matrix and tracked in `compat-gap/cursor/*` issues. They will close as Cursor's hook surface evolves.

## ZDR considerations

Cursor's enterprise privacy mode applies to vendor-side data handling. The hook payload Mara receives reflects what Cursor chose to include; Mara cannot extract what isn't there.

`capture_optin` in the Mara policy still gates whether prompt content (when present in the payload) is recorded in canonical events.

## Verify

```bash
mara diag
# Look for: hooks adapter "cursor_hooks" — accepting connections at <socket>
```

After using Cursor briefly:

```bash
curl -s http://127.0.0.1:9099/metrics | grep cursor
```

You should see incrementing event counts.

## Common pitfalls

- **`mara cursor-hook` not invoked** — Cursor caches hook configuration; restart Cursor after editing settings.
- **Socket permission denied** — Cursor and Mara run as different users; ensure both run as the same user (per-user installs only).
- **Hook command timeout** — Cursor times out hooks aggressively (sub-second). `mara cursor-hook` is designed to be near-instant (writes one line to a socket and exits); if you see timeout errors in Cursor logs, file an issue.
- **Long agent sessions losing structure** — without session ids in payloads, Mara generates UUIDs; if cross-event correlation matters, configure Cursor to emit session id when possible.

## Reference documents

- Cursor Hooks: <https://cursor.com/docs/hooks.md>.
- Cursor enterprise compliance & monitoring: <https://cursor.com/docs/enterprise/compliance-and-monitoring>.
- Mara Cursor runtime preset: `crates/mara-runtimes/cursor/`.
- AI runtime telemetry surfaces (Cursor section): [`../01-landscape/08-ai-runtime-telemetry-surfaces.md`](../01-landscape/08-ai-runtime-telemetry-surfaces.md).
