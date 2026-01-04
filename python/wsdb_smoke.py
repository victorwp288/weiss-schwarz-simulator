from pathlib import Path

import numpy as np
import weiss_sim


def main() -> None:
    repo_root = Path(__file__).resolve().parents[1]
    db_path = repo_root / "scraper" / "out" / "cards.wsdb"
    if not db_path.exists():
        raise SystemExit(f"cards.wsdb not found at {db_path}")

    legal_deck = (list(range(1, 14)) * 4)[:50]
    pool = weiss_sim.EnvPool.new_rl_train(
        1,
        str(db_path),
        deck_lists=[legal_deck, legal_deck],
        deck_ids=[1, 2],
        max_decisions=200,
        max_ticks=10_000,
        seed=123,
        num_threads=None,
    )

    buf = weiss_sim.EnvPoolBuffers(pool)
    buf.reset()
    print("obs", buf.obs.shape, "masks", buf.masks.shape, "actor", buf.actor)

    for step in range(10):
        ids_flat, offsets = buf.legal_action_ids()
        start, end = int(offsets[0]), int(offsets[1])
        if end <= start:
            raise RuntimeError("no legal actions available")
        action = int(ids_flat[start])
        buf.step(np.array([action], dtype=np.uint32))
        print(
            "step",
            step,
            "reward",
            float(buf.rewards[0]),
            "terminated",
            bool(buf.terminated[0]),
        )
        if bool(buf.terminated[0]) or bool(buf.truncated[0]):
            break


if __name__ == "__main__":
    main()
