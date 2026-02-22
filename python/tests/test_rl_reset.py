from pathlib import Path

import numpy as np
import weiss_sim


def _make_pool(seed=1234, num_envs=2, *, layout="mask"):
    fixture_dir = Path(__file__).parent / "fixtures"
    db_path = fixture_dir / "cards.wsdb"
    legal_deck = (list(range(1, 14)) * 4)[:50]
    pool, _ = weiss_sim.make_pool(
        mode="train",
        num_envs=num_envs,
        db_path=str(db_path),
        deck_lists=[legal_deck, legal_deck],
        deck_ids=[31, 32],
        max_decisions=200,
        max_ticks=10_000,
        seed=seed,
        layout=layout,
    )
    return pool


def test_reset_rl_returns_masks_and_status():
    pool = _make_pool(layout="mask")
    out = weiss_sim.reset_rl(pool, layout="mask")
    assert out.obs.shape == (2, pool.obs_len)
    assert out.masks.shape == (2, pool.action_space)
    assert out.rewards.shape == (2,)
    assert out.terminated.shape == (2,)
    assert out.truncated.shape == (2,)
    assert out.actor.shape == (2,)
    assert out.engine_status.shape == (2,)
    assert out.decision_id.shape == (2,)
    assert out.spec_hash.shape == (2,)
    assert np.all(out.engine_status == 0)


def test_reset_rl_matches_deterministic_pool():
    pool_a = _make_pool(seed=2222, num_envs=2, layout="mask")
    pool_b = _make_pool(seed=2222, num_envs=2, layout="mask")
    out_a = weiss_sim.reset_rl(pool_a, layout="mask")
    out_b = weiss_sim.reset_rl(pool_b, layout="mask")
    assert np.array_equal(out_a.obs, out_b.obs)
    assert np.array_equal(out_a.masks, out_b.masks)
