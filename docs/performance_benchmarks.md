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
