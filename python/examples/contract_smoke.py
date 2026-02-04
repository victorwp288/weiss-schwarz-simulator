from pathlib import Path

import numpy as np
import weiss_sim


def make_pool(db_path: Path, deck: list[int], seed: int) -> weiss_sim.EnvPool:
    return weiss_sim.EnvPool.new_rl_eval(
        1,
        str(db_path),
        deck_lists=[deck, deck],
        deck_ids=[1, 2],
        max_decisions=200,
        max_ticks=10_000,
        seed=seed,
        error_policy="strict",
    )


def main() -> None:
    fixture_dir = Path(__file__).resolve().parents[1] / "tests" / "fixtures"
    db_path = fixture_dir / "cards.wsdb"
    legal_deck = (list(range(1, 14)) * 4)[:50]

    print("Spec bundle:")
    print(weiss_sim.spec_bundle())

    pool_a = make_pool(db_path, legal_deck, seed=123)
    pool_b = make_pool(db_path, legal_deck, seed=123)
    buffers_a = weiss_sim.EnvPoolBuffers(pool_a)
    buffers_b = weiss_sim.EnvPoolBuffers(pool_b)

    out_a = buffers_a.reset()
    out_b = buffers_b.reset()
    assert np.array_equal(out_a.obs, out_b.obs)
    assert np.array_equal(out_a.masks, out_b.masks)

    actions = np.empty(out_a.masks.shape[0], dtype=np.uint32)
    for _ in range(5):
        ids, offsets = buffers_a.legal_action_ids()
        for i in range(out_a.masks.shape[0]):
            start = int(offsets[i])
            end = int(offsets[i + 1])
            if start == end:
                actions[i] = weiss_sim.PASS_ACTION_ID
            else:
                actions[i] = int(ids[start])
        out_a = buffers_a.step(actions)
        out_b = buffers_b.step(actions)
        assert np.array_equal(out_a.obs, out_b.obs)
        assert np.array_equal(out_a.masks, out_b.masks)

    print("Determinism check OK.")


if __name__ == "__main__":
    main()
