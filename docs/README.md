# Documentation

This folder is intentionally small. Each page should be useful to a human reader and
durable enough to survive refactors.

## Read This Way

- [Quickstart](quickstart.md): install, first reset/step loop, deck inputs, troubleshooting, and local checks.
- [Python API](python_api.md): high-level `make/fast/inspect`, low-level `EnvPoolBuffers`, layouts, and RL helper surfaces.
- [RL Contract](rl_contract.md): step semantics, reward signs, output schema, legal-action payloads, and compatibility constants.
- [Architecture](architecture.md): Rust/Python layers, module map, rules/scraper boundaries, replay determinism, and release-sensitive invariants.
- [Performance](performance_benchmarks.md): benchmark commands, current baselines, perf gates, and RL hot-path guidance.
- [Contributing](../CONTRIBUTING.md): local validation, release flow, docs rules, and PR checklist.

## Compatibility Rule

If a change affects observation/action encodings, replay payloads, WSDB format, legal
payload semantics, or public Python names, update code, tests, and docs in the same PR.

Key boundaries:

- `OBS_LEN=378`
- `ACTION_SPACE_SIZE=527`
- `OBS_ENCODING_VERSION=2`
- `ACTION_ENCODING_VERSION=1`
- `POLICY_VERSION=2`
- `REPLAY_SCHEMA_VERSION=3`
- `WSDB_SCHEMA_VERSION=2`
- `SPEC_HASH=8590000130`

Docs checks:

```bash
python scripts/check_docs_links.py
python scripts/check_docs_constants.py
python scripts/gen_docs_snippets.py --check
```
