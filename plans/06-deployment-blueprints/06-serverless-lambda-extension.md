# Serverless and Lambda Extension Deployment

## Executive summary

Serverless environments — AWS Lambda, Google Cloud Run, Azure Functions, Cloudflare Workers — pose unique constraints for a telemetry agent: ephemeral lifecycle, no persistent disk, strict execution-time budgets, and limited file-system access. Mara's serverless story in v1 is the AWS Lambda Extension pattern via the Telemetry API and Logs API, with documentation and minimal scaffolds for Cloud Run sidecars and Azure Functions. Cloudflare Workers and similar edge runtimes do not support running a separate process; for those, the recommended pattern is to forward OTLP from the Worker to a remote Mara endpoint.

## AWS Lambda Extension

A Mara Lambda Extension runs in the same execution environment as the function, registers with the Extensions API, subscribes to the Telemetry API (function logs + extension logs + platform metrics), normalizes events, and ships to a configured sink before the execution environment terminates.

### Form factor

- Mara is packaged as an Extension layer: a zip with `extensions/mara-extension` (a script that execs the Mara binary in extension mode) and `bin/mara` (the binary).
- Layer published per region: `arn:aws:lambda:<region>:<account>:layer:mara-extension:<version>`.

### Attach to a function

```bash
aws lambda update-function-configuration \
  --function-name my-ai-agent \
  --layers arn:aws:lambda:us-east-1:123456789012:layer:mara-extension:1
```

Environment variables for configuration:

```
MARA_CONFIG = s3://my-bucket/mara/lambda.toml
MARA_SINK_OTLP_ENDPOINT = https://otlp.honeycomb.io:443
MARA_SINK_OTLP_HEADERS = x-honeycomb-team=...
```

### Extension lifecycle

1. **`INIT`**: extension registers with the Extensions API, reads config from S3 (or env), opens sink connections.
2. **`INVOKE`** (one or more): function runs; extension receives platform + function logs via the Telemetry API stream.
3. **`SHUTDOWN`**: extension receives a deadline (≤2 s typical); Mara flushes WAL-in-memory contents to the sink synchronously with a strict time budget.

### Constraints in Lambda

- No persistent disk: WAL lives in `/tmp` (up to 10 GiB but ephemeral and per-execution-environment). For zero-loss across cold starts, configure synchronous OTLP-with-ack sink.
- Strict shutdown deadline: Mara's flush MUST respect `Shutdown.deadlineMs`. Configurable budget.
- Cold-start overhead: extension adds ≈ 50–150 ms at INIT. Mitigation: pre-warm via reserved concurrency.
- Cost: extension execution time is billed; Mara's idle CPU between INVOKEs is minimal but non-zero.

### Telemetry API subscription

Mara subscribes to `platform`, `function`, and `extension` event streams. The function logs include any OTel SDK output the function emits; the extension correlates by Lambda request id and ships under the canonical schema.

### What Mara captures from Lambda

- Function logs (`/aws/lambda/<function>` content) — body, level, timestamp.
- Function-emitted OTLP if the function uses an OTel SDK pointed at the extension's local OTLP receiver (extension exposes `127.0.0.1:4318/v1/logs` etc.).
- Platform metrics — duration, billed duration, init duration, memory used, restore duration.
- Cold start indicator.

### Recommended sinks for Lambda

- OTLP to a remote backend (Honeycomb, Datadog, Grafana Cloud, self-hosted Tempo) — most common.
- Kinesis Firehose → S3 (Parquet) — high-volume, low-latency.
- Direct S3 PutObject — for small volumes, costly per-call.

Avoid sinks with TCP-keep-alive expectations across cold starts (Splunk HEC works because it's HTTP/2 over TLS; Kafka via librdkafka is overkill).

## Google Cloud Run sidecars

Cloud Run supports multi-container deployments (sidecars) since 2024 preview, GA in 2025. Mara as a sidecar in Cloud Run mirrors the Kubernetes sidecar pattern with one quirk: Cloud Run scales to zero, so the sidecar shares cold-start cost with the app.

```yaml
apiVersion: run.googleapis.com/v1
kind: Service
metadata:
  name: agent-service
spec:
  template:
    metadata:
      annotations:
        run.googleapis.com/container-dependencies: |
          {"agent": ["mara"]}
    spec:
      containers:
        - name: mara
          image: ghcr.io/ardurai/mara:1.0.0
          args: ["run", "--config", "/etc/mara/mara.toml"]
          ports:
            - containerPort: 4317
              name: otlp-grpc
          resources:
            limits:
              memory: 256Mi
              cpu: "0.25"
        - name: agent
          image: gcr.io/my-project/agent:1.0
          ports:
            - containerPort: 8080
              name: http
          env:
            - name: OTEL_EXPORTER_OTLP_ENDPOINT
              value: "http://127.0.0.1:4317"
```

### Cloud Run constraints

- 60-minute max request duration applies to the container; long-running pipelines fit.
- No persistent disk; emptyDir-equivalent is `/tmp` (limited).
- WAL persistence is limited; same considerations as Lambda.
- Egress through Cloud NAT or Serverless VPC Connector for sinks behind VPCs.

## Azure Functions

Azure Functions Premium plan supports sidecars (preview as of late 2025). Pattern mirrors Cloud Run.

For the Consumption plan (truly serverless, no sidecar support), the recommended pattern is to forward telemetry from the Function App to a remote Mara endpoint over OTLP/HTTPS.

## Cloudflare Workers and edge runtimes

Workers do not support running a separate process. The recommended pattern:

- Worker uses an OTLP HTTP/JSON exporter (e.g., `opentelemetry-cloudflare-workers` or similar) targeting a remote Mara endpoint (a Mara instance running outside Workers, e.g., on a VPS or in Kubernetes).
- The remote Mara receives OTLP, applies policy, ships to chosen sinks.
- Mara never runs inside a Workers isolate.

Tail Workers (Cloudflare's native log-tailing) can also forward to a remote Mara endpoint via a Tail Worker that POSTs structured logs.

## Vercel and Netlify

Similar to Cloudflare Workers — no in-process Mara. Use serverless functions' OTel SDKs targeting a remote Mara endpoint.

## Cross-serverless concerns

- **Cold starts vs. WAL durability** — short-lived containers cannot rely on WAL across invocations. Configure synchronous-ack sinks.
- **Per-invocation cost** — Mara's CPU during INVOKE is billed in Lambda; benchmark before deciding to attach to high-volume functions.
- **Region restrictions** — sinks must be reachable from the serverless egress region; VPC connectors or NAT may be needed.
- **Authentication** — sinks behind cloud-provider IAM (e.g., S3) authenticate via the function's execution role; Mara reads IAM credentials from the standard provider chain.

## Common pitfalls

- **Lambda extension that blocks SHUTDOWN beyond deadline** terminates the entire execution environment ungracefully. Mara's flush respects deadline minus a safety margin.
- **Cloud Run sidecar that never receives traffic** because the OTel SDK in the app starts before the sidecar is ready. Container dependency annotations (above) order startup.
- **WAL on `/tmp` filling up** on long-running Lambda environments. Default WAL budget for Lambda extension is 100 MiB, not 1 GiB.
- **OTel SDK pointing to a remote endpoint** when a local extension is present — wasted network. Document that env var.

## Self-telemetry in serverless

- Mara emits its own metrics to `127.0.0.1:9099/metrics` inside the extension/sidecar. In Lambda, scrape via a CloudWatch agent layer or skip; the extension self-logs to the function's CloudWatch log group.

## Compliance

- AWS Lambda extensions inherit the function's compliance posture (SOC 2, HIPAA-eligible, FedRAMP-eligible). Mara as an extension is in scope of the function's audit.
- Cloud Run is SOC 2/ISO 27001-certified.

## When not to use Mara in serverless

- Sub-100 ms functions with cost-sensitive concurrency — the overhead may not be worth it; forward OTLP to a remote Mara instead.
- Cloudflare Workers and similar isolate runtimes — by design.
- Very low-volume functions — operational overhead of managing the extension layer is more than the captured telemetry warrants.
