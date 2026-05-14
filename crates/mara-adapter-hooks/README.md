# mara-adapter-hooks

HTTP **Tier B** ingest for AI runtime hooks (for example Cursor-style `POST` of canonical [`mara_schema::Event`] JSON).

## Behavior

- Binds `http_listen` (from `mara.toml` → `HooksHttpAdapterConfig`).
- Accepts `POST` with a JSON body that is either:
  - a single `Event` object, or
  - `{"events":[ ... ]}` with an array of `Event` objects.
- Responds `202 Accepted` with `accepted N events` when enqueue succeeds.
- `413` when body exceeds `max_body_bytes`; `400` on malformed JSON.

## Limits

- No built-in authentication; use loopback bind or TLS + ACL in front for non-loopback (see `docs/otlp-http-receiver-threat-model.md` for the shared listener checklist).
- Large batches should be split client-side to stay under the body cap.
