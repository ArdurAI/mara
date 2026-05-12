# ADR-0001: License Mara under Apache 2.0

- **Status:** Accepted
- **Date:** 2026-05-12
- **Authors:** Mara M0 scope-lock review.

## Context

The Mara repository was bootstrapped under the MIT License with a single initial commit attributing copyright to ArdurAI. The MOS plan and the wider plans encyclopedia recommend Apache 2.0 as the appropriate license for an AI-native telemetry agent that we intend to release publicly and (likely) donate to a foundation.

## Decision

Mara core and all first-party crates are licensed under the Apache License, Version 2.0. The MIT LICENSE that shipped with the initial commit has been replaced; relicensing is unproblematic because the repository had no external contributors at the time of the change.

## Alternatives considered

- **MIT (status quo).** Pros: simplest, permissive. Cons: no patent grant; less aligned with the AI-native telemetry ecosystem (Fluent Bit, OpenTelemetry Collector, Vector are MPL/Apache). Rejected on patent-grant grounds.
- **AGPL 3.0.** Pros: strong protection against unmodified cloud reselling. Cons: incompatible with many corporate adoption policies; not eligible for CNCF; aligns Mara more with Grafana's Loki than with the broader CNCF ecosystem we're targeting. Rejected.
- **Business Source License 1.1 with Apache 2.0 transition.** Pros: source-available with eventual openness; HashiCorp / Sentry-style protection. Cons: not OSI-approved during BSL period; not eligible for CNCF Sandbox; community signal is poor for telemetry projects. Rejected for the core; revisitable for hosted control plane (v3).
- **Dual MIT/Apache-2.0.** Pros: common in Rust ecosystem. Cons: adds maintenance burden for a project where Apache 2.0 alone is sufficient. Rejected.

## Consequences

- All current and future dependencies must be license-compatible with Apache 2.0. AGPL/SSPL/Elastic License v2 are excluded. `cargo deny` enforces.
- The patent grant in Apache 2.0 §3 applies. Contributors grant downstream users a patent license for any patents necessarily infringed by their contribution.
- A `NOTICE` file is maintained alongside `LICENSE` per Apache 2.0 §4(d). All third-party attributions land there.
- The license aligns Mara with CNCF requirements; a CNCF Sandbox application can be filed in M5 / v1.x without an additional relicense.
- Contributors do not sign a CLA in v1; DCO sign-off (`git commit -s`) is required.

## References

- [`LICENSE`](../../LICENSE).
- [`NOTICE`](../../NOTICE).
- [`plans/01-landscape/05-licensing-and-governance.md`](../../plans/01-landscape/05-licensing-and-governance.md).
- Apache License 2.0: <https://www.apache.org/licenses/LICENSE-2.0>.
- Developer Certificate of Origin: <https://developercertificate.org>.
