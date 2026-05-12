# Security and Compliance for Telemetry Agents

## Executive summary

Telemetry agents sit close to sensitive data: they read application logs, sometimes prompts, sometimes raw API bodies, often credentials in passing. They run with file-system access and network egress. Compromise of an agent is high-impact. This document catalogs the threat model for telemetry agents, the supply-chain standards that apply, common CVE patterns in this category, and the compliance frameworks Mara aligns with.

## Threat model for a telemetry agent

### Attack surfaces

1. **File-system reads.** Adapters that tail user files can be tricked into reading wrong files (symlink traversal, race conditions).
2. **Network sockets.** OTLP receivers listen; analytics REST adapters dial out. Both are attack vectors.
3. **Config tampering.** A modifiable config file is an injection point (logs read from `/etc/shadow`, sinks pointed at attacker-controlled URLs).
4. **Policy bundle compromise.** Unsigned bundles can be replaced. Signed bundles with stolen keys can be replaced.
5. **WASM sandbox escape.** Malicious or buggy WASM policy modules trying to escape `wasmtime`.
6. **Supply chain.** Compromised dependencies, compromised CI, compromised release tooling, compromised distribution channel (Homebrew tap, deb repo).
7. **Log injection.** Crafted log lines that exploit downstream tools (the agent itself, sinks, viewers).
8. **DNS / egress data exfiltration.** A compromised agent could exfiltrate via DNS queries or HTTP to attacker servers.

### STRIDE per surface

- **Spoofing:** signed releases, signed policy bundles, mTLS for agent ↔ gateway.
- **Tampering:** config integrity via permissions; policy bundle verification; tamper-evident audit log.
- **Repudiation:** audit log with append-only + Merkle commits.
- **Information disclosure:** ZDR-aware capture defaults; PII redaction at agent; opt-in raw body capture.
- **Denial of service:** bounded queues, backpressure, rate limits, regex linear-time guarantee.
- **Elevation of privilege:** run as unprivileged user; capability-scoped file reads; WASM sandbox; no shell-out by default.

## Common CVE patterns in telemetry agents

Historical CVEs across Fluentd, Fluent Bit, Logstash, Filebeat, OTel components, and Splunk forwarders, 2020–2026:

- **Path traversal in input plugins** — file paths derived from untrusted input.
- **Regex denial-of-service** — backtracking regex on adversarial input.
- **YAML deserialization** — unsafe loading allowing object instantiation.
- **TLS verification disabled by default** — vendor "convenience" defaults; later corrected.
- **Hard-coded credentials** in example configs that ship in releases.
- **Container running as root** — defaults exploited in multi-tenant clusters.
- **Plugin loader without signature check** — arbitrary code execution via "plugin install."
- **HTTP receivers without auth** — open ingestion endpoints in default configs.
- **Log4j-class injection** in JVM-based agents (Logstash) — fixed in 2021–2022.
- **`hostPath` mount over-broad** in Kubernetes deployments — host file read by Pod.

Mara's countermeasures address each pattern by design:

- File-tail adapters use canonical-path resolution; symlinks rejected by default with explicit opt-in.
- Built-in regexes vetted for linear time via the `regex` crate's guarantees.
- TOML/YAML config parses to typed Rust; no arbitrary object instantiation possible.
- TLS verification on by default; `insecure_skip_verify` requires explicit per-sink opt-in with startup warning.
- Example configs use clearly-marked placeholders, not credentials.
- Containers run as UID 65532 nonroot by default.
- Plugins (WASM) require `cosign` verification; no `--allow-unsigned-policy` by default.
- OTLP receiver binds to `127.0.0.1` by default; opt-in to wider binds.
- Mara is Rust — no JVM; no log4j-class issues. Logging uses `tracing` which formats safely.
- Kubernetes Helm chart uses minimal hostPath mounts (only `/var/log` and `/var/lib/docker/containers` when container-log scraping is enabled; readOnly).

## Supply chain standards

### SLSA (Supply-chain Levels for Software Artifacts)

- **Level 1:** documentation of build process.
- **Level 2:** version-controlled source, scripted build, provenance generated and stored.
- **Level 3:** non-falsifiable provenance, hardened build platform, isolated builds.
- **Level 4 (deprecated naming in v1.0, replaced with build track and source track levels).**

Mara v1: SLSA Level 2 via `slsa-github-generator`.
Mara v2: SLSA Level 3 target.

References: <https://slsa.dev>.

### SBOM (Software Bill of Materials)

- **CycloneDX:** OWASP-led standard.
- **SPDX:** Linux Foundation standard.

Mara publishes both per release. Tools:

- `cargo cyclonedx` for CycloneDX.
- `cargo sbom` for SPDX.

References: <https://cyclonedx.org>, <https://spdx.dev>.

### sigstore / cosign

- **cosign:** signing CLI.
- **Fulcio:** keyless certificate authority.
- **Rekor:** transparency log.

Mara signs every release artifact with `cosign sign-blob` (keyless via GitHub OIDC). Verifiers use `cosign verify-blob`.

References: <https://www.sigstore.dev>.

### Reproducible builds

A reproducible build means the same source produces byte-identical artifacts on independent rebuilds. Mara targets best-effort reproducibility:

- Linux glibc and musl static targets: high feasibility.
- macOS universal2: harder due to code-signing and notarization.
- Windows MSI: harder due to signing.

References: <https://reproducible-builds.org>.

### in-toto attestations

Mara releases include in-toto provenance attestations describing the build environment. Stored in Rekor.

References: <https://in-toto.io>.

## Compliance frameworks

### SOC 2 Type I / Type II

- **Type I:** point-in-time design of controls.
- **Type II:** operating effectiveness over 6–12 months.
- Mara targets Type I at M5; Type II post-v1.
- See [`../05-evaluation/03-soc2-control-mapping.md`](../05-evaluation/03-soc2-control-mapping.md).

### ISO 27001:2022

- Information Security Management System.
- Annex A controls (93 controls).
- ~70% overlap with SOC 2 Common Criteria.
- Post-v1 target.

### HIPAA (US healthcare)

- PHI definition: 18 identifiers.
- Technical safeguards: access control, audit, integrity, transmission.
- Business Associate Agreement (BAA) required for vendors handling PHI.
- Mara's `builtin.phi` redaction pack and audit log support HIPAA workflows; ArdurAI does not sign BAAs in v1 (we're a software vendor, not a service provider).

### PCI-DSS v4.0

- Cardholder data environment (CDE).
- Tokenization preferred over storage.
- Mara's `builtin.pci` redaction pack supports CDE-adjacent workflows; the operator's deployment determines actual PCI scope.

### GDPR (EU)

- Data subject rights, lawful basis, data minimization, breach notification.
- Mara's redaction primitives, ZDR-aware defaults, and audit log support compliance evidence.
- DPA (Data Processing Agreement): not applicable to Mara as software; operator's deployment governs.

### CCPA / CPRA (California)

- Similar contours to GDPR for California residents.
- Same Mara support.

### FedRAMP / StateRAMP

- US government cloud authorization.
- Moderate baseline ≈ 325 controls (NIST 800-53).
- High baseline ≈ 421 controls.
- Out of v1 scope; v1.x → v2.0 consideration if there's demand.

### C5 (Germany) / IRAP (Australia)

- National-level cloud security frameworks.
- Out of v1 scope.

### EU AI Act + NIST AI RMF

- See [`../05-evaluation/04-eu-ai-act-alignment.md`](../05-evaluation/04-eu-ai-act-alignment.md).

## Network security

- **TLS 1.3 mandatory** for all network sinks; TLS 1.2 fallback with explicit opt-in.
- **Certificate verification** on by default.
- **mTLS** supported on OTLP, gateway client, webhook sink.
- **OIDC** for control plane (v2/v3) via standard providers.
- **Per-sink credential providers**: env, file, AWS IAM/IRSA, GCP service account, Azure managed identity, Vault.

## Runtime governance

- **WASM sandboxing** with `wasmtime`. No network, no filesystem, deterministic time inside the sandbox.
- **Policy bundle signing** via `cosign`.
- **Tamper-evident audit log** via append-only structure with periodic Merkle root export.
- **Policy attestation:** signed policies have an attestation referenceable by the sink.

## Container image security

- **Distroless base:** no shell, no package manager, no debugging tools in default image.
- **Non-root user:** UID 65532.
- **Read-only root filesystem:** enforceable in Kubernetes Pod spec.
- **Image signing:** `cosign` on every release.
- **SBOM in image:** SBOM attached as an OCI artifact.
- **Vulnerability scanning:** Trivy, Grype, Clair, Snyk supported. CI gates on high/critical.

## Recent (2025–2026) security events in observability tools

- **Snyk advisories on Fluent Bit input plugins** — historical pattern of path-traversal; Mara avoids by canonical-path resolution.
- **OpenTelemetry Collector receiver DoS issues** — DoS via malformed protobuf; Mara's OTLP receiver uses `tonic` with bounded message size.
- **Vector deserialization concerns** — addressed upstream; Mara's serde usage is `deny_unknown_fields` where strict, lenient where forgiving.

## Mara's M4 baseline security checklist

- ✓ `cargo-audit` in CI.
- ✓ `cargo-deny` in CI (license + ban + advisories + sources).
- ✓ `cargo-vet` baseline.
- ✓ OSV scanner in CI.
- ✓ Trivy fs + image in CI.
- ✓ STRIDE threat model published in `docs/threat-model.md`.
- ✓ SBOM (CycloneDX + SPDX) per release.
- ✓ SLSA Level 2 provenance per release.
- ✓ `cosign` keyless signatures per release.
- ✓ Default-non-root container.
- ✓ Distroless base image.
- ✓ Signed policy bundles.
- ✓ Tamper-evident audit log.
- ✓ ZDR-respecting defaults per runtime.
- ✓ Zero phone-home telemetry by default.

## Compliance roadmap

- **M5:** SOC 2 Type I control mapping draft; evidence pipeline scaffolded.
- **v1.0:** Apache 2.0 + Trademark Policy.
- **v1.1:** SOC 2 Type I audit kickoff; CNCF Sandbox application.
- **v1.2 – v1.3:** SOC 2 Type II observation window; SLSA Level 3.
- **v2.0:** SOC 2 Type II complete; CNCF Sandbox accepted; ISO 27001 Annex A control mapping.
- **v3.0:** Hosted control plane gets its own audit (SOC 2 Type II, FedRAMP-eligible).

## References

- NIST 800-53: <https://csrc.nist.gov/publications/detail/sp/800-53/rev-5/final>.
- NIST 800-218 SSDF: <https://csrc.nist.gov/publications/detail/sp/800-218/final>.
- SLSA: <https://slsa.dev>.
- sigstore: <https://www.sigstore.dev>.
- CIS Docker Benchmark.
- CNCF Security TAG: <https://github.com/cncf/tag-security>.
- OWASP LLM Top 10: <https://genai.owasp.org/llm-top-10>.
- MITRE ATLAS: <https://atlas.mitre.org>.
