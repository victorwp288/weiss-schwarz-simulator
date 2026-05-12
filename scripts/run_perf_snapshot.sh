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
PERF_SNAPSHOT_PROFILE="${PERF_SNAPSHOT_PROFILE:-full}"
mkdir -p "$OUT_DIR"
OUT_DIR="$(cd "$OUT_DIR" && pwd)"
cd "$REPO_DIR"

BENCH_OUT="$OUT_DIR/benches.txt"
PY_OUT="$OUT_DIR/python_bench.txt"

run_core_bench() {
  local filter="$1"
  local mode="$2"
  if [[ "$mode" == "replace" ]]; then
    cargo bench -p weiss_core --bench core_benches "$filter" -- "${CRITERION_ARGS[@]}" > "$BENCH_OUT"
  else
    cargo bench -p weiss_core --bench core_benches "$filter" -- "${CRITERION_ARGS[@]}" >> "$BENCH_OUT"
  fi
}

run_alloc_bench() {
  local filter="$1"
  cargo bench -p weiss_core --bench alloc_benches "$filter" -- "${CRITERION_ARGS[@]}" >> "$BENCH_OUT"
}

case "$PERF_SNAPSHOT_PROFILE" in
  full)
    CRITERION_ARGS=(--output-format bencher)
    cargo bench -p weiss_core --bench core_benches -- "${CRITERION_ARGS[@]}" > "$BENCH_OUT"
    cargo bench -p weiss_core --bench alloc_benches -- "${CRITERION_ARGS[@]}" >> "$BENCH_OUT"
    PY_STEPS=2000
    PY_WARMUP=200
    PY_RESET_REPS=200
    PY_NUM_ENVS=128
    ;;
  light)
    CRITERION_ARGS=(
      --output-format bencher
      --sample-size 10
      --measurement-time 0.3
      --warm-up-time 0.1
    )
    run_core_bench "reset_batch_256" replace
    run_core_bench "step_batch_fast_256_priority_off" append
    run_core_bench "step_first_legal_i16_legal_ids_nometa_256" append
    run_alloc_bench "alloc_"
    PY_STEPS=300
    PY_WARMUP=50
    PY_RESET_REPS=50
    PY_NUM_ENVS=64
    ;;
  *)
    echo "unknown PERF_SNAPSHOT_PROFILE '$PERF_SNAPSHOT_PROFILE' (expected full or light)" >&2
    exit 64
    ;;
esac

"$PYTHON_BIN" "$ROOT_DIR/python/examples/bench_python_boundary.py" \
  --num-envs "$PY_NUM_ENVS" \
  --steps "$PY_STEPS" \
  --warmup "$PY_WARMUP" \
  --reset-reps "$PY_RESET_REPS" \
  --reset-done \
  --mode both \
  --repo-root "$REPO_DIR" > "$PY_OUT"
