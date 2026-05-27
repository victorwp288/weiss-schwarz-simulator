# Human Decision View Progress

Date: 2026-05-27

## Current best metrics

- API: `weiss_sim.human_decision_view(pool, env_index=0, perspective_seat=None)`
- Schema: `human_decision_view_v1`
- Legal actions: decoded only from the current simulator legal-id cache, preserving simulator order and exact `action_id` submission.
- Local timing sample on Windows/Python 3.12, one reset-state env, 5,000 iterations:
  - `EnvPool.human_decision_view_json(...)`: 0.4899 s total, 97.98 us/call
  - `weiss_sim.human_decision_view(...)` including JSON parse: 0.9004 s total, 180.08 us/call
  - Payload size: 19,868 bytes
  - Legal actions in sample decision: 6

## Validation run

- `cargo fmt --all -- --check`
- `bash scripts/check_env_layering.sh`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test --workspace --all-features`
- `cargo build -p weiss_core --release`
- `cargo doc --workspace --all-features --no-deps`
- `python -m maturin build --release --manifest-path weiss_py/Cargo.toml`
- `python -m pip install --force-reinstall --no-deps target\wheels\weiss_sim-1.1.0-cp312-cp312-win_amd64.whl`
- `python -m ruff format --check python scraper scripts`
- `python -m ruff check python scraper scripts`
- `python scripts/check_docs_links.py`
- `python scripts/check_docs_constants.py`
- `python scripts/gen_docs_snippets.py --check`
- `python scripts/check_packaged_data.py`
- `python scripts/ability_coverage_report.py --output $env:TEMP\ability_coverage_report.json`
- `python scripts/ability_coverage_targets.py --report $env:TEMP\ability_coverage_report.json --output $env:TEMP\ability_coverage_targets.json`
- `python scripts/check_coverage_budget.py --report $env:TEMP\ability_coverage_report.json --baseline scripts/ability_coverage_baseline.json --min-parse-line-coverage-strict 0.52 --max-unsupported-lines-strict 14200 --min-card-coverage-approx 0.99`
- `python -m unittest scraper.test_convert`
- `python -m pytest -q python/tests`
- `cargo audit`
- `python -m pip_audit --progress-spinner off .`
- `python -m pip_audit --progress-spinner off -r scraper\requirements.txt`

2026-05-27 status: all commands above passed locally on Windows with Rust 1.95.0 and Python 3.12.10. The packaged Python test run reported `155 passed, 3 skipped`.

## Failed or rejected ideas

- Did not scan the full action space to construct UI choices. Legal action display is aligned to `action_ids_cache()` for the current decision.
- Did not reuse `render_ansi`; it is debug output and not strict enough as a public-view contract.
- Did not expose raw effect text or card instance ids in the human view. The event log is sanitized and strips `instance_id` keys before serialization.

## Next hypotheses

- Add richer card names/codes when the Rust DB carries those fields or when Python joins against the catalog for display only.
- Add targeted choice-prompt tests around revealed duplicate copies if the human UI starts surfacing choice option details heavily.
- If UI refresh frequency becomes high, consider a no-parse Python helper that returns the JSON string directly and lets the frontend parse it.
