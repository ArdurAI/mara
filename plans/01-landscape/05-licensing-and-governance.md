# Licensing and Governance in the Telemetry Ecosystem

## Executive summary

Telemetry and observability tools have undergone significant licensing turbulence in the 2020s. Several formerly-permissive projects relicensed under source-available terms (SSPL, BSL/BUSL, Elastic License v2, FSL), prompting community forks (OpenSearch, OpenTofu, Valkey). The leading open-source telemetry agents and collectors — Fluentd, Fluent Bit, OTel Collector, Vector — remain under permissive licenses (Apache 2.0, MPL 2.0). Mara is Apache 2.0 to align with this center of gravity and to retain a clear path to CNCF Sandbox.

## License inventory of major projects (May 2026)

### Permissive / open-source

- **Fluentd:** Apache 2.0. Hosted by CNCF (graduated).
- **Fluent Bit:** Apache 2.0. CNCF graduated.
- **OpenTelemetry Collector (core + contrib):** Apache 2.0. CNCF graduated.
- **Prometheus:** Apache 2.0. CNCF graduated.
- **Jaeger:** Apache 2.0. CNCF graduated.
- **Loki:** AGPL 3.0 (Grafana relicensed Grafana, Loki, Tempo, Mimir from Apache 2.0 to AGPL 3.0 in 2024).
- **Tempo:** AGPL 3.0.
- **Mimir:** AGPL 3.0.
- **Grafana:** AGPL 3.0.
- **Vector:** MPL 2.0 (Mozilla Public License).
- **Logstash:** Elastic License v2 / SSPL (dual; was Apache 2.0).
- **Elasticsearch:** Elastic License v2 / SSPL / AGPL 3.0 (re-added AGPL in 2024).
- **OpenSearch:** Apache 2.0 (AWS-led fork of Elasticsearch).
- **Promtail:** AGPL 3.0 (now under Grafana Alloy).
- **Grafana Alloy:** Apache 2.0 (the successor to Grafana Agent + Promtail).
- **OpenTelemetry SDK (all languages):** Apache 2.0.
- **rsyslog:** GPL 3.0 + commercial.
- **syslog-ng:** GPL 2.0 + commercial.
- **NXLog Community Edition:** Apache 2.0 (Enterprise is commercial).
- **Cribl LogStream (free tier):** proprietary; community has limits.

### Recent relicense events

#### Grafana → AGPL (2024)

In Aug 2024, Grafana Labs announced moving Grafana OSS, Loki, Mimir, Tempo, and Pyroscope from Apache 2.0 to AGPL 3.0. Rationale: protect against unfair competition from cloud vendors offering managed Grafana/Loki/etc. without contributing. The Grafana Cloud product remained closed-source; the OSS continued to be developed but under stronger copyleft.

Impact on Mara: Mara's Loki sink consumes Loki's HTTP API; the AGPL of Loki itself does not propagate to Mara because Mara is not a derivative work of Loki. The Mara Loki sink remains Apache 2.0.

#### Elastic → SSPL/Elastic License v2 → re-adding AGPL (2024)

Elastic moved Elasticsearch from Apache 2.0 to SSPL/Elastic License v2 in 2021. AWS forked OpenSearch under Apache 2.0. In 2024, Elastic re-added AGPL 3.0 as a third option alongside SSPL and Elastic License v2 — partly framed as a peace gesture with OSI-leaning communities, though OpenSearch remains the truly OSI-licensed option.

Impact on Mara: Mara's Elasticsearch sink can target either Elasticsearch or OpenSearch (compatible API). Sink license is Apache 2.0; sink targeting an SSPL or AGPL backend is fine since we don't statically link backend code.

#### HashiCorp → BSL/BUSL (2023)

HashiCorp moved Terraform, Vault, Consul, Nomad, etc. from Mozilla Public License to Business Source License 1.1 in Aug 2023. Community forked OpenTofu (Terraform), with the Linux Foundation as steward. Vault's fork (OpenBao) is in CNCF Sandbox.

Impact on Mara: not direct, but the precedent informs Mara's commitment to Apache 2.0 governance. Mara's policy bundles using OPA/Rego (Apache 2.0) are unaffected.

#### Redis → RSAL/SSPL (2024)

Redis Inc. moved Redis 7.4+ from BSD to dual RSAL/SSPL. AWS, Google, Oracle, and the Linux Foundation forked Valkey under BSD (Linux Foundation steward).

Impact on Mara: Mara doesn't use Redis in v1; if it did, we'd target Valkey for the same API. Caching in Mara is in-memory only; v2 gateway may grow optional Redis-compatible state, in which case Valkey is the default.

#### Sentry → FSL (2023)

Sentry adopted the Functional Source License v1.1 with Apache 2.0 transition after 2 years. Sentry SDKs remain MIT.

Impact on Mara: none direct; Sentry is not a Mara dependency. The FSL precedent is interesting for Mara's commercial wing — but Apache 2.0 is the right call for the core agent for ecosystem and CNCF reasons.

#### MongoDB → SSPL (2018, historical context)

The original "SSPL is not OSI-compliant" case. Established the pattern of cloud-vendor protection licenses.

## Governance models

### CNCF (Cloud Native Computing Foundation)

- **Sandbox** → **Incubating** → **Graduated** lifecycle.
- Apache 2.0 required for hosted projects.
- Trademark and IP transferred to LF.
- Mara's path: CNCF Sandbox application drafted in M5; Incubating ≈ 12–18 months after.

### Apache Software Foundation

- Apache 2.0 by definition.
- Strict process: incubation, "graduation," Top-Level Project.
- ASF doesn't typically host telemetry agents; OTel and Fluent* chose CNCF.

### Linux Foundation projects (outside CNCF)

- OpenSearch is now under Linux Foundation umbrella.
- OpenTofu is under Linux Foundation.
- LF projects allow more flexibility in licensing (not strict ALv2) but commit to open development.

### Vendor-led OSS

- Vector (Datadog).
- Loki / Grafana / Tempo / Mimir (Grafana).
- Langfuse (Langfuse Inc.).
- These projects are functionally open but vendor-controlled. Direction can change with corporate priorities.

### Foundation-incubated

- A modern pattern: announce a foundation home (CNCF, LF AI) early to signal intent. Mara's M5 includes the Sandbox application precisely for this reason.

## Patent grant considerations

Apache 2.0 includes an explicit patent grant: contributors grant downstream users a license to any patents necessarily infringed by their contribution. MIT and BSD do not include this; ISC is silent on patents in modern interpretations.

For a telemetry agent that touches many vendor APIs, the patent grant matters: it provides downstream users protection if a contributor later acquires a patent reading on the contribution.

MPL 2.0 (Vector) has a similar grant. GPL 3 has a stronger grant. AGPL 3 inherits GPL 3's.

Mara is Apache 2.0, which gives users this protection without requiring downstream code to be open.

## License compatibility

Mara is Apache 2.0. Acceptable dependencies:

- Apache 2.0 ✓
- MIT / BSD-2-Clause / BSD-3-Clause / ISC / Zlib ✓
- MPL 2.0 ✓ (with the file-level copyleft accepted)
- Unicode-DFS-2016 ✓ (for `unicode-ident` etc.)

Conditionally acceptable:

- GPL-3.0 with linking exception — rarely.
- LGPL-3.0 — dynamic-link only, typically avoid.

Unacceptable:

- GPL-3.0 (without exception) — incompatible with Apache 2.0 for static linking.
- AGPL-3.0 — incompatible; impacts our distribution.
- SSPL, BSL/BUSL, Elastic License v2, Sentry FSL — source-available, not OSI; would prevent CNCF eligibility.
- Commons Clause variants — not OSI; avoided.

`cargo deny` config (in M0) enforces this allowlist.

## License compliance for WASM policy modules

Mara's policy stage loads WASM bundles. The license of a WASM module is the bundle author's responsibility, not Mara's. Mara surfaces the declared license in the policy registry metadata so operators can audit.

Mara's first-party policy bundles (`builtin.pii`, `builtin.phi`, `builtin.pci`) are Apache 2.0.

## License compliance for runtime SDK dependencies

For each AI runtime Mara integrates with, the runtime itself has a license; that license does not propagate to Mara because we use the runtime as a black box (file watch, OTLP receive, hook subprocess). Mara doesn't statically link to runtime code.

- Claude Code: Anthropic proprietary.
- Codex CLI: OpenAI proprietary.
- Cursor: proprietary.
- Kimi CLI: Apache 2.0.
- Augment Code: proprietary.
- Gemini CLI: Apache 2.0.

## SBOM and license attestation

Every Mara release ships:

- A CycloneDX SBOM including license fields per component.
- An SPDX SBOM.
- `cargo deny` license report.
- `cargo about` HTML license report.

The SBOM is the canonical artifact for compliance reviewers.

## License recommendations for Mara

1. **Core agent and all first-party crates: Apache 2.0.** Done in M0.
2. **First-party policy bundles: Apache 2.0.**
3. **First-party WASM SDK: Apache 2.0.**
4. **Documentation (`plans/`, `docs/`): CC BY 4.0** — propose adding this to LICENSE-DOCS.
5. **Helm charts: Apache 2.0.**
6. **Container images: derived works; Apache 2.0 with NOTICE.**
7. **ArdurAI-hosted control plane (v3): proprietary; clean separation from core OSS.**

## Trademark

`Mara` is an ArdurAI trademark. The Apache 2.0 license does not grant trademark rights; this is standard. The trademark policy will be published in `TRADEMARK.md` post-v1, modeled on CNCF trademark policies.

## Governance principles for Mara

1. **Single open license for the core.** Apache 2.0.
2. **No CLA in v1.** DCO (Developer Certificate of Origin) sign-off in commits is sufficient. Re-evaluate if a foundation requires a CLA.
3. **Public roadmap and decision-making.** Material decisions via PR-of-an-ADR.
4. **Maintainer rotation.** Adding non-ArdurAI maintainers as the project grows.
5. **Donate to a foundation when feasible.** CNCF Sandbox in v1.x.

## References

- OSI license list: <https://opensource.org/licenses/>.
- Apache 2.0 text: <https://www.apache.org/licenses/LICENSE-2.0>.
- SLA "What Is Open Source" series: <https://opensource.com/resources/what-open-source>.
- Grafana 2024 relicense: <https://grafana.com/blog/2024/08/29/grafana-relicensing-licensing-changes-faq>.
- Elastic 2024 AGPL move: <https://www.elastic.co/blog/elasticsearch-is-open-source-again>.
- HashiCorp BSL move (2023): <https://www.hashicorp.com/blog/hashicorp-adopts-business-source-license>.
- OpenTofu: <https://opentofu.org>.
- Valkey: <https://valkey.io>.
- CNCF Sandbox process: <https://github.com/cncf/sandbox>.
