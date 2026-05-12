# Linux systemd Deployment

## Executive summary

On Linux, Mara runs as a systemd `--user` service for per-user installs (Persona 2's default) or a system-level service for shared/server installs. The deb and rpm packages install the binary, the unit file, and a default config. `journald` consumes Mara's stderr by default; structured-log mode emits JSON suitable for `journalctl -o cat | jq`.

## Install (deb / rpm / static binary)

Debian/Ubuntu:

```bash
curl -fsSL https://ardurai.dev/mara/apt-key.asc | sudo gpg --dearmor -o /usr/share/keyrings/ardurai-mara.gpg
echo "deb [signed-by=/usr/share/keyrings/ardurai-mara.gpg] https://ardurai.dev/mara/deb stable main" \
  | sudo tee /etc/apt/sources.list.d/ardurai-mara.list
sudo apt update && sudo apt install mara
```

Fedora/RHEL:

```bash
sudo dnf config-manager --add-repo https://ardurai.dev/mara/rpm/ardurai-mara.repo
sudo dnf install mara
```

Static binary (any distro, glibc 2.31+ or musl):

```bash
curl -LO https://github.com/ArdurAI/mara/releases/latest/download/mara-linux-amd64.tar.gz
tar xzf mara-linux-amd64.tar.gz
sudo install -m 0755 mara /usr/local/bin/mara
```

For arm64: substitute `mara-linux-arm64.tar.gz`. For musl/Alpine: `mara-linux-amd64-musl.tar.gz`.

## systemd unit (per-user)

`~/.config/systemd/user/mara.service`:

```ini
[Unit]
Description=Mara — AI-native telemetry shipper
Documentation=https://github.com/ArdurAI/mara
After=network-online.target
Wants=network-online.target

[Service]
Type=notify
ExecStart=/usr/local/bin/mara run --config %h/.config/mara/mara.toml
Restart=on-failure
RestartSec=5s
TimeoutStopSec=30s
KillMode=mixed
LimitNOFILE=65536

# Hardening
NoNewPrivileges=true
ProtectSystem=strict
ProtectHome=read-only
ReadWritePaths=%h/.local/state/mara %h/.cache/mara %h/.local/share/mara
PrivateTmp=true
ProtectKernelTunables=true
ProtectKernelModules=true
ProtectControlGroups=true
MemoryDenyWriteExecute=true
RestrictRealtime=true
RestrictNamespaces=true
LockPersonality=true

[Install]
WantedBy=default.target
```

Enable and start:

```bash
systemctl --user daemon-reload
systemctl --user enable --now mara
journalctl --user-unit mara -f
```

## systemd unit (system-wide)

`/etc/systemd/system/mara.service`:

```ini
[Unit]
Description=Mara — AI-native telemetry shipper
After=network-online.target
Wants=network-online.target

[Service]
Type=notify
User=mara
Group=mara
ExecStart=/usr/local/bin/mara run --config /etc/mara/mara.toml
Restart=on-failure
RestartSec=5s
TimeoutStopSec=30s
KillMode=mixed
LimitNOFILE=65536

ReadWritePaths=/var/lib/mara /var/log/mara

NoNewPrivileges=true
ProtectSystem=strict
ProtectHome=true
PrivateTmp=true
ProtectKernelTunables=true
ProtectKernelModules=true
ProtectControlGroups=true
MemoryDenyWriteExecute=true
RestrictRealtime=true
RestrictNamespaces=true
LockPersonality=true

[Install]
WantedBy=multi-user.target
```

The `mara` system user is created by the package post-install script. State at `/var/lib/mara`, config at `/etc/mara/mara.toml`.

## File locations

Per-user install:

- Binary: `/usr/local/bin/mara` (or `/usr/bin/mara` from deb/rpm).
- Config: `~/.config/mara/mara.toml`.
- State (WAL, checkpoints): `~/.local/state/mara/`.
- Cache: `~/.cache/mara/`.

System-wide install:

- Binary: `/usr/bin/mara`.
- Config: `/etc/mara/mara.toml`.
- State: `/var/lib/mara/`.
- Logs (self-telemetry to journald by default): `journalctl -u mara`.

## sd_notify integration

Mara implements `Type=notify`: it calls `sd_notify(READY=1)` once the pipeline is ready, `sd_notify(WATCHDOG=1)` periodically for the watchdog timer if `WatchdogSec` is set, and `sd_notify(STOPPING=1)` on graceful shutdown.

This lets systemd track Mara's true readiness rather than guessing from process start.

## journald integration

Default: Mara emits structured stderr; journald captures it; `journalctl -u mara` reads it. With `--log-format json`, lines parse via `journalctl -o cat -u mara | jq`.

For native journald structured fields, Mara can be configured to use `libsystemd`'s `sd_journal_send`-equivalent via the `tracing-journald` crate as an optional feature.

## Packaging

deb produced via `cargo deb` (community crate) with overrides:

- pre/post-install scripts to create the `mara` user (system install only) and reload systemd.
- conflicts with: `fluent-bit` (no, they coexist) — none.
- depends on: nothing (static binary).

rpm produced via `cargo generate-rpm` analogously.

## Container image

`ghcr.io/ardurai/mara:<version>` is a distroless image built `FROM gcr.io/distroless/static-debian12`:

```dockerfile
FROM gcr.io/distroless/static-debian12
COPY mara /usr/local/bin/mara
ENV MARA_CONFIG=/etc/mara/mara.toml
USER 65532:65532
EXPOSE 4317 4318 9099
ENTRYPOINT ["/usr/local/bin/mara", "run"]
```

A `<version>-debug` variant uses `gcr.io/distroless/static-debian12:debug` (with busybox shell) for troubleshooting.

## Common pitfalls

- **`ProtectHome=true` blocks reading `~/.claude/projects/`.** For per-user JSONL adapters, use `ProtectHome=read-only` in the per-user unit (already set above) or relax for the specific paths needed.
- **`MemoryDenyWriteExecute=true` blocks WASM JIT in some `wasmtime` configurations.** Mara's `wasmtime` is configured to use the Pulley interpreter or Cranelift's ahead-of-time mode to remain compatible with `MemoryDenyWriteExecute`. Verify with `systemd-analyze security mara`.
- **`LimitNOFILE` too low** for high-EPS scenarios with many file-tail sources. The unit sets `65536`; tune for fleets with thousands of JSONL files.
- **journald rate-limiting** can drop Mara's self-logs under heavy load. Configure `/etc/systemd/journald.conf` `RateLimitBurst=10000` for high-volume hosts, or have Mara emit self-logs to a file.

## Self-telemetry on Linux

- Metrics on `127.0.0.1:9099/metrics` (per-user) or bind elsewhere via config.
- Health on `127.0.0.1:9099/healthz`.
- For system-wide installs, scrape the metrics endpoint from a node-local Prometheus or push via Mara's own prom-rw sink (eat-your-own-dog-food).

## Compliance notes

- `systemd-analyze security mara` should report a score < 1.0 (good hardening) after the unit is installed.
- SELinux: the deb/rpm packages include an SELinux policy module placing the `mara` domain in `mara_t` and labeling the binary `mara_exec_t`. AppArmor profile included for Ubuntu/Debian.
- The deb and rpm repositories themselves are signed; package signatures verified by `apt`/`dnf`.

## Upgrade workflow

```bash
sudo apt update && sudo apt upgrade mara
# or
sudo dnf upgrade mara
```

Both packages restart Mara via systemd post-install with a TERM-then-KILL sequence allowing 30 s drain. WAL preserves events across the restart.

## Uninstall

```bash
sudo systemctl stop mara
sudo systemctl disable mara
sudo apt remove --purge mara   # or dnf remove
sudo rm -rf /var/lib/mara /var/log/mara
```

Per-user:

```bash
systemctl --user stop mara
systemctl --user disable mara
rm -rf ~/.config/mara ~/.local/state/mara ~/.cache/mara
```
