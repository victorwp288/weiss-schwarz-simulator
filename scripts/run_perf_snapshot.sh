#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

if [[ $# -lt 1 || $# -gt 2 ]]; then
  echo "usage: $0 <output-dir> [repo-dir]" >&2
  exit 64
fi

OUT_DIR="$1"
REPO_DIR="${2:-$ROOT_DIR}"
PYTHON_BIN="${PYTHON_BIN:-python}"
mkdir -p "$OUT_DIR"
OUT_DIR="$(cd "$OUT_DIR" && pwd)"
cd "$REPO_DIR"

BENCH_OUT="$OUT_DIR/benches.txt"
PY_OUT="$OUT_DIR/python_bench.txt"

cargo bench -p weiss_core --bench core_benches -- --output-format bencher > "$BENCH_OUT"
cargo bench -p weiss_core --bench alloc_benches -- --output-format bencher >> "$BENCH_OUT"

PYTHONPATH=python "$PYTHON_BIN" python/examples/bench_python_boundary.py \
  --num-envs 128 \
  --steps 2000 \
  --warmup 200 \
  --reset-reps 200 \
  --mode both > "$PY_OUT"
