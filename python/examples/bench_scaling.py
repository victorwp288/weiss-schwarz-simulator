import argparse
from pathlib import Path
from time import perf_counter

import numpy as np

import weiss_sim

LCG_MULT = np.uint64(6364136223846793005)
LCG_INC = np.uint64(1)


def select_first_ids(
    ids: np.ndarray, offsets: np.ndarray, actions_out: np.ndarray
) -> None:
    for i in range(actions_out.shape[0]):
        start = int(offsets[i])
        end = int(offsets[i + 1])
        if start == end:
            raise RuntimeError(f"no legal actions for env {i}")
        actions_out[i] = int(ids[start])


def run_case(
    num_envs: int,
    num_threads: int | None,
    steps: int,
    warmup: int,
    seed: int,
    mode: str,
    output_masks: bool,
) -> float:
    fixture_dir = Path(__file__).resolve().parents[1] / "tests" / "fixtures"
    db_path = fixture_dir / "cards.wsdb"
    legal_deck = (list(range(1, 14)) * 4)[:50]

    layout = "mask" if output_masks else "nomask"
    pool, buffers = weiss_sim.make_pool(
        mode="train",
        num_envs=num_envs,
        db_path=str(db_path),
        deck_lists=[legal_deck, legal_deck],
        deck_ids=[1, 2],
        seed=seed,
        num_threads=num_threads,
        layout=layout,
    )
    out = buffers.reset()
    actions = buffers.actions
    seeds = None
    if mode == "fast_random_legal":
        seeds = np.arange(num_envs, dtype=np.uint64) + np.uint64(seed)

    def step_once(out_step):
        done = np.logical_or(out_step.terminated, out_step.truncated)
        if bool(done.any()):
            out_step = buffers.reset_done(done)
        if mode == "baseline_ids":
            buffers.pool.legal_action_ids_into(buffers.legal_ids, buffers.legal_offsets)
            select_first_ids(buffers.legal_ids, buffers.legal_offsets, actions)
            return buffers.step(actions)
        if mode == "fast_first_legal":
            buffers.pool.step_first_legal_into(actions, buffers.out)
            return buffers.out
        if mode == "fast_random_legal":
            np.multiply(seeds, LCG_MULT, out=seeds)
            seeds += LCG_INC
            buffers.pool.step_sample_legal_action_ids_uniform_into(
                seeds, actions, buffers.out
            )
            return buffers.out
        raise RuntimeError(f"unknown mode: {mode}")

    for _ in range(warmup):
        out = step_once(out)

    start = perf_counter()
    for _ in range(steps):
        out = step_once(out)
    elapsed = perf_counter() - start
    return elapsed


def parse_int_list(raw: str) -> list[int]:
    return [int(x) for x in raw.split(",") if x.strip()]


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--envs", type=str, default="128,512,1024")
    parser.add_argument("--threads", type=str, default="1,2,4,8,16")
    parser.add_argument("--steps", type=int, default=2000)
    parser.add_argument("--warmup", type=int, default=200)
    parser.add_argument("--seed", type=int, default=0)
    parser.add_argument(
        "--mode",
        choices=("baseline_ids", "fast_first_legal", "fast_random_legal"),
        default="baseline_ids",
    )
    parser.add_argument("--output-masks", choices=("true", "false"), default=None)
    args = parser.parse_args()

    envs_list = parse_int_list(args.envs)
    threads_list = parse_int_list(args.threads)
    if args.output_masks is None:
        output_masks = args.mode == "baseline_ids"
    else:
        output_masks = args.output_masks == "true"

    print("num_envs,num_threads,steps,elapsed_s,env_steps_per_sec")
    for num_envs in envs_list:
        for threads in threads_list:
            elapsed = run_case(
                num_envs,
                threads,
                args.steps,
                args.warmup,
                args.seed,
                args.mode,
                output_masks,
            )
            env_steps = args.steps * num_envs
            eps = env_steps / max(elapsed, 1e-9)
            print(f"{num_envs},{threads},{args.steps},{elapsed:.4f},{eps:.0f}")


if __name__ == "__main__":
    main()
