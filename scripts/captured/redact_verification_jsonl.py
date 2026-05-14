#!/usr/bin/env python3
"""Redact machine-specific fields from Mara JSONL events for public verification bundles."""
from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path


def redact_event(obj: dict) -> dict:
    res = obj.get("resource")
    if isinstance(res, dict):
        if res.get("host_name") is not None:
            res["host_name"] = "host.redacted"
        if res.get("process_pid") is not None:
            res["process_pid"] = 1
    return obj


def main() -> None:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("input", type=Path, help="Source JSONL file")
    ap.add_argument("output", type=Path, help="Destination JSONL file")
    args = ap.parse_args()

    args.output.parent.mkdir(parents=True, exist_ok=True)
    with args.input.open(encoding="utf-8") as inf, args.output.open("w", encoding="utf-8") as outf:
        for line in inf:
            line = line.strip()
            if not line:
                continue
            obj = json.loads(line)
            outf.write(json.dumps(redact_event(obj), ensure_ascii=False) + "\n")


if __name__ == "__main__":
    main()
