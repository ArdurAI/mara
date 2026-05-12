# Security Advisories Process

This document describes how Mara handles confirmed security vulnerabilities. The user-facing reporting policy lives in [`../SECURITY.md`](../SECURITY.md); this document is for maintainers.

## Severity classification

We use CVSS v3.1 base scores, with these informal labels:

- **Critical** (CVSS 9.0–10.0): immediate emergency response; out-of-cycle patch within 48 hours; coordinated disclosure with downstream packagers.
- **High** (7.0–8.9): patch within 7 days; release notes call attention; CVE filed.
- **Medium** (4.0–6.9): patch within 30 days; included in the next planned release.
- **Low** (0.1–3.9): patch within 90 days or as opportunity allows.

## Workflow

1. Report received at `security@ardurai.dev`.
2. Acknowledge within 3 business days; assign tracking number.
3. Reproduce; assess severity.
4. Develop fix in a private fork or with a private branch + non-public CI.
5. File CVE if CVSS ≥ 4.0.
6. Prepare release with the fix.
7. Notify downstream packagers (Homebrew tap, deb/rpm repo, Helm chart) under embargo.
8. Public release + disclosure on or after the embargo date.
9. Post-mortem published in `docs/security-postmortems/<date>-<short-name>.md`.

## CVE filing

We file CVEs through the GitHub Security Advisory mechanism (`gh security advisory`). The advisory text includes:

- Affected versions.
- Attack vector and complexity.
- Impact summary.
- Workarounds (if any).
- Acknowledgements (when the reporter consents).

## Embargo periods

- Default: 14 days from confirmed fix to public disclosure.
- Maximum: 90 days from initial report (per industry coordinated-disclosure norms).
- We may publicly disclose earlier if active exploitation is observed.

## Hardening backports

Fixes for High and Critical issues are backported to all supported minor versions per the support policy in `SECURITY.md`.

## References

- CVSS v3.1: <https://www.first.org/cvss/v3-1/>.
- GitHub Security Advisories: <https://docs.github.com/en/code-security/security-advisories>.
- ISO/IEC 30111 (vulnerability handling).
