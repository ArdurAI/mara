#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
LOCK="$ROOT/docs/semconv.lock"
RUST_FILE="$ROOT/crates/mara-schema/src/lib.rs"
commit_lock="$(grep '^SEMCONV_COMMIT=' "$LOCK" | head -1 | cut -d= -f2)"
commit_rust="$(grep 'pub const SEMCONV_COMMIT' "$RUST_FILE" | sed -n 's/.*"\([^"]*\)".*/\1/p')"
ver_lock="$(grep '^SCHEMA_VERSION=' "$LOCK" | head -1 | cut -d= -f2)"
ver_rust="$(grep 'pub const SCHEMA_VERSION' "$RUST_FILE" | sed -n 's/.*"\([^"]*\)".*/\1/p')"
if [[ "$commit_lock" != "$commit_rust" ]]; then
  echo "semconv drift: docs/semconv.lock SEMCONV_COMMIT=$commit_lock vs crates/mara-schema SEMCONV_COMMIT=$commit_rust" >&2
  exit 1
fi
if [[ "$ver_lock" != "$ver_rust" ]]; then
  echo "schema version drift: docs/semconv.lock SCHEMA_VERSION=$ver_lock vs crates/mara-schema SCHEMA_VERSION=$ver_rust" >&2
  exit 1
fi
echo "semconv.lock matches mara-schema constants."
