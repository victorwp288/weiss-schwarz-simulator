from __future__ import annotations

import argparse
import importlib
import inspect
from importlib.machinery import EXTENSION_SUFFIXES
import sys
import types
from pathlib import Path
from time import perf_counter
from types import SimpleNamespace
from typing import Any

import numpy as np


weiss_sim: Any


def _clear_weiss_sim_modules() -> None:
    for name in list(sys.modules):
        if name == "weiss_sim" or name.startswith("weiss_sim."):
            sys.modules.pop(name, None)


def _repo_has_extension_module(repo_pkg: Path) -> bool:
    return any((repo_pkg / f"weiss_sim{suffix}").exists() for suffix in EXTENSION_SUFFIXES)


def _load_repo_weiss_sim(repo_python: Path, repo_pkg: Path) -> Any:
    sys.path.insert(0, str(repo_python))
    _clear_weiss_sim_modules()
    try:
        import weiss_sim as module

        return module
    except ImportError as exc:
        # PERF base refs can be in transitional states where __init__.py re-exports
        # symbols not present in `_buffers.py`. Fall back to loading submodules
        # directly so we can still collect throughput snapshots.
        if "make_batch_out_debug" not in str(exc):
            raise
        _clear_weiss_sim_modules()
        pkg = types.ModuleType("weiss_sim")
        pkg.__path__ = [str(repo_pkg)]  # type: ignore[attr-defined]
        pkg.__package__ = "weiss_sim"
        pkg.__file__ = str(repo_pkg / "__init__.py")
        sys.modules["weiss_sim"] = pkg
        core = importlib.import_module("weiss_sim.weiss_sim")
        buffers = importlib.import_module("weiss_sim._buffers")
        return SimpleNamespace(
            PASS_ACTION_ID=core.PASS_ACTION_ID,
            EnvPoolBuffers=buffers.EnvPoolBuffers,
            make_pool=buffers.make_pool,
        )
    finally:
        try:
            sys.path.remove(str(repo_python))
        except ValueError:
            pass


def _load_installed_weiss_sim() -> Any:
    _clear_weiss_sim_modules()
    import weiss_sim as module

    return module


def _load_weiss_sim(repo_root: Path, *, prefer_repo_extension: bool = False) -> Any:
    repo_python = repo_root / "python"
    repo_pkg = repo_python / "weiss_sim"
    if prefer_repo_extension and _repo_has_extension_module(repo_pkg):
        return _load_repo_weiss_sim(repo_python, repo_pkg)
    return _load_installed_weiss_sim()


def _module_origin(module: Any) -> str:
    return str(getattr(module, "__file__", inspect.getsourcefile(module) or "<unknown>"))


def pick_first_legal_from_mask(
    masks: np.ndarray, done: np.ndarray, actions_out: np.ndarray
) -> None:
    has_any = masks.any(axis=1)
    np.copyto(actions_out, masks.argmax(axis=1), casting="unsafe")
    actions_out[np.logical_or(done, ~has_any)] = int(weiss_sim.PASS_ACTION_ID)


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


def bench_step_mask(buffers: weiss_sim.EnvPoolBuffers, steps: int, reset_done: bool) -> float:
    actions = np.empty(buffers.pool.envs_len, dtype=np.uint32)
    out = buffers.out
    start = perf_counter()
    for _ in range(steps):
        done = np.logical_or(out.terminated, out.truncated)
        if reset_done and bool(done.any()):
            out = buffers.reset_done(done)
            done = np.logical_or(out.terminated, out.truncated)
        pick_first_legal_from_mask(out.masks, done, actions)
        out = buffers.step(actions)
    return perf_counter() - start


def bench_step_ids(buffers: weiss_sim.EnvPoolBuffers, steps: int, reset_done: bool) -> float:
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
                actions[i] = int(weiss_sim.PASS_ACTION_ID)
            else:
                actions[i] = int(buffers.legal_ids[start_idx])
        out = buffers.step(actions)
    return perf_counter() - start


def bench_step_fast_first_legal(
    buffers: weiss_sim.EnvPoolBuffers, steps: int, reset_done: bool
) -> float:
    actions = buffers.actions
    out = buffers.out
    start = perf_counter()
    for _ in range(steps):
        done = np.logical_or(out.terminated, out.truncated)
        if reset_done and bool(done.any()):
            out = buffers.reset_done(done)
        buffers.pool.step_first_legal_into(actions, buffers.out)
        out = buffers.out
    return perf_counter() - start


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--num-envs", type=int, default=256)
    parser.add_argument("--steps", type=int, default=5_000)
    parser.add_argument("--warmup", type=int, default=200)
    parser.add_argument("--reset-reps", type=int, default=200)
    parser.add_argument("--seed", type=int, default=0)
    parser.add_argument("--num-threads", type=int, default=None)
    parser.add_argument(
        "--repo-root",
        type=Path,
        default=Path(__file__).resolve().parents[2],
        help="Repository root containing python/weiss_sim.",
    )
    parser.add_argument(
        "--mode", choices=("mask", "ids", "fast_first_legal", "both"), default="both"
    )
    parser.add_argument(
        "--reset-done",
        action=argparse.BooleanOptionalAction,
        default=True,
        help="Reset terminated/truncated envs while timing step throughput.",
    )
    parser.add_argument(
        "--prefer-repo-extension",
        action="store_true",
        help="Load a native extension from python/weiss_sim if present. Off by default to avoid stale local binaries.",
    )
    args = parser.parse_args()

    global weiss_sim
    weiss_sim = _load_weiss_sim(
        args.repo_root.resolve(), prefer_repo_extension=args.prefer_repo_extension
    )
    print(f"# weiss_sim_module {_module_origin(weiss_sim)}")
    print(f"# reset_done {args.reset_done}")

    fixture_dir = args.repo_root.resolve() / "python" / "tests" / "fixtures"
    db_path = fixture_dir / "cards.wsdb"
    legal_deck = (list(range(1, 14)) * 4)[:50]

    pool, buffers = weiss_sim.make_pool(
        mode="train",
        num_envs=args.num_envs,
        db_path=str(db_path),
        deck_lists=[legal_deck, legal_deck],
        deck_ids=[1, 2],
        seed=args.seed,
        num_threads=args.num_threads,
        layout="mask",
    )

    out = buffers.reset()
    actions = np.zeros(args.num_envs, dtype=np.uint32)
    for _ in range(args.warmup):
        done = np.logical_or(out.terminated, out.truncated)
        if args.mode == "fast_first_legal":
            if args.reset_done and bool(done.any()):
                out = buffers.reset_done(done)
            buffers.pool.step_first_legal_into(buffers.actions, buffers.out)
            out = buffers.out
            continue
        if args.reset_done and bool(done.any()):
            out = buffers.reset_done(done)
            done = np.logical_or(out.terminated, out.truncated)
        pick_first_legal_from_mask(out.masks, done, actions)
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
            f"step(mask): {args.steps} steps in {mask_elapsed:.4f}s ({mask_eps:.0f} env-steps/sec)"
        )

    if args.mode in ("ids", "both"):
        ids_elapsed = bench_step_ids(buffers, args.steps, args.reset_done)
        ids_eps = (args.steps * args.num_envs) / max(ids_elapsed, 1e-9)
        print(f"step(ids): {args.steps} steps in {ids_elapsed:.4f}s ({ids_eps:.0f} env-steps/sec)")

    if args.mode == "fast_first_legal":
        fast_elapsed = bench_step_fast_first_legal(buffers, args.steps, args.reset_done)
        fast_eps = (args.steps * args.num_envs) / max(fast_elapsed, 1e-9)
        print(
            f"step(fast_first_legal): {args.steps} steps in {fast_elapsed:.4f}s "
            f"({fast_eps:.0f} env-steps/sec)"
        )


if __name__ == "__main__":
    main()
