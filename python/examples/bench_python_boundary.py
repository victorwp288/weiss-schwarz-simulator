import argparse
from pathlib import Path
from time import perf_counter

import numpy as np

import weiss_sim


def pick_first_legal_from_mask(masks: np.ndarray, actions_out: np.ndarray) -> None:
    for i in range(masks.shape[0]):
        row = masks[i]
        idx = int(np.argmax(row))
        actions_out[i] = idx


def pick_first_legal_from_ids(
    ids: np.ndarray, offsets: np.ndarray, actions_out: np.ndarray
) -> None:
    for i in range(actions_out.shape[0]):
        start = int(offsets[i])
        end = int(offsets[i + 1])
        if start == end:
            actions_out[i] = int(weiss_sim.PASS_ACTION_ID)
        else:
            actions_out[i] = int(ids[start])


def bench_reset(buffers: weiss_sim.EnvPoolBuffers, reps: int) -> float:
    start = perf_counter()
    for _ in range(reps):
        buffers.reset()
    return perf_counter() - start


def bench_step_mask(
    buffers: weiss_sim.EnvPoolBuffers, steps: int, reset_done: bool
) -> float:
    actions = np.empty(buffers.pool.envs_len, dtype=np.uint32)
    out = buffers.out
    start = perf_counter()
    for _ in range(steps):
        done = np.logical_or(out.terminated, out.truncated)
        if reset_done and bool(done.any()):
            out = buffers.reset_done(done)
        pick_first_legal_from_mask(out.masks, actions)
        out = buffers.step(actions)
    return perf_counter() - start


def bench_step_ids(
    buffers: weiss_sim.EnvPoolBuffers, steps: int, reset_done: bool
) -> float:
    actions = np.empty(buffers.pool.envs_len, dtype=np.uint32)
    out = buffers.out
    start = perf_counter()
    for _ in range(steps):
        done = np.logical_or(out.terminated, out.truncated)
        if reset_done and bool(done.any()):
            out = buffers.reset_done(done)
        buffers.pool.legal_action_ids_into(buffers.legal_ids, buffers.legal_offsets)
        for i in range(actions.shape[0]):
            start_idx = int(buffers.legal_offsets[i])
            end_idx = int(buffers.legal_offsets[i + 1])
            if start_idx == end_idx:
                if bool(done[i]):
                    actions[i] = int(weiss_sim.PASS_ACTION_ID)
                else:
                    raise RuntimeError(
                        f"no legal actions for live env {i}: "
                        f"decision_id={int(out.decision_id[i])} "
                        f"engine_status={int(out.engine_status[i])} "
                        f"actor={int(out.actor[i])}"
                    )
            else:
                actions[i] = int(buffers.legal_ids[start_idx])
        out = buffers.step(actions)
    return perf_counter() - start


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--num-envs", type=int, default=256)
    parser.add_argument("--steps", type=int, default=5_000)
    parser.add_argument("--warmup", type=int, default=200)
    parser.add_argument("--reset-reps", type=int, default=200)
    parser.add_argument("--seed", type=int, default=0)
    parser.add_argument("--mode", choices=("mask", "ids", "both"), default="both")
    parser.add_argument("--reset-done", action="store_true")
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
    actions = np.zeros(args.num_envs, dtype=np.uint32)
    for _ in range(args.warmup):
        done = np.logical_or(out.terminated, out.truncated)
        if args.reset_done and bool(done.any()):
            out = buffers.reset_done(done)
        pick_first_legal_from_mask(out.masks, actions)
        out = buffers.step(actions)

    reset_elapsed = bench_reset(buffers, args.reset_reps)
    print(
        f"reset_into: {args.reset_reps} reps in {reset_elapsed:.4f}s "
        f"({(reset_elapsed / args.reset_reps) * 1e6:.1f} us/reset)"
    )

    if args.mode in ("mask", "both"):
        mask_elapsed = bench_step_mask(buffers, args.steps, args.reset_done)
        mask_eps = (args.steps * args.num_envs) / max(mask_elapsed, 1e-9)
        print(
            f"step(mask): {args.steps} steps in {mask_elapsed:.4f}s "
            f"({mask_eps:.0f} env-steps/sec)"
        )

    if args.mode in ("ids", "both"):
        ids_elapsed = bench_step_ids(buffers, args.steps, args.reset_done)
        ids_eps = (args.steps * args.num_envs) / max(ids_elapsed, 1e-9)
        print(
            f"step(ids): {args.steps} steps in {ids_elapsed:.4f}s "
            f"({ids_eps:.0f} env-steps/sec)"
        )


if __name__ == "__main__":
    main()
