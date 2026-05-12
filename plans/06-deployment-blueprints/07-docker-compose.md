# Docker Compose Deployment

## Executive summary

Docker Compose is the simplest way for a developer to stand up Mara plus a complete observability backend (Loki + Grafana + sometimes Tempo) on their laptop or a single-node test environment. This blueprint provides a copy-pasteable `compose.yaml` that runs Mara alongside Loki for logs, Grafana for visualization, and an example workload that emits OTLP. It's Persona 2's "I want to see my Claude Code sessions in Grafana in five minutes" recipe.

## Minimum viable stack

```yaml
# compose.yaml
name: mara-stack

services:
  mara:
    image: ghcr.io/ardurai/mara:1.0.0
    container_name: mara
    command: ["run", "--config", "/etc/mara/mara.toml"]
    volumes:
      - ./mara.toml:/etc/mara/mara.toml:ro
      - mara_state:/var/lib/mara
      # Mount your host AI runtime transcripts so the JSONL adapter can tail them
      - "${HOME}/.claude/projects:/host/claude_projects:ro"
      - "${HOME}/.codex:/host/codex:ro"
    ports:
      - "4317:4317"   # OTLP gRPC
      - "4318:4318"   # OTLP HTTP
      - "9099:9099"   # Mara metrics + healthz
    networks: [observability]
    restart: unless-stopped
    healthcheck:
      test: ["CMD", "wget", "--spider", "-q", "http://127.0.0.1:9099/healthz"]
      interval: 30s
      timeout: 5s
      retries: 3

  loki:
    image: grafana/loki:3.4.0
    container_name: loki
    command: ["-config.file=/etc/loki/local-config.yaml"]
    ports:
      - "3100:3100"
    volumes:
      - ./loki-config.yaml:/etc/loki/local-config.yaml:ro
      - loki_data:/loki
    networks: [observability]
    restart: unless-stopped

  grafana:
    image: grafana/grafana-oss:11.5.0
    container_name: grafana
    ports:
      - "3000:3000"
    environment:
      - GF_AUTH_ANONYMOUS_ENABLED=true
      - GF_AUTH_ANONYMOUS_ORG_ROLE=Admin
      - GF_FEATURE_TOGGLES_ENABLE=lokiLogsDataplane
    volumes:
      - ./grafana-datasources.yaml:/etc/grafana/provisioning/datasources/datasources.yaml:ro
      - grafana_data:/var/lib/grafana
    networks: [observability]
    restart: unless-stopped

volumes:
  mara_state:
  loki_data:
  grafana_data:

networks:
  observability:
```

## `mara.toml`

```toml
[server]
metrics_addr = "0.0.0.0:9099"
log_format = "json"

[[adapters.otlp]]
name = "ingest_otlp"
grpc_listen = "0.0.0.0:4317"
http_listen = "0.0.0.0:4318"

[[adapters.jsonl]]
name = "claude_code_tail"
globs = ["/host/claude_projects/**/*.jsonl"]
checkpoint_path = "/var/lib/mara/checkpoints/claude_code"

[[adapters.jsonl]]
name = "codex_history_tail"
globs = ["/host/codex/history.jsonl"]
checkpoint_path = "/var/lib/mara/checkpoints/codex"

[[pipelines]]
name = "primary"
adapters = ["ingest_otlp", "claude_code_tail", "codex_history_tail"]
policy_chain = "redact_basic"
sinks = ["loki_local"]
wal = { dir = "/var/lib/mara/wal", max_bytes = "256MiB", max_age = "4h" }

[[policies.redact_basic]]
type = "redact"
pack = "builtin.pii"

[[sinks.loki]]
name = "loki_local"
url = "http://loki:3100/loki/api/v1/push"
labels = ["runtime", "event_kind"]
```

## `loki-config.yaml`

Minimal Loki single-node config:

```yaml
auth_enabled: false

server:
  http_listen_port: 3100

common:
  path_prefix: /loki
  storage:
    filesystem:
      chunks_directory: /loki/chunks
      rules_directory: /loki/rules
  replication_factor: 1
  ring:
    kvstore:
      store: inmemory

schema_config:
  configs:
    - from: 2024-01-01
      store: tsdb
      object_store: filesystem
      schema: v13
      index:
        prefix: index_
        period: 24h

limits_config:
  allow_structured_metadata: true
  retention_period: 168h
```

## `grafana-datasources.yaml`

```yaml
apiVersion: 1
datasources:
  - name: Loki
    type: loki
    access: proxy
    url: http://loki:3100
    isDefault: true
    jsonData:
      timeout: 60
```

## Launch

```bash
docker compose up -d
open http://localhost:3000        # Grafana
```

In Grafana → Explore → Loki datasource, query:

```logql
{event_kind="completion"} |= "claude"
```

Or for cost data:

```logql
sum by (gen_ai_request_model) (rate({event_kind="cost"} | json | unwrap mara_cost_usd [5m]))
```

## Variants

### With Tempo for traces

Add a Tempo service:

```yaml
  tempo:
    image: grafana/tempo:2.7.0
    command: ["-config.file=/etc/tempo/tempo.yaml"]
    ports:
      - "3200:3200"   # tempo http
      - "4319:4317"   # OTLP gRPC (remapped, mara already on 4317)
    volumes:
      - ./tempo-config.yaml:/etc/tempo/tempo.yaml:ro
      - tempo_data:/var/tempo
    networks: [observability]
```

Add an OTLP sink to Mara pointing at Tempo:

```toml
[[sinks.otlp]]
name = "tempo_grpc"
endpoint = "http://tempo:4317"
protocol = "grpc"
```

And include `tempo_grpc` in the `primary` pipeline sinks list.

### With Prometheus for cost metrics

Add Prometheus and a `prometheus_remote_write` sink:

```yaml
  prometheus:
    image: prom/prometheus:v3.0.1
    command:
      - "--config.file=/etc/prometheus/prometheus.yml"
      - "--web.enable-remote-write-receiver"
    ports:
      - "9090:9090"
    volumes:
      - ./prometheus.yml:/etc/prometheus/prometheus.yml:ro
    networks: [observability]
```

```toml
[[sinks.prom_rw]]
name = "prom_local"
endpoint = "http://prometheus:9090/api/v1/write"
metrics = ["gen_ai.usage.input_tokens", "gen_ai.usage.output_tokens", "mara.cost.usd"]
```

## Mounting host AI runtime transcripts

For the JSONL adapters to see Claude Code or Codex transcripts on your host, mount them read-only:

```yaml
volumes:
  - "${HOME}/.claude/projects:/host/claude_projects:ro"
  - "${HOME}/.codex:/host/codex:ro"
  - "${HOME}/.kimi/logs:/host/kimi_logs:ro"
```

On macOS with Docker Desktop, `${HOME}` resolves to the host home; the bind mount is shared into the container.

For Cursor's hooks adapter, point your Cursor hook config to run `docker exec mara mara cursor-hook` (the in-container CLI subcommand) to forward hook events into the Mara container.

## Resource sizing

Default compose stack on a developer laptop:

- Mara: 200–400 MiB RSS typical, < 0.5 vCPU.
- Loki: 200–400 MiB.
- Grafana: 200 MiB.

Total: ≈ 1 GiB RAM for a complete local stack.

## Common pitfalls

- **Permission errors on macOS bind mounts** — Docker Desktop's file sharing must include `${HOME}` (default in recent versions).
- **`localhost` vs container name** — inside compose's network, services address each other by service name (`loki`, not `localhost:3100`).
- **Time skew between host and containers** — Docker Desktop syncs reasonably well, but for time-sensitive analysis run `docker run --rm alpine date` to verify.
- **WAL on bind-mounted volume** can be slow on macOS due to Docker Desktop's filesystem; prefer named volumes (`mara_state:/var/lib/mara`) over bind mounts for hot paths.
- **Grafana anonymous-admin** is for local-dev convenience only; never use in shared environments.

## Production-adjacent variant

For a small single-VM "real" deployment (not just dev):

- Replace `grafana-oss` with `grafana-enterprise` if you have a license.
- Add reverse-proxy (Caddy or Traefik) with Let's Encrypt for HTTPS.
- Pin all image tags to specific versions (no `latest`).
- Use a dedicated host volume on SSD for Loki chunks.
- Add log retention policies appropriate to volume.
- Add authentication to Grafana (OAuth, LDAP, or basic auth).

## Cleanup

```bash
docker compose down
docker compose down -v   # also remove volumes
```
