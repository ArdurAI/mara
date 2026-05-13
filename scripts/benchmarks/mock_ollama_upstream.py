#!/usr/bin/env python3
"""Minimal HTTP stub: Ollama-style /api/tags, /api/generate, /api/chat (for Mara proxy CI smoke)."""
from __future__ import annotations

import json
from http.server import BaseHTTPRequestHandler, HTTPServer

_counter = 0


class _H(BaseHTTPRequestHandler):
    def log_message(self, *_args) -> None:
        return

    def do_GET(self) -> None:
        if self.path.startswith("/api/tags"):
            body = json.dumps({"models": [{"name": "mock:1"}]}).encode()
        else:
            body = b"{}"
        self.send_response(200)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)

    def do_POST(self) -> None:
        global _counter
        n = int(self.headers.get("Content-Length", "0") or 0)
        if n:
            _ = self.rfile.read(n)
        _counter += 1
        c = _counter
        if "/api/chat" in self.path:
            body_obj = {
                "model": "mock:1",
                "created_at": "2026-01-01T00:00:00Z",
                "message": {"role": "assistant", "content": f"chat-ack-{c}"},
                "done": True,
                "done_reason": "stop",
                "prompt_eval_count": 40 + c,
                "eval_count": 12 + (c % 5),
                "total_duration": 2_000_000_000 + c * 100_000,
                "load_duration": 40_000_000,
                "prompt_eval_duration": 70_000_000,
                "eval_duration": 50_000_000 + c * 1000,
            }
        else:
            body_obj = {
                "model": "mock:1",
                "response": f"gen-ack-{c}",
                "done": True,
                "done_reason": "stop",
                "prompt_eval_count": 100 + c,
                "eval_count": 10 + (c % 7),
                "total_duration": 1_000_000_000 + c * 1_000_000,
                "load_duration": 50_000_000,
                "prompt_eval_duration": 80_000_000,
                "eval_duration": 45_000_000 + c * 1000,
            }
        raw = json.dumps(body_obj).encode()
        self.send_response(200)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(raw)))
        self.end_headers()
        self.wfile.write(raw)


if __name__ == "__main__":
    import sys

    host = "127.0.0.1"
    port = int(sys.argv[1]) if len(sys.argv) > 1 else 18080
    HTTPServer((host, port), _H).serve_forever()
