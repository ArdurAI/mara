# Privacy capture modes (M1-07)

Mara can strip or fingerprint optional `Event.body` payloads before they reach sinks. Configure a **`privacy`** stage on a policy chain in `mara.toml`.

## TOML

```toml
[[policies.zdr]]
type = "privacy"
mode = "metadata_only"   # optional; this is also the default when omitted
```

Use the same `[[policies.<chain_name>]]` pattern as other stages (see `examples/mara.toml` for `redact` / `sample`).

## Modes

| `mode` | Behavior |
|--------|----------|
| `metadata_only` (default) | Clears `event.body` and `mara.body_hashes` so sinks see metadata only. |
| `hashed_bodies` | Fills `mara.body_hashes` (SHA-256, lowercase hex) from prompt/completion/tool payloads, then clears `event.body`. |
| `body_opt_in` | Keeps `event.body` only when `mara.policy_capture_optin` is `true`; otherwise same as `metadata_only`. |

Stages record a `mara.policy_decisions` entry (`Transformed` for stripping/hashing, `Allowed` when opt-in keeps the body).

## Runtime contract

- **`policy_capture_optin`**: Set by the producing adapter/normalizer when the deployment explicitly allows raw body capture for that event. The privacy stage does not infer consent from headers alone.
- **Order**: Place `privacy` before sinks that must not see raw bodies; combine with `redact` / `sample` as needed (chain order is applied left-to-right in config).

## Tests

Behavior is covered in `crates/mara-policy/src/builtin/privacy.rs` (per-mode unit tests) and `crates/mara-core/src/config.rs` (`parses_privacy_policy_stage` for TOML deserialization).
