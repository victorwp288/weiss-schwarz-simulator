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


def _make_pool(seed=8080, num_envs=1):
    fixture_dir = Path(__file__).parent / "fixtures"
    db_path = fixture_dir / "cards.wsdb"
    legal_deck = (list(range(1, 14)) * 4)[:50]
    return weiss_sim.EnvPool.new_rl_train(
        num_envs,
        str(db_path),
        deck_lists=[legal_deck, legal_deck],
        deck_ids=[11, 12],
        max_decisions=200,
        max_ticks=10_000,
        seed=seed,
    )


def test_engine_status_codes_exposed():
    pool = _make_pool()
    out = weiss_sim.BatchOutMinimal(1)
    pool.reset_into(out)
    actions = _first_legal_actions(out.masks)
    pool.step_into(np.array(actions, dtype=np.uint32), out)
    assert out.engine_status.shape == (1,)
    assert int(out.engine_status[0]) == 0


def test_auto_reset_on_engine_error_codes_updates_count():
    pool = _make_pool(seed=4242, num_envs=2)
    out = weiss_sim.BatchOutMinimal(2)
    pool.reset_into(out)

    codes = np.zeros((2,), dtype=np.uint8)
    assert pool.engine_error_reset_count() == 0
    assert pool.auto_reset_on_error_codes_into(codes, out) == 0
    assert pool.engine_error_reset_count() == 0

    codes[0] = 1
    assert pool.auto_reset_on_error_codes_into(codes, out) == 1
    assert pool.engine_error_reset_count() == 1

    pool.reset_engine_error_reset_count()
    assert pool.engine_error_reset_count() == 0


def test_python_wrapper_exposes_engine_error_helpers():
    pool = _make_pool(seed=9090, num_envs=1)
    buf = weiss_sim.EnvPoolBuffers(pool)
    out = buf.reset()

    assert buf.engine_error.shape == (1,)
    assert buf.reset_recommended.shape == (1,)
    assert buf.actor_known.shape == (1,)
    assert not bool(buf.engine_error[0])
    assert not bool(buf.reset_recommended[0])
    assert bool(buf.actor_known[0])

    out.engine_status[0] = 3
    out.actor[0] = weiss_sim.ACTOR_NONE
    assert bool(buf.engine_error[0])
    assert bool(buf.reset_recommended[0])
    assert not bool(buf.actor_known[0])
