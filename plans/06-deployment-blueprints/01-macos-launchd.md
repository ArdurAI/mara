# macOS launchd Deployment

## Executive summary

On macOS, Mara runs as a launchd `LaunchAgent` for per-user operation (the recommended default for developer machines) or as a `LaunchDaemon` for system-wide operation. The Homebrew tap installs the binary, the LaunchAgent plist, and a default per-user config. Apple's hardened-runtime, notarization, and full-disk-access requirements all apply.

## Install (Homebrew, recommended)

```bash
brew tap ardurai/mara
brew install mara
mara setup claude-code   # or codex, cursor, kimi, gemini
brew services start mara
```

`brew services start mara` writes `~/Library/LaunchAgents/dev.ardurai.mara.plist` and loads it.

## LaunchAgent plist (the agent ships this; reference)

```xml
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>Label</key>            <string>dev.ardurai.mara</string>
  <key>ProgramArguments</key>
  <array>
    <string>/opt/homebrew/bin/mara</string>
    <string>run</string>
    <string>--config</string>
    <string>/Users/USER/Library/Application Support/mara/mara.toml</string>
  </array>
  <key>RunAtLoad</key>        <true/>
  <key>KeepAlive</key>
  <dict>
    <key>SuccessfulExit</key> <false/>
    <key>Crashed</key>        <true/>
  </dict>
  <key>StandardOutPath</key>  <string>/Users/USER/Library/Logs/mara/mara.out.log</string>
  <key>StandardErrorPath</key><string>/Users/USER/Library/Logs/mara/mara.err.log</string>
  <key>EnvironmentVariables</key>
  <dict>
    <key>XDG_CONFIG_HOME</key><string>/Users/USER/Library/Application Support</string>
    <key>XDG_STATE_HOME</key> <string>/Users/USER/Library/Application Support</string>
    <key>XDG_DATA_HOME</key>  <string>/Users/USER/Library/Application Support</string>
  </dict>
  <key>ProcessType</key>      <string>Background</string>
  <key>Nice</key>             <integer>5</integer>
</dict>
</plist>
```

The path `Application Support/mara/mara.toml` is the default config location for per-user installs.

## File locations (macOS conventions)

- Binary: `/opt/homebrew/bin/mara` (Apple Silicon) or `/usr/local/bin/mara` (Intel).
- Config: `~/Library/Application Support/mara/mara.toml`.
- State (WAL, checkpoints): `~/Library/Application Support/mara/state/`.
- Logs (self-telemetry): `~/Library/Logs/mara/`.
- LaunchAgent plist: `~/Library/LaunchAgents/dev.ardurai.mara.plist`.

## Permissions and full-disk-access

Reading `~/.claude/projects/*.jsonl`, `~/.codex/history.jsonl`, `~/.kimi/logs/`, etc., requires that Mara run as the same user that owns those files. The LaunchAgent (per-user) pattern satisfies this naturally.

For Cursor's hooks adapter, no special permission is needed — Cursor invokes Mara as a subprocess and pipes JSON over stdio.

Mara does not require Full Disk Access (FDA) for any v1 use case. If a future feature reads `~/Library/Mail` or another protected location, FDA would be required and prompted on first run.

## Code signing and notarization

Releases are:

1. Compiled with `cargo build --release` against the universal2 target on a macOS runner.
2. Code-signed with an Apple Developer ID Application certificate.
3. Notarized with `notarytool submit ... --wait`.
4. Stapled with `xcrun stapler staple mara`.

Homebrew tap uses `--with-cask`-like installation for the signed binary; system-wide installs use a `pkg` produced via `pkgbuild` + `productbuild`.

## Uninstall

```bash
brew services stop mara
brew uninstall mara
rm -rf "~/Library/Application Support/mara" "~/Library/Logs/mara"
launchctl unload ~/Library/LaunchAgents/dev.ardurai.mara.plist 2>/dev/null || true
rm -f ~/Library/LaunchAgents/dev.ardurai.mara.plist
```

## System-wide LaunchDaemon (advanced)

For shared-machine or fleet-managed installs:

- Plist at `/Library/LaunchDaemons/dev.ardurai.mara.plist`, owner `root:wheel`, mode `0644`.
- Runs as a dedicated `_mara` system user (added by the `pkg` installer).
- Config at `/Library/Application Support/mara/mara.toml`.
- State at `/Library/Application Support/mara/state/`, owned by `_mara`.

LaunchDaemon caveat: it cannot read per-user files like `~/.claude/projects/*.jsonl`. Use this pattern only for OTLP-receive workloads or for ingest of system-wide logs (`/var/log/...`).

## Per-user vs system-wide guidance

- **Developer laptop, single user:** LaunchAgent (per-user).
- **Shared workstation:** LaunchAgent per user; each user has their own Mara instance and sinks.
- **MDM-managed fleet (Jamf, Kandji):** push the `pkg` to install the binary + a per-user LaunchAgent plist via `~/Library/LaunchAgents`; can use launchctl bootstrap from a privileged helper to install the same plist for all current and future users.

## Common pitfalls

- **Forgetting to `brew services start mara`** after install. Document this in the quickstart.
- **macOS Gatekeeper blocking unnotarized builds.** Release process MUST notarize; CI checks notarization with `stapler validate` before publishing.
- **TCC (Transparency, Consent, Control) prompts** if a future feature reads protected dirs — surfaces unexpectedly to users; avoid until v1.x.
- **System Integrity Protection** does not affect Mara (we write to user home, not protected system locations).
- **Apple Silicon Rosetta**: the universal2 binary makes Rosetta moot. CI verifies arch coverage with `lipo -info`.
- **Sleep/wake**: macOS suspends background processes during sleep; KeepAlive handles wake. WAL absorbs the sleep window cleanly.

## Self-update channel

Mara does not auto-update. Users update via `brew upgrade mara`. For MDM-managed fleets, the MDM tooling pushes new pkg versions.

## Self-telemetry on macOS

- Metrics on `127.0.0.1:9099/metrics`.
- Health on `127.0.0.1:9099/healthz`.
- Logs to stderr → `~/Library/Logs/mara/mara.err.log`.

If a user wants to integrate with macOS's unified logging (`log show`), Mara can emit to it via the `os_log` Rust crate as an opt-in feature flag in v1.x.

## Compliance notes

- Mara macOS releases are signed and notarized — required for any enterprise rollout.
- The signing certificate fingerprint is published in `SECURITY.md`.
- SBOM and provenance attestations are published per release.
