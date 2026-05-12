# Quickstart — Ollama (local LLM runtime)

## Executive summary

Five-minute setup that captures every Ollama inference (native `/api/*` and OpenAI-compatible `/v1/*` endpoints) and ships it to your chosen sink with token counts, latency breakdown, tokens-per-second, model identifier, and Anthropic / OpenAI API keys redacted before any byte leaves your machine. Ollama runtime is a **proxy-mode integration**: Mara binds the conventional port `11434` and Ollama is reconfigured to listen on `11435`. Your AI clients (the `ollama` CLI, Open WebUI, Continue.dev, OpenAI-SDK code) keep pointing at `11434` and never know Mara is in the path.

## Prerequisites

- macOS (Apple Silicon) or Linux x86_64. Windows in MVP+1.
- [Ollama](https://ollama.com) installed (`ollama --version` ≥ 0.21).
- [Mara](https://github.com/ArdurAI/mara) installed (`mara --version` ≥ 0.2.0).
- A sink endpoint to ship to. For local testing, use the Docker Compose stack in [`../06-deployment-blueprints/07-docker-compose.md`](../06-deployment-blueprints/07-docker-compose.md).

## Step 1 — Move Ollama to port 11435

The conventional Ollama port is `11434`. We free it up for Mara by telling Ollama to listen on `11435`.

**macOS:**

```bash
sudo launchctl setenv OLLAMA_HOST '127.0.0.1:11435'
brew services restart ollama
```

**Linux (systemd user service):**

```bash
mkdir -p ~/.config/systemd/user/ollama.service.d
cat > ~/.config/systemd/user/ollama.service.d/override.conf <<'EOF'
[Service]
Environment="OLLAMA_HOST=127.0.0.1:11435"
EOF
systemctl --user daemon-reload
systemctl --user restart ollama
```

Verify Ollama is now on `11435`:

```bash
curl -s http://127.0.0.1:11435/api/version | jq
```

## Step 2 — Apply the Ollama preset to Mara

```bash
mara setup ollama
```

This writes a Mara config to the OS-appropriate location (`~/Library/Application Support/mara/mara.toml` on macOS, `~/.config/mara/mara.toml` on Linux) that:

- Binds the proxy adapter on `127.0.0.1:11434`.
- Forwards to Ollama on `127.0.0.1:11435`.
- Applies the built-in PII redaction pack.
- Writes a default sink target (you'll edit this in step 3).

## Step 3 — Configure your sink

Edit the generated config. Example for Honeycomb (OTLP):

```toml
[[sinks.otlp]]
name = "honeycomb"
endpoint = "https://api.honeycomb.io"
protocol = "http"
headers = { "x-honeycomb-team" = "${HONEYCOMB_API_KEY}" }
```

Example for Grafana Cloud Loki:

```toml
[[sinks.loki]]
name = "grafana_cloud"
url = "https://logs-prod-006.grafana.net/loki/api/v1/push"
auth = { type = "basic", username = "<user id>", password = "${GRAFANA_CLOUD_API_KEY}" }
labels = ["runtime", "event_kind"]
```

Set the env var for whichever you chose:

```bash
export HONEYCOMB_API_KEY=<your key>
# or
export GRAFANA_CLOUD_API_KEY=<your key>
```

## Step 4 — Start Mara

**macOS:**

```bash
brew services start mara
```

**Linux:**

```bash
systemctl --user enable --now mara
```

Verify Mara is on `11434`:

```bash
curl -s http://127.0.0.1:11434/api/version | jq
# Should return Ollama's version — Mara forwarded transparently.
```

## Step 5 — Use Ollama as normal

```bash
ollama run llama3.2 "Why is the sky blue?"
```

Within a couple of seconds, the call appears in your sink with:

- `gen_ai.system = "ollama"`.
- `gen_ai.request.model = "llama3.2"`.
- `gen_ai.usage.input_tokens` and `gen_ai.usage.output_tokens` populated.
- `mara.ollama.total_duration_ms`, `mara.ollama.eval_duration_ms`.
- `mara.ollama.tokens_per_sec` computed from response timings.
- `mara.cost.usd = 0`, `mara.cost.source = "local_inference"`, `mara.compute.is_local = true`.

Inspect Mara's view:

```bash
mara diag
curl -s http://127.0.0.1:9099/metrics | grep ollama
```

## What gets captured

Per-request, fields populated from Ollama's response body:

- **Identity:** model name, operation name (chat / completion / embeddings).
- **Token usage:** input and output tokens.
- **Latency:** total duration, model load duration, prompt eval duration, eval duration, all converted from nanoseconds to milliseconds.
- **Throughput:** computed tokens-per-second.
- **Local-inference flag:** so dashboards can distinguish local from cloud.
- **Prompt and completion content:** opt-in only via `mara.policy.capture_optin = true`. Hashes when opt-in is off.

## OpenAI-compat clients

If your code uses an OpenAI SDK pointed at Ollama (`base_url="http://127.0.0.1:11434/v1"`), it works identically; the proxy captures both API shapes. For streaming, ensure `include_usage: true` so the final SSE chunk carries usage fields.

## ZDR considerations

- Prompt and completion content are NOT captured by default. Mara emits SHA-256 hashes so downstream dedup and audit still work.
- To enable content capture: set `mara.policy.capture_optin = true` in the relevant pipeline policy.
- Local inference never leaves your machine (Mara on loopback + Ollama on loopback); the "ZDR" question is moot for the actual workload, but Mara's redaction still applies to the events Mara emits to your sink — if you have a sink that's a cloud service, prompts going through Mara's redaction packs is still your defense.

## Verify

```bash
mara test pipeline --name primary --pretty | head
```

Or watch live:

```bash
mara diag --watch
```

## Common pitfalls

- **`OLLAMA_HOST` not set in the right scope.** macOS `launchctl setenv` only affects services started after the command runs; restart Ollama (`brew services restart ollama`). The setting does not persist across reboots; for permanent: add to `~/.zshrc` and use `launchctl setenv` again at login, or use a launchd plist override.
- **Mara fails to bind 11434.** Ollama still on it. Re-run step 1 and confirm with `lsof -i :11434` that Mara is the listener.
- **Client hangs on streaming.** Streaming chunks are forwarded in real time; if you see buffering, file an issue with the client name and Mara version — proxy transparency is SC-9 of the MVP sign-off criteria.
- **Tokens-per-second is implausibly high or zero.** Check that `eval_duration` in the response body is non-zero (a stream that disconnected mid-response may have zero).
- **`mara.cost.usd` is missing.** For local inference, cost is intentionally zero; check `mara.cost.source = "local_inference"`. The presence of cost = 0 is the indicator, not the absence of the field.

## Self-telemetry

- Metrics: `http://127.0.0.1:9099/metrics`.
- Health: `http://127.0.0.1:9099/healthz`.
- Per-adapter labels include `adapter="ollama-proxy"`.

## Reference documents

- Ollama official docs: <https://docs.ollama.com>.
- Ollama API usage page (token + duration fields): <https://docs.ollama.com/api/usage>.
- Ollama OpenAI-compat endpoints: <https://docs.ollama.com/api/openai-compatibility>.
- Ollama FAQ (env vars, reverse proxy): <https://docs.ollama.com/faq>.
- Mara Ollama runtime preset: `crates/mara-runtime-ollama/`.
- Mara HTTP proxy adapter: `crates/mara-adapter-llm-proxy/`.
- Ollama integration design: [`../08-mvp/12-ollama-integration-design.md`](../08-mvp/12-ollama-integration-design.md).
- Compatibility matrix row: [`../05-evaluation/02-compatibility-matrix-spec.md`](../05-evaluation/02-compatibility-matrix-spec.md).
