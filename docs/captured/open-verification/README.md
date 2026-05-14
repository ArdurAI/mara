# Open verification bundles (live Ollama + Mara)

These artifacts are **real** runs captured from Mara’s `llm_proxy` in front of a local Ollama daemon. They are checked in so anyone can:

1. **Inspect** canonical JSONL and workload logs without running hardware.
2. **Re-check integrity** with pinned SHA256 sums.
3. **Reproduce** similar outputs on their machine (timestamps, UUIDs, and exact token counts will differ).

## Privacy / redaction

- **`events.jsonl`** files are passed through `scripts/captured/redact_verification_jsonl.py`, which sets `resource.host_name` to `host.redacted` and `resource.process_pid` to `1`.
- **UUIDs** (`mara.session_id`, `mara.request_id`, etc.) are left as-is for structural verification; treat them as opaque when comparing across runs.
- **Model outputs** in `results.ndjson` and upstream JSON are unmodified model text (may include “thinking” fields from some models).

## Bundles

| Directory | What it is | Typical reproduce command |
|-----------|------------|---------------------------|
| **`ollama-cloud-smoke/`** | Single `POST /api/generate` through Mara using a **`-cloud`** model (`gpt-oss:20b-cloud`). Two JSONL lines after shutdown. | `bash scripts/realworld/ollama_cloud_smoke.sh` |
| **`ollama-varied-short/`** | ~100 s run of `scripts/realworld/ollama_cloud_varied_workload.py` (mixed tasks). | `python3 scripts/realworld/ollama_cloud_varied_workload.py --duration 100 --pause-min 5 --pause-max 12` |
| **`ollama-varied-15min/`** | ~15 min varied workload (same script, default pauses). | `python3 scripts/realworld/ollama_cloud_varied_workload.py` |
| **`ollama-live-local/`** | Short local run (tags + generate) with a **local** model; notes in `FINDINGS.md`. | `bash tmp/ollama-live-test/run-live-test.sh` (or adapt paths from repo root) |

## Verify checksums

From the repository root:

```bash
bash scripts/captured/verify_open_verification.sh
```

This runs `shasum -a 256 -c` against `SHASUMS256` in this directory.

**Updating bundles (maintainers)** after a new capture:

1. Copy fresh `events.jsonl` / `results.ndjson` / upstream JSON into the appropriate subdirectory (do not commit raw `tmp/` paths).
2. Redact events:  
   `python3 scripts/captured/redact_verification_jsonl.py <src-events.jsonl> <dest-events.jsonl>`
3. Regenerate `SHASUMS256`:  
   `(cd docs/captured/open-verification && find . -type f ! -name SHASUMS256 ! -name README.md ! -path './ollama-live-local/FINDINGS.md' -print0 | sort -z | xargs -0 shasum -a 256)`  
   then paste into `SHASUMS256` with **paths relative to this directory** (same format as today).

## Files

- `SHASUMS256` — expected hashes (verified in CI).
- `ollama-cloud-smoke/events.jsonl` — redacted Mara events.
- `ollama-cloud-smoke/upstream-generate-response.json` — raw upstream JSON body for the generate call (reference only).
- `ollama-varied-*/events.jsonl` — redacted Mara events for workload runs.
- `ollama-varied-*/results.ndjson` — one JSON object per task plus a final `summary` object from the Python driver.
- `ollama-live-local/FINDINGS.md` — short operator notes.

## Related

- `docs/ollama-null-fields.md` — field semantics and nulls.
- `scripts/realworld/ollama_cloud_smoke.sh` — short cloud smoke driver.
- `scripts/realworld/ollama_cloud_varied_workload.py` — long / varied driver + optional dashboard.
