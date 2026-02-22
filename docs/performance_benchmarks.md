# Performance & Benchmarks

This page documents the benchmark workflow that is actually used by scripts/CI.

## Benchmark layers

- Rust micro/engine benches (`core_benches`, `alloc_benches`)
- Python boundary throughput (`python/examples/bench_python_boundary.py`)
- optional scaling study (`python/examples/bench_scaling.py`)

All Python benchmark scripts use the canonical low-level API surface:

- `make_pool(mode=..., layout=...)`
- `EnvPoolBuffers(pool, layout=...)`
- `reset_rl(...)` / `step_rl(...)` where appropriate

## Canonical snapshot command path

Used by `scripts/run_perf_snapshot.sh`:

```bash
cargo bench -p weiss_core --bench core_benches -- --output-format bencher
cargo bench -p weiss_core --bench alloc_benches -- --output-format bencher
PYTHONPATH=python python python/examples/bench_python_boundary.py \
  --num-envs 128 \
  --steps 2000 \
  --warmup 200 \
  --reset-reps 200 \
  --mode both
```

## Perf budget gate

Used in local parity/freeze/CI workflows:

```bash
python scripts/check_perf_budget.py \
  --baseline-benches benchmark/benches.txt \
  --current-benches /tmp/wss_perf_after/benches.txt \
  --baseline-python benchmark/python_bench.txt \
  --current-python /tmp/wss_perf_after/python_bench.txt \
  --max-core-regression-pct 15 \
  --max-python-regression-pct 10 \
  --require-zero-alloc
```

Gate intent:

- block large regression in shared core bencher rows
- block large regression in shared python env-steps/sec rows
- keep zero-alloc critical rows at zero when baseline is zero

## Local baseline regeneration

```bash
mkdir -p /tmp/wss_perf_after
cargo bench -p weiss_core --bench core_benches -- --output-format bencher > /tmp/wss_perf_after/benches.txt
cargo bench -p weiss_core --bench alloc_benches -- --output-format bencher >> /tmp/wss_perf_after/benches.txt
PYTHONPATH=python python python/examples/bench_python_boundary.py \
  --num-envs 128 --steps 2000 --warmup 200 --reset-reps 200 --mode both \
  > /tmp/wss_perf_after/python_bench.txt
python scripts/check_perf_budget.py \
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

## Optional exploratory runs

For deeper local profiling (not baseline gate inputs), you can run larger scenarios, for example:

```bash
python python/examples/bench_scaling.py --envs 128,512,1024 --threads 1,2,4,8,16 --steps 200 --warmup 50
```

Or a single low-level step-throughput run:

```bash
PYTHONPATH=python python python/examples/bench_envpool_step.py --num-envs 128 --steps 2000
```

## Measurement discipline

When comparing changes:

1. keep hardware/build profile consistent
2. run multiple times and compare medians
3. separate Rust-core and Python-boundary regressions
4. pair timing changes with allocation deltas
5. include commit SHA and command lines in PR notes

## CI benchmark publishing

`.github/workflows/benchmarks.yml`:

- runs benchmark jobs
- updates benchmark artifacts/charts
- updates root README benchmark snapshot block

## Related

- [Engine Architecture](engine_architecture.md)
- [RL Contract](rl_contract.md)
- [Contributing](contributing.md)
