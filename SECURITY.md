# Security Policy

## Supported versions

While Mara is pre-1.0 (`0.x.y`), only the latest minor version receives security fixes.

Once 1.0 ships, the latest two minor releases on the current major will receive security fixes.

## Reporting a vulnerability

**Do not file a public GitHub issue for security reports.**

Email `security@ardurai.dev` with:

- A description of the issue and where it lives in the codebase.
- A proof-of-concept or reproduction if you have one.
- The version or commit hash that exhibits the issue.
- Your name and how you'd like to be credited (or that you prefer to remain anonymous).

You should receive an acknowledgement within 3 business days and a triage decision within 10 business days.

## Coordinated disclosure

- We follow a 90-day disclosure window by default.
- For actively exploited issues we may publish earlier.
- We will request a CVE for any vulnerability with a CVSS v3.1 base score of 4.0 or higher.

## Scope

In scope:

- The Mara agent binary and all crates in this repository.
- Official container images and release artifacts published by ArdurAI.
- Default Mara-published policy bundles and signed configurations.

Out of scope (report to the upstream maintainer instead):

- Vulnerabilities in third-party AI runtimes (Claude Code, Codex, Cursor, Kimi, Augment, Gemini, etc.).
- Vulnerabilities in user-supplied policy WASM modules.
- Vulnerabilities in user-operated sink backends.

## Hardening guarantees we make

- All releases ship with SLSA Level 2 build provenance.
- All releases ship with CycloneDX and SPDX SBOMs.
- All release artifacts are signed with `cosign` keyless signatures.
- The agent ships with zero phone-home telemetry by default. All telemetry destinations are user-configured.
- Prompt and raw-API-body capture is opt-in everywhere and respects each runtime's ZDR toggle.
