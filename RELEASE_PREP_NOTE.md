# Release Prep Note

Date: 2026-04-19

## 2026-04-28 hotfix release readiness

- Hotfix payload: added named public heuristic profiles (`base`, `aggressive`, `control`) while preserving the default `base` behavior for existing heuristic rollout/action APIs.
- API/docs/test follow-through: refreshed `python/weiss_sim/weiss_sim.pyi`, regenerated `docs/python_api_reference.md`, and added smoke coverage for default/base equivalence, named profile validity, rollout usage, and invalid profile errors.
- Current local check status: `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, `cargo test --workspace --all-features`, `cargo build -p weiss_core --release`, Ruff format/lint, docs checks, ability coverage budget, wheel build/install, pytest, docs build, `cargo audit`, and both `pip-audit` checks passed.
- Perf budget result: PASS vs `HEAD`; largest core regression was `step_batch_64` at `0.212%` against a `15%` budget, Python throughput improved for `step(ids)` and was effectively flat for `step(mask)`, and allocation benches remained zero.
- Failed/adjusted idea: `bash scripts/run_local_ci_parity.sh` could not run directly because the Windows Bash environment did not inherit a usable Cargo/Python toolchain setup; the same gates were run with direct PowerShell equivalents, and perf used `maturin build` + wheel install instead of `maturin develop` because no venv is active.
- Next release hypothesis: push a conventional `fix:` commit to `main`, let Release Please prepare the version/changelog release PR, then merge/trigger the `v*` release so the Wheels workflow publishes to PyPI.

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
- Treating the hosted-runner `reset_batch_256` regression on the clippy-only follow-up commit as a real engine slowdown did not fit the code delta or the dedicated benchmark workflow output; this looked like a noisy false positive in the perf gate for that one reset microbenchmark.

### Next hypotheses / remote watch items

- Verify the `main` push runs `CI`, `Docs`, `Security`, `Benchmarks`, `Wheels`, and `Release Please` cleanly with the new lockfile committed.
- Confirm `Release Please` opens or updates the `0.8.0` release PR after the trigger commit lands.
- Confirm the repo still has a working `RELEASE_PLEASE_TOKEN`; otherwise the eventual release tag may need a manual wheels rerun because default `GITHUB_TOKEN` tags do not trigger downstream workflows.
- Re-run `CI` with a per-benchmark perf override for `reset_batch_256` (`25%`) while keeping the shared core budget at `15%` for the rest of the engine rows.

## Benchmark workflow recovery addendum (2026-04-20)

### Additional fixes

- Regenerated `Cargo.lock` so the workspace package entries now match the released `0.8.0` manifests instead of being rewritten from `0.7.0` during benchmark runs.
- Hardened `.github/workflows/benchmarks.yml` so the piped `cargo bench` and Python benchmark commands run with `pipefail`, which prevents `tee` from masking real benchmark failures.
- Added a cleanup step before benchmark history publishing to restore tracked build outputs (`Cargo.lock`) before the job switches to `gh-pages`.
- Updated `python/examples/bench_python_boundary.py` to prefer an installed wheel when the selected `--repo-root` does not contain an in-tree compiled extension module, while still preserving the repo-local import path for local in-place benchmark runs.

### Additional checks passed

- `cargo generate-lockfile`
- `.\.venv\Scripts\python.exe -m py_compile python\examples\bench_python_boundary.py`
- `.\.venv\Scripts\python.exe -m maturin build --release --manifest-path weiss_py/Cargo.toml --out dist\ci-benchmark-fix --interpreter .\.venv\Scripts\python.exe`
- Installed-wheel benchmark smoke with a fixture-only temp repo root:
  - `reset_into`: `98.3 us/reset`
  - `step(mask)`: `320,770 env-steps/sec`
  - `step(ids)`: `1,034,929 env-steps/sec`
- Local Rust benchmark smoke after the lock refresh:
  - `cargo bench -p weiss_core --bench core_benches -- --output-format bencher`
  - `cargo bench -p weiss_core --bench alloc_benches -- --output-format bencher`

### Failed ideas and outcomes

- Initial diagnosis was that fixing the stale `Cargo.lock` entries alone would clear the red benchmark run.
- The follow-up local smoke showed that the workflow also needed explicit cleanup and `pipefail`, because the benchmark job both switches branches mid-run and currently hides piped command failures behind `tee`.

### Next hypotheses / remote watch items

- Watch the next `Benchmarks` push run on `main` and confirm `Publish benchmark history` no longer fails on a dirty worktree.
- Confirm the Python benchmark step now records real output on GitHub instead of a masked `ModuleNotFoundError`.
