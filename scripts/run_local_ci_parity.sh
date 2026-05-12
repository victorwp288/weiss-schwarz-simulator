#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

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

for module in maturin ruff pytest; do
  if ! "$PYTHON_BIN" -m "$module" --version >/dev/null 2>&1; then
    echo "ERROR: $module is required for local parity checks." >&2
    exit 127
  fi
done

log_step() {
  echo
  echo "==> $1"
}

run() {
  log_step "$1"
  shift
  "$@"
}

can_write_dir() {
  local dir="$1"
  local probe="$dir/.wss_write_probe.$$"
  if ( : >"$probe" ) 2>/dev/null; then
    rm -f "$probe" >/dev/null 2>&1 || true
    return 0
  fi
  return 1
}

can_resolve_host() {
  local host="$1"
  "$PYTHON_BIN" - <<PY >/dev/null 2>&1
import socket
socket.getaddrinfo("$host", 443)
PY
}

MIN_PARSE_LINE_COVERAGE_STRICT="${MIN_PARSE_LINE_COVERAGE_STRICT:-0.52}"
MAX_UNSUPPORTED_LINES_STRICT="${MAX_UNSUPPORTED_LINES_STRICT:-14200}"
MIN_CARD_COVERAGE_APPROX="${MIN_CARD_COVERAGE_APPROX:-0.99}"
SKIP_BENCHMARKS="${SKIP_BENCHMARKS:-0}"

run "Check env layering" ./scripts/check_env_layering.sh
run "Docs link check" "$PYTHON_BIN" scripts/check_docs_links.py
run "Docs constants check" "$PYTHON_BIN" scripts/check_docs_constants.py
run "Generated docs check" "$PYTHON_BIN" scripts/gen_docs_snippets.py --check
run "Packaged data freshness" "$PYTHON_BIN" scripts/check_packaged_data.py

run "Cargo fmt" cargo fmt --all -- --check
run "Cargo clippy" cargo clippy --workspace --all-targets --all-features -- -D warnings
run "Cargo test" cargo test --workspace --all-features
run "Cargo doc (missing docs denied)" env RUSTDOCFLAGS="-D missing-docs" cargo doc -p weiss_core --all-features --no-deps

run "Ruff format" "$PYTHON_BIN" -m ruff format --check python scraper scripts
run "Ruff check" "$PYTHON_BIN" -m ruff check python scraper scripts

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
run "Build wheel" "$PYTHON_BIN" -m maturin build --release --manifest-path weiss_py/Cargo.toml --out /tmp/wss_dist --interpreter "$PYTHON_BIN"
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
    --core-budget-override reset_batch_256=25 \
    --max-python-regression-pct 10 \
    --require-zero-alloc

  trap - EXIT
  cleanup_perf_workdirs
fi

PIP_AUDIT_CACHE_DIR="${PIP_AUDIT_CACHE_DIR:-/tmp/pip-audit-cache}"

AUDIT_DB_DEFAULT="${HOME:-}/.cargo/advisory-db"
AUDIT_DB_PARENT="${HOME:-}/.cargo"
AUDIT_DB_FALLBACK="$ROOT_DIR/target/advisory-db"
AUDIT_DB_ARGS=()

if [[ -n "${CARGO_AUDIT_DB:-}" ]]; then
  AUDIT_DB_ARGS=(--db "$CARGO_AUDIT_DB")
elif can_write_dir "$AUDIT_DB_PARENT"; then
  AUDIT_DB_ARGS=(--db "$AUDIT_DB_DEFAULT")
else
  if [[ ! -d "$AUDIT_DB_FALLBACK/.git" && -d "$AUDIT_DB_DEFAULT/.git" ]]; then
    rm -rf "$AUDIT_DB_FALLBACK"
    mkdir -p "$(dirname "$AUDIT_DB_FALLBACK")"
    cp -R "$AUDIT_DB_DEFAULT" "$AUDIT_DB_FALLBACK"
  fi
  AUDIT_DB_ARGS=(--db "$AUDIT_DB_FALLBACK" --no-fetch --stale)
fi

run "Cargo audit" cargo audit "${AUDIT_DB_ARGS[@]}"

if [[ "${PIP_AUDIT_OFFLINE:-0}" == "1" ]] || ! can_resolve_host "pypi.org"; then
  echo
  echo "WARNING: pip-audit offline dry-run does not check for known vulnerabilities." >&2
  run "pip-audit (offline dry-run; no vuln check)" \
    "$PYTHON_BIN" -m pip_audit \
    --local \
    --dry-run \
    --cache-dir "$PIP_AUDIT_CACHE_DIR" \
    --progress-spinner off
else
  run "pip-audit project" \
    "$PYTHON_BIN" -m pip_audit --cache-dir "$PIP_AUDIT_CACHE_DIR" --progress-spinner off .
  run "pip-audit scraper requirements" \
    "$PYTHON_BIN" -m pip_audit --cache-dir "$PIP_AUDIT_CACHE_DIR" --progress-spinner off -r scraper/requirements.txt
fi

echo
echo "Local CI parity checks completed."
