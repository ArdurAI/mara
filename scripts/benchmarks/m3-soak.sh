#!/usr/bin/env bash
# M3-13: lightweight sustained-throughput smoke (EPS sanity).
# Sends N synthetic OTLP HTTP batches to a local receiver or prints instructions.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"

: "${M3_SOAK_OTLP_URL:=http://127.0.0.1:4318}"
: "${M3_SOAK_SECONDS:=60}"
: "${M3_SOAK_RPS:=20}"

echo "M3 soak: POSTing minimal OTLP-ish traffic to ${M3_SOAK_OTLP_URL} for ${M3_SOAK_SECONDS}s at ~${M3_SOAK_RPS} req/s"
echo "(This script does not ship protobuf bodies; use a real OTel collector or extend with curl+protobuf.)"
echo "Placeholder: run 'mara run' with an OTLP adapter on 4318, then use otel-cli or your exporter."
echo "OK (placeholder exit 0)"
