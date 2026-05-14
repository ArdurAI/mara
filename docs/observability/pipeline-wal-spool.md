# Pipeline WAL spool (`wal_spool_path`)

When a pipeline sets optional `wal_spool_path` to a directory, Mara appends **one JSON line per post-policy delivered event** (`ChainOutcome::Deliver`) before fan-out to sinks.

## Durability tier (M3)

- **Intent**: crash-tolerant **best-effort** capture of what the dispatcher considered “delivered” after policy, for offline replay or forensics.
- **Sync policy**: each append opens the day file, writes one `serde_json` line + newline, then calls `sync_data()` on the file. Work runs in `spawn_blocking` off the hot dispatcher path; ordering relative to sink delivery is **not** transactional with sinks.
- **Filenames**: `{sanitized_pipeline_name}-{UTC_date}.wal` (e.g. `my_pipeline-2026-05-13.wal`).
- **Not included**: Policy drops (unless you enable `audit_policy_drops` separately), pre-policy adapter buffering, or sink-side acknowledgements. Full segmented WAL + replay per ADR-0003 remains future work.

## Configuration

```toml
[[pipelines]]
name = "main"
adapters = ["in"]
sinks = ["out"]
wal_spool_path = "/var/lib/mara/wal"
```

The directory is created on first append if missing.
