#!/usr/bin/env python3
"""M2-02 gate: required-field fill-rate on canonical Event JSONL fixtures.

Scores each fixture (runtime) using rows whose gen_ai.operation_name is one of
chat / text_completion / embeddings. Per qualifying row, fill-rate is the
fraction of the seven M0-03-aligned required fields that are present; the
runtime score is the mean of those row rates. Fails if fewer than MIN_RUNTIMES
fixtures score >= THRESHOLD percent.

Run from repository root:

    python3 scripts/benchmarks/schema_completeness_gate.py
"""
from __future__ import annotations

import json
import sys
from pathlib import Path

THRESHOLD = 85.0
MIN_RUNTIMES = 3
OPS_REQUIRED = {"chat", "text_completion", "embeddings"}

# Paths relative to repository root (parent of scripts/).
REPO_ROOT = Path(__file__).resolve().parents[2]

FIXTURES: list[tuple[str, Path]] = [
    ("ollama", REPO_ROOT / "docs/captured/fixtures/ollama-proxy-smoke-sample.jsonl"),
    ("claude_code", REPO_ROOT / "docs/captured/fixtures/claude-otlp-sample.jsonl"),
    ("codex", REPO_ROOT / "docs/captured/fixtures/codex-otlp-sample.jsonl"),
    ("kimi", REPO_ROOT / "docs/captured/fixtures/kimi-jsonl-sample.jsonl"),
    ("cursor", REPO_ROOT / "docs/captured/fixtures/cursor-hooks-sample.jsonl"),
]

# (label, json path as tuple of keys)
REQUIRED_FIELDS: list[tuple[str, tuple[str, ...]]] = [
    ("gen_ai.request.model", ("gen_ai", "request", "model")),
    ("gen_ai.response.model", ("gen_ai", "response", "model")),
    ("gen_ai.operation_name", ("gen_ai", "operation_name")),
    ("gen_ai.usage.input_tokens", ("gen_ai", "usage", "input_tokens")),
    ("gen_ai.usage.output_tokens", ("gen_ai", "usage", "output_tokens")),
    ("resource.host_name", ("resource", "host_name")),
    ("resource.process_pid", ("resource", "process_pid")),
]


def get_in(obj: object, *path: str) -> object:
    cur: object = obj
    for p in path:
        if not isinstance(cur, dict) or p not in cur:
            return None
        cur = cur[p]
    return cur


def is_present(v: object) -> bool:
    if v is None:
        return False
    if isinstance(v, bool):
        return True
    if isinstance(v, (int, float)):
        return True
    if isinstance(v, str):
        return len(v) > 0
    if isinstance(v, list):
        return len(v) > 0
    if isinstance(v, dict):
        return len(v) > 0
    return True


def row_rate(row: dict[str, object]) -> float | None:
    op = get_in(row, "gen_ai", "operation_name")
    if op not in OPS_REQUIRED:
        return None
    present = sum(1 for _, path in REQUIRED_FIELDS if is_present(get_in(row, *path)))
    return 100.0 * present / len(REQUIRED_FIELDS)


def fixture_mean_rate(path: Path) -> tuple[float, int]:
    if not path.is_file():
        raise FileNotFoundError(path)
    rates: list[float] = []
    for line in path.read_text().splitlines():
        line = line.strip()
        if not line:
            continue
        row = json.loads(line)
        r = row_rate(row)
        if r is not None:
            rates.append(r)
    if not rates:
        return 0.0, 0
    return sum(rates) / len(rates), len(rates)


def main() -> int:
    results: list[tuple[str, float, int]] = []
    missing_files: list[str] = []
    for name, path in FIXTURES:
        if not path.is_file():
            missing_files.append(str(path.relative_to(REPO_ROOT)))
            continue
        score, n = fixture_mean_rate(path)
        results.append((name, score, n))

    print("# Schema completeness (M2-02 gate)\n")
    print(f"- **Threshold:** ≥ {THRESHOLD:.0f}% mean per-row required-field fill (M0-03-aligned set of 7 fields).")
    print(f"- **Minimum runtimes passing:** {MIN_RUNTIMES}\n")
    print("| Runtime fixture | Qualifying rows | Mean fill % | Pass |")
    print("|------------------|----------------:|------------:|:----:|")

    passed = [name for name, score, n in results if score >= THRESHOLD and n > 0]
    for name, score, n in results:
        ok = "yes" if name in passed else "no"
        print(f"| `{name}` | {n} | {score:.1f} | {ok} |")

    if missing_files:
        print("\n**Missing files:**")
        for m in missing_files:
            print(f"- `{m}`")
        print("", file=sys.stderr)
        for m in missing_files:
            print(f"missing fixture: {m}", file=sys.stderr)
        return 2

    if len(passed) < MIN_RUNTIMES:
        print(
            f"\n**FAIL:** only {len(passed)} runtime(s) meet ≥{THRESHOLD:.0f}% "
            f"(need {MIN_RUNTIMES}): {', '.join(passed) or '(none)'}",
            file=sys.stderr,
        )
        return 1

    print(f"\n**OK:** {len(passed)} runtime(s) meet threshold: {', '.join(sorted(passed))}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
