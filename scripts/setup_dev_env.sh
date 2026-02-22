#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

if [[ -x "$ROOT_DIR/.venv/bin/python" ]]; then
  PYTHON_BIN="$ROOT_DIR/.venv/bin/python"
elif command -v python3 >/dev/null 2>&1; then
  PYTHON_BIN="python3"
elif command -v python >/dev/null 2>&1; then
  PYTHON_BIN="python"
else
  echo "ERROR: python is required (python3 or python not found)." >&2
  exit 127
fi

if ! command -v cargo >/dev/null 2>&1; then
  echo "ERROR: cargo is required. Install Rust stable first (rustup default stable)." >&2
  exit 127
fi

echo "Using Python: $PYTHON_BIN"
"$PYTHON_BIN" -m pip install -U pip
"$PYTHON_BIN" -m pip install -U ".[dev]"
"$PYTHON_BIN" -m maturin develop --release --manifest-path "$ROOT_DIR/weiss_py/Cargo.toml"

echo "Dev environment ready."
