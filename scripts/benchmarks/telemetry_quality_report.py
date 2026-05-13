#!/usr/bin/env python3
"""Emit fill-rate stats for Mara JSONL (one canonical Event per line).

Used for M0-04 baseline telemetry quality reports. Reads JSONL from argv[1], writes Markdown to stdout.
"""
from __future__ import annotations

import json
import sys
from pathlib import Path
from typing import Any


def get_in(obj: Any, *path: str) -> Any:
    cur = obj
    for p in path:
        if not isinstance(cur, dict) or p not in cur:
            return None
        cur = cur[p]
    return cur


def is_present(v: Any) -> bool:
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


FIELDS: list[tuple[str, tuple[str, ...]]] = [
    ("`resource.service_name`", ("resource", "service_name")),
    ("`resource.service_version`", ("resource", "service_version")),
    ("`resource.host_name`", ("resource", "host_name")),
    ("`resource.process_pid`", ("resource", "process_pid")),
    ("`gen_ai.operation_name`", ("gen_ai", "operation_name")),
    ("`gen_ai.conversation_id`", ("gen_ai", "conversation_id")),
    ("`gen_ai.request.model`", ("gen_ai", "request", "model")),
    ("`gen_ai.response.model`", ("gen_ai", "response", "model")),
    ("`gen_ai.usage.input_tokens`", ("gen_ai", "usage", "input_tokens")),
    ("`gen_ai.usage.output_tokens`", ("gen_ai", "usage", "output_tokens")),
    ("`mara.session_id`", ("mara", "session_id")),
    ("`mara.turn_id`", ("mara", "turn_id")),
]


def main() -> int:
    if len(sys.argv) < 2:
        print("usage: telemetry_quality_report.py <events.jsonl>", file=sys.stderr)
        return 2
    path = Path(sys.argv[1])
    if not path.is_file():
        print(f"missing file: {path}", file=sys.stderr)
        return 2

    rows: list[dict[str, Any]] = []
    for line in path.read_text().splitlines():
        line = line.strip()
        if not line:
            continue
        rows.append(json.loads(line))

    n = len(rows)
    print("# Telemetry quality (fill-rate)\n")
    print(f"- **Source:** `{path.name}`")
    print(f"- **Rows:** {n}\n")
    print("| Field | Present | Fill rate |")
    print("|-------|--------:|----------:|")

    for label, path_t in FIELDS:
        if n == 0:
            rate = 0.0
            c = 0
        else:
            c = sum(1 for r in rows if is_present(get_in(r, *path_t)))
            rate = 100.0 * c / n
        print(f"| {label} | {c} / {n} | {rate:.1f}% |")

    print("\n*Present* means non-null, non-empty string, or any number/bool/list/dict with content.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
