#!/usr/bin/env python3
"""M2-05: aggregate Mara JSONL events into a per-session / per-agent run summary (stdout)."""
from __future__ import annotations

import argparse
import json
import math
import sys
from collections import defaultdict
from pathlib import Path


def main() -> None:
    p = argparse.ArgumentParser(description="Summarize agent-ish fields from Mara events JSONL.")
    p.add_argument("jsonl", type=Path, help="Path to events.jsonl")
    p.add_argument("--by", choices=("session", "agent"), default="session", help="Group key")
    args = p.parse_args()

    groups: dict[str, list[dict]] = defaultdict(list)
    with args.jsonl.open(encoding="utf-8") as f:
        for line in f:
            line = line.strip()
            if not line:
                continue
            try:
                ev = json.loads(line)
            except json.JSONDecodeError:
                continue
            mara = ev.get("mara") or {}
            if args.by == "agent":
                key = mara.get("agent_id") or mara.get("session_id") or "(none)"
            else:
                key = mara.get("session_id") or "(none)"
            groups[str(key)].append(ev)

    out = {}
    for key, evs in groups.items():
        steps = sum(1 for e in evs if (e.get("mara") or {}).get("step_id"))
        errs = sum(1 for e in evs if e.get("event_kind") == "error")
        toks_in = 0
        toks_out = 0
        cost = 0.0
        lat = []
        for e in evs:
            ga = e.get("gen_ai") or {}
            u = ga.get("usage") or {}
            toks_in += int(u.get("input_tokens") or 0)
            toks_out += int(u.get("output_tokens") or 0)
            mara = e.get("mara") or {}
            if isinstance(mara.get("cost_usd"), (int, float)):
                cost += float(mara["cost_usd"])
            attrs = e.get("attributes") or {}
            if isinstance(attrs.get("mara.ollama.total_duration_ms"), (int, float)):
                lat.append(float(attrs["mara.ollama.total_duration_ms"]))
        lat_sorted = sorted(lat)
        idx = max(0, int(math.ceil(0.95 * len(lat_sorted))) - 1) if lat_sorted else 0
        hotspot = round(lat_sorted[idx], 3) if lat_sorted else None
        out[key] = {
            "events": len(evs),
            "steps_with_step_id": steps,
            "errors": errs,
            "input_tokens": toks_in,
            "output_tokens": toks_out,
            "cost_usd_sum": round(cost, 8),
            "latency_ms_hotspot_p95": hotspot,
        }
    json.dump(out, sys.stdout, indent=2)
    sys.stdout.write("\n")


if __name__ == "__main__":
    main()
