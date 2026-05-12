# MVP — FAQ and Troubleshooting

## Executive summary

The questions users will ask on day 1 of the launch, with honest answers, plus the failure modes we expect to see most often with concrete diagnostic steps. This document is operational — when a question gets posted to GitHub Discussions or a user is stuck, the responder should be able to point at a section of this doc rather than retype the answer. If the responder can't point at it, the question becomes a new entry.

Two sections: the FAQ (questions people will ask) and the troubleshooting playbook (failure modes and fixes).

## FAQ

### About the project

**Q: What is Mara?**

An open-source telemetry agent for AI tools — Claude Code, Codex, Cursor Agents, Kimi, Augment Code, Gemini CLI, and Ollama. It captures, normalizes, redacts, and ships AI activity to whatever observability backend you already use (Honeycomb, Grafana stack, Datadog, etc.). Apache 2.0, single Rust binary.

**Q: Is Mara a fork of [Fluent Bit / Vector / OTel Collector]?**

No. Zero source code shared. Same architectural pattern (input → policy → output), original implementation, specialized for AI runtimes. Details in [`09-differentiation-and-moat.md`](09-differentiation-and-moat.md).

**Q: Why not just use OTel Collector?**

For pure infrastructure observability, you should. For AI-runtime telemetry specifically, Mara ships curated AI-vendor secret detection, token-based cost computation, runtime-specific presets, and a 5-minute setup that OTel Collector requires ~150 lines of OTTL / processor configuration to approximate. Concrete side-by-side: [`17-otel-collector-cookbook.md`](17-otel-collector-cookbook.md).

**Q: Why not just use Langfuse / Phoenix / LangSmith / Helicone?**

They are excellent for observability inside your own application code (where you can drop their SDK or proxy). They cannot observe Claude Code on your laptop, Cursor in your IDE, or Ollama via the `ollama` CLI — none of those run code you own. Mara fills that gap. Mara also ships to Langfuse / Phoenix as a sink, so it complements them rather than replacing them.

**Q: Who maintains it?**

[ArdurAI](https://ardurai.dev) at MVP. We plan to add external maintainers as the project grows; see [`19-community-governance.md`](19-community-governance.md). CNCF Sandbox application at v1.0.

**Q: Is it production-ready?**

No. v0.2.0-alpha is pre-1.0; breaking changes can happen in any minor release. Use it where alpha software is acceptable (developer laptops, internal tooling). Wait for v1.0 for production-critical environments.

**Q: Is it free?**

Apache 2.0. Free to use, modify, redistribute. ArdurAI's commercial product is the hosted control plane (v3, not yet shipped); the OSS edge agent is never gated behind a paid plan.

**Q: Will it always be open source?**

The edge agent (`mara`) and gateway (`mara-gateway`) are committed Apache 2.0 forever. We've internalized the Grafana / Elastic / HashiCorp relicense lessons; relicensing the core is not on the roadmap. See [`../01-landscape/05-licensing-and-governance.md`](../01-landscape/05-licensing-and-governance.md).

### Installation and setup

**Q: How do I install it?**

macOS: `brew install ardurai/mara/mara`. Linux: deb/rpm from the repo at `https://ardurai.dev/mara/`. Docker: `docker pull ghcr.io/ardurai/mara:latest`. Per-platform details in [`../06-deployment-blueprints/`](../06-deployment-blueprints/).

**Q: Does it support Windows?**

Not at MVP. Windows packaging ships in MVP+1 (v0.3.0-alpha).

**Q: Does it require Kubernetes?**

No. Mara is an edge agent — runs on a laptop, a server, or a Kubernetes node as a DaemonSet. The Helm chart is for the K8s case; everything else uses launchd / systemd / Windows Service / Docker / Lambda Extension.

**Q: Does it run on Apple Silicon?**

Yes. macOS aarch64 builds are first-class.

**Q: Does it work with Open WebUI / Continue.dev / Aider / Cody / Copilot?**

For Ollama-backed users of Open WebUI and Continue.dev, yes — Mara sits in front of Ollama and captures every call. For Aider when running Claude Code, yes — same OTLP path. Cody and Copilot are not target runtimes at MVP; if they emit OTel and you point them at Mara's receiver, they should work generically, but we don't ship presets for them.

### Telemetry scope

**Q: What does Mara capture?**

Per AI runtime: session lifecycle, prompts and completions (subject to opt-in), tool calls and tool results, token usage (input / output / cached / reasoning), cost (vendor-emitted or Mara-computed), latency (TTFT, total, per-tool-call), errors, MCP traffic, agent-loop spans. Detail per runtime in [`../05-evaluation/02-compatibility-matrix-spec.md`](../05-evaluation/02-compatibility-matrix-spec.md).

**Q: What does Mara NOT capture?**

Prompt and completion content by default — capture is opt-in. Host-level metrics (CPU, memory of your machine) — that's not Mara's job; use `node_exporter`. Training data, embedding vectors, model weights. Application-code traces from your own services — use an OTel SDK for that.

**Q: Does Mara block prompt injection or guardrail my LLM?**

No. Mara observes; guardrails (Lakera Guard, NeMo Guardrails, Llama Guard) block. Mara can receive their decisions as canonical events and ship them, but it's not a guardrail. See LLM01 in [`10-owasp-alignment.md`](10-owasp-alignment.md).

**Q: Does Mara see my plaintext prompts?**

Only when you opt in via `mara.policy.capture_optin = true` AND the runtime's own toggle (`OTEL_LOG_USER_PROMPTS=true` for Claude Code, `log_user_prompt = true` for Codex, etc.). Default is off; Mara only sees a SHA-256 hash.

**Q: Does Mara phone home to ArdurAI?**

No. With no sinks configured, Mara produces zero outbound packets. This is sign-off criterion SC-7 and is regression-tested in CI.

### Privacy and compliance

**Q: Is Mara HIPAA-compliant?**

Mara is software. HIPAA compliance is a deployment-and-process attribute, not a software attribute. Mara ships PHI redaction primitives (in MVP+1) and audit-log capabilities (in v1.0) that support a HIPAA deployment, but ArdurAI does not sign Business Associate Agreements at MVP — we're a software vendor, not a service provider. Operators integrate Mara into their own HIPAA compliance program.

**Q: Is Mara SOC 2 compliant?**

Same answer. Mara's design supports SOC 2 controls (see [`../05-evaluation/03-soc2-control-mapping.md`](../05-evaluation/03-soc2-control-mapping.md)). ArdurAI's Type I attestation is targeted at v1.0; Type II within the year after. Until then: control mappings exist, the audit is in flight.

**Q: Is Mara EU AI Act compliant?**

Mara is infrastructure software, not an AI system. The AI Act doesn't directly apply to Mara. Mara provides technical capabilities that support operators' AI Act compliance for *their* AI systems — see [`../05-evaluation/04-eu-ai-act-alignment.md`](../05-evaluation/04-eu-ai-act-alignment.md).

**Q: Where does Mara store data?**

In the WAL on local disk (default `~/.local/state/mara/wal/`, bounded by size and age) and in whatever sink you configured. ArdurAI never sees or stores your event data unless you explicitly configure a Mara-hosted sink — and at MVP that doesn't exist as a product.

### Performance and operations

**Q: How much memory does it use?**

Target ≤ 128 MiB RSS idle, ≤ 512 MiB at sustained 50k EPS. In practice for an indie-dev workload (handful of events per minute), expect 100-150 MiB RSS.

**Q: How much CPU?**

≤ 5% of one core at 10k EPS steady-state. For dev-machine workloads, effectively unmeasurable.

**Q: Will it slow down my AI tools?**

For Claude Code / Codex / Gemini CLI (OTLP receive): no measurable impact. Mara is downstream.

For Ollama (HTTP proxy): adds one TCP-loopback hop, ≈50 μs latency floor. Imperceptible in interactive use.

**Q: Can I run multiple Mara instances on the same machine?**

Yes, with distinct state dirs and metrics ports. Useful if you want separate redaction policies per project, or to test config changes alongside production.

**Q: What if Mara crashes?**

Launchd / systemd / Windows Service restart it automatically. Events in the in-memory queue are lost (MVP); events in the WAL persist (v1.0). On Ollama proxy mode, a Mara crash causes Ollama clients to see connection refused until Mara restarts (typically < 1 second).

### Sinks and integrations

**Q: Which observability backends does Mara ship to?**

At MVP: OTLP (Honeycomb / Logfire / Datadog / Grafana Tempo / Signoz / Hyperdx / any OTel-compatible) and Loki. Post-MVP: Splunk HEC, Elasticsearch, S3/GCS/Azure (JSONL + Parquet), Kafka, Prometheus Remote Write, generic webhook.

**Q: Can I write my own sink?**

Yes. At MVP via fork-and-PR. Post-1.0 via the plugin ABI documented in [`19-community-governance.md`](19-community-governance.md) §"RFC process."

**Q: Does it integrate with Datadog?**

Yes via OTLP. Point a Mara OTLP sink at the Datadog Agent's OTLP receiver, or at the Datadog OTLP intake endpoint directly. The events appear as logs / spans in Datadog with `gen_ai.*` attributes preserved.

**Q: Does it integrate with Splunk?**

In MVP+1 via the Splunk HEC sink. At MVP, you can use the generic webhook approach (deferred) or wait.

### Pricing / commercial

**Q: How does ArdurAI make money?**

The hosted control plane (v3, planned 2028). The OSS edge agent and gateway are free. There is no enterprise tier of the OSS itself — the OSS is the whole OSS.

**Q: Can I get commercial support?**

Not yet. ArdurAI may offer paid support contracts post-v1.0 for companies that want SLAs. Email `oss@ardurai.dev` if you want to be on the list.

**Q: Can I embed Mara in my product?**

Yes — Apache 2.0 permits it. Two requirements: maintain the LICENSE + NOTICE; do not use the "Mara" name in a way that implies endorsement (see trademark policy at [`19-community-governance.md`](19-community-governance.md)).

## Troubleshooting playbook

### Mara won't start

1. `mara version` — does the binary execute? If no, install is broken; reinstall.
2. `mara validate --config <path>` — does the config parse? If no, error message includes the file path and line.
3. Are the configured state directories writable? Try `ls -la $(dirname <state_dir>)` to verify.
4. Is the configured metrics port already in use? `lsof -i :9099`. Move Mara to a different port or kill the conflicting process.
5. Check OS-specific logs: `journalctl --user-unit mara` (Linux), `~/Library/Logs/mara/mara.err.log` (macOS), Windows Event Log.

### `mara setup claude-code` writes a config but no events appear

1. Verify Claude Code is emitting OTel: `echo $CLAUDE_CODE_ENABLE_TELEMETRY` (should be `1`), `echo $OTEL_EXPORTER_OTLP_ENDPOINT` (should be `http://127.0.0.1:4318` or `:4317`).
2. Verify Mara is listening: `curl -v http://127.0.0.1:4318/v1/logs -X POST -H 'content-type: application/x-protobuf' --data-binary @/dev/null`. Should return a 200 or 400 (not connection refused).
3. Verify Claude Code session is active: tail `~/.claude/projects/*.jsonl` while a session runs.
4. Verify Mara metrics: `curl -s http://127.0.0.1:9099/metrics | grep mara_pipeline_events_total`. The count should rise as you use Claude Code.
5. If counts rise but no events in your sink: the sink is broken; see "Sink errors" below.

### Ollama proxy: clients see connection refused on `:11434`

1. Is Mara actually running and listening on 11434? `lsof -i :11434` should show `mara`. If not, Mara isn't bound — check Mara logs.
2. Did Ollama also try to bind 11434? Verify `OLLAMA_HOST=127.0.0.1:11435` is set in Ollama's environment. On macOS: `launchctl getenv OLLAMA_HOST`. On Linux: check the systemd override.
3. Did Ollama restart after the `OLLAMA_HOST` change? `brew services restart ollama` / `systemctl --user restart ollama`.
4. Test the proxy hop: `curl http://127.0.0.1:11434/api/version` should return Ollama's version (Mara forwards transparently).

### Ollama: events appear but token counts are zero

1. Are you streaming responses without `include_usage` (OpenAI-compat mode)? Add `include_usage: true` to the request body.
2. Did the stream disconnect mid-response? Check `mara.ollama.partial = true` on the event — that's the indicator.
3. Are you using an embeddings endpoint? Embeddings responses don't include token counts in all Ollama versions — `eval_count` may be absent.

### Sink errors

1. `curl -s http://127.0.0.1:9099/metrics | grep mara_sink_errors_total{sink="..."}` — which sink, what label.
2. Sink endpoint reachable? `curl -v <endpoint>`.
3. Auth credentials correct? Check the env var or `@file:` / `@vault:` reference resolves.
4. TLS certificate trusted? Try `openssl s_client -connect <host>:<port>`.
5. Backend full / rate limiting? Check the backend's status page or rate-limit headers.
6. Dead-letter queue: `mara dlq list` (MVP+1) — events that have given up retrying.

### Redaction not firing

1. Verify the policy chain is configured: `mara diag` shows policy stages active.
2. Verify the pattern is in the configured pack: `mara policy show --pack builtin.pii` (MVP+1).
3. Is the value in a captured field? If `capture_optin = false`, prompt content is hashed before policy sees it — redaction effectively isn't reached for body content.
4. Pattern false-negative: file an issue with a sample. We add patterns aggressively.

### High memory growth

1. Cardinality explosion. Check `mara_pipeline_attributes_unique` (MVP+1). High unique attribute values eat memory.
2. Slow sink. Check `mara_sink_lag_seconds`; if high, events accumulate in the in-memory queue.
3. WAL not draining (v1.0+). Check `mara_wal_bytes_used` vs `mara_wal_bytes_limit`.
4. Memory leak. File an issue with `ps -o rss <pid>` over time and `mara_pipeline_events_total` over the same window.

### Self-telemetry endpoint returns 503

`/healthz` returns 503 when any pipeline component is `Failed`. Check `mara diag` for the failed component; logs will say which one and why.

### Tests in my CI are flaky after adding Mara

1. Is Mara's bench gate (PR-level perf check) the cause? Re-run; check the regression delta.
2. Are tests racing the Mara startup? Add a `wait-for-port 4318` step before tests that depend on Mara being ready.
3. Is the WAL state persisting across test runs? Set `--state-dir` to a per-test tempdir.

## Where this FAQ goes after MVP

The launch FAQ lives here at MVP. As patterns stabilize, the content migrates:

- **Questions about installation:** move into per-runtime quickstarts under `plans/07-quickstarts/`.
- **Questions about operations:** move into [`../../docs/runbook.md`](../../docs/runbook.md).
- **Questions about architecture:** move into ADRs.
- **Questions answered "no" / "out of scope":** consolidate into [`../00-overview/02-non-goals.md`](../00-overview/02-non-goals.md).

The FAQ exists because operators land on the docs from Google with a specific question; it's a landing page, not a permanent home for the answers.

## Cross-references

- [`14-launch-and-early-adopter-experience.md`](14-launch-and-early-adopter-experience.md) — first 30 days of support.
- [`20-migration-guides.md`](20-migration-guides.md) — moving from competing tools.
- [`../../docs/runbook.md`](../../docs/runbook.md) — operational reference.
- [`../05-evaluation/02-compatibility-matrix-spec.md`](../05-evaluation/02-compatibility-matrix-spec.md) — what's supported per runtime.
- [`../../SECURITY.md`](../../SECURITY.md) — security reporting.
