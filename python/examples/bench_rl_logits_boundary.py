from __future__ import annotations

import argparse
import importlib
from importlib.machinery import EXTENSION_SUFFIXES
import statistics
import sys
import types
from pathlib import Path
from time import perf_counter
from types import SimpleNamespace
from typing import Any

import numpy as np


LCG_MULT = np.uint64(6364136223846793005)
LCG_INC = np.uint64(1)

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
            EnvPoolBuffers=buffers.EnvPoolBuffers,
            make_pool=buffers.make_pool,
            PASS_ACTION_ID=core.PASS_ACTION_ID,
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


def _load_weiss_sim(repo_root: Path) -> Any:
    repo_python = repo_root / "python"
    repo_pkg = repo_python / "weiss_sim"
    if _repo_has_extension_module(repo_pkg):
        return _load_repo_weiss_sim(repo_python, repo_pkg)
    return _load_installed_weiss_sim()


def parse_int_list(raw: str) -> list[int]:
    return [int(part) for part in raw.split(",") if part.strip()]


def parse_mode_list(raw: str) -> list[str]:
    modes = [part.strip() for part in raw.split(",") if part.strip()]
    allowed = {"python_sample_logp", "fused_sample_logp"}
    unknown = sorted(set(modes) - allowed)
    if unknown:
        raise ValueError(f"unknown mode(s): {', '.join(unknown)}")
    return modes


def parse_layout_list(raw: str) -> list[str]:
    layouts = [part.strip() for part in raw.split(",") if part.strip()]
    allowed = {"i16_legal_ids", "i16_legal_ids_nometa"}
    unknown = sorted(set(layouts) - allowed)
    if unknown:
        raise ValueError(f"unknown layout(s): {', '.join(unknown)}")
    return layouts


def parse_reset_done_list(raw: str) -> list[bool]:
    if raw == "both":
        return [False, True]
    if raw == "true":
        return [True]
    if raw == "false":
        return [False]
    raise ValueError("--reset-done must be false, true, or both")


def advance_seeds(seeds: np.ndarray) -> None:
    np.multiply(seeds, LCG_MULT, out=seeds)
    seeds += LCG_INC


def sample_python_legal_logp(
    logits: np.ndarray,
    legal_ids: np.ndarray,
    legal_offsets: np.ndarray,
    seeds: np.ndarray,
    actions: np.ndarray,
    action_logp: np.ndarray,
) -> None:
    for env_index in range(actions.shape[0]):
        start = int(legal_offsets[env_index])
        end = int(legal_offsets[env_index + 1])
        ids = legal_ids[start:end]
        if ids.size == 0:
            actions[env_index] = int(weiss_sim.PASS_ACTION_ID)
            action_logp[env_index] = 0.0
            continue
        row = np.asarray(logits[env_index, ids], dtype=np.float64)
        max_logit = float(np.max(row))
        weights = np.exp(row - max_logit)
        total = float(np.sum(weights))
        if total <= 0.0 or not np.isfinite(total):
            actions[env_index] = int(ids[0])
            action_logp[env_index] = 0.0
            continue
        threshold = (float(seeds[env_index]) / float(np.iinfo(np.uint64).max)) * total
        chosen_index = int(ids.size - 1)
        for idx, weight in enumerate(weights):
            threshold -= float(weight)
            if threshold <= 0.0:
                chosen_index = idx
                break
        actions[env_index] = int(ids[chosen_index])
        action_logp[env_index] = float(row[chosen_index] - max_logit - np.log(total))


def build_case(
    repo_root: Path,
    num_envs: int,
    num_threads: int | None,
    seed: int,
    layout: str,
) -> tuple[Any, Any]:
    fixture_dir = repo_root / "python" / "tests" / "fixtures"
    db_path = fixture_dir / "cards.wsdb"
    legal_deck = (list(range(1, 14)) * 4)[:50]

    return weiss_sim.make_pool(
        mode="train",
        num_envs=num_envs,
        db_path=str(db_path),
        deck_lists=[legal_deck, legal_deck],
        deck_ids=[1, 2],
        max_decisions=100_000,
        max_ticks=1_000_000,
        seed=seed,
        num_threads=num_threads,
        layout=layout,
    )


def run_case(
    repo_root: Path,
    num_envs: int,
    num_threads: int | None,
    steps: int,
    warmup: int,
    seed: int,
    mode: str,
    reset_done: bool,
    layout: str,
) -> tuple[float, int, bool]:
    pool, buffers = build_case(repo_root, num_envs, num_threads, seed, layout)
    out = buffers.reset()
    rng = np.random.default_rng(seed)
    logits = rng.standard_normal((num_envs, int(pool.action_space)), dtype=np.float32)
    seeds = np.arange(num_envs, dtype=np.uint64) + np.uint64(seed + 1)
    actions = buffers.actions
    action_logp = np.empty(num_envs, dtype=np.float32)
    done_seen = False

    def step_once(out_step: Any) -> tuple[Any, bool]:
        nonlocal done_seen
        done = np.logical_or(out_step.terminated, out_step.truncated)
        if bool(done.any()):
            done_seen = True
            if not reset_done:
                return out_step, False
            out_step = buffers.reset_done(done)
        advance_seeds(seeds)
        if mode == "python_sample_logp":
            sample_python_legal_logp(
                logits,
                out_step.legal_ids,
                out_step.legal_offsets,
                seeds,
                actions,
                action_logp,
            )
            return buffers.step(actions), True
        if mode == "fused_sample_logp":
            out_step, _, _ = buffers.step_sample_from_logits_with_logp(
                logits,
                seeds,
                action_logp,
            )
            return out_step, True
        raise RuntimeError(f"unknown mode: {mode}")

    for _ in range(warmup):
        out, ok = step_once(out)
        if not ok:
            break

    executed = 0
    start = perf_counter()
    for _ in range(steps):
        out, ok = step_once(out)
        if not ok:
            break
        executed += 1
    elapsed = perf_counter() - start
    return elapsed, executed, done_seen


def main() -> None:
    parser = argparse.ArgumentParser(
        description="Benchmark RL-shaped logits sampling across the Python/Rust boundary."
    )
    parser.add_argument("--envs", type=str, default="128,256,512")
    parser.add_argument("--steps", type=int, default=2000)
    parser.add_argument("--warmup", type=int, default=200)
    parser.add_argument("--repeats", type=int, default=3)
    parser.add_argument("--seed", type=int, default=0)
    parser.add_argument("--num-threads", type=int, default=None)
    parser.add_argument(
        "--modes",
        type=str,
        default="python_sample_logp,fused_sample_logp",
        help="Comma-separated: python_sample_logp,fused_sample_logp",
    )
    parser.add_argument("--reset-done", choices=("false", "true", "both"), default="both")
    parser.add_argument(
        "--layouts",
        type=str,
        default="i16_legal_ids",
        help="Comma-separated: i16_legal_ids,i16_legal_ids_nometa",
    )
    parser.add_argument(
        "--repo-root",
        type=Path,
        default=Path(__file__).resolve().parents[2],
        help="Repository root containing python/weiss_sim.",
    )
    args = parser.parse_args()

    if args.repeats <= 0:
        raise ValueError("--repeats must be positive")

    global weiss_sim
    repo_root = args.repo_root.resolve()
    weiss_sim = _load_weiss_sim(repo_root)

    print(
        "num_envs,num_threads,reset_done,layout,mode,steps,warmup,repeats,"
        "median_elapsed_s,median_env_steps_per_sec,min_env_steps_per_sec,"
        "max_env_steps_per_sec,median_executed_steps,done_seen"
    )
    for num_envs in parse_int_list(args.envs):
        for reset_done in parse_reset_done_list(args.reset_done):
            for layout in parse_layout_list(args.layouts):
                for mode in parse_mode_list(args.modes):
                    elapsed_values: list[float] = []
                    eps_values: list[float] = []
                    executed_values: list[int] = []
                    done_seen_any = False
                    for repeat in range(args.repeats):
                        elapsed, executed, done_seen = run_case(
                            repo_root=repo_root,
                            num_envs=num_envs,
                            num_threads=args.num_threads,
                            steps=args.steps,
                            warmup=args.warmup,
                            seed=args.seed + repeat * 10_000 + num_envs,
                            mode=mode,
                            reset_done=reset_done,
                            layout=layout,
                        )
                        done_seen_any = done_seen_any or done_seen
                        elapsed_values.append(elapsed)
                        executed_values.append(executed)
                        eps_values.append((executed * num_envs) / max(elapsed, 1e-9))
                    median_elapsed = statistics.median(elapsed_values)
                    median_eps = statistics.median(eps_values)
                    median_executed = int(statistics.median(executed_values))
                    print(
                        f"{num_envs},{args.num_threads or ''},{str(reset_done).lower()},"
                        f"{layout},{mode},{args.steps},{args.warmup},{args.repeats},"
                        f"{median_elapsed:.6f},{median_eps:.0f},{min(eps_values):.0f},"
                        f"{max(eps_values):.0f},{median_executed},{str(done_seen_any).lower()}"
                    )


if __name__ == "__main__":
    main()
