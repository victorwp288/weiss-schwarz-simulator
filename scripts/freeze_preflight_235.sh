#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

usage() {
  cat <<'EOF'
Usage: scripts/freeze_preflight_235.sh [output-dir]

Runs freeze prep checks for:
  2) perf + coverage gates
  3) environment/version snapshot
  5) RL contract constants consistency

Environment toggles:
  SKIP_COVERAGE=1    Skip coverage report + budget gate
  SKIP_PERF=1        Skip perf snapshot + perf budget gate
  VENV_PYTHON=...    Force Python interpreter path
EOF
}

if [[ "${1:-}" == "-h" || "${1:-}" == "--help" ]]; then
  usage
  exit 0
fi

is_windows_bash_shell() {
  case "${OSTYPE:-}" in
    msys*|cygwin*) return 0 ;;
  esac
  case "$(uname -s 2>/dev/null || echo)" in
    MINGW*|MSYS*|CYGWIN*) return 0 ;;
  esac
  [[ -n "${MSYSTEM:-}" ]]
}

if [[ -n "${VENV_PYTHON:-}" ]]; then
  PYTHON_BIN="$VENV_PYTHON"
elif [[ -x "$ROOT_DIR/.venv/bin/python" ]]; then
  PYTHON_BIN="$ROOT_DIR/.venv/bin/python"
elif is_windows_bash_shell && [[ -x "$ROOT_DIR/.venv/Scripts/python.exe" ]]; then
  PYTHON_BIN="$ROOT_DIR/.venv/Scripts/python.exe"
elif command -v python3 >/dev/null 2>&1; then
  PYTHON_BIN="python3"
else
  PYTHON_BIN="python"
fi

if ! command -v "$PYTHON_BIN" >/dev/null 2>&1; then
  echo "ERROR: python interpreter not found: $PYTHON_BIN" >&2
  exit 127
fi

for cmd in cargo git; do
  if ! command -v "$cmd" >/dev/null 2>&1; then
    echo "ERROR: required command not found: $cmd" >&2
    exit 127
  fi
done

if ! "$PYTHON_BIN" -c "import pip" >/dev/null 2>&1; then
  echo "ERROR: pip is required for environment snapshot." >&2
  exit 127
fi

if ! "$PYTHON_BIN" -m maturin --version >/dev/null 2>&1; then
  echo "ERROR: maturin is required for freeze preflight." >&2
  exit 127
fi

OUT_DIR="${1:-/tmp/wss_freeze_preflight_$(date -u +%Y%m%dT%H%M%SZ)}"
mkdir -p "$OUT_DIR"
OUT_DIR="$(cd "$OUT_DIR" && pwd)"

SKIP_COVERAGE="${SKIP_COVERAGE:-0}"
SKIP_PERF="${SKIP_PERF:-0}"

MIN_PARSE_LINE_COVERAGE_STRICT="${MIN_PARSE_LINE_COVERAGE_STRICT:-0.52}"
MAX_UNSUPPORTED_LINES_STRICT="${MAX_UNSUPPORTED_LINES_STRICT:-14200}"
MIN_CARD_COVERAGE_APPROX="${MIN_CARD_COVERAGE_APPROX:-0.99}"

log_step() {
  echo
  echo "==> $1"
}

sha256_file() {
  local target="$1"
  if command -v shasum >/dev/null 2>&1; then
    shasum -a 256 "$target"
  elif command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$target"
  else
    echo "sha256 unavailable for $target"
  fi
}

capture_environment_snapshot() {
  log_step "Environment snapshot (item 3)"
  {
    echo "captured_utc=$(date -u +%Y-%m-%dT%H:%M:%SZ)"
    echo "repo_root=$ROOT_DIR"
    echo "git_head=$(git rev-parse HEAD)"
    echo "python_bin=$PYTHON_BIN"
    echo "python_version=$("$PYTHON_BIN" --version 2>&1)"
    echo "pip_version=$("$PYTHON_BIN" -m pip --version 2>&1)"
    echo "maturin_version=$("$PYTHON_BIN" -m maturin --version 2>&1)"
    echo "rustc_version=$(rustc --version 2>&1)"
    echo "cargo_version=$(cargo --version 2>&1)"
    if command -v rustup >/dev/null 2>&1; then
      echo "rustup_active_toolchain=$(rustup show active-toolchain 2>/dev/null || true)"
    fi
  } > "$OUT_DIR/environment_snapshot.txt"

  "$PYTHON_BIN" -m pip freeze | LC_ALL=C sort > "$OUT_DIR/pip_freeze.txt"
  git status --short > "$OUT_DIR/git_status_short.txt"
  git rev-parse HEAD > "$OUT_DIR/git_head.txt"

  {
    sha256_file "$ROOT_DIR/pyproject.toml"
    sha256_file "$ROOT_DIR/Cargo.lock"
    sha256_file "$ROOT_DIR/scripts/ability_coverage_baseline.json"
    sha256_file "$ROOT_DIR/benchmark/benches.txt"
    sha256_file "$ROOT_DIR/benchmark/python_bench.txt"
    sha256_file "$ROOT_DIR/docs/rl_contract.md"
  } > "$OUT_DIR/input_sha256.txt"
}

run_contract_consistency() {
  log_step "Contract constants check (item 5)"
  "$PYTHON_BIN" scripts/check_docs_constants.py
}

run_coverage_gate() {
  if [[ "$SKIP_COVERAGE" == "1" ]]; then
    log_step "Coverage gate skipped (SKIP_COVERAGE=1)"
    return
  fi

  log_step "Coverage report + budget gate (item 2)"
  "$PYTHON_BIN" scripts/ability_coverage_report.py --output "$OUT_DIR/ability_coverage_report.json"
  "$PYTHON_BIN" scripts/ability_coverage_targets.py \
    --report "$OUT_DIR/ability_coverage_report.json" \
    --output "$OUT_DIR/ability_coverage_targets.json"
  "$PYTHON_BIN" scripts/check_coverage_budget.py \
    --report "$OUT_DIR/ability_coverage_report.json" \
    --baseline scripts/ability_coverage_baseline.json \
    --min-parse-line-coverage-strict "$MIN_PARSE_LINE_COVERAGE_STRICT" \
    --max-unsupported-lines-strict "$MAX_UNSUPPORTED_LINES_STRICT" \
    --min-card-coverage-approx "$MIN_CARD_COVERAGE_APPROX"
}

run_perf_gate() {
  if [[ "$SKIP_PERF" == "1" ]]; then
    log_step "Perf gate skipped (SKIP_PERF=1)"
    return
  fi

  log_step "Build extension for perf snapshot (item 2)"
  "$PYTHON_BIN" -m maturin develop --release --manifest-path weiss_py/Cargo.toml

  log_step "Perf snapshot + budget gate (item 2)"
  local perf_out="$OUT_DIR/perf_snapshot"
  PYTHON_BIN="$PYTHON_BIN" scripts/run_perf_snapshot.sh "$perf_out" "$ROOT_DIR"
  "$PYTHON_BIN" scripts/check_perf_budget.py \
    --baseline-benches benchmark/benches.txt \
    --current-benches "$perf_out/benches.txt" \
    --baseline-python benchmark/python_bench.txt \
    --current-python "$perf_out/python_bench.txt" \
    --max-core-regression-pct 15 \
    --max-python-regression-pct 10 \
    --require-zero-alloc
}

capture_environment_snapshot
run_contract_consistency
run_coverage_gate
run_perf_gate

log_step "Freeze preflight 2/3/5 completed"
echo "Artifacts: $OUT_DIR"
