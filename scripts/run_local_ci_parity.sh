#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

if [[ -n "${VENV_PYTHON:-}" ]]; then
  PYTHON_BIN="$VENV_PYTHON"
elif [[ -x "$ROOT_DIR/.venv/bin/python" ]]; then
  PYTHON_BIN="$ROOT_DIR/.venv/bin/python"
else
  PYTHON_BIN="python"
fi

if ! command -v "$PYTHON_BIN" >/dev/null 2>&1; then
  echo "ERROR: python interpreter not found: $PYTHON_BIN" >&2
  exit 127
fi

if ! command -v maturin >/dev/null 2>&1; then
  echo "ERROR: maturin is required for local parity checks." >&2
  exit 127
fi

if ! command -v ruff >/dev/null 2>&1; then
  echo "ERROR: ruff is required for local parity checks." >&2
  exit 127
fi

if ! command -v pytest >/dev/null 2>&1; then
  echo "ERROR: pytest is required for local parity checks." >&2
  exit 127
fi

if ! command -v cargo >/dev/null 2>&1; then
  echo "ERROR: cargo is required for local parity checks." >&2
  exit 127
fi

if ! command -v git >/dev/null 2>&1; then
  echo "ERROR: git is required for local parity checks." >&2
  exit 127
fi

if ! "$PYTHON_BIN" -c "import pip_audit" >/dev/null 2>&1; then
  echo "ERROR: pip-audit is required for local parity checks." >&2
  exit 127
fi

log_step() {
  echo
  echo "==> $1"
}

run() {
  log_step "$1"
  shift
  "$@"
}

MIN_PARSE_LINE_COVERAGE_STRICT="${MIN_PARSE_LINE_COVERAGE_STRICT:-0.52}"
MAX_UNSUPPORTED_LINES_STRICT="${MAX_UNSUPPORTED_LINES_STRICT:-14200}"
MIN_CARD_COVERAGE_APPROX="${MIN_CARD_COVERAGE_APPROX:-0.99}"
SKIP_BENCHMARKS="${SKIP_BENCHMARKS:-0}"

run "Check env layering" ./scripts/check_env_layering.sh
run "Docs link check" "$PYTHON_BIN" scripts/check_docs_links.py
run "Docs constants check" "$PYTHON_BIN" scripts/check_docs_constants.py

run "Cargo fmt" cargo fmt --all -- --check
run "Cargo clippy" cargo clippy --workspace --all-targets -- -D warnings
run "Cargo test" cargo test --workspace --features test-harness
run "Cargo doc (missing docs denied)" env RUSTDOCFLAGS="-D missing-docs" cargo doc --workspace --no-deps

run "Ruff format" ruff format --check python scraper scripts
run "Ruff check" ruff check python scraper scripts

run "Ability coverage report" "$PYTHON_BIN" scripts/ability_coverage_report.py --output /tmp/ability_coverage_report.json
run "Ability coverage targets" "$PYTHON_BIN" scripts/ability_coverage_targets.py --report /tmp/ability_coverage_report.json --output /tmp/ability_coverage_targets.json
run "Coverage budget gate" \
  "$PYTHON_BIN" scripts/check_coverage_budget.py \
  --report /tmp/ability_coverage_report.json \
  --baseline scripts/ability_coverage_baseline.json \
  --min-parse-line-coverage-strict "$MIN_PARSE_LINE_COVERAGE_STRICT" \
  --max-unsupported-lines-strict "$MAX_UNSUPPORTED_LINES_STRICT" \
  --min-card-coverage-approx "$MIN_CARD_COVERAGE_APPROX"

run "Clean wheel output dir" rm -rf /tmp/wss_dist
run "Build wheel" maturin build --release --manifest-path weiss_py/Cargo.toml --out /tmp/wss_dist --interpreter "$PYTHON_BIN"
run "Install wheel" "$PYTHON_BIN" -m pip install --force-reinstall --no-deps /tmp/wss_dist/*.whl
run "Pytest" "$PYTHON_BIN" -m pytest -q python/tests

if [[ "$SKIP_BENCHMARKS" == "1" ]]; then
  echo
  echo "==> Benchmarks skipped (SKIP_BENCHMARKS=1)"
else
  PERF_BASE_REF="${PERF_BASE_REF:-HEAD}"
  PERF_BASE_WORKTREE=""
  PERF_BASE_SNAPSHOT_DIR="$(mktemp -d /tmp/wss_perf_base.XXXXXX)"
  PERF_HEAD_SNAPSHOT_DIR="$(mktemp -d /tmp/wss_perf_head.XXXXXX)"

  cleanup_perf_workdirs() {
    if [[ -n "${PERF_BASE_WORKTREE:-}" ]]; then
      git worktree remove --force "$PERF_BASE_WORKTREE" >/dev/null 2>&1 || true
      rm -rf "$PERF_BASE_WORKTREE" >/dev/null 2>&1 || true
    fi
    rm -rf "$PERF_BASE_SNAPSHOT_DIR" "$PERF_HEAD_SNAPSHOT_DIR"
  }

  trap cleanup_perf_workdirs EXIT
  PERF_BASE_WORKTREE="$(mktemp -d /tmp/wss_perf_base_worktree.XXXXXX)"
  run "Prepare perf base worktree (${PERF_BASE_REF})" \
    git worktree add --detach "$PERF_BASE_WORKTREE" "$PERF_BASE_REF"
  run "Build perf baseline python extension" \
    "$PYTHON_BIN" -m maturin develop --release --manifest-path "$PERF_BASE_WORKTREE/weiss_py/Cargo.toml"
  run "Capture perf baseline snapshot" \
    env PYTHON_BIN="$PYTHON_BIN" "$ROOT_DIR/scripts/run_perf_snapshot.sh" "$PERF_BASE_SNAPSHOT_DIR" "$PERF_BASE_WORKTREE"
  run "Build perf head python extension" \
    "$PYTHON_BIN" -m maturin develop --release --manifest-path "$ROOT_DIR/weiss_py/Cargo.toml"
  run "Capture perf head snapshot" \
    env PYTHON_BIN="$PYTHON_BIN" "$ROOT_DIR/scripts/run_perf_snapshot.sh" "$PERF_HEAD_SNAPSHOT_DIR" "$ROOT_DIR"
  run "Performance budget gate" \
    "$PYTHON_BIN" scripts/check_perf_budget.py \
    --baseline-benches "$PERF_BASE_SNAPSHOT_DIR/benches.txt" \
    --current-benches "$PERF_HEAD_SNAPSHOT_DIR/benches.txt" \
    --baseline-python "$PERF_BASE_SNAPSHOT_DIR/python_bench.txt" \
    --current-python "$PERF_HEAD_SNAPSHOT_DIR/python_bench.txt" \
    --max-core-regression-pct 15 \
    --max-python-regression-pct 10 \
    --require-zero-alloc

  trap - EXIT
  cleanup_perf_workdirs
fi

run "Cargo audit" cargo audit
run "pip-audit project" "$PYTHON_BIN" -m pip_audit .
run "pip-audit scraper requirements" "$PYTHON_BIN" -m pip_audit -r scraper/requirements.txt

echo
echo "Local CI parity checks completed."
