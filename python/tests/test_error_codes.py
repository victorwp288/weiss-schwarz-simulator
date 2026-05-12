import numpy as np
import weiss_sim

from tests.support import (
    _DEFAULT_LEGAL_DECK,
    _FIXTURE_DB_PATH,
    first_legal_actions as _first_legal_actions,
    make_rl_train_pool,
)


def _make_pool(seed=8080, num_envs=1):
    return make_rl_train_pool(
        seed=seed,
        num_envs=num_envs,
        deck_ids=(11, 12),
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


def test_auto_reset_on_error_codes_zero_codes_noop_and_deterministic():
    pool_a = _make_pool(seed=6001, num_envs=2)
    pool_b = _make_pool(seed=6001, num_envs=2)
    out_a = weiss_sim.BatchOutMinimal(2)
    out_b = weiss_sim.BatchOutMinimal(2)

    pool_a.reset_into(out_a)
    pool_b.reset_into(out_b)
    actions = np.array(_first_legal_actions(out_a.masks), dtype=np.uint32)
    pool_a.step_into(actions, out_a)
    pool_b.step_into(actions, out_b)

    zero_codes = np.zeros((2,), dtype=np.uint8)
    assert pool_a.auto_reset_on_error_codes_into(zero_codes, out_a) == 0
    assert np.array_equal(out_a.obs, out_b.obs)
    assert np.array_equal(out_a.masks, out_b.masks)
    assert np.array_equal(out_a.engine_status, out_b.engine_status)

    next_actions = np.array(_first_legal_actions(out_a.masks), dtype=np.uint32)
    pool_a.step_into(next_actions, out_a)
    pool_b.step_into(next_actions, out_b)
    assert np.array_equal(out_a.obs, out_b.obs)
    assert np.array_equal(out_a.masks, out_b.masks)


def test_auto_reset_on_error_codes_matches_manual_reset_for_flagged_env():
    pool_auto = _make_pool(seed=6002, num_envs=2)
    pool_manual = _make_pool(seed=6002, num_envs=2)
    out_auto = weiss_sim.BatchOutMinimal(2)
    out_manual = weiss_sim.BatchOutMinimal(2)

    pool_auto.reset_into(out_auto)
    pool_manual.reset_into(out_manual)
    actions = np.array(_first_legal_actions(out_auto.masks), dtype=np.uint32)
    pool_auto.step_into(actions, out_auto)
    pool_manual.step_into(actions, out_manual)
    assert np.array_equal(out_auto.obs, out_manual.obs)

    forced_codes = np.array([1, 0], dtype=np.uint8)
    assert pool_auto.auto_reset_on_error_codes_into(forced_codes, out_auto) == 1
    pool_manual.reset_indices_into([0], out_manual)

    assert np.array_equal(out_auto.obs[1], out_manual.obs[1])
    assert np.array_equal(out_auto.masks[1], out_manual.masks[1])
    assert int(out_auto.actor[1]) == int(out_manual.actor[1])

    actions_auto = np.array(_first_legal_actions(out_auto.masks), dtype=np.uint32)
    actions_manual = np.array(_first_legal_actions(out_manual.masks), dtype=np.uint32)
    assert int(actions_auto[1]) == int(actions_manual[1])
    pool_auto.step_into(actions_auto, out_auto)
    pool_manual.step_into(actions_manual, out_manual)

    assert np.array_equal(out_auto.obs[1], out_manual.obs[1])
    assert np.array_equal(out_auto.masks[1], out_manual.masks[1])
    assert int(out_auto.actor[1]) == int(out_manual.actor[1])


def test_make_batch_out_debug_uses_pool_ring_capacity_default():
    pool = _make_pool(seed=7007, num_envs=2)
    out = weiss_sim.make_batch_out_debug(pool)
    pool.reset_debug_into(out)
    assert out.event_codes.shape[0] == pool.num_envs
    assert out.event_codes.shape[1] == pool.debug_event_ring_capacity()
    assert out.reward_components.shape == (pool.num_envs, weiss_sim.REWARD_COMPONENT_WIDTH)


def test_debug_reward_components_sum_to_reward():
    pool = weiss_sim.EnvPool.new_rl_train(
        1,
        str(_FIXTURE_DB_PATH),
        deck_lists=[_DEFAULT_LEGAL_DECK, _DEFAULT_LEGAL_DECK],
        deck_ids=[11, 12],
        seed=7009,
        reward_json='{"enable_shaping":true,"damage_reward":0.1}',
    )
    out = weiss_sim.make_batch_out_debug(pool)
    pool.reset_debug_into(out)
    actions = _first_legal_actions(out.masks)
    pool.step_debug_into(np.array(actions, dtype=np.uint32), out)
    assert out.reward_components.shape == (pool.num_envs, weiss_sim.REWARD_COMPONENT_WIDTH)
    assert np.allclose(out.reward_components.sum(axis=1), out.rewards)


def test_make_batch_out_debug_rejects_negative_capacity():
    pool = _make_pool(seed=7008, num_envs=1)
    with np.testing.assert_raises(ValueError):
        weiss_sim.make_batch_out_debug(pool, event_capacity=-1)
