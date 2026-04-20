from pathlib import Path

import numpy as np
import weiss_sim


_FIXTURE_DB_PATH = Path(__file__).parent / "fixtures" / "cards.wsdb"
_LEGAL_DECK = (list(range(1, 14)) * 4)[:50]


def _make_pool(seed=123, num_envs=1, *, layout="mask", **kwargs):
    return weiss_sim.make_pool(
        mode="train",
        num_envs=num_envs,
        db_path=str(_FIXTURE_DB_PATH),
        deck_lists=[_LEGAL_DECK, _LEGAL_DECK],
        deck_ids=[63, 64],
        max_decisions=200,
        max_ticks=10_000,
        seed=seed,
        layout=layout,
        **kwargs,
    )


def _env_legal_ids(legal_ids, legal_offsets, env_index=0):
    start = int(legal_offsets[env_index])
    end = int(legal_offsets[env_index + 1])
    return legal_ids[start:end]


def test_legal_id_buffers_auto_disable_output_masks_and_step_logits_select():
    pool, buffers = _make_pool(seed=1201, layout="i16_legal_ids")

    out = buffers.reset()
    legal = _env_legal_ids(out.legal_ids, out.legal_offsets)
    assert np.all(out.engine_status == 0)
    assert legal.size > 0

    target = int(legal[-1])
    logits = np.full((1, pool.action_space), -3.0, dtype=np.float32)
    logits[0, target] = 2.0

    out_step, actions = buffers.step_select_from_logits(logits)
    assert out_step is buffers.out
    assert int(actions[0]) == target
    assert int(out_step.engine_status[0]) == 0


def test_legal_id_trajectory_buffers_auto_disable_output_masks():
    pool, _ = _make_pool(seed=1202, layout="i16_legal_ids")
    buffers = weiss_sim.EnvPoolTrajectoryBuffers(pool, steps=2, layout="i16_legal_ids")

    out = buffers.rollout_first_legal()
    assert buffers.episode_seed is not None
    assert buffers.episode_seed is out.episode_seed
    assert buffers.episode_seed.shape == (2, 1)
    assert buffers.episode_seed.dtype == np.uint64
    assert out.engine_status.shape == (2, 1)
    assert np.all(out.engine_status == 0)
    assert int(out.legal_offsets[0, -1]) > 0
    assert int(out.legal_offsets[1, -1]) > 0


def test_make_pool_nomask_path_and_step_logit_helpers():
    pool, buffers = _make_pool(seed=1301, layout="nomask")
    assert isinstance(buffers, weiss_sim.EnvPoolBuffers)
    assert buffers.layout == "nomask"

    out = buffers.reset()
    legal_ids, legal_offsets = buffers.legal_action_ids()
    legal = _env_legal_ids(legal_ids, legal_offsets)
    assert int(out.engine_status[0]) == 0
    assert legal.size > 0

    target = int(legal[-1])
    logits = np.full((1, pool.action_space), -2.0, dtype=np.float32)
    logits[0, target] = 4.0

    out_step, step_actions = buffers.step_select_from_logits(logits)
    assert int(step_actions[0]) == target
    assert int(out_step.engine_status[0]) == 0

    sample_logits = np.zeros((1, pool.action_space), dtype=np.float32)
    out_sample, sample_actions = buffers.step_sample_from_logits(
        sample_logits,
        np.array([777], dtype=np.uint64),
    )
    assert int(out_sample.engine_status[0]) == 0
    assert 0 <= int(sample_actions[0]) < pool.action_space


def test_nomask_select_and_sample_action_helpers_deterministic():
    pool_a, buffers_a = _make_pool(seed=1401, layout="nomask")
    pool_b, buffers_b = _make_pool(seed=1401, layout="nomask")
    buffers_a.reset()
    buffers_b.reset()

    legal_ids, legal_offsets = buffers_a.legal_action_ids()
    legal = set(_env_legal_ids(legal_ids, legal_offsets).tolist())

    logits = np.linspace(-1.0, 1.0, pool_a.action_space, dtype=np.float32)[None, :]
    seeds = np.array([991], dtype=np.uint64)

    selected_a = buffers_a.select_actions_from_logits(logits)
    selected_b = buffers_b.select_actions_from_logits(logits)
    sampled_a = buffers_a.sample_actions_from_logits(logits, seeds)
    sampled_b = buffers_b.sample_actions_from_logits(logits, seeds)

    assert np.array_equal(selected_a, selected_b)
    assert np.array_equal(sampled_a, sampled_b)
    assert int(selected_a[0]) in legal
    assert int(sampled_a[0]) in legal


def test_i16_legal_ids_logit_helpers_select_and_sample_deterministic():
    pool_a, _ = _make_pool(seed=1501, layout="i16_legal_ids")
    pool_b, _ = _make_pool(seed=1501, layout="i16_legal_ids")

    step_a = weiss_sim.reset_rl(pool_a, layout="i16_legal_ids")
    step_b = weiss_sim.reset_rl(pool_b, layout="i16_legal_ids")
    assert np.array_equal(step_a.obs, step_b.obs)
    assert np.array_equal(step_a.legal_offsets, step_b.legal_offsets)

    legal = _env_legal_ids(step_a.legal_ids, step_a.legal_offsets)
    target = int(legal[-1])
    select_logits = np.full((1, pool_a.action_space), -1.0, dtype=np.float32)
    select_logits[0, target] = 5.0

    step_sel_a, actions_sel_a = weiss_sim.step_rl_select_from_logits(
        pool_a,
        select_logits,
        layout="i16_legal_ids",
    )
    step_sel_b, actions_sel_b = weiss_sim.step_rl_select_from_logits(
        pool_b,
        select_logits,
        layout="i16_legal_ids",
    )

    assert int(actions_sel_a[0]) == target
    assert np.array_equal(actions_sel_a, actions_sel_b)
    assert np.array_equal(step_sel_a.obs, step_sel_b.obs)

    legal_after_select = set(
        _env_legal_ids(step_sel_a.legal_ids, step_sel_a.legal_offsets).tolist()
    )
    sample_logits = np.zeros((1, pool_a.action_space), dtype=np.float32)
    seeds = np.array([551], dtype=np.uint64)

    step_samp_a, actions_samp_a = weiss_sim.step_rl_sample_from_logits(
        pool_a,
        sample_logits,
        seeds,
        layout="i16_legal_ids",
    )
    step_samp_b, actions_samp_b = weiss_sim.step_rl_sample_from_logits(
        pool_b,
        sample_logits,
        seeds,
        layout="i16_legal_ids",
    )

    assert np.array_equal(actions_samp_a, actions_samp_b)
    assert np.array_equal(step_samp_a.obs, step_samp_b.obs)
    assert int(actions_samp_a[0]) in legal_after_select
