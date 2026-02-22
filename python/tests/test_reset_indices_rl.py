from pathlib import Path

import numpy as np
import weiss_sim


def _make_pool(seed=7777, num_envs=2, *, layout="mask"):
    fixture_dir = Path(__file__).parent / "fixtures"
    db_path = fixture_dir / "cards.wsdb"
    legal_deck = (list(range(1, 14)) * 4)[:50]
    return weiss_sim.make_pool(
        mode="train",
        num_envs=num_envs,
        db_path=str(db_path),
        deck_lists=[legal_deck, legal_deck],
        deck_ids=[51, 52],
        max_decisions=200,
        max_ticks=10_000,
        seed=seed,
        layout=layout,
    )


def test_reset_indices_into_preserves_other_envs():
    _, buffers = _make_pool(layout="mask")
    out = buffers.reset()
    obs_before = out.obs.copy()

    buffers.reset_indices([0])
    assert np.array_equal(out.obs[1], obs_before[1])


def test_reset_done_into_matches_indices():
    _, buffers_a = _make_pool(seed=8888, layout="mask")
    _, buffers_b = _make_pool(seed=8888, layout="mask")
    out_a = buffers_a.reset()
    out_b = buffers_b.reset()

    buffers_a.reset_indices([0])
    done_mask = np.array([True, False], dtype=np.bool_)
    buffers_b.reset_done(done_mask)

    assert np.array_equal(out_a.obs, out_b.obs)
    assert np.array_equal(out_a.masks, out_b.masks)
