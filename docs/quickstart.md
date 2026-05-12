# Quickstart

This page gets you from install to a working batched simulator loop.

## Install

From PyPI:

```bash
python -m pip install -U weiss-sim numpy
```

From a local checkout:

```bash
python -m pip install -U maturin numpy
python -m maturin develop --release --manifest-path weiss_py/Cargo.toml
```

Contributor setup:

```bash
rustup component add rustfmt clippy
python -m venv .venv
# PowerShell: .\.venv\Scripts\Activate.ps1
# Bash/zsh: source .venv/bin/activate
python -m pip install -U pip
python -m pip install -e ".[dev]"
python -m maturin develop --release --manifest-path weiss_py/Cargo.toml
```

## First Reset And Step

```python
import weiss_sim

with weiss_sim.fast(num_envs=32, seed=0) as sim:
    batch = sim.reset()
    actions = batch.legal.sample_uniform(seed=123)
    step = sim.step(actions)
    print(step.obs.shape, step.reward.shape, step.engine_status[:4])
```

Expected result:

- no exception
- `obs.shape[0] == num_envs`
- `engine_status == 0` for healthy envs

## Which API To Use

- Use `weiss_sim.fast(...)` or `weiss_sim.make(...)` for normal Python integration.
- Use `make_pool(...)` and `EnvPoolBuffers(...)` for high-throughput training loops.
- Use `inspect(...)` when you need dense masks, debug-friendly dtypes, or richer inspection.

High-level defaults:

| mode | runtime | legal repr | obs dtype |
| --- | --- | --- | --- |
| `fast` | speed | packed ids | `i16` |
| `inspect` | eval/debug | masks + ids | `i32` |

`runtime_mode=` is intentionally not accepted in the high-level API; choose `fast` or
`inspect`.

## Deck Inputs

`deck` and `opponent_deck` accept:

- `Sequence[int]`
- `Mapping[int | str, int]`
- preset strings such as `"preset:starter_deck_ws02_v1"`
- path-like strings

Bundled presets are the four release decklists: `starter_deck_ws02_v1`,
`control_deck_jj_s66_v1`, `main_deck_5hy_yotsuba_v1`, and
`aggro_deck_5hy_nino_v1`. Use `weiss_sim.cards.presets()` to list the presets in the
installed package. These presets require `rules_profile="approx"` because they include
partially parsed card abilities.

For experiments with toy decks or generated card ids, use `card_pool="all"`.
For packaged catalog compatibility checks, use the default `card_pool="parsed_only"`.

Minimal deck builder:

```python
import weiss_sim

b = weiss_sim.cards.builder(initial="starter_deck_ws02_v1")
report = b.validate(rules_profile="approx", card_pool="all")
if report.ok:
    deck_ids = b.build(rules_profile="approx", card_pool="all")
```

## Low-Level Training Loop

```python
import numpy as np
import weiss_sim

deck_a = (list(range(1, 14)) * 4)[:50]
deck_b = deck_a

pool, buffers = weiss_sim.make_pool(
    mode="train",
    num_envs=128,
    deck_lists=[deck_a, deck_b],
    deck_ids=[1, 2],
    seed=0,
    profile="fast",
    layout="i16_legal_ids_nometa",
)

out = buffers.reset()
actions = np.full(pool.envs_len, weiss_sim.PASS_ACTION_ID, dtype=np.uint32)
out = buffers.step(actions)
```

For policy-gradient loops that need sampled actions and behavior log-probabilities:

```python
logits = np.zeros((pool.envs_len, weiss_sim.ACTION_SPACE_SIZE), dtype=np.float32)
seeds = np.arange(pool.envs_len, dtype=np.uint64)
step, actions, action_logp = buffers.step_sample_from_logits_with_logp(logits, seeds)
```

Use `layout="i16_legal_ids"` instead of `i16_legal_ids_nometa` when the learner consumes
`legal_action_meta`.

## Determinism

Reproducible trajectories require fixed:

1. seed
2. deck lists and deck ids
3. action sequence
4. curriculum/reward/end-condition settings
5. compatibility constants from [RL Contract](rl_contract.md)

## Common Issues

- `ModuleNotFoundError: weiss_sim`: install the package in the active environment.
- maturin build errors: check active Python, Rust toolchain, and virtualenv.
- deck validation failures: decks must be 50 cards and legal under the selected profile.
- unexpected reset/terminal rows: inspect `engine_status`, `terminated`, `truncated`, and `StepBatch.needs_reset`.

## Local Checks

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
python -m ruff format --check python scraper scripts
python -m ruff check python scraper scripts
python scripts/check_docs_links.py
python scripts/check_docs_constants.py
python scripts/gen_docs_snippets.py --check
python -m pytest -q python/tests
```

Next: [Python API](python_api.md), [RL Contract](rl_contract.md), or
[Performance](performance_benchmarks.md).
