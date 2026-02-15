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
  run "Capture perf baselines" cp benchmark/benches.txt /tmp/wss_perf_before_benches.txt
  run "Capture python perf baseline" cp benchmark/python_bench.txt /tmp/wss_perf_before_python_bench.txt
  log_step "Rust benches (core + alloc)"
  cargo bench -p weiss_core --bench core_benches -- --output-format bencher > /tmp/wss_perf_after_benches.txt
  cargo bench -p weiss_core --bench alloc_benches -- --output-format bencher >> /tmp/wss_perf_after_benches.txt

  log_step "Python boundary bench"
  PYTHONPATH=python "$PYTHON_BIN" python/examples/bench_python_boundary.py \
    --num-envs 128 \
    --steps 2000 \
    --warmup 200 \
    --reset-reps 200 \
    --mode both | tee /tmp/wss_perf_after_python_bench.txt

  run "Performance budget gate" \
    "$PYTHON_BIN" scripts/check_perf_budget.py \
    --baseline-benches /tmp/wss_perf_before_benches.txt \
    --current-benches /tmp/wss_perf_after_benches.txt \
    --baseline-python /tmp/wss_perf_before_python_bench.txt \
    --current-python /tmp/wss_perf_after_python_bench.txt \
    --max-core-regression-pct 15 \
    --max-python-regression-pct 10 \
    --require-zero-alloc
fi

run "Cargo audit" cargo audit
run "pip-audit project" "$PYTHON_BIN" -m pip_audit .
run "pip-audit scraper requirements" "$PYTHON_BIN" -m pip_audit -r scraper/requirements.txt

echo
echo "Local CI parity checks completed."
