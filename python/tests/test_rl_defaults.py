import numpy as np
import weiss_sim


def test_envpool_new_rl_defaults():
    legal_deck = (list(range(1, 14)) * 4)[:50]
    pool = weiss_sim.EnvPool.new_rl_train(
        1,
        deck_lists=[legal_deck, legal_deck],
        deck_ids=[7, 8],
        max_decisions=200,
        max_ticks=10_000,
        seed=777,
    )
    out = weiss_sim.BatchOutMinimal(1)
    pool.reset_into(out)
    assert out.obs.shape == (1, pool.obs_len)
    assert out.masks.shape == (1, pool.action_space)
    actions = [int(np.flatnonzero(out.masks[0])[0])]
    pool.step_into(np.array(actions, dtype=np.uint32), out)


def test_make_train_pool_defaults_to_bundled_db():
    legal_deck = (list(range(1, 14)) * 4)[:50]
    pool, buffers = weiss_sim.make_train_pool(
        1,
        deck_lists=[legal_deck, legal_deck],
        deck_ids=[9, 10],
        max_decisions=200,
        max_ticks=10_000,
        seed=778,
    )
    out = buffers.reset()
    assert out.obs.shape == (1, pool.obs_len)
    assert out.rewards.shape == (1,)


def test_pass_action_id_mapping():
    assert weiss_sim.pass_action_id_for_decision_kind("Main") == weiss_sim.PASS_ACTION_ID
    assert weiss_sim.pass_action_id_for_decision_kind(2) == weiss_sim.PASS_ACTION_ID
    assert weiss_sim.pass_action_id_for_decision_kind("Clock") == weiss_sim.PASS_ACTION_ID
    assert weiss_sim.pass_action_id_for_decision_kind("Choice") == weiss_sim.PASS_ACTION_ID
