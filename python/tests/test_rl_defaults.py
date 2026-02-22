import numpy as np
import weiss_sim


def test_make_pool_and_rl_helpers_mask_layout():
    legal_deck = (list(range(1, 14)) * 4)[:50]
    pool, _ = weiss_sim.make_pool(
        mode="train",
        num_envs=1,
        deck_lists=[legal_deck, legal_deck],
        deck_ids=[7, 8],
        max_decisions=200,
        max_ticks=10_000,
        seed=777,
        layout="mask",
    )
    out = weiss_sim.reset_rl(pool, layout="mask")
    assert out.obs.shape == (1, pool.obs_len)
    assert out.masks.shape == (1, pool.action_space)
    actions = [int(np.flatnonzero(out.masks[0])[0])]
    step = weiss_sim.step_rl(pool, np.array(actions, dtype=np.uint32), layout="mask")
    assert step.obs.shape == (1, pool.obs_len)
    assert step.rewards.shape == (1,)


def test_make_pool_defaults_to_bundled_db():
    legal_deck = (list(range(1, 14)) * 4)[:50]
    pool, buffers = weiss_sim.make_pool(
        mode="train",
        num_envs=1,
        deck_lists=[legal_deck, legal_deck],
        deck_ids=[9, 10],
        max_decisions=200,
        max_ticks=10_000,
        seed=778,
    )
    out = buffers.reset()
    assert out.obs.shape == (1, pool.obs_len)
    assert out.rewards.shape == (1,)
    assert buffers.layout == "i16_legal_ids"
    assert out.legal_ids is not None
    assert out.legal_offsets is not None


def test_low_level_symbol_surface_is_canonical():
    canonical = (
        "make_pool",
        "EnvPoolBuffers",
        "EnvPoolTrajectoryBuffers",
        "reset_rl",
        "step_rl",
        "step_rl_select_from_logits",
        "step_rl_sample_from_logits",
    )
    removed = (
        "make_train_pool",
        "make_eval_pool",
        "EnvPoolBuffersNoMask",
        "EnvPoolBuffersI16",
        "EnvPoolBuffersI16LegalIds",
        "EnvPoolTrajectoryBuffersNoMask",
        "EnvPoolTrajectoryBuffersI16",
        "EnvPoolTrajectoryBuffersI16LegalIds",
        "reset_rl_nomask",
        "reset_rl_i16_legal_ids",
        "step_rl_nomask",
        "step_rl_i16_legal_ids",
        "step_rl_select_from_logits_nomask",
        "step_rl_select_from_logits_i16_legal_ids",
        "step_rl_sample_from_logits_nomask",
        "step_rl_sample_from_logits_i16_legal_ids",
    )

    for name in canonical:
        assert hasattr(weiss_sim, name), f"missing canonical symbol: {name}"
    for name in removed:
        assert not hasattr(weiss_sim, name), f"legacy symbol should be absent: {name}"


def test_pass_action_id_mapping():
    assert weiss_sim.pass_action_id_for_decision_kind("Main") == weiss_sim.PASS_ACTION_ID
    assert weiss_sim.pass_action_id_for_decision_kind(2) == weiss_sim.PASS_ACTION_ID
    assert weiss_sim.pass_action_id_for_decision_kind("Clock") == weiss_sim.PASS_ACTION_ID
    assert weiss_sim.pass_action_id_for_decision_kind("Choice") == weiss_sim.PASS_ACTION_ID
