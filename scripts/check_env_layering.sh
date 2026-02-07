#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

if ! command -v rg >/dev/null 2>&1; then
  echo "ERROR: rg (ripgrep) is required for scripts/check_env_layering.sh" >&2
  exit 127
fi

RG_BASE=(rg --type rust --pcre2 -n -g '!target/**' -g '!docs/**')
RG_SANITIZE=(--pre "./scripts/strip_rust_layering.py")

fail=0

check() {
  local label="$1"
  local pattern="$2"
  local path="$3"
  local rc=0
  if "${RG_BASE[@]}" "$pattern" "$path"; then
    rc=0
  else
    rc=$?
  fi
  if [[ $rc -eq 0 ]]; then
    echo "ERROR: ${label}"
    fail=1
  elif [[ $rc -ne 1 ]]; then
    echo "ERROR: rg failed for ${label} (exit $rc)"
    fail=1
  fi
}

check_sanitized() {
  local label="$1"
  local pattern="$2"
  local path="$3"
  local rc=0
  if "${RG_BASE[@]}" "${RG_SANITIZE[@]}" "$pattern" "$path"; then
    rc=0
  else
    rc=$?
  fi
  if [[ $rc -eq 0 ]]; then
    echo "ERROR: ${label}"
    fail=1
  elif [[ $rc -ne 1 ]]; then
    echo "ERROR: rg failed for ${label} (exit $rc)"
    fail=1
  fi
}

advance_dir="weiss_core/src/env/advance"
movement_dir="weiss_core/src/env/movement"
interaction_dir="weiss_core/src/env/interaction"
phases_dir="weiss_core/src/env/phases"
shared_file="weiss_core/src/env/shared.rs"

advance_sub_phases="rule_actions|trigger_pipeline|priority_window|stand|end|attack|losses"
advance_sub_movement="play|level|encore|zones|stock|power|requirements|counter|draw"
advance_sub_interaction="choice|targeting|priority|stack|effects|costs|damage|queue"

check_sanitized "advance must not import env::phases::<submodule>" \
  "^(?!\\s*//).*\\b(?:crate::env|env|super|self)::phases::(?:${advance_sub_phases})\\b" \
  "$advance_dir"
check_sanitized "advance must not import env::movement::<submodule>" \
  "^(?!\\s*//).*\\b(?:crate::env|env|super|self)::movement::(?:${advance_sub_movement})\\b" \
  "$advance_dir"
check_sanitized "advance must not import env::interaction::<submodule>" \
  "^(?!\\s*//).*\\b(?:crate::env|env|super|self)::interaction::(?:${advance_sub_interaction})\\b" \
  "$advance_dir"

check "movement must not import env::interaction" \
  "^(?!\\s*//).*\\b(?:crate::env|env|super|self)::interaction(?:\\b|::)" \
  "$movement_dir"
check "movement must not import env::phases" \
  "^(?!\\s*//).*\\b(?:crate::env|env|super|self)::phases(?:\\b|::)" \
  "$movement_dir"
check "movement must not import env::advance" \
  "^(?!\\s*//).*\\b(?:crate::env|env|super|self)::advance(?:\\b|::)" \
  "$movement_dir"

check "interaction must not import env::movement" \
  "^(?!\\s*//).*\\b(?:crate::env|env|super|self)::movement(?:\\b|::)" \
  "$interaction_dir"
check "interaction must not import env::phases" \
  "^(?!\\s*//).*\\b(?:crate::env|env|super|self)::phases(?:\\b|::)" \
  "$interaction_dir"
check "interaction must not import env::advance" \
  "^(?!\\s*//).*\\b(?:crate::env|env|super|self)::advance(?:\\b|::)" \
  "$interaction_dir"

check "phases must not import env::movement" \
  "^(?!\\s*//).*\\b(?:crate::env|env|super|self)::movement(?:\\b|::)" \
  "$phases_dir"
check "phases must not import env::interaction" \
  "^(?!\\s*//).*\\b(?:crate::env|env|super|self)::interaction(?:\\b|::)" \
  "$phases_dir"
check "phases must not import env::advance" \
  "^(?!\\s*//).*\\b(?:crate::env|env|super|self)::advance(?:\\b|::)" \
  "$phases_dir"

check "shared must not import crate modules outside db/state/encode/config" \
  "^(?!\\s*//).*\\bcrate::(?!db\\b|db::|state\\b|state::|encode\\b|encode::|config\\b|config::)" \
  "$shared_file"

if [[ $fail -ne 0 ]]; then
  exit 1
fi
