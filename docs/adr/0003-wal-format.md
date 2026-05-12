# ADR-0003: Segmented append-only WAL with per-sink offsets

- **Status:** Accepted
- **Date:** 2026-05-12
- **Authors:** Mara M1 architecture review.

## Context

Mara must guarantee durability across both graceful shutdown (zero loss) and ungraceful termination (SIGKILL, power loss — ≤1 second of loss). It must also tolerate downstream sink outages without dropping events while sinks recover. Per-sink retry independence means each sink needs its own logical "offset" into the buffered event stream.

The WAL is the foundational durability primitive of the pipeline. Choosing its format wisely is a one-time decision with long consequences.

## Decision

Mara implements a **segmented append-only WAL** with the following characteristics:

- The WAL directory contains numbered segment files: `0000000001.maraw`, `0000000002.maraw`, etc.
- Each segment is bounded by size (default 64 MiB) and time (default 30 minutes), whichever is reached first; a new segment opens when the active one rolls.
- Each record is `length (u32 LE) | crc32c (u32) | type (u8) | payload (length bytes)`.
- A small header (magic + version + flags) prefaces each segment file.
- An `offsets/` subdirectory contains per-sink offset files keyed by sink name, written via atomic rename. Each offset is `(segment_id, byte_position)`.
- Garbage collection: a segment is eligible for deletion when all configured sinks have committed offsets beyond its end-byte.
- Total WAL size and age are bounded by global limits (defaults: 1 GiB and 4 hours). When the limits are reached, the oldest segment is deleted regardless of sink offsets and a `mara_wal_drops_total` metric is incremented; the policy can be flipped to `wal.overflow = "block"` to apply backpressure instead.

The implementation lives in `crates/mara-core/src/wal/`. Records are flushed to disk on every batch boundary; fdatasync (or `FlushFileBuffers` on Windows) is called per batch.

## Alternatives considered

- **Single growing file with periodic compaction.** Pros: simpler. Cons: GC requires expensive rewrites; per-sink offsets are messy; deletion of old data is awkward. Rejected.
- **Per-sink WAL (one file per sink).** Pros: trivially independent. Cons: duplicates events on disk for fan-out pipelines; multiplies fsync cost. Rejected.
- **Embedded KV store (redb / sled / fjall).** Pros: pre-baked ACID; offset tracking trivial. Cons: heavier on-disk format; the access pattern is sequential append + sequential read, which a KV index is overkill for; KV stores tend to write more bytes per record. Decision: use redb for the offsets index only (small, point lookups); use raw segment files for the event stream.
- **RocksDB.** Pros: production-proven. Cons: heavyweight native dependency; build complexity; far more features than we need. Rejected.
- **External Kafka as the WAL.** Pros: durable spine. Cons: deployment dependency; defeats edge-first design; appropriate at the gateway tier (v2) but not at the edge. Rejected for v1.

## Consequences

- The WAL format is versioned via a magic byte in the segment header. The current version is `WAL_FORMAT_V1`. Future format changes require ADR + migration code.
- Cross-platform care: fsync semantics differ between Linux (fdatasync), macOS (F_FULLFSYNC for true durability), and Windows (FlushFileBuffers). Documented per OS.
- The implementation is a non-trivial subset of M2 work and must be well-tested with kill-9 simulation.
- The benchmarks include a "WAL replay throughput" suite (NFR-1.7).
- The implementation goes into `mara-core` (not a separate crate) because the core orchestrator is the only consumer; future re-use can be promoted to its own crate if needed.

## References

- [`plans/04-implementation/01-functional-requirements.md`](../../plans/04-implementation/01-functional-requirements.md) FR-6.
- [`plans/04-implementation/02-non-functional-requirements.md`](../../plans/04-implementation/02-non-functional-requirements.md) NFR-2.
- [`plans/05-evaluation/01-benchmark-methodology.md`](../../plans/05-evaluation/01-benchmark-methodology.md) Scenarios 5 (startup), 6 (WAL replay), 7 (crash durability).
