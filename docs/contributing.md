# Contributing

This project prioritizes deterministic behavior, stable contracts, and reproducible performance.

A contribution is complete only when code, tests, and docs all agree.

## Contribution Workflow

1. Create a focused branch and keep changes scoped.
2. Implement behavior with tests first (or in lock-step).
3. Update docs in the same PR.
4. Run quality gates locally.
5. Open PR with explicit notes on determinism and contract impact.

## Repository Areas

- `weiss_core/`: Rust engine implementation and encode/state contracts.
- `weiss_py/`: PyO3 boundary.
- `python/weiss_sim/`: Python wrappers and ergonomics.
- `python/tests/`: Python API and integration tests.
- `docs/`: user/developer documentation.
- `scripts/`: CI and local quality checks.

## Local Quality Gates

Run these before pushing.

### Rust

```bash
scripts/check_env_layering.sh
python scripts/check_docs_links.py
python scripts/check_docs_constants.py
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --features test-harness
cargo build -p weiss_core --release
```

### Python

```bash
ruff format --check python scraper scripts
ruff check python scraper scripts
pytest -q python/tests
```

## Determinism and Contract Rules

If behavior changes can affect encoding/action semantics, update all of:

1. source constants and implementation
2. [RL Contract](rl_contract.md) checksum table
3. [Encodings Changelog](encodings_changelog.md)
4. relevant tests (Rust and/or Python)

Never change version constants casually. Bump only with an intentional compatibility decision.

## Docs Definition of Done

For each behavior PR:

- Update at least one canonical doc page (not only inline comments).
- Keep `PROJECT_STATE.md` aligned with actual engine behavior.
- Ensure links remain valid (`check_docs_links.py`).
- Ensure checksum constants remain accurate (`check_docs_constants.py`).

## PR Template Expectations

A strong PR description includes:

- behavior change summary
- determinism impact (or explicit "none")
- encoding/schema impact (or explicit "none")
- test evidence (commands + results)
- docs updated (which files)

## Benchmarks (when relevant)

If your change touches hot paths or allocation behavior, run and include benchmark data:

```bash
cargo bench -p weiss_core --bench core_benches
cargo bench -p weiss_core --bench alloc_benches
python python/examples/bench_python_boundary.py --num-envs 256 --steps 5000 --mode both
```

## Related

- [Docs hub](README.md)
- [Python API guide](python_api.md)
- [Engine architecture](engine_architecture.md)
- [Rules coverage](rules_coverage.md)
- [Project state](../PROJECT_STATE.md)
