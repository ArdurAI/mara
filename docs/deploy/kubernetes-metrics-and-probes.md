# Kubernetes metrics and probes (M2-12)

Mara exposes:

- `GET /healthz` — process liveness (always 200 when the HTTP server is up).
- `GET /readyz` — aggregate adapter/sink readiness (200 only when every component reports **healthy** or **degraded**; 503 otherwise). Wired automatically when using `mara run`. Details: [`/readyz` semantics](../observability/mara-readyz-semantics.md).
- `GET /metrics` — Prometheus text (self-telemetry).

## Example Deployment fragment

```yaml
apiVersion: apps/v1
kind: Deployment
spec:
  template:
    spec:
      containers:
        - name: mara
          image: your-registry/mara:latest
          args: ["run", "--config", "/etc/mara/mara.toml"]
          ports:
            - name: metrics
              containerPort: 9099
          env:
            - name: MARA_SERVICE_NAME
              value: "mara"
          livenessProbe:
            httpGet:
              path: /healthz
              port: metrics
            initialDelaySeconds: 10
            periodSeconds: 20
          readinessProbe:
            httpGet:
              path: /readyz
              port: metrics
            initialDelaySeconds: 5
            periodSeconds: 10
---
apiVersion: v1
kind: Service
metadata:
  name: mara-metrics
spec:
  selector:
    app: mara
  ports:
    - name: metrics
      port: 9099
      targetPort: metrics
```

Set `[server] metrics_addr = "0.0.0.0:9099"` (or a pod-only interface) so Prometheus can scrape the `Service`. For non-loopback binds, Mara defaults to **64** concurrent metrics HTTP tasks unless overridden by `[server] metrics_max_in_flight_connections` (M2-15).
