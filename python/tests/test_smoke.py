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


def test_envpool_smoke(tmp_path):
    assert hasattr(weiss_sim, "EnvPool")
    assert isinstance(weiss_sim.__version__, str)


def test_envpool_step_shapes_and_turn_cycle():
    fixture_dir = Path(__file__).parent / "fixtures"
    db_path = fixture_dir / "cards.wsdb"
    pool = weiss_sim.EnvPool(
        1,
        str(db_path),
        deck_lists=[[1] * 20, [2] * 20],
        deck_ids=[1, 2],
        max_decisions=200,
        max_ticks=10_000,
        seed=123,
        observation_visibility="public",
    )
    obs = pool.reset_all()
    assert obs.shape == (1, pool.obs_len)

    starting_player = int(pool.get_current_player_batch()[0])
    seen_other_turn = False
    for _ in range(50):
        masks = pool.action_masks_batch()
        actions = _first_legal_actions(masks)
        result = pool.step_batch_fast(actions)
        assert len(result) == 9
        next_obs, rewards, terminated, truncated, current_player, decision_kind, actor, illegal_action, engine_error = result
        assert next_obs.shape == (1, pool.obs_len)
        assert rewards.shape == (1,)
        assert terminated.shape == (1,)
        assert truncated.shape == (1,)
        assert current_player.shape == (1,)
        assert decision_kind.shape == (1,)
        assert actor.shape == (1,)
        assert illegal_action.shape == (1,)
        assert engine_error.shape == (1,)
        if int(current_player[0]) != starting_player and int(decision_kind[0]) == 1:
            seen_other_turn = True
            break
    assert seen_other_turn


def test_action_mask_legality_alignment():
    fixture_dir = Path(__file__).parent / "fixtures"
    db_path = fixture_dir / "cards.wsdb"
    pool = weiss_sim.EnvPool(
        1,
        str(db_path),
        deck_lists=[[1] * 20, [2] * 20],
        deck_ids=[1, 2],
        max_decisions=200,
        max_ticks=10_000,
        seed=456,
        observation_visibility="public",
    )
    pool.reset_all()
    for _ in range(30):
        masks = pool.action_masks_batch()
        actions = _first_legal_actions(masks)
        assert masks[0, actions[0]] == 1
        pool.step_batch_fast(actions)
