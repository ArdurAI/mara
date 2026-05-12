# Kubernetes Sidecar Deployment

## Executive summary

The sidecar pattern places a Mara container inside the same Pod as an AI-using application container, sharing the Pod's network namespace and (optionally) a volume. This is appropriate when per-app policy isolation is required: different teams in the same cluster need different redaction profiles, different sinks, different tenant attribution, or when an app needs ZDR-strict capture defaults that the cluster-wide DaemonSet cannot enforce.

For most users, the DaemonSet pattern in [`04-kubernetes-daemonset.md`](04-kubernetes-daemonset.md) is preferable. Use the sidecar pattern only when the isolation requirement is real.

## When to use sidecar

- Per-app policy bundles (e.g., team-A redacts PII at higher strictness than team-B).
- Tenant-bounded resource limits (a hot path can't starve a quiet one).
- Per-app ZDR mode (one app captures prompts; another never does).
- Compliance scopes (a PCI-handling app needs an audit log distinct from the cluster's).

## When not to use sidecar

- You want centralized policy management → use DaemonSet + gateway (v2).
- Memory budget per Pod is tight → DaemonSet amortizes Mara's footprint across all Pods on the node.
- Most apps in the cluster have the same telemetry needs → DaemonSet.

## Pod spec (sidecar)

```yaml
apiVersion: v1
kind: Pod
metadata:
  name: agent-service-xyz
  labels:
    app: agent-service
spec:
  shareProcessNamespace: false
  containers:
    - name: agent
      image: my-ai-agent:1.4.0
      env:
        # App emits OTLP to the sidecar over loopback
        - name: OTEL_EXPORTER_OTLP_ENDPOINT
          value: "http://127.0.0.1:4317"
        - name: OTEL_SERVICE_NAME
          value: "agent-service"
        - name: OTEL_RESOURCE_ATTRIBUTES
          value: "tenant.id=$(TENANT_ID),env=$(ENVIRONMENT)"
      ports:
        - containerPort: 8080
          name: http

    - name: mara
      image: ghcr.io/ardurai/mara:1.0.0
      args: ["run", "--config", "/etc/mara/mara.toml"]
      ports:
        - containerPort: 9099
          name: mara-metrics
      env:
        - name: POD_NAME
          valueFrom:
            fieldRef:
              fieldPath: metadata.name
        - name: NAMESPACE
          valueFrom:
            fieldRef:
              fieldPath: metadata.namespace
      volumeMounts:
        - name: mara-config
          mountPath: /etc/mara
          readOnly: true
        - name: mara-wal
          mountPath: /var/lib/mara
      resources:
        requests:
          memory: 96Mi
          cpu: 50m
        limits:
          memory: 256Mi
          cpu: 250m
      securityContext:
        runAsUser: 65532
        runAsGroup: 65532
        runAsNonRoot: true
        readOnlyRootFilesystem: true
        allowPrivilegeEscalation: false
        capabilities:
          drop: ["ALL"]
        seccompProfile:
          type: RuntimeDefault
      livenessProbe:
        httpGet:
          path: /healthz
          port: mara-metrics
        initialDelaySeconds: 10
        periodSeconds: 30

  volumes:
    - name: mara-config
      configMap:
        name: mara-agent-config
    - name: mara-wal
      # See WAL persistence section below
      emptyDir:
        sizeLimit: 200Mi
```

## Sidecar config (per-app policy profile)

```toml
[server]
metrics_addr = "127.0.0.1:9099"

[[adapters.otlp]]
name = "app_otlp"
grpc_listen = "127.0.0.1:4317"

[[pipelines]]
name = "agent_pipeline"
adapters = ["app_otlp"]
policy_chain = "strict_pii"
sinks = ["cluster_loki"]

[[policies.strict_pii]]
type = "redact"
pack = "builtin.pii"
[[policies.strict_pii]]
type = "redact"
pack = "builtin.phi"
[[policies.strict_pii]]
type = "deny"
attribute = "gen_ai.tool.name"
match = "regex"
pattern = "^internal\\."

[[sinks.cluster_loki]]
name = "cluster_loki"
url = "http://loki.observability.svc:3100/loki/api/v1/push"
```

## WAL persistence considerations

emptyDir is lost on Pod restart. For sidecar deployments where ungraceful Pod restart is a real concern:

- Use `emptyDir.medium: Memory` + tighter WAL budget for ephemeral capture (events not in WAL are lost on restart).
- Or mount a `PersistentVolumeClaim` per Pod (only practical with `StatefulSet`, not `Deployment`).
- Or accept higher loss tolerance and rely on the gateway tier (v2) to absorb sidecar reboots.

A pragmatic default: emptyDir 200 MiB, WAL budget 150 MiB, accept that violently-killed Pods may lose up to a few minutes of buffered events.

## Resource budgeting

Per-sidecar baseline: 96 MiB request, 256 MiB limit. At 50 sidecars per node this is ≈ 5 GiB request, ≈ 13 GiB limit dedicated to Mara — significant. DaemonSet amortizes this to ≈ 256–768 MiB per node total.

The math justifies sidecar only when the per-Pod isolation is worth the multiplied cost.

## Pod Security Standards

Same as DaemonSet — Mara's container satisfies the `restricted` profile.

## Networking

- App talks to sidecar over `127.0.0.1` — no Service, no NetworkPolicy ingress rule needed.
- Sidecar talks to sink — same NetworkPolicy egress as in DaemonSet pattern.
- Cross-Pod traffic between Mara sidecars (e.g., for clustering): not used in v1. If a sidecar wants to ship to a per-Namespace aggregator (v2 gateway), add an egress rule for that endpoint.

## ConfigMap layout

For per-app policy profiles, ConfigMap names follow `mara-<app>-config` so policies can be templated by Helm or generated by an admission controller.

## Common pitfalls

- **App OTel SDK pre-aggregating** before Mara sees it. Use OTel SDK in BatchSpanProcessor mode with reasonable batch sizes; avoid `OTEL_TRACES_SAMPLER=parentbased_traceidratio` set to anything below 1.0 unless Mara is intentionally part of the sampling decision.
- **Sidecar OOM kills app's Pod** if memory limits are shared too tightly. The example above sets per-container limits, so a sidecar OOM kills only the sidecar; kube-proxy/CNI rules keep the app running.
- **Loopback OTLP fast path**: gRPC over loopback is the fastest option. HTTP/protobuf works too. Don't use HTTP/JSON in sidecar mode — extra serialization cost.
- **Restart ordering**: `restartPolicy: Always` doesn't sequence sidecar before app on Pod start. For most workloads this is fine; the OTel SDK retries. For strict ordering, use the Kubernetes 1.28+ sidecar containers feature (`restartPolicy: Always` on a `initContainers` entry — actually a true sidecar with lifecycle).
- **Native sidecar (k8s 1.28+)**: prefer the native sidecar lifecycle (`restartPolicy: Always` on init container) for proper startup/shutdown sequencing. Mara's Helm chart conditionally renders this when `kube-version >= 1.28`.

## Multi-tenant sidecar pattern

For a SaaS that puts each tenant in its own Pod:

```yaml
env:
  - name: TENANT_ID
    valueFrom:
      configMapKeyRef:
        name: tenant-info
        key: id
```

Mara picks up `tenant.id` from OTLP resource attributes and uses it for policy selection:

```toml
[[pipelines]]
name = "tenant_aware"
adapters = ["app_otlp"]
policy_selector.attribute = "tenant.id"
policy_profiles = { default = "strict_pii", tenant_a = "pci_strict", tenant_b = "minimal" }
sinks = ["cluster_loki"]
```

This is a v1 feature. Multi-tenant policy bundle distribution is v2.

## Observability of the sidecar

Per-sidecar metrics expose at `127.0.0.1:9099/metrics`. To aggregate, run a separate Prometheus that scrapes each Pod's sidecar metrics endpoint via a `PodMonitor`:

```yaml
apiVersion: monitoring.coreos.com/v1
kind: PodMonitor
metadata:
  name: mara-sidecars
  namespace: monitoring
spec:
  selector:
    matchExpressions:
      - { key: app, operator: Exists }
  podMetricsEndpoints:
    - port: mara-metrics
      interval: 30s
      path: /metrics
```

## Comparison to admission-controller injection

A common k8s pattern is a Mutating Admission Webhook that auto-injects the Mara sidecar (à la Istio, Linkerd). This is a v1.x consideration. v1 ships a plain Helm chart pattern (Pod spec includes the sidecar by author choice).

## Migration path from sidecar to DaemonSet

If a team starts with sidecar and later wants to move to DaemonSet:

1. Deploy the DaemonSet alongside; verify it receives traffic from a test workload.
2. Configure the app's OTel SDK endpoint to point to `http://$(HOST_IP):4317` instead of `http://127.0.0.1:4317`.
3. Remove the sidecar from the Pod spec.
4. Verify telemetry continuity.

Mara's canonical schema is identical in both modes; downstream sinks see no shape difference.
