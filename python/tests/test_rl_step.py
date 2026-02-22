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


def _make_pool(seed=888, num_envs=2, *, layout="mask"):
    fixture_dir = Path(__file__).parent / "fixtures"
    db_path = fixture_dir / "cards.wsdb"
    legal_deck = (list(range(1, 14)) * 4)[:50]
    return weiss_sim.make_pool(
        mode="train",
        num_envs=num_envs,
        db_path=str(db_path),
        deck_lists=[legal_deck, legal_deck],
        deck_ids=[1, 2],
        max_decisions=200,
        max_ticks=10_000,
        seed=seed,
        layout=layout,
    )


def test_envpool_buffers_step_reuses_buffers():
    pool, buffers = _make_pool(layout="mask")
    out_reset = buffers.reset()
    assert out_reset is buffers.out

    actions = _first_legal_actions(buffers.masks)
    out_step = buffers.step(np.array(actions, dtype=np.uint32))
    assert out_step is buffers.out
    assert buffers.obs is buffers.out.obs
    assert buffers.masks is buffers.out.masks
    assert buffers.rewards is buffers.out.rewards
    assert buffers.terminated is buffers.out.terminated
    assert buffers.truncated is buffers.out.truncated
    assert buffers.actor is buffers.out.actor
    assert buffers.decision_id is buffers.out.decision_id
    assert buffers.engine_status is buffers.out.engine_status
    assert buffers.layout == "mask"
    assert pool.envs_len == 2
