#!/usr/bin/env bash
# Short live smoke: Mara llm_proxy → local Ollama → Ollama Cloud model (`*-cloud` suffix).
#
# Prerequisites:
#   - Ollama daemon reachable at UPSTREAM (default http://127.0.0.1:11434)
#   - Cloud auth on that daemon (e.g. `ollama signin`); see https://docs.ollama.com/cloud
#   - MODEL pulled locally (e.g. `ollama pull gpt-oss:20b-cloud`)
#
# Environment (all optional):
#   OUTDIR          Output directory (default: <repo>/tmp/ollama-cloud-smoke)
#   PROXY_PORT      Mara proxy listen port (default: 11453; avoid 11434/11435 collisions)
#   UPSTREAM        Ollama base URL (default: http://127.0.0.1:11434)
#   MODEL           Model id including -cloud if applicable (default: gpt-oss:20b-cloud)
#   PROMPT          Single generate prompt (default: short one-word reply test)
#   CURL_MAXTIME    Per-request curl timeout seconds (default: 180)
#   SKIP_BUILD      Set to 1 to skip `cargo build -p mara-cli`
#   MARA_BIN        Path to mara binary (default: ./target/debug/mara)
#   RUST_LOG        Passed through when starting mara (default: info)
#
# Artifacts: OUTDIR/mara.toml, mara-run.log, events.jsonl, last-response.json
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$ROOT"

OUTDIR="${OUTDIR:-$ROOT/tmp/ollama-cloud-smoke}"
PROXY_PORT="${PROXY_PORT:-11453}"
UPSTREAM="${UPSTREAM:-http://127.0.0.1:11434}"
MODEL="${MODEL:-gpt-oss:20b-cloud}"
PROMPT="${PROMPT:-Reply with exactly one word: pong}"
CURL_MAXTIME="${CURL_MAXTIME:-180}"
MARA_BIN="${MARA_BIN:-$ROOT/target/debug/mara}"
RUST_LOG="${RUST_LOG:-info}"

EVENTS="$OUTDIR/events.jsonl"
CFG="$OUTDIR/mara.toml"
LOG="$OUTDIR/mara-run.log"
RESP="$OUTDIR/last-response.json"
PROXY="http://127.0.0.1:${PROXY_PORT}"

mkdir -p "$OUTDIR"
rm -f "$EVENTS" "$LOG" "$RESP"

if ! curl -sS -o /dev/null --max-time 8 "${UPSTREAM}/api/tags"; then
  echo "error: cannot reach Ollama at ${UPSTREAM}/api/tags (start Ollama or set UPSTREAM)" >&2
  exit 2
fi

if [[ "${SKIP_BUILD:-0}" != "1" ]]; then
  cargo build -q -p mara-cli
fi

if [[ ! -f "$MARA_BIN" ]]; then
  echo "error: mara binary not found at $MARA_BIN (build first or set MARA_BIN)" >&2
  exit 2
fi

cat >"$CFG" <<EOF
schema_version = "1"

[server]
log_format = "text"

[[adapters.llm_proxy]]
name = "ollama_proxy"
http_listen = "127.0.0.1:${PROXY_PORT}"
upstream = "${UPSTREAM}"
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

MP=""
cleanup() {
  if [[ -n "${MP}" ]]; then
    kill -TERM "$MP" 2>/dev/null || true
    wait "$MP" 2>/dev/null || true
  fi
}
trap cleanup EXIT

RUST_LOG="$RUST_LOG" "$MARA_BIN" run --config "$CFG" >"$LOG" 2>&1 &
MP=$!

proxy_up=0
for _ in $(seq 1 120); do
  if grep -q "llm http proxy listening" "$LOG" 2>/dev/null; then
    proxy_up=1
    break
  fi
  sleep 0.25
done
if [[ "$proxy_up" -ne 1 ]]; then
  echo "error: mara never logged llm http proxy listening; see $LOG" >&2
  exit 1
fi

# Wait until TCP accepts (log can race ahead of bind on some hosts).
for _ in $(seq 1 40); do
  if curl -sS -o /dev/null --connect-timeout 1 --max-time 2 "${PROXY}/api/tags" 2>/dev/null; then
    break
  fi
  sleep 0.25
done

JSON_PAYLOAD="$(python3 -c 'import json,sys; print(json.dumps({"model":sys.argv[1],"prompt":sys.argv[2],"stream":False}))' "$MODEL" "$PROMPT")"

HTTP_CODE="$(
  curl -sS -o "$RESP" -w "%{http_code}" --max-time "$CURL_MAXTIME" \
    "${PROXY}/api/generate" \
    -H 'content-type: application/json' \
    -d "$JSON_PAYLOAD" || echo "000"
)"

sleep 2

if [[ "$HTTP_CODE" != "200" ]]; then
  echo "error: expected HTTP 200 from /api/generate, got ${HTTP_CODE}; body (truncated):" >&2
  head -c 800 "$RESP" 2>/dev/null || true
  echo >&2
  exit 1
fi

# Flush file sink (BufWriter drains on graceful shutdown); reading events before exit avoids empty files.
kill -TERM "$MP" 2>/dev/null || true
wait "$MP" 2>/dev/null || true
MP=""
trap - EXIT

echo "ok: HTTP ${HTTP_CODE} model=${MODEL} proxy=${PROXY} upstream=${UPSTREAM}"
echo "    response (first 400 bytes):"
head -c 400 "$RESP" || true
echo
lines="$(wc -l <"$EVENTS" | tr -d ' ')"
echo "    events: ${EVENTS} lines=${lines}"
if [[ "${lines}" -lt 1 ]]; then
  echo "error: expected at least one JSONL line in ${EVENTS} after shutdown (see ${LOG})" >&2
  exit 1
fi
tail -n 1 "$EVENTS" | head -c 320 || true
echo " ..."
