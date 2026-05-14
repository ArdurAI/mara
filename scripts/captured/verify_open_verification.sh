#!/usr/bin/env bash
# Verify SHA256 of public open-verification bundles (run from repo root).
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$ROOT/docs/captured/open-verification"
shasum -a 256 -c SHASUMS256
echo "ok: SHASUMS256 matches for all bundles under docs/captured/open-verification/"
