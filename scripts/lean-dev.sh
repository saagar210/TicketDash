#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

TEMP_ROOT="$(mktemp -d "${TMPDIR:-/tmp}/ticketdash-lean.XXXXXX")"
export CARGO_TARGET_DIR="$TEMP_ROOT/cargo-target"
export VITE_CACHE_DIR="$TEMP_ROOT/vite-cache"
export XDG_CACHE_HOME="$TEMP_ROOT/xdg-cache"

cleanup() {
  local exit_code=$?

  npm run clean:heavy >/dev/null 2>&1 || true
  rm -rf "$TEMP_ROOT"

  exit "$exit_code"
}

trap cleanup EXIT INT TERM

echo "Lean dev cache root: $TEMP_ROOT"
npm run tauri dev
