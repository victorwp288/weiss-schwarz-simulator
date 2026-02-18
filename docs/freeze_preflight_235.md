# Freeze Preflight (Items 2, 3, 5)

Runbook for `scripts/freeze_preflight_235.sh`.

Targets:

- item 2: perf + coverage gates
- item 3: environment snapshot
- item 5: RL contract consistency check

## One-command run

```bash
scripts/freeze_preflight_235.sh
```

Optional output directory:

```bash
scripts/freeze_preflight_235.sh /tmp/wss_freeze_candidate
```

## Optional toggles

```bash
SKIP_COVERAGE=1 scripts/freeze_preflight_235.sh
SKIP_PERF=1 scripts/freeze_preflight_235.sh
VENV_PYTHON=/path/to/python scripts/freeze_preflight_235.sh
```

## What the script runs

1. environment snapshot (`environment_snapshot.txt`, `pip_freeze.txt`, git status/head)
2. input hash capture (`input_sha256.txt`)
3. contract check: `python scripts/check_docs_constants.py`
4. coverage report + budget gate (unless `SKIP_COVERAGE=1`)
5. extension build + perf snapshot + perf budget gate (unless `SKIP_PERF=1`)

Coverage/perf thresholds are controlled by env vars in script defaults:

- `MIN_PARSE_LINE_COVERAGE_STRICT` (default `0.52`)
- `MAX_UNSUPPORTED_LINES_STRICT` (default `14200`)
- `MIN_CARD_COVERAGE_APPROX` (default `0.99`)

## Artifact layout

Typical output directory contents:

- `environment_snapshot.txt`
- `pip_freeze.txt`
- `git_status_short.txt`
- `git_head.txt`
- `input_sha256.txt`
- `ability_coverage_report.json` (if coverage enabled)
- `ability_coverage_targets.json` (if coverage enabled)
- `perf_snapshot/benches.txt` (if perf enabled)
- `perf_snapshot/python_bench.txt` (if perf enabled)

Keep this folder alongside freeze commits/tags.

## Related

- [RL Contract](rl_contract.md)
- [Performance & Benchmarks](performance_benchmarks.md)
- [Rules Coverage](rules_coverage.md)
