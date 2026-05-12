# Performance

Performance work is split into Rust-core benchmarks, Python-boundary throughput, and
RL-shaped logits benchmarks. Keep those layers separate when diagnosing regressions.

## Canonical Snapshot

`scripts/run_perf_snapshot.sh` runs the checked-in budget path:

```bash
cargo bench -p weiss_core --bench core_benches -- --output-format bencher
cargo bench -p weiss_core --bench alloc_benches -- --output-format bencher
PYTHONPATH=python python python/examples/bench_python_boundary.py \
  --num-envs 128 \
  --steps 2000 \
  --warmup 200 \
  --reset-reps 200 \
  --reset-done \
  --mode both
```

Budget gate:

```bash
python scripts/check_perf_budget.py \
  --baseline-benches benchmark/benches.txt \
  --current-benches /tmp/wss_perf_after/benches.txt \
  --baseline-python benchmark/python_bench.txt \
  --current-python /tmp/wss_perf_after/python_bench.txt \
  --max-core-regression-pct 15 \
  --core-budget-override reset_batch_256=25 \
  --max-python-regression-pct 10 \
  --require-zero-alloc
```

Gate intent:

- block large Rust micro-benchmark regressions
- allow extra variance for `reset_batch_256`
- block large Python boundary regressions
- keep critical allocation benches at zero allocations

## Current Baselines

Checked-in Rust snapshot:

| Benchmark | Baseline |
| --- | ---: |
| `advance_until_decision` | 21272 ns/iter |
| `step_batch_64` | 9581 ns/iter |
| `reset_batch_256` | 512004 ns/iter |
| `step_batch_fast_256_priority_off` | 39563 ns/iter |
| `step_batch_fast_256_priority_on` | 42891 ns/iter |
| `legal_actions` | 4 ns/iter |
| `observation_encode` | 80 ns/iter |
| `mask_construction` | 180 ns/iter |

Checked-in Python snapshot:

| Benchmark | Baseline |
| --- | ---: |
| `reset_into` | 110.0 us/reset |
| `step(mask)` | 1296605 env-steps/sec |
| `step(ids)` | 1414805 env-steps/sec |

The Python snapshot is an installed `1.1.0` release wheel measured with
`reset_done=True`, so step throughput reflects live gameplay instead of repeated
stepping of already-terminated environments. The benchmark script prints the loaded
`weiss_sim` module path at the top of the output to catch stale local extension
artifacts.

## RL Hot Path

For policy-gradient loops that sample actions from logits and need behavior
log-probabilities, benchmark with:

```bash
python python/examples/bench_rl_logits_boundary.py \
  --envs 128,256,512 \
  --steps 500 \
  --warmup 50 \
  --repeats 3 \
  --reset-done true \
  --modes fused_sample_logp \
  --layouts i16_legal_ids,i16_legal_ids_nometa \
  --repo-root .
```

Representative local `1.1.0` fused sampled-logp results:

| envs | `i16_legal_ids` | `i16_legal_ids_nometa` |
| ---: | ---: | ---: |
| 128 | 481944 env-steps/sec | 518554 env-steps/sec |
| 256 | 1227289 env-steps/sec | 1269536 env-steps/sec |
| 512 | 1607473 env-steps/sec | 1756885 env-steps/sec |

Use `i16_legal_ids_nometa` when the learner does not consume `legal_action_meta`.
Use `i16_legal_ids` when metadata is part of the observation/action feature pipeline.

`legal_action_context_v1(...)` is deliberately opt-in. It can improve learning features
by providing dynamic source-card context for each legal action, but it adds work if
materialized every decision.

## Local Baseline Regeneration

```bash
mkdir -p /tmp/wss_perf_after
bash scripts/run_perf_snapshot.sh /tmp/wss_perf_after
python scripts/check_perf_budget.py \
  --baseline-benches benchmark/benches.txt \
  --current-benches /tmp/wss_perf_after/benches.txt \
  --baseline-python benchmark/python_bench.txt \
  --current-python /tmp/wss_perf_after/python_bench.txt \
  --max-core-regression-pct 15 \
  --core-budget-override reset_batch_256=25 \
  --max-python-regression-pct 10 \
  --require-zero-alloc
cp /tmp/wss_perf_after/benches.txt benchmark/benches.txt
cp /tmp/wss_perf_after/python_bench.txt benchmark/python_bench.txt
```

## Measurement Discipline

1. keep hardware/build profile consistent
2. compare medians, not one-off timings
3. isolate Rust-core regressions from Python-boundary regressions
4. include allocation deltas for hot paths
5. record command lines and commit SHA in PR notes

Related: [RL Contract](rl_contract.md), [Architecture](architecture.md).
