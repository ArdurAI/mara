# `/readyz` semantics (M2-09)

Mara’s self-telemetry HTTP server exposes:

- **`GET /healthz`** — process is up (always 200 when the listener is running).
- **`GET /readyz`** — aggregate readiness from configured pipelines (200 vs 503).
- **`GET /metrics`** — Prometheus text.

## What “ready” means today

`mara run` wires `/readyz` to [`pipelines_aggregate_ready`](../../crates/mara-core/src/pipeline.rs): **every** adapter and sink on **every** pipeline must report a health status that [`Health::is_aggregate_ready`](../../crates/mara-core/src/health.rs) accepts as **ready**.

That helper returns **ready** only for [`HealthStatus::Healthy`](../../crates/mara-core/src/health.rs) or [`HealthStatus::Degraded`](../../crates/mara-core/src/health.rs). **`Starting`**, **`Stopping`**, **`Stopped`**, and **`Failed`** all yield **not ready** (503).

The [`Adapter`](../../crates/mara-core/src/traits.rs) and [`Sink`](../../crates/mara-core/src/traits.rs) traits default [`health()`](../../crates/mara-core/src/traits.rs) to [`Health::healthy()`](../../crates/mara-core/src/health.rs), so components that do not implement fine-grained readiness still participate in `/readyz` as ready. Override `health()` to return [`Health::default()`](../../crates/mara-core/src/health.rs) (status `starting`) while warming up, or `stopping` / `stopped` while draining, if you need those phases to block readiness probes.

So `/readyz` means **“every component explicitly reports running (healthy or degraded)”**, not “upstream Ollama is reachable” unless a sink or adapter encodes that into [`Health`](../../crates/mara-core/src/health.rs).

Earlier iterations treated `starting` like ready for aggregate probes; the current rule matches typical Kubernetes expectations.

## Operational guidance

- Use **`/healthz`** for Kubernetes **liveness** (restart if the process wedged).
- Use **`/readyz`** for **readiness** when you accept the semantics above, or override with your own checks if you need stricter warm-up guarantees.
- For the LLM HTTP proxy adapter, health is currently a static **healthy** report; readiness does **not** prove upstream Ollama (or another engine) is reachable—only that Mara’s tasks report ready unless you add richer health to the adapter.

See also: [Kubernetes metrics and probes](../deploy/kubernetes-metrics-and-probes.md).
