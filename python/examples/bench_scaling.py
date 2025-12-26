import argparse
from pathlib import Path
from time import perf_counter

import numpy as np

import weiss_sim


def select_first_ids(
    ids: np.ndarray, offsets: np.ndarray, actions_out: np.ndarray
) -> None:
    for i in range(actions_out.shape[0]):
        start = int(offsets[i])
        end = int(offsets[i + 1])
        if start == end:
            raise RuntimeError(f"no legal actions for env {i}")
        actions_out[i] = int(ids[start])


def run_case(num_envs: int, num_threads: int | None, steps: int, warmup: int, seed: int) -> float:
    fixture_dir = Path(__file__).resolve().parents[1] / "tests" / "fixtures"
    db_path = fixture_dir / "cards.wsdb"
    legal_deck = (list(range(1, 14)) * 4)[:50]

    pool = weiss_sim.EnvPool.new_rl_train(
        num_envs,
        str(db_path),
        deck_lists=[legal_deck, legal_deck],
        deck_ids=[1, 2],
        seed=seed,
        num_threads=num_threads,
    )
    buffers = weiss_sim.EnvPoolBuffers(pool)
    out = buffers.reset()
    actions = np.empty(num_envs, dtype=np.uint32)

    for _ in range(warmup):
        done = np.logical_or(out.terminated, out.truncated)
        if bool(done.any()):
            out = buffers.reset_done(done)
        buffers.pool.legal_action_ids_into(buffers.legal_ids, buffers.legal_offsets)
        select_first_ids(buffers.legal_ids, buffers.legal_offsets, actions)
        out = buffers.step(actions)

    start = perf_counter()
    for _ in range(steps):
        done = np.logical_or(out.terminated, out.truncated)
        if bool(done.any()):
            out = buffers.reset_done(done)
        buffers.pool.legal_action_ids_into(buffers.legal_ids, buffers.legal_offsets)
        select_first_ids(buffers.legal_ids, buffers.legal_offsets, actions)
        out = buffers.step(actions)
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
    args = parser.parse_args()

    envs_list = parse_int_list(args.envs)
    threads_list = parse_int_list(args.threads)

    print("num_envs,num_threads,steps,elapsed_s,env_steps_per_sec")
    for num_envs in envs_list:
        for threads in threads_list:
            elapsed = run_case(num_envs, threads, args.steps, args.warmup, args.seed)
            env_steps = args.steps * num_envs
            eps = env_steps / max(elapsed, 1e-9)
            print(f"{num_envs},{threads},{args.steps},{elapsed:.4f},{eps:.0f}")


if __name__ == "__main__":
    main()
