# Compatibility Matrix Specification

## Executive summary

The compatibility matrix is Mara's public statement of what it captures, how well, for each AI runtime. It is published at `docs/compat-matrix.md` and updated on every release. This document specifies the matrix schema, the per-cell evidence rules, and the tier-based acceptance bars.

## Matrix shape

For each (runtime, signal) pair, the matrix records: a status, a coverage percent, a list of known gaps, the test fixture used to verify, the runtime version range tested, and the date last verified.

## Runtimes (rows)

- Claude Code (CLI + desktop)
- Codex (CLI + desktop)
- Cursor Agents (IDE + CLI)
- Kimi (CLI + app)
- Augment Code (IDE + remote agents)
- Gemini CLI (+ Gemini API consumers via OTel)

## Signals (columns)

Each cell describes how well Mara captures the named signal class for the given runtime. The signal classes:

- **session_lifecycle** — session start, switch, end events.
- **prompt** — user, system, tool-result-as-input prompt content (subject to ZDR opt-in).
- **completion** — model completion content (subject to ZDR opt-in).
- **tool_call** — model-requested tool invocations.
- **tool_result** — tool result returned to model.
- **token_usage** — `gen_ai.usage.input_tokens` / `output_tokens` / `cached_tokens` / `reasoning_tokens`.
- **cost** — vendor-reported or Mara-computed USD cost.
- **latency** — TTFT, total turn latency.
- **error** — agent or model errors.
- **eval** — eval results when produced (Claude Code, Codex).
- **feedback** — explicit user feedback.
- **mcp_traffic** — MCP tool calls and resource accesses.
- **agent_loop** — multi-turn agent trace structure with parent/child spans.

## Per-cell status codes

- ✅ **PASS** — feature works at tier-appropriate fidelity; golden test green; documented in runtime preset.
- 🟡 **PARTIAL** — feature works in some configurations or covers a subset; gaps documented; planned work tracked in issue.
- ❌ **GAP** — feature not supported; runtime does not expose the signal or Mara has not implemented capture; documented as a known gap.
- ⚠️ **EXPERIMENTAL** — feature behind a `--experimental` flag; not subject to stability guarantees.
- 🚫 **N/A** — signal does not apply to this runtime (e.g., `eval` for runtimes that have no eval surface).

## Per-cell evidence requirements

For ✅ PASS:

1. A golden-file test in `crates/mara-runtimes/<runtime>/tests/golden/<signal>.snap`.
2. A coverage report from the smoke test showing the canonical fields populated.
3. A line in the runtime preset documentation referencing the test.

For 🟡 PARTIAL:

1. A golden test that documents the partial behavior.
2. A list of known gaps in the cell footnote.
3. An open issue tagged `compat-gap/<runtime>/<signal>`.

For ❌ GAP:

1. A footnote explaining why (no upstream surface, upstream not stable, deferred to v1.x).
2. An open issue tagged `compat-gap/<runtime>/<signal>` with rationale.

## Tier-based acceptance bars

The tier model defined in [`../01-landscape/08-ai-runtime-telemetry-surfaces.md`](../01-landscape/08-ai-runtime-telemetry-surfaces.md) determines the v1 acceptance bar per cell.

### Tier A acceptance (Claude Code, Codex, Gemini CLI)

For each of the following signals, status MUST be ✅ PASS by v1.0:

- session_lifecycle
- prompt (when capture is opted in)
- completion (when capture is opted in)
- tool_call
- tool_result
- token_usage
- latency
- error

For each of: `cost`, `eval`, `feedback`, `mcp_traffic`, `agent_loop` — status MAY be 🟡 PARTIAL or ✅ PASS depending on upstream support.

### Tier B acceptance (Cursor Agents, Kimi)

For each of the following signals, status MUST be ✅ PASS or 🟡 PARTIAL by v1.0:

- session_lifecycle
- tool_call
- tool_result
- error

Capture of prompt/completion and token usage status depends on runtime support. Cursor: `prompt` and `completion` available via hooks (PARTIAL because not all hook event types carry them). Kimi: JSONL tail captures what kimi.log emits, which depends on debug verbosity.

### Tier C acceptance (Augment Code)

For each of the following signals, status MUST be 🟡 PARTIAL or ❌ GAP with clear evidence:

- token_usage (PARTIAL if Augment analytics exposes it)
- cost (PARTIAL if Augment analytics exposes it)
- error (PARTIAL or GAP)
- session_lifecycle (PARTIAL via analytics)

All other signals: GAP with documented rationale.

## v1 target matrix (compact form)

Reading: `R1=Claude Code, R2=Codex, R3=Cursor, R4=Kimi, R5=Augment, R6=Gemini`.

- session_lifecycle: R1✅ R2✅ R3✅ R4🟡 R5🟡 R6✅
- prompt: R1✅ R2✅ R3🟡 R4🟡 R5❌ R6✅
- completion: R1✅ R2✅ R3🟡 R4🟡 R5❌ R6✅
- tool_call: R1✅ R2✅ R3✅ R4🟡 R5❌ R6✅
- tool_result: R1✅ R2✅ R3✅ R4🟡 R5❌ R6✅
- token_usage: R1✅ R2✅ R3❌ R4🟡 R5🟡 R6✅
- cost: R1✅ R2🟡 R3❌ R4🟡 R5🟡 R6✅
- latency: R1✅ R2✅ R3✅ R4🟡 R5🟡 R6✅
- error: R1✅ R2✅ R3✅ R4🟡 R5🟡 R6✅
- eval: R1🟡 R2🟡 R3🚫 R4🚫 R5❌ R6🚫
- feedback: R1🟡 R2🟡 R3🚫 R4🚫 R5❌ R6🚫
- mcp_traffic: R1✅ R2✅ R3✅ R4🟡 R5❌ R6🟡
- agent_loop: R1✅ R2✅ R3🟡 R4🟡 R5❌ R6✅

Subject to verification against actual implementations during M3.

## Versioning and stability

Each cell records: the runtime version range tested (e.g., `claude-code 0.42.x – 0.46.x`), the Mara version that verified it, and the date.

When a runtime ships a payload-format breaking change, the affected cells degrade to 🟡 PARTIAL pending Mara adapter update, then return to ✅ PASS once the preset is updated and golden tests regenerate.

## Update cadence

- Smoke tests for all six runtimes run nightly in CI.
- Matrix is regenerated from smoke test results and published on every tagged release.
- Matrix changes between releases are highlighted in `CHANGELOG.md`.

## Consumption by other documents

- Success metric FC-1 (runtime coverage) reads from this matrix.
- Per-runtime quickstart in [`../07-quickstarts/`](../07-quickstarts/) links to the corresponding row.
- v2 design RFC consumes the GAP cells to scope additional adapters.

## Reporter UI considerations

The matrix is Markdown for v1. v1.x may add a generated HTML view with filtering by runtime, signal, status, and date. The Markdown stays the source of truth.
