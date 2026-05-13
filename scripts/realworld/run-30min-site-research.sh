#!/usr/bin/env bash
# ≥30 min: local Mara demo site + Ollama cloud (via Mara) + JSONL telemetry.
# Requires: Ollama with cloud model available (default gpt-oss:120b-cloud), network.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$ROOT"
HERE="$ROOT/tmp/mara-30min-realworld"
SITE_DIR="$ROOT/scripts/realworld/demo-site"
SITE_PORT="${SITE_PORT:-18865}"
MARA_PORT="${MARA_PORT:-18866}"
PROXY="http://127.0.0.1:${MARA_PORT}"
DURATION_SEC="${DURATION_SEC:-1860}"
MODEL="${MODEL:-gpt-oss:120b-cloud}"
CURL_MAXTIME="${CURL_MAXTIME:-480}"
PAUSE_SEC="${PAUSE_SEC:-60}"

iso_now() { date -u +"%Y-%m-%dT%H:%M:%SZ"; }

cargo build -q -p mara-cli
rm -f "$HERE/events.jsonl" "$HERE/mara-run.log" "$HERE/run.log" "$HERE/FINDINGS.md"
mkdir -p "$HERE"

cat >"$HERE/mara.toml" <<EOF
schema_version = "1"

[server]
log_format = "text"

[[adapters.llm_proxy]]
name = "ollama_proxy"
http_listen = "127.0.0.1:${MARA_PORT}"
upstream = "http://127.0.0.1:11434"
normalizer = "ollama"

[[sinks.file]]
name = "ev_out"
path = "${HERE}/events.jsonl"
format = "jsonl"
rotate_bytes = 104857600

[[pipelines]]
name = "ollama"
adapters = ["ollama_proxy"]
policy_chain = "default"
sinks = ["ev_out"]
EOF

echo "$(iso_now) start site=:${SITE_PORT} mara_proxy=:${MARA_PORT} duration=${DURATION_SEC}s model=${MODEL}" | tee -a "$HERE/run.log"

python3 -m http.server "$SITE_PORT" --directory "$SITE_DIR" >>"$HERE/site-server.log" 2>&1 &
SPID=$!

cleanup() {
  kill "$SPID" 2>/dev/null || true
  wait "$SPID" 2>/dev/null || true
  echo "$(iso_now) site server stopped" | tee -a "$HERE/run.log"
}
trap cleanup EXIT

RUST_LOG="${RUST_LOG:-info}" ./target/debug/mara run --config "$HERE/mara.toml" >"$HERE/mara-run.log" 2>&1 &
MP=$!

for _ in $(seq 1 150); do
  if grep -q "llm http proxy listening" "$HERE/mara-run.log" 2>/dev/null; then
    break
  fi
  sleep 0.2
done
if ! grep -q "llm http proxy listening" "$HERE/mara-run.log" 2>/dev/null; then
  echo "Mara did not start listening; see $HERE/mara-run.log" | tee -a "$HERE/run.log"
  exit 1
fi

curl -sS -o /dev/null --max-time 5 "http://127.0.0.1:${SITE_PORT}/" || {
  echo "Demo site not reachable" | tee -a "$HERE/run.log"
  exit 1
}

deadline=$((SECONDS + DURATION_SEC))
n=0
prompts=(
  "You are a technical editor. The following HTML is from a self-hosted explainer site for the Mara project. List exactly 3 concrete improvements (markdown bullets) for clarity or accuracy. Keep under 200 words."
  "Compare Mara (from the HTML) to 'Fluent Bit for logs' in one short paragraph. Is the analogy fair? What nuance is missing?"
  "From the HTML: what is the single strongest value proposition sentence you would put above the fold? Reply with one sentence only."
  "Identify any ambiguous or marketing-heavy phrases in the HTML and suggest plainer replacements (max 4 bullets, markdown)."
  "Draft a 4-line 'FAQ' block (Q/A format) that a developer would want after reading the HTML. Ground answers only in the HTML text."
  "What security or privacy claims does the HTML imply? List gaps or risks the page should acknowledge (markdown, max 5 bullets)."
)

while ((SECONDS < deadline)); do
  n=$((n + 1))
  remain=$((deadline - SECONDS))
  idx=$(( (n - 1) % ${#prompts[@]} ))
  instr="${prompts[$idx]}"

  export SITE_PORT INSTR="$instr" MODEL
  payload="$(python3 <<'PY'
import json, os, subprocess
site_port = os.environ["SITE_PORT"]
html = subprocess.check_output(
    ["curl", "-sS", "--max-time", "15", f"http://127.0.0.1:{site_port}/"],
    text=True,
)[:14000]
body = {
    "model": os.environ["MODEL"],
    "stream": False,
    "messages": [
        {"role": "system", "content": "You help improve documentation. Be concise and practical."},
        {"role": "user", "content": os.environ["INSTR"] + "\n\n--- HTML ---\n" + html},
    ],
}
print(json.dumps(body))
PY
)"

  echo "$(iso_now) chat iteration #$n (~${remain}s left) prompt_idx=$idx" | tee -a "$HERE/run.log"
  code="$(
    curl -sS -o "$HERE/last-response-$n.json" -w "%{http_code}" --max-time "$CURL_MAXTIME" \
      "$PROXY/api/chat" \
      -H 'content-type: application/json' \
      -d "$payload" \
      2>>"$HERE/run.log" || echo "000"
  )"
  echo "$(iso_now)  http=$code bytes=$(wc -c <"$HERE/last-response-$n.json" 2>/dev/null || echo 0)" | tee -a "$HERE/run.log"

  sleep "$PAUSE_SEC"
done

echo "$(iso_now) duration wall-clock complete iterations=$n; stopping Mara to flush events" | tee -a "$HERE/run.log"
kill -TERM "$MP" 2>/dev/null || true
wait "$MP" 2>/dev/null || true

export REPO_ROOT="$ROOT"
python3 <<'PY' >"$HERE/FINDINGS.md"
from pathlib import Path
import json, collections, datetime, os

root = Path(os.environ["REPO_ROOT"])
here = root / "tmp/mara-30min-realworld"
evp = here / "events.jsonl"
lines = evp.read_text().splitlines() if evp.exists() else []
rows = [json.loads(x) for x in lines]

print("# Real-world run: Mara site + Ollama cloud + Mara telemetry\n")
print(f"Generated (local): {datetime.datetime.now().isoformat()}\n")
print("## Artifacts\n")
print(f"- Events: `{evp}` ({evp.stat().st_size if evp.exists() else 0} bytes)\n")
print(f"- Mara log: `{here / 'mara-run.log'}`\n")
print(f"- Client log: `{here / 'run.log'}`\n")
print(f"- Last model reply (per iteration): `{here}/last-response-*.json`\n")
print("## Event summary\n")
print(f"- Total JSONL lines: **{len(rows)}**\n")
if rows:
    print(f"- By `event_kind`: {dict(collections.Counter(r.get('event_kind') for r in rows))}\n")
    ops = dict(collections.Counter((r.get('gen_ai') or {}).get('operation_name') for r in rows))
    print(f"- By `gen_ai.operation_name`: {ops}\n")
    rm = dict(collections.Counter((r.get('gen_ai') or {}).get('response', {}).get('model') for r in rows))
    print(f"- By `gen_ai.response.model`: {rm}\n")
    ut = sum(1 for r in rows if ((r.get('gen_ai') or {}).get('usage') or {}).get('input_tokens') is not None)
    print(f"- Rows with non-null usage input_tokens: **{ut}**\n")
    req_m = sum(1 for r in rows if ((r.get('gen_ai') or {}).get('request') or {}).get('model'))
    print(f"- Rows with non-null `gen_ai.request.model`: **{req_m}**\n")
print("\n## How to replay\n\n```bash\nbash scripts/realworld/run-30min-site-research.sh\n```\n")
PY

echo "Wrote $HERE/FINDINGS.md"
