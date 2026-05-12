# Quickstart — Augment Code

## Executive summary

Augment Code is the **hardest of the six runtimes** to integrate with as of May 2026: it ships no local transcript path, no hooks API, and no OTLP exporter for the IDE agent loop. The only first-party programmatic surface is the **Augment Analytics REST API** (preview), available to enterprise customers. Mara's v1 integration is **Tier C** — a polling adapter against the Analytics REST API. It is best-effort and the compatibility matrix labels it as such.

If Augment ships hooks or OTLP in the future, Mara will add those adapters and graduate to Tier B or A.

## Prerequisites

- An Augment Code Enterprise account with access to the Analytics API (preview).
- An Augment Analytics API key (from your org's Augment admin).
- Mara installed.
- A sink endpoint.

## Step 1 — Apply the Augment preset

```bash
mara setup augment
```

This writes a Mara config with the analytics adapter configured but not enabled (you must supply credentials).

## Step 2 — Provide Augment credentials

Edit the generated config:

```toml
[[adapters.analytics]]
name = "augment_analytics"
runtime = "augment"
endpoint = "https://api.augmentcode.com/v1/analytics/events"
auth = { type = "bearer", token = "@file:/etc/mara/secrets/augment.txt" }
poll_interval = "60s"
dedupe_key = "event_id"
cursor_state = "/var/lib/mara/state/augment_cursor"
backoff = { base = "5s", max = "5m", jitter = 0.2 }
```

Place the API key (one line) at `/etc/mara/secrets/augment.txt` with mode `0600` owned by the Mara user.

## Step 3 — Configure your sink

(Same patterns as other quickstarts.)

## Step 4 — Start Mara

```bash
brew services start mara
```

Mara begins polling Augment's Analytics API on the configured interval, normalizes events into canonical form, and ships them.

## What gets captured

Subject to what Augment Analytics chooses to expose (this is a vendor-controlled surface):

- Session / conversation lifecycle.
- Aggregate token usage (sometimes per-event, sometimes summarized).
- Error counts.
- Tool / completion counts.

Mara documents the fields that the Analytics API returns at the time of the v1 release. As Augment adds fields, Mara's normalizer maps them into the canonical schema where possible.

## Known gaps

- **Prompt and completion content**: not available through Analytics REST. **GAP**.
- **Per-event tool calls**: not always granular. **PARTIAL or GAP**.
- **Real-time**: minimum polling latency = `poll_interval`. Default 60s; you can set lower at the cost of API rate limits. The Augment Analytics API is **not** real-time.
- **Per-user attribution**: only if Augment's API includes user identifier per event.
- **MCP traffic**: not exposed.
- **Token-level latency / TTFT**: not exposed.

The compatibility matrix marks Augment as **best-effort v1**.

## ZDR / privacy

- All Augment-side data handling follows Augment's own enterprise privacy controls.
- Mara's analytics adapter does not send Augment's data back to Augment; it pulls and forwards to the operator's chosen sink.
- The Mara `capture_optin` flag is moot for Augment because no prompt content is available to capture.

## Network considerations

The Analytics adapter polls `api.augmentcode.com`. Allow outbound HTTPS to that host in any restrictive network policy. The vendor's telemetry host (`evs.grdt.augmentcode.com`) is a separate concern; it is the destination of Augment's client-side analytics phone-home, not something Mara reads.

## Verify

```bash
mara diag
# Look for: analytics adapter "augment_analytics" — polling at 60s — last cursor: 2026-05-12T17:30:00Z
```

```bash
mara test pipeline --name primary --pretty | head
```

## Common pitfalls

- **API key without analytics scope** — your Augment admin must grant the analytics read scope.
- **Rate limiting** — at poll intervals < 30s, expect 429 responses. Mara backs off; tune `backoff.base`.
- **Cursor desync** after a config change — delete `cursor_state` to start over (you may receive duplicate events that the `dedupe_key` will then handle).
- **Augment policy of treating Analytics as preview** — endpoint stability is not contractual. Pin to a specific Augment org plan.

## Upgrade path

If/when Augment ships:

- A hooks API → Mara adds a hooks adapter; preset reconfigures to combine analytics REST (for historical) + hooks (for live).
- An OTLP exporter → Mara graduates Augment to Tier A.

These would arrive via a `mara setup augment` update in a future release.

## Reference documents

- Augment Analytics overview: <https://docs.augmentcode.com/analytics/overview>.
- Augment Analytics API: <https://docs.augmentcode.com/analytics/analytics-api>.
- Augment network config: <https://docs.augmentcode.com/setup-augment/network-configuration>.
- Mara Augment runtime preset: `crates/mara-runtimes/augment/`.
- AI runtime telemetry surfaces (Augment section): [`../01-landscape/08-ai-runtime-telemetry-surfaces.md`](../01-landscape/08-ai-runtime-telemetry-surfaces.md).
