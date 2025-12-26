from pathlib import Path

import numpy as np
import weiss_sim


def _make_pool(seed=7777, num_envs=2):
    fixture_dir = Path(__file__).parent / "fixtures"
    db_path = fixture_dir / "cards.wsdb"
    legal_deck = (list(range(1, 14)) * 4)[:50]
    return weiss_sim.EnvPool.new_rl_train(
        num_envs,
        str(db_path),
        deck_lists=[legal_deck, legal_deck],
        deck_ids=[51, 52],
        max_decisions=200,
        max_ticks=10_000,
        seed=seed,
    )


def test_reset_indices_into_preserves_other_envs():
    pool = _make_pool()
    out = weiss_sim.BatchOutMinimal(2)
    pool.reset_into(out)
    obs_before = out.obs.copy()

    pool.reset_indices_into([0], out)
    assert np.array_equal(out.obs[1], obs_before[1])


def test_reset_done_into_matches_indices():
    pool_a = _make_pool(seed=8888)
    pool_b = _make_pool(seed=8888)
    out_a = weiss_sim.BatchOutMinimal(2)
    out_b = weiss_sim.BatchOutMinimal(2)
    pool_a.reset_into(out_a)
    pool_b.reset_into(out_b)

    pool_a.reset_indices_into([0], out_a)
    done_mask = np.array([True, False], dtype=np.bool_)
    pool_b.reset_done_into(done_mask, out_b)

    assert np.array_equal(out_a.obs, out_b.obs)
    assert np.array_equal(out_a.masks, out_b.masks)
