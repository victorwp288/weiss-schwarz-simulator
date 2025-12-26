from pathlib import Path

import numpy as np
import weiss_sim


def _first_legal_actions(masks):
    actions = []
    for i in range(masks.shape[0]):
        row = masks[i]
        idxs = np.flatnonzero(row)
        assert idxs.size > 0
        actions.append(int(idxs[0]))
    return actions


def _make_pool(seed=2024):
    fixture_dir = Path(__file__).parent / "fixtures"
    db_path = fixture_dir / "cards.wsdb"
    legal_deck = (list(range(1, 14)) * 4)[:50]
    return weiss_sim.EnvPool.new_rl_train(
        1,
        str(db_path),
        deck_lists=[legal_deck, legal_deck],
        deck_ids=[9, 10],
        max_decisions=200,
        max_ticks=10_000,
        seed=seed,
    )


def test_step_into_buffers_match_between_pools():
    pool_a = _make_pool(seed=2024)
    pool_b = _make_pool(seed=2024)
    out_a = weiss_sim.BatchOutMinimal(1)
    out_b = weiss_sim.BatchOutMinimal(1)
    pool_a.reset_into(out_a)
    pool_b.reset_into(out_b)
    for _ in range(10):
        actions = _first_legal_actions(out_a.masks)
        pool_a.step_into(np.array(actions, dtype=np.uint32), out_a)
        pool_b.step_into(np.array(actions, dtype=np.uint32), out_b)
        assert np.array_equal(out_a.obs, out_b.obs)
        assert np.array_equal(out_a.masks, out_b.masks)
        if bool(out_a.terminated[0]) or bool(out_a.truncated[0]):
            break
