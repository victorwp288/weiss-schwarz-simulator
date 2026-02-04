# Performance Audit (Weiss Schwarz Simulator)

Date: 2025-12-26

## Benchmark commands
```
cargo bench -p weiss_core --bench core_benches
cargo bench -p weiss_core --bench alloc_benches
python python/examples/bench_python_boundary.py --num-envs 256 --steps 5000 --mode both
python python/examples/bench_scaling.py --envs 128,512,1024 --threads 1,2,4,8,16 --steps 200 --warmup 50
```

## Environment
- Host CPU: Apple M4
- CPU cores: 10 physical / 10 logical
- OS: macOS 26.1 (25B78)
- Build profile: `bench` (optimized)
- Plot backend: plotters (gnuplot not installed)

## Results (Criterion)
All times are per-iteration (mean/median range shown by Criterion).

### Core benches (with black_box + forced variants)
- advance_until_decision: 30.108 µs .. 30.276 µs
- step_batch_64: 19.929 µs .. 19.959 µs
- step_batch_fast_256_priority_off: 77.943 µs .. 77.997 µs
- step_batch_fast_256_priority_on: 78.053 µs .. 78.393 µs
- legal_actions: 38.651 ns .. 38.978 ns
- legal_actions_forced: 37.132 ns .. 37.279 ns
- on_reverse_decision_frequency_on: 659.60 ns .. 667.88 ns
- on_reverse_decision_frequency_off: 666.80 ns .. 675.36 ns
- observation_encode: 121.98 ns .. 122.09 ns
- observation_encode_forced: 119.87 ns .. 120.92 ns
- mask_construction: 293.37 ns .. 294.05 ns
- mask_construction_forced: 290.12 ns .. 291.66 ns

### Allocation-count benches (counting allocator)
Reported once per bench (avg allocs per iteration):
- alloc_legal_actions: 1 alloc/iter
- alloc_observation_encode: 0 alloc/iter
- alloc_action_masks_batch_into: 0 alloc/iter (after switching to into buffers)

Notes:
- `ACTION_SPACE_SIZE` is now 527 (`MAX_HAND=50`), and `OBS_LEN` grew with it.
- `legal_actions` allocates a new Vec per call, but cached legality (used in env hot paths) does not.
- `encode_observation` is allocation-free when reusing the output buffer.
- `action_masks_batch` allocates a new Vec per call; prefer `action_masks_batch_into` in hot paths.

## Gaps in current benchmarks
- No IO-focused benchmarks: replay writing/reading, wsdb load/pack, or replay dump.
- No benchmarks for visibility policies / replay sanitization hot paths.
- Worst-case choice paging bench recorded (see below).

## Scaling benchmark (recorded)
Script: `python/examples/bench_scaling.py`

What it measures:
- Throughput across thread counts (1/2/4/8/16 by default).
- Throughput across env counts (128/512/1024 by default).
- Uses ids-based action selection and reset-on-done to reflect training loops.

Run (example):
```
python python/examples/bench_scaling.py --envs 128,512,1024 --threads 1,2,4,8,16 --steps 2000
```

Recorded results (2025-12-26, steps=200, warmup=50; shorter run than the example command):
```
num_envs,num_threads,steps,elapsed_s,env_steps_per_sec
128,1,200,3.6289,7055
128,2,200,1.8908,13539
128,4,200,1.1026,23218
128,8,200,0.8265,30975
128,16,200,0.8103,31592
512,1,200,14.6517,6989
512,2,200,7.4846,13681
512,4,200,4.3301,23648
512,8,200,3.2684,31330
512,16,200,3.0708,33347
1024,1,200,28.4432,7200
1024,2,200,14.9668,13684
1024,4,200,8.7535,23396
1024,8,200,6.5425,31303
1024,16,200,6.0381,33918
```

Takeaways:
- Scaling is strong through 8 threads; gains flatten from 8 -> 16 (expected on 10-core CPU).
- These are end-to-end Python training loop steps with resets and ids selection; absolute throughput is lower than the tight boundary bench by design.

## Python boundary benchmark (recorded)
Script: `python/examples/bench_python_boundary.py`

What it measures:
- Reset throughput into preallocated buffers.
- Step throughput when selecting actions from masks (`np.argmax` scan).
- Step throughput when selecting actions from Rust-provided legal action ids.

Run (example):
```
python python/examples/bench_python_boundary.py --num-envs 256 --steps 5000 --mode both
```

Notes:
- `legal_action_ids_into` requires preallocated `ids` and `offsets` buffers.
- Expected to show whether Python-side mask scanning dominates at current core speeds.
- Use `--reset-done` to include reset cost in the loop; default is no reset to focus on action selection overhead.

Recorded results (2025-12-26):
```
reset_into: 200 reps in 5.4761s (27380.4 us/reset)
step(mask): 5000 steps in 19.2248s (66581 env-steps/sec)
step(ids): 5000 steps in 10.8557s (117910 env-steps/sec)
```

Takeaways:
- ids-based action selection is ~1.77x faster than mask scanning at 256 envs on this machine.
- Python-side mask scanning is now a dominant overhead relative to core step time.
- Effective per-env core time from Rust bench: 78 µs / 256 ≈ 0.305 µs per env per step.

## Worst-case choice paging benchmark (recorded)
Bench: `choice_paging_worst_case_mask` (200 choice options)

Time (Criterion):
- choice_paging_worst_case_mask: 94.642 ns .. 96.366 ns

Allocations (counting allocator):
- alloc_choice_paging_worst_case: 0 allocs/iter (after scratch reuse)

## Recent rule-accuracy changes that affect perf
- `MAX_HAND` increased to 50; action space and observation lengths increased accordingly.
- Rule-action power<=0 check now runs in rule-action loop (adds a power lookup pass per rule-action sweep).
- Deck legality checks now include deck size = 50.
