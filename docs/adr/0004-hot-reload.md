# ADR-0004: Hot reload via SIGHUP + inotify with debounce

- **Status:** Accepted
- **Date:** 2026-05-12
- **Authors:** Mara M1 architecture review.

## Context

Operators expect to change Mara's configuration without restarting the agent — adding a sink, modifying a policy, changing an OTLP listener port — and have the change apply with minimal disruption. Restarts cost a WAL replay window and brief telemetry coverage gaps; hot reload avoids them.

Hot reload also needs to be safe: a malformed configuration must not take down a running agent.

## Decision

Mara implements hot reload triggered by two signals:

1. **SIGHUP** (POSIX) — explicit operator request to re-read configuration.
2. **inotify / FSEvents / ReadDirectoryChangesW** on the configuration file's parent directory, debounced by 500 ms to coalesce rapid edits.

On either signal:

1. The configuration is parsed and validated against the JSON Schema.
2. If validation fails, the running configuration is unchanged; an error is emitted to stderr and the metric `mara_config_reload_failures_total` is incremented.
3. If validation succeeds, the new configuration is diffed against the running configuration. The pipeline reconciler:
   - Adds new adapters / sinks / policies.
   - Removes obsolete components (draining in-flight events first).
   - Updates parameters of components whose identity has not changed.
4. The reload is logged with a config-version hash for audit purposes.

The reload mechanism is implemented in `mara-core::config::reload`.

## Alternatives considered

- **Restart-only.** Pros: simplest. Cons: violates operator expectations; causes WAL replay; brief coverage gap. Rejected.
- **HTTP control endpoint.** Pros: programmatic. Cons: extra surface to secure; adds a binding port; not the conventional Unix idiom for daemons. Decision: not in v1 by default; possibly v1.x as an opt-in management API.
- **Polling the config file periodically.** Pros: no platform-specific watching. Cons: latency; wasted I/O. Rejected.
- **Config-pushed via control plane.** Pros: fits v3 hosted-control-plane story. Cons: deployment-dependent; not what edge operators want. Decision: v2 gateway can push; v1 stays file-based.

## Consequences

- The configuration loader runs in a separate task; its panic does not affect the data plane.
- The reconciler logic in `mara-core::pipeline` must support graceful component swap-out — implemented via task-cancellation tokens.
- inotify watcher needs cross-platform support; the `notify` crate (Apache 2.0 / MIT) is the dependency.
- Documentation: every config option has a "hot-reloadable" flag in the documentation. Options that require a restart (e.g., changing the metrics bind port mid-flight) are documented as such.
- Operators are encouraged to use `mara validate` before reloading to catch errors before SIGHUP.

## References

- [`plans/04-implementation/01-functional-requirements.md`](../../plans/04-implementation/01-functional-requirements.md) FR-1.3.
- [`notify` crate](https://github.com/notify-rs/notify).
- Unix daemon SIGHUP convention.
