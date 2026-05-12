## Summary

<!-- concise summary -->

## Behavior and Contract Impact

- [ ] Simulator behavior changed
- [ ] Public API changed
- [ ] Encoding, replay, or WSDB constants changed
- [ ] Performance-sensitive path changed
- [ ] Documentation updated

## Validation

- [ ] `cargo fmt --all -- --check`
- [ ] `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- [ ] `cargo test --workspace --all-features`
- [ ] `cargo doc --workspace --all-features --no-deps`
- [ ] `python -m ruff format --check python scraper scripts`
- [ ] `python -m ruff check python scraper scripts`
- [ ] `python -m pytest -q python/tests`
- [ ] `python -m unittest scraper.test_convert`
- [ ] `python scripts/check_docs_constants.py`
- [ ] `python scripts/check_docs_links.py`
- [ ] `python scripts/gen_docs_snippets.py --check`
- [ ] Benchmarks or perf-budget check, if a hot path changed
- [ ] Changelog/release notes updated, if user-facing behavior changed

## Notes

<!-- reviewer context, risks, or follow-up work -->
