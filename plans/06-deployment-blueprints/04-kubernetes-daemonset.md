# Kubernetes DaemonSet Deployment

## Executive summary

The DaemonSet pattern runs one Mara Pod per Kubernetes node. The Mara Pod receives OTLP from application Pods on the node, optionally tails container logs via hostPath, and ships to cluster or external sinks. This is the right default for fleet-wide AI telemetry in Kubernetes. The Helm chart at `oci://ghcr.io/ardurai/charts/mara` provides production-ready defaults; this document covers the chart values, the underlying manifests, and the operational concerns.

## When to use the DaemonSet pattern

- Fleet-wide telemetry collection from any application Pod that emits OTLP to the local node.
- Container-log scraping (Tier B fallback for runtimes whose only signal is stdout).
- Node-level AI agent telemetry (e.g., a self-hosted Codex CLI in a privileged build container).

When **not** to use DaemonSet: when per-Pod policy isolation is required (use sidecar, [`05-kubernetes-sidecar.md`](05-kubernetes-sidecar.md)).

## Helm install

```bash
helm install mara oci://ghcr.io/ardurai/charts/mara \
  --version 1.0.0 \
  --namespace mara \
  --create-namespace \
  --values values.yaml
```

Minimal `values.yaml`:

```yaml
mara:
  image:
    repository: ghcr.io/ardurai/mara
    tag: "1.0.0"
    pullPolicy: IfNotPresent

  resources:
    requests:
      memory: 256Mi
      cpu: 100m
    limits:
      memory: 768Mi
      cpu: 1000m

  config:
    pipelines:
      - name: cluster_default
        adapters: ["otlp"]
        policy_chain: pii_redact
        sinks: ["loki"]

  adapters:
    otlp:
      grpc_listen: "0.0.0.0:4317"
      http_listen: "0.0.0.0:4318"

  sinks:
    loki:
      url: "http://loki.observability.svc:3100/loki/api/v1/push"

  policy:
    pii_redact:
      - type: redact
        pack: builtin.pii
      - type: sample
        strategy: head
        rate: 1.0

  servicePrometheusEnabled: true
  serviceMonitor:
    enabled: true
    namespace: monitoring
    interval: 30s
```

## DaemonSet manifest (rendered, abridged)

```yaml
apiVersion: apps/v1
kind: DaemonSet
metadata:
  name: mara
  namespace: mara
  labels:
    app.kubernetes.io/name: mara
spec:
  selector:
    matchLabels:
      app.kubernetes.io/name: mara
  updateStrategy:
    type: RollingUpdate
    rollingUpdate:
      maxUnavailable: 10%
  template:
    metadata:
      labels:
        app.kubernetes.io/name: mara
    spec:
      serviceAccountName: mara
      priorityClassName: system-node-critical
      tolerations:
        - operator: Exists
      hostNetwork: false  # default; flip to true only if you need privileged port binding
      containers:
        - name: mara
          image: ghcr.io/ardurai/mara:1.0.0
          args: ["run", "--config", "/etc/mara/mara.toml"]
          ports:
            - name: otlp-grpc
              containerPort: 4317
              hostPort: 4317
              protocol: TCP
            - name: otlp-http
              containerPort: 4318
              hostPort: 4318
              protocol: TCP
            - name: metrics
              containerPort: 9099
              protocol: TCP
          env:
            - name: NODE_NAME
              valueFrom:
                fieldRef:
                  fieldPath: spec.nodeName
            - name: POD_IP
              valueFrom:
                fieldRef:
                  fieldPath: status.podIP
          volumeMounts:
            - name: config
              mountPath: /etc/mara
              readOnly: true
            - name: state
              mountPath: /var/lib/mara
            # optional: hostPath for container-log scraping
            - name: varlog
              mountPath: /var/log
              readOnly: true
            - name: varlibdockercontainers
              mountPath: /var/lib/docker/containers
              readOnly: true
          livenessProbe:
            httpGet:
              path: /healthz
              port: metrics
            initialDelaySeconds: 10
            periodSeconds: 30
          readinessProbe:
            httpGet:
              path: /healthz
              port: metrics
            initialDelaySeconds: 3
            periodSeconds: 10
          resources:
            requests:
              memory: 256Mi
              cpu: 100m
            limits:
              memory: 768Mi
              cpu: 1000m
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
      volumes:
        - name: config
          configMap:
            name: mara-config
        - name: state
          hostPath:
            path: /var/lib/mara
            type: DirectoryOrCreate
        - name: varlog
          hostPath:
            path: /var/log
        - name: varlibdockercontainers
          hostPath:
            path: /var/lib/docker/containers
```

## Service for OTLP receive

Application Pods send OTLP to a node-local endpoint. The simplest pattern is `hostPort` on the DaemonSet (above), plus a `HOST_IP`-driven OTel SDK config:

```yaml
env:
  - name: HOST_IP
    valueFrom:
      fieldRef:
        fieldPath: status.hostIP
  - name: OTEL_EXPORTER_OTLP_ENDPOINT
    value: "http://$(HOST_IP):4317"
```

Alternative: a `ClusterIP` Service with `internalTrafficPolicy: Local` to route OTLP only to the same node's Mara Pod.

## ServiceAccount + RBAC

```yaml
apiVersion: v1
kind: ServiceAccount
metadata:
  name: mara
  namespace: mara
---
apiVersion: rbac.authorization.k8s.io/v1
kind: ClusterRole
metadata:
  name: mara
rules:
  # Only required if Mara enriches events with Pod metadata
  - apiGroups: [""]
    resources: ["pods", "namespaces", "nodes"]
    verbs: ["get", "list", "watch"]
---
apiVersion: rbac.authorization.k8s.io/v1
kind: ClusterRoleBinding
metadata:
  name: mara
subjects:
  - kind: ServiceAccount
    name: mara
    namespace: mara
roleRef:
  apiGroup: rbac.authorization.k8s.io
  kind: ClusterRole
  name: mara
```

If you do not need Pod-metadata enrichment, the ClusterRole/Binding can be omitted entirely (least-privilege default).

## ServiceMonitor (Prometheus Operator)

```yaml
apiVersion: monitoring.coreos.com/v1
kind: ServiceMonitor
metadata:
  name: mara
  namespace: monitoring
spec:
  namespaceSelector:
    matchNames: ["mara"]
  selector:
    matchLabels:
      app.kubernetes.io/name: mara
  endpoints:
    - port: metrics
      interval: 30s
      path: /metrics
```

## NetworkPolicy

```yaml
apiVersion: networking.k8s.io/v1
kind: NetworkPolicy
metadata:
  name: mara
  namespace: mara
spec:
  podSelector:
    matchLabels:
      app.kubernetes.io/name: mara
  policyTypes: ["Ingress", "Egress"]
  ingress:
    - ports:
        - port: 4317
          protocol: TCP
        - port: 4318
          protocol: TCP
      from:
        - podSelector: {}      # any pod in namespace
        - namespaceSelector: {}# or constrain further per your policy
  egress:
    - to:
        - namespaceSelector:
            matchLabels:
              kubernetes.io/metadata.name: observability
      ports:
        - port: 3100
          protocol: TCP
    - to:
        - namespaceSelector:
            matchLabels:
              kubernetes.io/metadata.name: kube-system
      ports:
        - port: 53
          protocol: UDP
```

## Pod Security Admission

The chart sets `securityContext` to satisfy `restricted` Pod Security Standards. No privileged operations required for the default DaemonSet.

## Container-log scraping (optional Tier B)

To tail container logs for AI runtimes that emit only to stdout:

```toml
[[adapters.jsonl]]
name = "k8s_container_logs"
globs = ["/var/log/pods/*/*/*.log"]
parser = "cri"
```

CRI log format (timestamp + stream + log) is parsed by the JSONL adapter's `cri` mode. Container-log scraping requires the hostPath mounts shown in the DaemonSet manifest above.

## Common pitfalls

- **`hostPort` conflicts** if another telemetry agent (Fluent Bit, OTel Collector) is already running on the node. Coexistence pattern: change Mara's hostPort or use ClusterIP+`internalTrafficPolicy: Local`.
- **`hostPath` for state** loses data on node replacement; for clusters with frequent node churn, use a small PVC per node via a `StatefulSet`-shaped pattern instead of a DaemonSet, or accept short-window WAL on emptyDir.
- **Container-log size limits** on EKS / GKE / AKS rotate logs aggressively; tune `kubelet`'s log rotation if Mara is falling behind.
- **PodSecurity `restricted` profile** rejects `runAsUser: 0`; Mara's chart already uses non-root 65532.
- **Service `internalTrafficPolicy: Local`** needs each node to have a Mara Pod (DaemonSet handles this naturally), otherwise app Pods on Mara-less nodes drop OTLP.

## Observability of the agent itself

ServiceMonitor scrapes `metrics` port 9099. A Grafana dashboard at `helm/dashboards/mara-self.json` is shipped with the chart.

## Upgrade workflow

```bash
helm upgrade mara oci://ghcr.io/ardurai/charts/mara --version 1.1.0 --reuse-values
kubectl -n mara rollout status daemonset/mara
```

`RollingUpdate` with `maxUnavailable: 10%` keeps coverage during the rollout.

## Uninstall

```bash
helm uninstall mara --namespace mara
kubectl delete namespace mara
```

## When Mara coexists with the OTel Collector DaemonSet

A common pattern: existing OTel Collector handles metrics + traces from app code; Mara handles AI-specific signals from on-node AI runtimes. Both run as DaemonSets, on different ports. They do not conflict.
