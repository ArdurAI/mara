#!/usr/bin/env python3
"""
Long-running varied workload: Mara llm_proxy → local Ollama → cloud-capable models.

Mixes /api/generate, /api/chat, a real HTTP fetch + summarization ("web research"),
and multi-call "planner / specialist" chains ("subagents" as sequential model calls).

Prerequisites: Ollama on UPSTREAM (default http://127.0.0.1:11434), cloud sign-in,
pulled MODEL (default gpt-oss:20b-cloud). Stdlib only.

Examples:
  python3 scripts/realworld/ollama_cloud_varied_workload.py
  python3 scripts/realworld/ollama_cloud_varied_workload.py --duration 120 --pause-min 5 --pause-max 12
"""
from __future__ import annotations

import argparse
import json
import os
import random
import signal
import subprocess
import sys
import time
import urllib.error
import urllib.request
from dataclasses import dataclass
from datetime import datetime, timezone
from pathlib import Path
from typing import Any, Callable


def repo_root() -> Path:
    return Path(__file__).resolve().parents[2]


def write_mara_toml(
    path: Path,
    *,
    proxy_port: int,
    upstream: str,
    events_path: Path,
    metrics_port: int,
) -> None:
    path.write_text(
        f'''schema_version = "1"

[server]
log_format = "text"
metrics_addr = "127.0.0.1:{metrics_port}"

[[adapters.llm_proxy]]
name = "ollama_proxy"
http_listen = "127.0.0.1:{proxy_port}"
upstream = "{upstream}"
normalizer = "ollama"

[[sinks.file]]
name = "ev_out"
path = "{events_path.as_posix()}"
format = "jsonl"
rotate_bytes = 104857600

[[pipelines]]
name = "ollama"
adapters = ["ollama_proxy"]
policy_chain = "default"
sinks = ["ev_out"]
''',
        encoding="utf-8",
    )


def http_post_json(url: str, payload: dict[str, Any], timeout: float) -> tuple[int, dict[str, Any] | str]:
    data = json.dumps(payload).encode("utf-8")
    req = urllib.request.Request(
        url,
        data=data,
        headers={"Content-Type": "application/json"},
        method="POST",
    )
    try:
        with urllib.request.urlopen(req, timeout=timeout) as resp:
            raw = resp.read().decode("utf-8", errors="replace")
            status = resp.getcode() or 200
    except urllib.error.HTTPError as e:
        raw = e.read().decode("utf-8", errors="replace")
        try:
            return e.code, json.loads(raw)
        except json.JSONDecodeError:
            return e.code, raw
    try:
        return status, json.loads(raw)
    except json.JSONDecodeError:
        return status, raw


def http_get(url: str, timeout: float) -> tuple[int, str]:
    req = urllib.request.Request(url, method="GET")
    with urllib.request.urlopen(req, timeout=timeout) as resp:
        return resp.getcode() or 200, resp.read().decode("utf-8", errors="replace")


def fetch_public_text(url: str, max_bytes: int = 6000) -> str:
    _, body = http_get(url, timeout=20.0)
    return body[:max_bytes]


def append_ndjson(path: Path, row: dict[str, Any]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("a", encoding="utf-8") as f:
        f.write(json.dumps(row, ensure_ascii=False) + "\n")


@dataclass
class Ctx:
    proxy: str
    model: str
    request_timeout: float
    results_path: Path


def task_generate_short(ctx: Ctx, n: int) -> dict[str, Any]:
    prompts = [
        "In under 40 words: what is RED metrics in SRE?",
        "Name two differences between histogram_quantile and average latency.",
        "One paragraph: when would you choose JSONL over Parquet for agent telemetry?",
    ]
    prompt = prompts[n % len(prompts)]
    t0 = time.perf_counter()
    code, body = http_post_json(
        f"{ctx.proxy}/api/generate",
        {"model": ctx.model, "prompt": prompt, "stream": False},
        ctx.request_timeout,
    )
    dt = (time.perf_counter() - t0) * 1000
    excerpt = ""
    if isinstance(body, dict):
        excerpt = str(body.get("response", ""))[:400]
    return {
        "task": "generate_short",
        "http": code,
        "latency_ms": round(dt, 1),
        "excerpt": excerpt,
    }


def task_chat_multi_turn(ctx: Ctx) -> dict[str, Any]:
    messages = [
        {"role": "system", "content": "You are a concise SRE coach. Keep answers under 60 words."},
        {"role": "user", "content": "What is a saturation signal for a worker pool?"},
        {"role": "assistant", "content": "Queue depth growing while throughput flatlines often indicates saturation."},
        {"role": "user", "content": "Give one mitigation in one sentence."},
    ]
    t0 = time.perf_counter()
    code, body = http_post_json(
        f"{ctx.proxy}/api/chat",
        {"model": ctx.model, "messages": messages, "stream": False},
        ctx.request_timeout,
    )
    dt = (time.perf_counter() - t0) * 1000
    excerpt = ""
    if isinstance(body, dict):
        msg = body.get("message") or {}
        excerpt = str(msg.get("content", ""))[:400]
    return {"task": "chat_multi_turn", "http": code, "latency_ms": round(dt, 1), "excerpt": excerpt}


def task_web_fetch_summarize(ctx: Ctx) -> dict[str, Any]:
    url = "https://www.w3.org/TR/trace-context/"
    try:
        snippet = fetch_public_text(url, max_bytes=5000)
    except Exception as e:
        return {"task": "web_fetch_summarize", "http": 0, "error": f"fetch_failed:{e!s}"}
    prompt = (
        "You are summarizing a fetched spec excerpt for an engineer.\n\n"
        f"SOURCE URL: {url}\n\nEXCERPT (truncated):\n{snippet}\n\n"
        "In under 90 words: what problem does distributed trace context solve?"
    )
    t0 = time.perf_counter()
    code, body = http_post_json(
        f"{ctx.proxy}/api/generate",
        {"model": ctx.model, "prompt": prompt, "stream": False},
        ctx.request_timeout,
    )
    dt = (time.perf_counter() - t0) * 1000
    excerpt = ""
    if isinstance(body, dict):
        excerpt = str(body.get("response", ""))[:500]
    return {
        "task": "web_fetch_summarize",
        "http": code,
        "latency_ms": round(dt, 1),
        "excerpt": excerpt,
        "fetch_url": url,
    }


def task_subagent_chain(ctx: Ctx) -> dict[str, Any]:
    t0 = time.perf_counter()
    c1, b1 = http_post_json(
        f"{ctx.proxy}/api/generate",
        {
            "model": ctx.model,
            "prompt": (
                "You are PlannerAgent. Output exactly 3 numbered one-line tasks for "
                "instrumenting a new HTTP proxy with metrics (no preamble, no markdown fences)."
            ),
            "stream": False,
        },
        ctx.request_timeout,
    )
    plan = ""
    if isinstance(b1, dict):
        plan = str(b1.get("response", ""))[:1200]
    c2, b2 = http_post_json(
        f"{ctx.proxy}/api/generate",
        {
            "model": ctx.model,
            "prompt": (
                "You are SpecialistAgent. Here is the plan from PlannerAgent:\n\n"
                f"{plan}\n\n"
                "Expand ONLY item 2 into 3 concise bullet sub-tasks. No preamble."
            ),
            "stream": False,
        },
        ctx.request_timeout,
    )
    dt = (time.perf_counter() - t0) * 1000
    ex2 = ""
    if isinstance(b2, dict):
        ex2 = str(b2.get("response", ""))[:500]
    return {
        "task": "subagent_chain",
        "http_plan": c1,
        "http_specialist": c2,
        "latency_ms": round(dt, 1),
        "excerpt": ex2,
    }


def task_code_snippet(ctx: Ctx) -> dict[str, Any]:
    t0 = time.perf_counter()
    code, body = http_post_json(
        f"{ctx.proxy}/api/generate",
        {
            "model": ctx.model,
            "prompt": (
                "Write a minimal Python function `tail_ndjson(path: str, n: int) -> list[dict]` "
                "that reads the last n JSON lines from a file. Under 35 lines. No explanation outside code block."
            ),
            "stream": False,
        },
        ctx.request_timeout,
    )
    dt = (time.perf_counter() - t0) * 1000
    excerpt = ""
    if isinstance(body, dict):
        excerpt = str(body.get("response", ""))[:600]
    return {"task": "code_snippet", "http": code, "latency_ms": round(dt, 1), "excerpt": excerpt}


def task_openai_style_chat(ctx: Ctx) -> dict[str, Any]:
    """Hit OpenAI-compatible route if upstream supports it (best-effort)."""
    t0 = time.perf_counter()
    code, body = http_post_json(
        f"{ctx.proxy}/v1/chat/completions",
        {
            "model": ctx.model,
            "messages": [{"role": "user", "content": "Reply in under 15 words: what is a span in tracing?"}],
            "stream": False,
        },
        ctx.request_timeout,
    )
    dt = (time.perf_counter() - t0) * 1000
    excerpt = ""
    if isinstance(body, dict):
        ch = (body.get("choices") or [{}])[0]
        msg = ch.get("message") or {}
        excerpt = str(msg.get("content", ""))[:400]
    return {"task": "openai_style_chat", "http": code, "latency_ms": round(dt, 1), "excerpt": excerpt}


def wait_upstream(upstream: str, deadline: float) -> None:
    base = upstream.rstrip("/")
    while time.monotonic() < deadline:
        try:
            c, _ = http_get(f"{base}/api/tags", timeout=5.0)
            if c == 200:
                return
        except OSError:
            pass
        time.sleep(0.3)
    raise SystemExit(f"upstream not reachable: {base}/api/tags")


def log_tail_contains(log_path: Path, needle: str, max_chars: int = 96_000) -> bool:
    if not log_path.is_file():
        return False
    text = log_path.read_text(encoding="utf-8", errors="replace")
    return needle in text[-max_chars:]


def wait_mara_proxy(proxy: str, log_path: Path, deadline: float) -> None:
    while time.monotonic() < deadline:
        if log_tail_contains(log_path, "llm http proxy listening"):
            try:
                c, _ = http_get(f"{proxy}/api/tags", timeout=2.0)
                if c == 200:
                    return
            except OSError:
                pass
        time.sleep(0.25)
    raise SystemExit("mara proxy did not become ready in time; see mara-run.log")


def main() -> None:
    root = repo_root()
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--duration", type=int, default=900, help="Minimum run duration in seconds (default 900 = 15 min)")
    ap.add_argument("--proxy-port", type=int, default=11460)
    ap.add_argument("--metrics-port", type=int, default=0, help="0 = derive from proxy port")
    ap.add_argument("--upstream", default=os.environ.get("UPSTREAM", "http://127.0.0.1:11434"))
    ap.add_argument("--model", default=os.environ.get("MODEL", "gpt-oss:20b-cloud"))
    ap.add_argument("--outdir", type=Path, default=None)
    ap.add_argument("--request-timeout", type=float, default=420.0)
    ap.add_argument("--pause-min", type=float, default=40.0)
    ap.add_argument("--pause-max", type=float, default=75.0)
    ap.add_argument("--skip-build", action="store_true")
    ap.add_argument("--mara-bin", type=Path, default=None)
    ap.add_argument("--no-dashboard", action="store_true", help="Do not spawn http.server for index.html")
    ap.add_argument("--dashboard-port", type=int, default=18766)
    args = ap.parse_args()

    outdir = args.outdir or (root / "tmp" / f"ollama-varied-{datetime.now(timezone.utc).strftime('%Y%m%dT%H%M%SZ')}")
    outdir.mkdir(parents=True, exist_ok=True)
    events_path = outdir / "events.jsonl"
    results_path = outdir / "results.ndjson"
    mara_log = outdir / "mara-run.log"
    cfg_path = outdir / "mara.toml"
    for p in (events_path, results_path, mara_log):
        if p.exists():
            p.unlink()

    metrics_port = args.metrics_port or (19000 + (args.proxy_port % 700))
    proxy = f"http://127.0.0.1:{args.proxy_port}"
    mara_bin = args.mara_bin or (root / "target" / "debug" / "mara")

    wait_upstream(args.upstream, time.monotonic() + 30.0)

    if not args.skip_build:
        subprocess.run(
            ["cargo", "build", "-q", "-p", "mara-cli"],
            cwd=root,
            check=True,
        )
    if not mara_bin.is_file():
        raise SystemExit(f"mara binary missing: {mara_bin} (build or pass --mara-bin)")

    write_mara_toml(
        cfg_path,
        proxy_port=args.proxy_port,
        upstream=args.upstream,
        events_path=events_path,
        metrics_port=metrics_port,
    )

    html_src = Path(__file__).with_name("varied_workload_index.html")
    (outdir / "index.html").write_text(html_src.read_text(encoding="utf-8"), encoding="utf-8")

    env = os.environ.copy()
    env.setdefault("RUST_LOG", "info")
    mara_proc: subprocess.Popen | None = None
    http_proc: subprocess.Popen | None = None
    log_fp = None

    def shutdown(_a=None, _b=None) -> None:
        nonlocal mara_proc, http_proc, log_fp
        if mara_proc and mara_proc.poll() is None:
            mara_proc.send_signal(signal.SIGTERM)
            try:
                mara_proc.wait(timeout=60)
            except subprocess.TimeoutExpired:
                mara_proc.kill()
        if log_fp is not None:
            try:
                log_fp.close()
            except OSError:
                pass
            log_fp = None
        if http_proc and http_proc.poll() is None:
            http_proc.terminate()
            try:
                http_proc.wait(timeout=5)
            except subprocess.TimeoutExpired:
                http_proc.kill()

    signal.signal(signal.SIGINT, shutdown)
    signal.signal(signal.SIGTERM, shutdown)

    log_fp = mara_log.open("w", encoding="utf-8")
    mara_proc = subprocess.Popen(
        [str(mara_bin), "run", "--config", str(cfg_path)],
        stdout=log_fp,
        stderr=subprocess.STDOUT,
        cwd=root,
        env=env,
    )

    try:
        wait_mara_proxy(proxy, mara_log, time.monotonic() + 120.0)

        if not args.no_dashboard:
            http_proc = subprocess.Popen(
                [
                    sys.executable,
                    "-m",
                    "http.server",
                    str(args.dashboard_port),
                    "--bind",
                    "127.0.0.1",
                ],
                cwd=outdir,
                stdout=subprocess.DEVNULL,
                stderr=subprocess.DEVNULL,
            )
            print(
                f"Dashboard: http://127.0.0.1:{args.dashboard_port}/index.html\n"
                f"Artifacts: {outdir}\n",
                flush=True,
            )

        ctx = Ctx(proxy=proxy, model=args.model, request_timeout=args.request_timeout, results_path=results_path)

        task_runners: list[Callable[[Ctx], dict[str, Any]]] = [
            lambda c: task_generate_short(c, int(time.time()) % 10),
            task_chat_multi_turn,
            task_web_fetch_summarize,
            task_subagent_chain,
            task_code_snippet,
            task_openai_style_chat,
        ]

        deadline = time.monotonic() + float(args.duration)
        iteration = 0
        while time.monotonic() < deadline:
            fn = task_runners[iteration % len(task_runners)]
            iteration += 1
            row: dict[str, Any] = {
                "ts": datetime.now(timezone.utc).isoformat(),
                "iteration": iteration,
                "model": args.model,
            }
            t0 = time.perf_counter()
            try:
                row.update(fn(ctx))
                row["ok"] = True
            except Exception as e:
                row["ok"] = False
                row["error"] = repr(e)
            row["elapsed_s"] = round(time.perf_counter() - t0, 3)
            append_ndjson(results_path, row)
            print(json.dumps(row, ensure_ascii=False)[:500], flush=True)

            if time.monotonic() >= deadline:
                break
            pause = random.uniform(args.pause_min, args.pause_max)
            # Do not oversleep past global deadline
            pause = min(pause, max(0.0, deadline - time.monotonic()))
            if pause > 0:
                time.sleep(pause)

        summary = {
            "finished_ts": datetime.now(timezone.utc).isoformat(),
            "iterations": iteration,
            "outdir": str(outdir),
            "events_jsonl": str(events_path),
        }
        append_ndjson(results_path, {"summary": True, **summary})
        print(json.dumps(summary, indent=2), flush=True)
    finally:
        shutdown()

    n_events = 0
    if events_path.is_file():
        with events_path.open(encoding="utf-8") as ef:
            n_events = sum(1 for _ in ef)
    print(f"Done. Telemetry lines: {n_events}", flush=True)


if __name__ == "__main__":
    main()
