# Windows Service Deployment

## Executive summary

On Windows, Mara runs as a Windows Service for system-wide deployment or as a scheduled task at user logon for per-user deployment. v1 ships amd64 only. Installers: MSI (primary), Chocolatey, and winget. Self-logging integrates with ETW (Event Tracing for Windows) and the Application event log, with structured JSON to stderr as an alternative.

## Install (winget, recommended)

```powershell
winget install ArdurAI.Mara
mara setup claude-code   # or codex, cursor, kimi, gemini
Start-Service mara
```

## Install (Chocolatey)

```powershell
choco install mara
Start-Service mara
```

## Install (MSI)

Download `Mara-x64.msi` from the release page and run. The installer:

- Installs `mara.exe` to `C:\Program Files\Mara\`.
- Creates a `mara` local service account (or runs as `NT SERVICE\mara` virtual account in v1).
- Registers the `mara` Windows Service via `sc.exe`.
- Writes default config to `C:\ProgramData\Mara\mara.toml`.

## Service configuration (PowerShell, manual)

```powershell
$exe = "C:\Program Files\Mara\mara.exe"
$cfg = "C:\ProgramData\Mara\mara.toml"

New-Service -Name mara `
  -BinaryPathName "`"$exe`" run --config `"$cfg`"" `
  -DisplayName "Mara — AI-native telemetry shipper" `
  -Description "Collects, normalizes, and ships AI agent and LLM telemetry." `
  -StartupType Automatic

# Configure restart on failure
sc.exe failure mara reset= 86400 actions= restart/5000/restart/5000/restart/5000

# Recommended: run as the virtual NT SERVICE\mara account
sc.exe config mara obj= "NT SERVICE\mara"

Start-Service mara
```

## File locations (Windows conventions)

System-wide install:

- Binary: `C:\Program Files\Mara\mara.exe`.
- Config: `C:\ProgramData\Mara\mara.toml`.
- State (WAL, checkpoints): `C:\ProgramData\Mara\state\`.
- Logs (self-telemetry): `C:\ProgramData\Mara\logs\` plus Event Log.

Per-user install (scheduled task at logon):

- Binary: `%LOCALAPPDATA%\Programs\Mara\mara.exe`.
- Config: `%APPDATA%\Mara\mara.toml`.
- State: `%LOCALAPPDATA%\Mara\state\`.
- Logs: `%LOCALAPPDATA%\Mara\logs\`.

## Per-user scheduled task

For Persona 2 on Windows (Mara captures user's Claude Code / Codex / Cursor / Kimi sessions), running as a scheduled task at logon is friendlier than a system service:

```powershell
$action = New-ScheduledTaskAction `
  -Execute "$env:LOCALAPPDATA\Programs\Mara\mara.exe" `
  -Argument "run --config `"$env:APPDATA\Mara\mara.toml`""

$trigger = New-ScheduledTaskTrigger -AtLogOn
$settings = New-ScheduledTaskSettingsSet `
  -AllowStartIfOnBatteries -DontStopIfGoingOnBatteries `
  -StartWhenAvailable -RestartCount 3 -RestartInterval (New-TimeSpan -Minutes 1)

Register-ScheduledTask -TaskName "Mara" -Action $action -Trigger $trigger -Settings $settings
```

## Code signing

Releases are signed with an EV code-signing certificate. v0.x rcs may ship with a self-signed cert and a warning; v1.0 GA is EV-signed.

SmartScreen reputation is established through download volume; expect early-release friction.

## ETW (Event Tracing for Windows)

Mara emits an ETW provider via the `tracing-etw` crate. Provider GUID published in `SECURITY.md`. ETW captures can be analyzed with `wpa.exe`, `tracelog.exe`, or PerfView.

Standard Windows logging also targets the Application event log via `tracing-eventlog`:

- Source name: `Mara`.
- Event IDs documented in `docs/eventlog-ids.md`.

## Configuration on Windows

Mara reads `MARA_CONFIG` env var and `%PROGRAMDATA%\Mara\mara.toml` by default. Paths in TOML can use forward slashes (Rust's `std::path::Path` handles both on Windows) or escaped backslashes.

Environment-variable expansion uses `${VAR}` syntax, identical to other platforms.

## Permissions

`NT SERVICE\mara` (virtual account) is the default. Specific access requirements:

- File-tail of `%USERPROFILE%\.claude\projects\` — requires per-user install (the scheduled-task pattern) since `NT SERVICE\mara` cannot read another user's profile.
- OTLP receiver bind on `127.0.0.1:4317` — no elevation required.

For multi-user shared Windows hosts that need to ingest all users' transcripts, document the elevation requirement and use a system service with file-system ACLs explicitly granted.

## Firewall

Default install configures Windows Firewall rules:

- Inbound `127.0.0.1:4317` (OTLP gRPC) — localhost only, no firewall rule needed by default.
- Outbound: no rules added (Mara only initiates connections to user-configured sinks).

## Common pitfalls

- **SmartScreen on first-run prompts user**. Expected until reputation is established. Document that the binary is EV-signed.
- **Per-user scheduled task doesn't catch SYSTEM-level AI tools**. Most AI runtimes run in user context, so this is fine; document the edge case.
- **UAC and "elevated" Mara service** are unnecessary for v1. If they ever become necessary, use service triggers (`sc.exe triggerinfo`) rather than elevation.
- **Long paths** (>260 chars) require `LongPathsEnabled = 1` in registry; document for users with deeply nested AI project paths.
- **Antivirus heuristics** can flag any small-binary telemetry tool. Submit the signed binary to Microsoft Security Intelligence for false-positive triage when Defender flags it.

## Self-telemetry on Windows

- Metrics on `127.0.0.1:9099/metrics`.
- Health on `127.0.0.1:9099/healthz`.
- ETW + Event Log for self-logging.
- File logs at `%PROGRAMDATA%\Mara\logs\` for environments that don't consume ETW.

## Update workflow

- `winget upgrade ArdurAI.Mara` — primary user channel.
- `choco upgrade mara` — Chocolatey.
- MSI install with `REINSTALLMODE=amus REINSTALL=ALL` for system-wide updates.

In all cases, the installer stops the service with a 30 s drain, replaces the binary, and restarts. WAL preserves events.

## Uninstall

```powershell
Stop-Service mara
sc.exe delete mara
Remove-Item -Recurse -Force "C:\Program Files\Mara"
Remove-Item -Recurse -Force "C:\ProgramData\Mara"
```

Or via winget: `winget uninstall ArdurAI.Mara`.

## Compliance notes

- EV-signed binaries are a baseline for Windows enterprise rollout.
- BitLocker / EFS-encrypted disks are transparent to Mara.
- For FIPS-mode Windows, Mara needs to use platform crypto (CNG via `rustls-platform-verifier` or `rustls-rustcrypto-fips` once available). Track as a v1.x deliverable.

## Known gaps (v1)

- arm64 Windows: not in v1; tracked for v1.x.
- WSL2 ingest: WSL is a Linux environment; install the Linux Mara inside WSL alongside any Windows-side Mara if needed.
- Windows Server Core: should work but not tested in v1 CI; track in matrix.
