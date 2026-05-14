# Local Mara + Ollama test — notes (reference)

This folder holds **redacted** canonical `events.jsonl` from a local-only run (no cloud model required). See the repository script `tmp/ollama-live-test/run-live-test.sh` (or copy under `scripts/realworld/` in future) for the full procedure.

**Takeaways**

- Proxy pass-through to `127.0.0.1:11434` works; `/api/generate` returns token and duration fields in the upstream JSON body.
- `event_kind` for native `/api/generate` may remain `system` while `gen_ai.operation_name` is `text_completion` — see `docs/ollama-null-fields.md`.
- File sinks flush on graceful shutdown; read `events.jsonl` after `SIGTERM` + `wait` on the `mara` process.

**Reproduce (outline)**

1. Build `mara-cli`, write a single `llm_proxy` pipeline to a JSONL file sink.
2. Run `mara run --config …`, curl `/api/tags` then `/api/generate` through the proxy port.
3. Stop `mara` with `SIGTERM` and inspect `events.jsonl`.

Machine-specific fields (`resource.host_name`, `resource.process_pid`) are normalized in the checked-in JSONL for privacy.
