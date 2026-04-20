# Release Prep Note

Date: 2026-04-19

## Fixed bugs and inconsistencies

- Hardened Python legal-action metadata loading so mismatched row counts now fail fast instead of silently returning misaligned metadata.
- Fixed `WeissEnv.render()` perspective selection when the actor seat is unknown (`to_play_seat == -1`) by falling back to the starting seat.
- Corrected the Python stub/runtime contract for `BatchOutDebug` so typed consumers see `main_move_action` and `main_pass_action`.
- Refreshed the golden transcript fixture after validating that only config/state hash drift changed and the action/event sequence stayed stable.
- Fixed Windows-local docs link checking by reading markdown files as UTF-8 explicitly.
- Closed real CI path-filter gaps in `.github/workflows/ci.yml` so docs, rust, and perf jobs now rerun when their own guard scripts or repo line-ending policy change.
- Fixed Bash helper portability on Windows checkouts by enforcing LF for shell scripts and `scripts/*.py`, which unblocks `bash ./scripts/check_env_layering.sh` and other shebang-based helpers under Git Bash/WSL.
- Fixed Bash helper venv detection so `run_local_ci_parity.sh`, `freeze_preflight_235.sh`, and `setup_dev_env.sh` now use the shell-native interpreter in WSL/Unix shells while still finding `.venv/Scripts/python.exe` in Windows-native Bash shells.
- Fixed Bash helper tooling resolution so parity/preflight now use `python -m maturin|ruff|pytest`, matching the documented venv-first workflow instead of requiring those tools to be globally on PATH.
- Fixed the local parity script so its Rust lint/test/doc gates now run with `--all-features`, matching the actual CI/release surface instead of a weaker subset.
- Restored the `BatchOutTrajectoryI16LegalIds` contract so `spec_hash` again carries the simulator compatibility hash and heuristic-public/native legal-id rollouts expose per-step `episode_seed` through a dedicated field instead of overloading `spec_hash`.

## Refactors and polish

- Synced Python API docs, generated reference docs, and RL contract docs with the current exported surfaces:
  `action_factorization_v1`, `action_meta_v1`, factorized action helpers, card table export, legal action metadata, and main-action flags.
- Tightened CI so Rust lint/test coverage now runs with `--all-features`, matching the real release surface.
- Updated contributor setup and Windows verification instructions in the README to use direct `pip`/`maturin` commands instead of assuming Bash.
- Added/extended Python tests around factorized action helpers, card table export, legal action metadata, render fallback behavior, and unknown action decode handling.
- Updated release/contributor/troubleshooting docs to use explicit `bash scripts/...` invocations for Bash helpers, document `rustup component add rustfmt clippy`, and remove a stale hardcoded release version example from the release guide.
- Normalized install/test snippets across the Markdown docs so venv-first flows now consistently use `python -m maturin`, `python -m pytest`, and `python -m ruff` where the project scripts expect module-based tool invocation.

## Dead code removal

- No public modules or compatibility layers were removed in this pass.
- Suspected cleanup candidates were left in place when they still formed part of the Python/RL contract or public release surface.

## Checks passed

- `cargo fmt --check`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test --workspace`
- `cargo test --workspace --all-features`
- `bash ./scripts/check_env_layering.sh`
- `bash ./scripts/freeze_preflight_235.sh --help`
- `cargo doc --workspace --all-features --no-deps` with `RUSTDOCFLAGS="-D missing-docs"`
- `python -m ruff format --check python scraper scripts`
- `python -m ruff check python scraper scripts`
- `python -m pytest -q python/tests`
- `python scripts/check_docs_links.py`
- `python scripts/check_docs_constants.py`
- `python scripts/gen_docs_snippets.py --check`
- `python scripts/ability_coverage_report.py --output ...`
- `python scripts/ability_coverage_targets.py --report ... --output ...`
- `python scripts/check_coverage_budget.py --report ... --baseline scripts/ability_coverage_baseline.json ...`
- `python -m maturin develop --release --manifest-path weiss_py/Cargo.toml`
- `python -m maturin build --release --manifest-path weiss_py/Cargo.toml --out dist --interpreter .\.venv\Scripts\python.exe`
- Installed-wheel smoke test: import, reset, sample legal actions, and step on the built wheel

## Remaining risks or blockers

- No code blockers found in this pass.
- Operational note: on Windows, `maturin develop` can fail if another Python process is still holding the built `.pyd`; rerunning the command serially resolved it.
- Operational note: the Bash helper scripts now resolve the project venv correctly on Windows, but an end-to-end `bash ./scripts/run_local_ci_parity.sh` still depends on the Bash-side Rust toolchain having `rustfmt`/`clippy` installed. The equivalent direct checks all passed from PowerShell in this pass.

## Audit follow-up (2026-04-20)

### Additional fixes

- Reformatted the remaining Python files left dirty by the cleanup pass so the simulator repo is formatter-clean again:
  - `python/tests/test_legal_ids_nomask_logits.py`
  - `python/tests/test_smoke.py`
  - `python/weiss_sim/_buffers.py`
  - `python/weiss_sim/weiss_sim.pyi`

### Additional checks passed

- `python -m ruff format --check python scraper scripts`
- `python -m ruff check python scraper scripts`
- `python -m pytest -q python/tests` -> `135 passed, 4 skipped`
- `python scripts/check_docs_links.py`
- `python scripts/check_docs_constants.py`
- `python scripts/gen_docs_snippets.py --check`
- `python -m maturin build --release --manifest-path weiss_py/Cargo.toml --out dist/audit --interpreter .\.venv\Scripts\python.exe`
- `git diff --check`

### Audit result

- No additional simulator correctness bugs or release blockers were found in this follow-up audit.
- The repo now passes the Rust, Python, docs, and wheel-build verification surface exercised in this pass.

## Release 0.8 readiness addendum (2026-04-20)

### Additional fixes

- Added a committed `Cargo.lock` for the workspace and pinned `constant_time_eq` to `0.4.2` so fresh checkouts keep resolving a Rust-1.93-compatible dependency set.
- Verified that a clean detached checkout of the current tree now rebuilds successfully with `python -m maturin develop --release --manifest-path weiss_py/Cargo.toml` once the lockfile is present.

### Additional checks passed

- `cargo generate-lockfile`
- `cargo update -p constant_time_eq --precise 0.4.2`
- `cargo audit`
- `python -m pip_audit .`
- `python -m pip_audit -r scraper/requirements.txt`
- Fresh-checkout rebuild smoke test from a detached worktree using the committed `Cargo.lock`
- Perf budget comparison against `origin/main` code using the pinned lockfile:
  - `advance_until_decision`: `20,996ns -> 22,347ns` (`+6.44%`)
  - `step_batch_fast_256_priority_off`: `35,850ns -> 40,630ns` (`+13.33%`)
  - `default::step(mask)`: `1,951,514 eps -> 1,813,346 eps` (`+7.08%`)
  - overall result: `python scripts/check_perf_budget.py ...` -> `PASS`

### Failed ideas and outcomes

- Tried validating perf/rebuild parity from clean detached worktrees before introducing a lockfile.
- That failed because fresh dependency resolution pulled `constant_time_eq 0.4.3`, which requires Rust `1.95.0`, so clean `maturin develop` runs could fail on fresh runners even though this workstation still had warm local build state.

### Next hypotheses / remote watch items

- Verify the `main` push runs `CI`, `Docs`, `Security`, `Benchmarks`, `Wheels`, and `Release Please` cleanly with the new lockfile committed.
- Confirm `Release Please` opens or updates the `0.8.0` release PR after the trigger commit lands.
- Confirm the repo still has a working `RELEASE_PLEASE_TOKEN`; otherwise the eventual release tag may need a manual wheels rerun because default `GITHUB_TOKEN` tags do not trigger downstream workflows.
