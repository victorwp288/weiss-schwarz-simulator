# Release Prep Note

Date: 2026-04-19

## Fixed bugs and inconsistencies

- Hardened Python legal-action metadata loading so mismatched row counts now fail fast instead of silently returning misaligned metadata.
- Fixed `WeissEnv.render()` perspective selection when the actor seat is unknown (`to_play_seat == -1`) by falling back to the starting seat.
- Corrected the Python stub/runtime contract for `BatchOutDebug` so typed consumers see `main_move_action` and `main_pass_action`.
- Refreshed the golden transcript fixture after validating that only config/state hash drift changed and the action/event sequence stayed stable.
- Fixed Windows-local docs link checking by reading markdown files as UTF-8 explicitly.

## Refactors and polish

- Synced Python API docs, generated reference docs, and RL contract docs with the current exported surfaces:
  `action_factorization_v1`, `action_meta_v1`, factorized action helpers, card table export, legal action metadata, and main-action flags.
- Tightened CI so Rust lint/test coverage now runs with `--all-features`, matching the real release surface.
- Updated contributor setup and Windows verification instructions in the README to use direct `pip`/`maturin` commands instead of assuming Bash.
- Added/extended Python tests around factorized action helpers, card table export, legal action metadata, render fallback behavior, and unknown action decode handling.

## Dead code removal

- No public modules or compatibility layers were removed in this pass.
- Suspected cleanup candidates were left in place when they still formed part of the Python/RL contract or public release surface.

## Checks passed

- `cargo fmt --check`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test --workspace`
- `cargo test --workspace --all-features`
- `ruff format --check python scraper scripts`
- `ruff check python scraper scripts`
- `pytest -q python/tests`
- `python scripts/check_docs_links.py`
- `python scripts/check_docs_constants.py`
- `python scripts/gen_docs_snippets.py --check`
- `python scripts/ability_coverage_report.py --output ...`
- `python scripts/ability_coverage_targets.py --report ... --output ...`
- `python scripts/check_coverage_budget.py --report ... --baseline scripts/ability_coverage_baseline.json ...`
- `maturin develop --release --manifest-path weiss_py/Cargo.toml`
- `maturin build --release --manifest-path weiss_py/Cargo.toml --out dist --interpreter .\.venv\Scripts\python.exe`
- Installed-wheel smoke test: import, reset, sample legal actions, and step on the built wheel

## Remaining risks or blockers

- No code blockers found in this pass.
- Operational note: on Windows, `maturin develop` can fail if another Python process is still holding the built `.pyd`; rerunning the command serially resolved it.
