#!/usr/bin/env bash
# CI-friendly smoke: mock Ollama upstream + Mara llm_proxy + curl + JSONL field checks (~1–2 min).
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$ROOT"

MOCK_PORT="${MOCK_PORT:-19980}"
PROXY_PORT="${PROXY_PORT:-19981}"
EVENTS="${EVENTS:-$(mktemp -t mara-smoke-events.XXXXXX.jsonl)}"
MARA_LOG="${MARA_LOG:-$(mktemp -t mara-smoke-mara.XXXXXX.log)}"

cleanup() {
  kill "$MP" 2>/dev/null || true
  wait "$MP" 2>/dev/null || true
  kill "$MOCKPID" 2>/dev/null || true
  wait "$MOCKPID" 2>/dev/null || true
  rm -f "$CFG"
}
trap cleanup EXIT

CFG="$(mktemp -t mara-smoke.XXXXXX.toml)"
cat >"$CFG" <<EOF
schema_version = "1"

[server]
log_format = "text"

[[adapters.llm_proxy]]
name = "ollama_proxy"
http_listen = "127.0.0.1:${PROXY_PORT}"
upstream = "http://127.0.0.1:${MOCK_PORT}"
normalizer = "ollama"

[[sinks.file]]
name = "ev_out"
path = "${EVENTS}"
format = "jsonl"
rotate_bytes = 104857600

[[pipelines]]
name = "ollama"
adapters = ["ollama_proxy"]
policy_chain = "default"
sinks = ["ev_out"]
EOF

python3 "$ROOT/scripts/benchmarks/mock_ollama_upstream.py" "$MOCK_PORT" &
MOCKPID=$!
sleep 0.25

cargo build -q -p mara-cli
RUST_LOG="${RUST_LOG:-info}" ./target/debug/mara run --config "$CFG" >"$MARA_LOG" 2>&1 &
MP=$!

for _ in $(seq 1 120); do
  if grep -q "llm http proxy listening" "$MARA_LOG" 2>/dev/null; then
    break
  fi
  sleep 0.1
done
if ! grep -q "llm http proxy listening" "$MARA_LOG" 2>/dev/null; then
  echo "Mara did not start; log tail:" >&2
  tail -50 "$MARA_LOG" >&2 || true
  exit 1
fi

PROXY="http://127.0.0.1:${PROXY_PORT}"
curl -sS -o /dev/null --max-time 10 "$PROXY/api/tags"

curl -sS --max-time 30 "$PROXY/api/generate" \
  -H 'content-type: application/json' \
  -d '{"model":"mock:1","prompt":"smoke-generate","stream":false}' \
  -o /dev/null

curl -sS --max-time 30 "$PROXY/api/chat" \
  -H 'content-type: application/json' \
  -d '{"model":"mock:1","messages":[{"role":"user","content":"smoke-chat"}],"stream":false}' \
  -o /dev/null

kill -TERM "$MP" 2>/dev/null || true
wait "$MP" 2>/dev/null || true
MP=0

python3 "$ROOT/scripts/benchmarks/check_ollama_proxy_events.py" "$EVENTS"
echo "ollama_proxy_smoke ok (events at $EVENTS)"
