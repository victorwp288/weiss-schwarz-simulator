from pathlib import Path

import numpy as np
import weiss_sim


def make_pool(db_path: Path, deck: list[int], seed: int):
    pool, _ = weiss_sim.make_pool(
        mode="eval",
        num_envs=1,
        db_path=str(db_path),
        deck_lists=[deck, deck],
        deck_ids=[11, 12],
        seed=seed,
        max_decisions=200,
        max_ticks=10_000,
        layout="mask",
    )
    return pool


def first_legal_action(step) -> np.ndarray:
    legal = np.flatnonzero(step.masks[0])
    if legal.size == 0:
        return np.array([weiss_sim.PASS_ACTION_ID], dtype=np.uint32)
    return np.array([int(legal[0])], dtype=np.uint32)


def main() -> None:
    fixture_dir = Path(__file__).resolve().parents[1] / "tests" / "fixtures"
    db_path = fixture_dir / "cards.wsdb"
    legal_deck = (list(range(1, 14)) * 4)[:50]

    print("Spec bundle:")
    print(weiss_sim.spec_bundle())

    pool_a = make_pool(db_path, legal_deck, seed=123)
    pool_b = make_pool(db_path, legal_deck, seed=123)

    out_a = weiss_sim.reset_rl(pool_a, layout="mask")
    out_b = weiss_sim.reset_rl(pool_b, layout="mask")
    assert np.array_equal(out_a.obs, out_b.obs)
    assert np.array_equal(out_a.masks, out_b.masks)

    for _ in range(5):
        actions = first_legal_action(out_a)
        out_a = weiss_sim.step_rl(pool_a, actions, layout="mask")
        out_b = weiss_sim.step_rl(pool_b, actions, layout="mask")
        assert np.array_equal(out_a.obs, out_b.obs)
        assert np.array_equal(out_a.masks, out_b.masks)

    print("Determinism check OK.")


if __name__ == "__main__":
    main()
