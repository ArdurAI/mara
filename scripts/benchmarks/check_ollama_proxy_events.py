#!/usr/bin/env python3
"""Validate Mara JSONL from Ollama proxy smoke: required gen_ai fields on completion-like rows."""
from __future__ import annotations

import json
import sys
from pathlib import Path


def main() -> int:
    path = Path(sys.argv[1])
    if not path.is_file():
        print(f"missing events file: {path}", file=sys.stderr)
        return 2
    lines = path.read_text().splitlines()
    if not lines:
        print("events file is empty", file=sys.stderr)
        return 1

    ops_required = {"chat", "text_completion"}
    checked = 0
    failures: list[str] = []

    for i, line in enumerate(lines, 1):
        row = json.loads(line)
        op = (row.get("gen_ai") or {}).get("operation_name")
        if op not in ops_required:
            continue
        checked += 1
        g = row.get("gen_ai") or {}
        req = g.get("request") or {}
        resp = g.get("response") or {}
        usage = g.get("usage") or {}
        prefix = f"line {i} op={op}"

        if not req.get("model"):
            failures.append(f"{prefix}: missing gen_ai.request.model")
        if not resp.get("model"):
            failures.append(f"{prefix}: missing gen_ai.response.model")
        if usage.get("input_tokens") is None:
            failures.append(f"{prefix}: missing gen_ai.usage.input_tokens")
        if usage.get("output_tokens") is None:
            failures.append(f"{prefix}: missing gen_ai.usage.output_tokens")
        res = row.get("resource") or {}
        if res.get("process_pid") is None:
            failures.append(f"{prefix}: missing resource.process_pid")
        if not res.get("host_name"):
            failures.append(f"{prefix}: missing resource.host_name")
        if res.get("service_name") != "mara-ollama-smoke":
            failures.append(
                f"{prefix}: expected resource.service_name 'mara-ollama-smoke', got {res.get('service_name')!r}"
            )
        if res.get("service_version") != "ci":
            failures.append(
                f"{prefix}: expected resource.service_version 'ci', got {res.get('service_version')!r}"
            )

    if checked < 2:
        failures.append(f"expected at least 2 completion rows (chat+generate), got {checked}")

    if failures:
        for f in failures:
            print(f, file=sys.stderr)
        return 1
    print(f"ok: validated {checked} completion-like rows in {path}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
