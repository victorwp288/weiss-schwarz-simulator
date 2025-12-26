import argparse
from pathlib import Path
from time import perf_counter

import numpy as np

import weiss_sim


def first_legal(mask_row: np.ndarray) -> int:
    idxs = np.flatnonzero(mask_row)
    if idxs.size == 0:
        raise RuntimeError("no legal actions")
    return int(idxs[0])


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--num-envs", type=int, default=64)
    parser.add_argument("--steps", type=int, default=5_000)
    parser.add_argument("--seed", type=int, default=0)
    args = parser.parse_args()

    fixture_dir = Path(__file__).resolve().parents[1] / "tests" / "fixtures"
    db_path = fixture_dir / "cards.wsdb"
    legal_deck = (list(range(1, 14)) * 4)[:50]

    pool = weiss_sim.EnvPool.new_rl_train(
        args.num_envs,
        str(db_path),
        deck_lists=[legal_deck, legal_deck],
        deck_ids=[1, 2],
        seed=args.seed,
    )
    buffers = weiss_sim.EnvPoolBuffers(pool)
    out = buffers.reset()

    # Warmup
    for _ in range(100):
        actions = [first_legal(out.masks[i]) for i in range(args.num_envs)]
        out = buffers.step(np.array(actions, dtype=np.uint32))

    start = perf_counter()
    for _ in range(args.steps):
        actions = [first_legal(out.masks[i]) for i in range(args.num_envs)]
        out = buffers.step(np.array(actions, dtype=np.uint32))
    elapsed = perf_counter() - start

    steps_per_sec = (args.steps * args.num_envs) / max(elapsed, 1e-9)
    print(f"{args.num_envs=} {args.steps=} {elapsed:.3f}s {steps_per_sec:.0f} env-steps/sec")


if __name__ == "__main__":
    main()
