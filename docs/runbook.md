# Mara Operational Runbook

This runbook covers the operational lifecycle of Mara: install, configure, validate, monitor, troubleshoot, upgrade, and uninstall. It's the operator-facing reference; engineering reference is in [`../plans/04-implementation/`](../plans/04-implementation/).

## Install

| Platform | Method | Quickstart |
|---|---|---|
| macOS | Homebrew | [`plans/06-deployment-blueprints/01-macos-launchd.md`](../plans/06-deployment-blueprints/01-macos-launchd.md) |
| Linux | deb/rpm/static binary | [`plans/06-deployment-blueprints/02-linux-systemd.md`](../plans/06-deployment-blueprints/02-linux-systemd.md) |
| Windows | winget / Chocolatey / MSI | [`plans/06-deployment-blueprints/03-windows-service.md`](../plans/06-deployment-blueprints/03-windows-service.md) |
| Kubernetes | Helm chart | [`plans/06-deployment-blueprints/04-kubernetes-daemonset.md`](../plans/06-deployment-blueprints/04-kubernetes-daemonset.md) |
| Docker Compose | local dev stack | [`plans/06-deployment-blueprints/07-docker-compose.md`](../plans/06-deployment-blueprints/07-docker-compose.md) |
| Lambda | extension layer | [`plans/06-deployment-blueprints/06-serverless-lambda-extension.md`](../plans/06-deployment-blueprints/06-serverless-lambda-extension.md) |

## Configure

1. Copy `examples/mara.toml` to the conventional location (`~/Library/Application Support/mara/mara.toml`, `~/.config/mara/mara.toml`, `%APPDATA%\Mara\mara.toml`, or `/etc/mara/mara.toml` for system-wide installs).
2. Run `mara setup <runtime>` to apply a runtime preset (Claude Code, Codex, Cursor, Kimi, Augment, or Gemini).
3. Run `mara validate --config <path>` to confirm syntax + cross-references.
4. Reload running agent with SIGHUP after editing.

## Monitor

Self-telemetry endpoints (default `127.0.0.1:9099`):

- `GET /metrics` — Prometheus metrics (`mara_pipeline_events_total`, `mara_sink_errors_total`, `mara_wal_*`, `mara_policy_*`).
- `GET /healthz` — 200 when all components are healthy, 503 otherwise.
- `mara diag` (CLI) — human-readable view of adapter / policy / sink / WAL health.

Recommended alerts:

- `rate(mara_sink_errors_total[5m]) > 0` → sink is failing.
- `mara_wal_bytes_used / mara_wal_bytes_limit > 0.8` → WAL near capacity.
- `mara_policy_traps_total > 0` → WASM policy crashed.
- `mara_pipeline_lag_seconds > 60` → adapter is falling behind.

## Troubleshoot

### Mara won't start

1. `mara validate --config <path>` — does config parse?
2. Are required directories writable? (WAL, checkpoint dirs)
3. Are configured sink endpoints reachable? (curl from the host)
4. Check OS-specific logs: `journalctl -u mara`, `~/Library/Logs/mara/mara.err.log`, Windows Event Log.

### Events not flowing

1. `mara diag` — which adapter is failing or lagging?
2. Is the AI runtime emitting OTLP / writing JSONL? (e.g., is `CLAUDE_CODE_ENABLE_TELEMETRY=1`?)
3. Are file permissions correct on transcript paths?
4. For OTLP: is the firewall allowing `127.0.0.1:4317`?

### Sink errors

1. `mara_sink_errors_total{sink="..."}` — which sink, what error label?
2. Network egress (curl from Mara's host to the sink endpoint).
3. Sink credentials valid? (check `mara_sink_auth_failures_total` if exposed by sink).
4. Dead-letter queue: `mara dlq list`.

### Memory growth

1. RSS via `ps`, `htop`, or Activity Monitor.
2. Cardinality explosion? (`mara_pipeline_attributes_unique` if exposed).
3. WAL not draining? (slow sink → WAL grows → memory grows on indices).

## Upgrade

```bash
# macOS
brew upgrade mara
# Linux
sudo apt upgrade mara   # or dnf upgrade mara
# Windows
winget upgrade ArdurAI.Mara
# Kubernetes
helm upgrade mara oci://ghcr.io/ardurai/charts/mara --version <new>
```

Always read the CHANGELOG before upgrading. Mara's package install scripts perform graceful restart with WAL drain.

## Uninstall

Stop the service, remove the binary, optionally remove state.

```bash
# macOS
brew services stop mara && brew uninstall mara
rm -rf ~/Library/Application\ Support/mara

# Linux
sudo systemctl stop mara && sudo systemctl disable mara
sudo apt remove --purge mara

# Windows
Stop-Service mara
sc.exe delete mara
winget uninstall ArdurAI.Mara

# Kubernetes
helm uninstall mara --namespace mara
```

## Incident response

1. Mark severity (Critical / High / Medium / Low).
2. Collect: timestamps, version, OS, config (redacted), `mara diag` output, recent logs, sink response codes.
3. If suspected vulnerability: see [`../SECURITY.md`](../SECURITY.md); do NOT open a public issue.
4. Open a tracking issue (non-vuln) or contact `security@ardurai.dev` (vuln).
5. Apply mitigation; document in `docs/security-postmortems/` after disclosure.

## Reference

- [`SECURITY.md`](../SECURITY.md) — security policy + reporting.
- [`plans/04-implementation/01-functional-requirements.md`](../plans/04-implementation/01-functional-requirements.md) — FR-1 through FR-12.
- [`plans/04-implementation/02-non-functional-requirements.md`](../plans/04-implementation/02-non-functional-requirements.md) — SLOs.
- [`docs/threat-model.md`](threat-model.md) — STRIDE.
- [`docs/compat-matrix.md`](compat-matrix.md) — what Mara captures per runtime.
- [`docs/adr/`](adr/) — architecture decisions.
