# Deployment Patterns Overview

## Executive summary

This is the index and decision guide for all Mara deployment blueprints. Each detailed blueprint lives in [`../06-deployment-blueprints/`](../06-deployment-blueprints/). The right pattern depends on platform, fleet size, isolation requirements, and which AI runtime is in use.

## Pattern catalogue

### Per platform

- macOS — launchd LaunchAgent (per-user) or LaunchDaemon (system-wide). See [`../06-deployment-blueprints/01-macos-launchd.md`](../06-deployment-blueprints/01-macos-launchd.md).
- Linux — systemd user service (per-user) or system service (system-wide); journald-aware logging. See [`../06-deployment-blueprints/02-linux-systemd.md`](../06-deployment-blueprints/02-linux-systemd.md).
- Windows — Windows Service with `sc.exe` or PowerShell `New-Service`; ETW for self-telemetry. See [`../06-deployment-blueprints/03-windows-service.md`](../06-deployment-blueprints/03-windows-service.md).
- Kubernetes DaemonSet — one Mara per node, hostPath access to container logs. See [`../06-deployment-blueprints/04-kubernetes-daemonset.md`](../06-deployment-blueprints/04-kubernetes-daemonset.md).
- Kubernetes sidecar — one Mara per Pod for per-app policy isolation. See [`../06-deployment-blueprints/05-kubernetes-sidecar.md`](../06-deployment-blueprints/05-kubernetes-sidecar.md).
- Serverless — Lambda Extension API for capturing AI activity in Lambda. See [`../06-deployment-blueprints/06-serverless-lambda-extension.md`](../06-deployment-blueprints/06-serverless-lambda-extension.md).
- Docker Compose — local-dev pattern. See [`../06-deployment-blueprints/07-docker-compose.md`](../06-deployment-blueprints/07-docker-compose.md).
- CI runners — ephemeral GitHub Actions / GitLab / Buildkite. See [`../06-deployment-blueprints/08-ci-runners.md`](../06-deployment-blueprints/08-ci-runners.md).

## Decision matrix

The picks below assume Topology 1 (edge-only) unless stated. For aggregated topologies see [`05-pipeline-topologies.md`](05-pipeline-topologies.md).

- **A developer running Claude Code / Codex / Cursor on a Mac:** macOS LaunchAgent (per-user). Install via `brew install mara && mara setup claude-code`.
- **A developer on a Linux desktop:** systemd `--user` service. Install via deb/rpm + `systemctl --user enable mara`.
- **A developer on Windows:** scheduled task running at login, or Windows Service if they want it independent of session.
- **A team running AI agents in a Kubernetes app:** DaemonSet for OTLP receive + node-wide policy. Sidecar only if per-app ZDR profile is needed.
- **A team running AI agents in serverless functions:** Lambda Extension at first; consider Cloud Run sidecars for GCP.
- **A startup running Mara + sinks locally for dev:** Docker Compose with Loki + Grafana + Mara.
- **A team capturing AI activity in CI:** install Mara at the start of each job; tail JSONL output of `codex exec --json` and `claude --print --output-format json`.

## Cross-cutting concerns

### Permissions

- Mara MUST run as an unprivileged user by default.
- File-tail adapters need read access to the runtime's log/transcript directories, which are typically per-user; running per-user (LaunchAgent / systemd `--user`) is the simplest path.
- For multi-user systems where Mara needs to ingest other users' transcripts, document the privilege requirement explicitly and prefer dropping privileges after binding ports.

### Network egress

- Mara emits no telemetry by default. Each configured sink is an explicit allow-list entry.
- Document the sink endpoints required for popular setups so security teams can pre-approve them.

### Updates

- Homebrew tap, deb/rpm repository, OCI image tags, and Helm chart versions are the four update vectors.
- Mara MUST NOT auto-update by default. Auto-update can be enabled via configuration but emits a warning on first start.

### Observability of the agent itself

- Self-telemetry is on by default at `127.0.0.1:9099/metrics` and `/healthz`.
- When Mara is deployed via Helm, the chart adds Prometheus ServiceMonitor and a default Grafana dashboard.

### Multi-instance hosts

- Multiple Mara processes on one host are supported. Each must have its own `--state-dir` (default `${XDG_STATE_HOME}/mara/<instance>`) and its own metrics port.
- Provide `mara init --instance <name>` to scaffold a second instance.

## Resource sizing guidance

Per-host baseline at the v1 SLOs (NFR-1):

- **Idle:** 128 MiB RSS, < 1% CPU.
- **Light (1k EPS):** 200 MiB RSS, 1% CPU.
- **Moderate (10k EPS):** 350 MiB RSS, 5% CPU.
- **Heavy (50k EPS):** 512 MiB RSS, ~80% of one core.

For Kubernetes DaemonSet, set `resources.requests.memory = 256Mi`, `resources.limits.memory = 768Mi`, `resources.requests.cpu = 100m`, `resources.limits.cpu = 1000m` as the starting point. Tune per workload.

## Failure-mode posture per pattern

- **LaunchAgent / systemd user:** if Mara crashes, launchd / systemd restart it. WAL preserves events across restarts.
- **DaemonSet:** kubelet restarts on crash; node restart triggers WAL replay; cordoned nodes still drain via emergency dump to local disk.
- **Sidecar:** if Mara sidecar OOMs, the app Pod stays up (configurable behavior); on Pod restart, sidecar WAL on emptyDir is lost — so sidecars should configure WAL to a persistent volume or use shorter buffer windows.
- **Lambda Extension:** Lambda environment terminates on idle; in-flight events flush via the extension's shutdown hook (with a tight time budget).
- **CI runner:** ephemeral by definition; configure WAL to be small and rely on synchronous sink commits at job end.

## Cross-platform packaging summary

- macOS: universal2 binary, code-signed, notarized; Homebrew tap; pkg installer for system-wide installs.
- Linux: static glibc binary, static musl binary, deb + rpm repositories, AppImage for distro-agnostic install.
- Windows: amd64 binary, signed; MSI installer; Chocolatey package; winget package.
- Container: distroless image at `ghcr.io/ardurai/mara:<version>` and `ghcr.io/ardurai/mara:<version>-debug` (with shell for troubleshooting).
- Helm: chart at `oci://ghcr.io/ardurai/charts/mara`.

## Upgrade guidance

- Always read the CHANGELOG before upgrading; minor version bumps are additive, but adapter or sink defaults may shift.
- For DaemonSets, use `RollingUpdate` strategy with `maxUnavailable: 10%` to avoid losing telemetry coverage during upgrade.
- For LaunchAgent / systemd, the package install scripts handle graceful restart with WAL drain.

## Per-runtime install pairing

Each runtime quickstart references its preferred host topology:

- Claude Code → macOS LaunchAgent or Linux systemd `--user`.
- Codex → macOS LaunchAgent or Linux systemd `--user`; CI variant available.
- Cursor → macOS LaunchAgent or Linux systemd `--user`; Cursor hook script wired into Mara hooks adapter.
- Kimi → macOS LaunchAgent, Linux systemd `--user`, or Windows Service.
- Augment → matches the host where the IDE runs; analytics REST adapter runs once per organization (not per machine) by default.
- Gemini CLI → matches the host where the CLI runs.

See per-runtime quickstarts in [`../07-quickstarts/`](../07-quickstarts/).
