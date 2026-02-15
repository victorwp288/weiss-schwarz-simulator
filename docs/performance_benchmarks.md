# Performance & Benchmarks

**TL;DR**
- Measure both Rust-core and Python-boundary throughput.
- Compare medians across repeated runs; ignore single noisy samples.
- Prefer legal-id and preallocated-buffer pathways in hot loops.

[Overview](README.md) | [Quickstart](quickstart.md) | [Engine](engine_architecture.md) | [RL Contract](rl_contract.md) | [Encodings](encodings.md) | Performance | [Replays](replays_determinism.md) | [Rules](rules_coverage.md) | [Invariants](invariants_validation.md) | [Contributing](contributing.md)

---

## Benchmark suite

Run all primary benchmarks:

```bash
cargo bench -p weiss_core --bench core_benches
cargo bench -p weiss_core --bench alloc_benches
python python/examples/bench_python_boundary.py --num-envs 256 --steps 5000 --mode both
python python/examples/bench_scaling.py --envs 128,512,1024 --threads 1,2,4,8,16 --steps 200 --warmup 50
```

What each target captures:

- `core_benches`: core stepping/legality/encoding path timing
- `alloc_benches`: allocation pressure in critical paths
- `bench_python_boundary.py`: end-to-end Py<->Rust stepping cost
- `bench_scaling.py`: scaling behavior vs env count and thread count

---

## Recommended measurement protocol

For reliable comparisons:

1. run on an idle machine
2. use same build profile and same hardware
3. run each benchmark multiple times
4. compare medians and spread, not one-off best values
5. record commit SHA with results

If you change memory-heavy code, pair timing numbers with allocation bench deltas.

---

## Hot-path guidance

Patterns that typically improve throughput:

- reuse preallocated output buffers (`*_into` APIs)
- use legal ids instead of full action-space mask scans in Python
- reduce Python-level loops where possible (batch operations)
- avoid per-step object allocation in custom wrappers

Patterns that often regress throughput:

- per-step shape conversions or dtype casts
- scanning full masks when legal-id slices are available
- rebuilding pools or buffers repeatedly inside training loops

---

## Interpreting regressions

When a benchmark regresses:

1. localize whether regression is Rust-core or Python-boundary
2. inspect allocation changes first (`alloc_benches`)
3. check for new branching/ordering work in advance loop
4. verify no debug-only instrumentation leaked into hot path
5. compare with and without priority windows where relevant

---

## Fault-Containment Perf Gate

Per-env panic containment adds mandatory control-flow checks in step/reset worker paths.
When touching fault handling, compare against the checked-in baseline files:

- `benchmark/benches.txt`
- `benchmark/python_bench.txt`

Recommended acceptance criteria for routine refactors:

- no meaningful regression in batch step throughput (`step_batch_*`, Python `step(mask|ids)`)
- allocation benches stay at zero allocs/iter for critical paths
- reset-path deltas are explained and justified if they move materially

Use at least two local runs before concluding a regression; tiny ns-level deltas can be noise.

## Hard Perf Budget Gate

Use `scripts/check_perf_budget.py` to enforce deterministic regression thresholds against a baseline snapshot:

```bash
python scripts/check_perf_budget.py \
  --baseline-benches /tmp/wss_perf_before/benches.txt \
  --current-benches /tmp/wss_perf_after/benches.txt \
  --baseline-python /tmp/wss_perf_before/python_bench.txt \
  --current-python /tmp/wss_perf_after/python_bench.txt \
  --max-core-regression-pct 15 \
  --max-python-regression-pct 10 \
  --require-zero-alloc
```

Gate policy:

- core bencher rows: each shared benchmark must stay within `<=15%` regression (`ns/iter` higher is worse), evaluated with bencher uncertainty bounds to reduce one-run noise false positives
- python boundary throughput rows: each shared `env-steps/sec` metric must stay within `<=10%` regression (lower is worse)
- allocation-critical rows: if baseline is zero allocs/iter, current must remain zero when `--require-zero-alloc` is set
- parser compatibility: accepts bencher lines with or without explicit `(+/- ...)`, and Python output in both sectioned and non-sectioned formats
- Python metric normalization: sectioned labels like `default` and `explicit_serial` are normalized so baseline/current key matching remains stable across output styles

Implementation note (February 12, 2026):

- Step-path execution now uses an adaptive cutoff in the pool worker path:
  small/medium batches run serially while reset keeps per-env parallel containment.

## Latest local snapshot (February 13, 2026)

Commands run:

```bash
.venv/bin/python -m maturin develop --release
cargo bench -p weiss_core --bench core_benches -- --output-format bencher
cargo bench -p weiss_core --bench alloc_benches -- --output-format bencher
PYTHONPATH=python .venv/bin/python python/examples/bench_python_boundary.py --num-envs 256 --steps 5000 --mode both
PYTHONPATH=python .venv/bin/python python/examples/bench_python_boundary.py --num-envs 256 --steps 5000 --mode both --num-threads 1
PYTHONPATH=python .venv/bin/python python/examples/bench_scaling.py --envs 128,512,1024 --threads 1,2,4,8,16 --steps 200 --warmup 50
```

Compared to the previous local snapshot from February 12, 2026:

| Metric | Baseline | Current | Delta |
| --- | ---: | ---: | ---: |
| `step_batch_64` | 14.061 us | 13.723 us | -2.40% |
| `step_batch_fast_256_priority_off` | 55.267 us | 55.244 us | -0.04% |
| `step_batch_fast_256_priority_on` | 54.425 us | 55.390 us | +1.77% |
| `alloc_action_masks_batch_into` | 445 ns | 391 ns | -12.13% |
| `on_reverse_decision_frequency_on` | 521 ns | 584 ns | +12.09% |
| Python `reset_into` (RL default auto-thread; median of 3) | 202.8 us/reset | 199.2 us/reset | -1.78% |
| Python `step(mask)` (RL default auto-thread; median of 3) | 4,242,406 env-steps/s | 4,141,632 env-steps/s | -2.38% |
| Python `step(ids)` (RL default auto-thread; median of 3) | 4,757,821 env-steps/s | 4,613,871 env-steps/s | -3.03% |
| Python `reset_into` (`--num-threads 1`; median of 3) | 614.2 us/reset | 598.2 us/reset | -2.61% |

Interpretation:

- Core step throughput remains within low single-digit movement and inside the
  `<=10%` guardrail.
- Allocation-critical benches remain zero-allocation in hot paths, and
  `alloc_action_masks_batch_into` improved materially in this run.
- Python boundary throughput is stable with small low single-digit movement.
- Python RL default reset remains within the historical range shown in this
  section's baseline/current comparison table.

Scaling snapshot (`bench_scaling.py`):

- 128 envs: best at 2,460,257 env-steps/s (`num_threads=1`)
- 512 envs: best at 2,411,511 env-steps/s (`num_threads=4`)
- 1024 envs: best at 2,957,064 env-steps/s (`num_threads=2`)

Refreshed local baselines from this run are checked in at:

- `benchmark/benches.txt`
- `benchmark/python_bench.txt`

### Baseline regeneration recipe

The snapshot commands above are exploratory high-load runs; the regeneration recipe below intentionally uses smaller fixed parameters for reproducible budget gating.

```bash
mkdir -p /tmp/wss_perf_after
cargo bench -p weiss_core --bench core_benches -- --output-format bencher > /tmp/wss_perf_after/benches.txt
cargo bench -p weiss_core --bench alloc_benches -- --output-format bencher >> /tmp/wss_perf_after/benches.txt
PYTHONPATH=python .venv/bin/python python/examples/bench_python_boundary.py --num-envs 128 --steps 2000 --warmup 200 --reset-reps 200 --mode both > /tmp/wss_perf_after/python_bench.txt
.venv/bin/python scripts/check_perf_budget.py \
  --baseline-benches benchmark/benches.txt \
  --current-benches /tmp/wss_perf_after/benches.txt \
  --baseline-python benchmark/python_bench.txt \
  --current-python /tmp/wss_perf_after/python_bench.txt \
  --max-core-regression-pct 15 \
  --max-python-regression-pct 10 \
  --require-zero-alloc
cp /tmp/wss_perf_after/benches.txt benchmark/benches.txt
cp /tmp/wss_perf_after/python_bench.txt benchmark/python_bench.txt
```

---

## CI and published benchmark history

Benchmark automation runs from `.github/workflows/benchmarks.yml` and publishes chart history to GitHub Pages.

Use published charts for trend direction; use local runs for precise diagnosis.

---

## Suggested PR evidence (perf-sensitive changes)

Include:

- benchmark commands executed
- before/after numbers (median where possible)
- whether regression/improvement is core, boundary, or both
- short explanation of likely cause

---

## Related

- [RL contract](rl_contract.md)
- [Engine architecture](engine_architecture.md)
- [Troubleshooting](troubleshooting.md)
