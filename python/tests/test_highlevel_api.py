from __future__ import annotations

import importlib.util
import json
from pathlib import Path

import numpy as np
import pytest
import weiss_sim
import weiss_sim.api as api_mod
import weiss_sim.decks as decks_mod
import weiss_sim.runner as runner_mod
from tests.support import (
    assert_reset_batches_equal as _assert_reset_batches_equal,
    assert_step_batches_equal as _assert_step_batches_equal,
    first_legal_actions as _first_legal_actions,
)


def _assert_legal_ids_strictly_sorted(
    legal_ids: np.ndarray, legal_offsets: np.ndarray, num_envs: int
) -> None:
    for i in range(num_envs):
        start = int(legal_offsets[i])
        end = int(legal_offsets[i + 1])
        env_ids = legal_ids[start:end]
        if env_ids.shape[0] <= 1:
            continue
        assert np.all(env_ids[1:] > env_ids[:-1])


def _assert_common_reset_contract(batch, *, num_envs: int, obs_dtype: np.dtype) -> None:
    assert batch.obs.shape == (num_envs, weiss_sim.OBS_LEN)
    assert batch.obs.dtype == obs_dtype
    assert batch.to_play_seat.shape == (num_envs,)
    assert batch.to_play_seat.dtype == np.int8
    assert set(np.unique(batch.to_play_seat).tolist()).issubset({-1, 0, 1})
    assert batch.starting_seat.shape == (num_envs,)
    assert batch.starting_seat.dtype == np.uint8
    assert set(np.unique(batch.starting_seat).tolist()).issubset({0, 1})
    assert batch.episode_seed.shape == (num_envs,)
    assert batch.episode_seed.dtype == np.uint64
    assert batch.episode_index.shape == (num_envs,)
    assert batch.episode_index.dtype == np.uint32
    assert batch.env_index.shape == (num_envs,)
    assert batch.env_index.dtype == np.uint32
    assert batch.episode_key.shape == (num_envs,)
    assert batch.episode_key.dtype == np.uint64
    assert batch.decision_id.shape == (num_envs,)
    assert batch.decision_id.dtype == np.uint32
    assert batch.engine_status.shape == (num_envs,)
    assert batch.engine_status.dtype == np.uint8
    assert batch.spec_hash.shape == (num_envs,)
    assert batch.spec_hash.dtype == np.uint64
    assert np.all(batch.spec_hash == np.uint64(weiss_sim.SPEC_HASH))
    assert batch.main_move_action.shape == (num_envs,)
    assert batch.main_move_action.dtype == np.bool_
    assert batch.main_pass_action.shape == (num_envs,)
    assert batch.main_pass_action.dtype == np.bool_


def _assert_common_step_contract(step, *, num_envs: int, obs_dtype: np.dtype) -> None:
    _assert_common_reset_contract(step, num_envs=num_envs, obs_dtype=obs_dtype)
    assert step.reward.shape == (num_envs,)
    assert step.reward.dtype == np.float32
    assert step.terminated.shape == (num_envs,)
    assert step.terminated.dtype == np.bool_
    assert step.truncated.shape == (num_envs,)
    assert step.truncated.dtype == np.bool_
    assert not np.any(np.logical_and(step.terminated, step.truncated))
    assert step.terminal_during_internal_opponent.shape == (num_envs,)
    assert step.terminal_during_internal_opponent.dtype == np.bool_
    assert step.decision_count.shape == (num_envs,)
    assert step.decision_count.dtype == np.uint32
    assert step.tick_count.shape == (num_envs,)
    assert step.tick_count.dtype == np.uint32
    assert step.no_progress_count.shape == (num_envs,)
    assert step.no_progress_count.dtype == np.uint32
    assert step.main_move_action.shape == (num_envs,)
    assert step.main_move_action.dtype == np.bool_
    assert step.main_pass_action.shape == (num_envs,)
    assert step.main_pass_action.dtype == np.bool_


@pytest.mark.parametrize(
    ("legal_repr", "obs_dtype", "expects_mask", "expects_ids", "ids_dtype"),
    [
        ("ids_u16", "i16", False, True, np.uint16),
        ("ids_u32", "i32", False, True, np.uint32),
        ("mask_u8", "i32", True, False, None),
        ("both", "i32", True, True, np.uint32),
    ],
)
def test_make_output_contract_by_legal_repr(
    legal_repr, obs_dtype, expects_mask, expects_ids, ids_dtype
):
    sim = weiss_sim.make(
        mode="inspect",
        num_envs=2,
        seed=123,
        legal_repr=legal_repr,
        obs_dtype=obs_dtype,
        card_pool="all",
    )
    reset = sim.reset()
    expected_obs_dtype = np.int16 if obs_dtype == "i16" else np.int32
    _assert_common_reset_contract(reset, num_envs=2, obs_dtype=expected_obs_dtype)

    if expects_mask:
        assert reset.legal_mask is not None
        assert reset.legal_mask.shape == (2, weiss_sim.ACTION_SPACE_SIZE)
        assert reset.legal_mask.dtype == np.uint8
    else:
        assert reset.legal_mask is None

    if expects_ids:
        assert reset.legal_ids is not None
        assert reset.legal_offsets is not None
        assert reset.legal_offsets.shape == (3,)
        assert reset.legal_offsets.dtype == np.uint32
        assert reset.legal_ids.dtype == ids_dtype
    else:
        assert reset.legal_ids is None
        assert reset.legal_offsets is None

    actions = _first_legal_actions(reset, 2)
    step = sim.step(actions)
    _assert_common_step_contract(step, num_envs=2, obs_dtype=expected_obs_dtype)

    if expects_mask:
        assert step.legal_mask is not None
        assert step.legal_mask.shape == (2, weiss_sim.ACTION_SPACE_SIZE)
        assert step.legal_mask.dtype == np.uint8
    else:
        assert step.legal_mask is None

    if expects_ids:
        assert step.legal_ids is not None
        assert step.legal_offsets is not None
        assert step.legal_offsets.shape == (3,)
        assert step.legal_offsets.dtype == np.uint32
        assert step.legal_ids.dtype == ids_dtype
    else:
        assert step.legal_ids is None
        assert step.legal_offsets is None


def test_fast_and_inspect_zero_config_reset_step_contract():
    with weiss_sim.fast(num_envs=2, seed=123) as fast_sim:
        fast_reset = fast_sim.reset()
        fast_actions = _first_legal_actions(fast_reset, 2)
        fast_step = fast_sim.step(fast_actions)
        _assert_common_step_contract(fast_step, num_envs=2, obs_dtype=np.int16)
        assert fast_reset.legal_mask is None
        assert fast_reset.legal_ids is not None
        assert fast_step.legal_mask is None
        assert fast_step.legal_ids is not None

    with weiss_sim.inspect(num_envs=2, seed=123) as inspect_sim:
        inspect_reset = inspect_sim.reset()
        inspect_actions = _first_legal_actions(inspect_reset, 2)
        inspect_step = inspect_sim.step(inspect_actions)
        _assert_common_step_contract(inspect_step, num_envs=2, obs_dtype=np.int32)
        assert inspect_reset.legal_mask is not None
        assert inspect_reset.legal_ids is not None
        assert inspect_step.legal_mask is not None
        assert inspect_step.legal_ids is not None


def test_mode_mapping_defaults_and_runtime_mode_rejected():
    with weiss_sim.make(mode="fast", num_envs=2, seed=41, card_pool="all") as fast_sim:
        cfg = fast_sim.effective_config()
        assert cfg["mode"] == "fast"
        assert cfg["runtime_mode"] == "speed"
        assert cfg["legal_repr"] == "ids_u16"
        assert cfg["obs_dtype"] == "i16"
        assert cfg["ids_safety"] == "checked"

    with weiss_sim.make(mode="inspect", num_envs=2, seed=41, card_pool="all") as inspect_sim:
        cfg = inspect_sim.effective_config()
        assert cfg["mode"] == "inspect"
        assert cfg["runtime_mode"] == "eval_debug"
        assert cfg["legal_repr"] == "both"
        assert cfg["obs_dtype"] == "i32"
        assert cfg["ids_safety"] is None

    with pytest.raises(TypeError, match="unexpected keyword argument 'runtime_mode'"):
        weiss_sim.make(runtime_mode="eval_debug")


def test_removed_legacy_entrypoints_and_weissenv_exported():
    for legacy in ("create", "train", "evaluate"):
        assert not hasattr(weiss_sim, legacy)
        assert not hasattr(api_mod, legacy)
    assert weiss_sim.WeissEnv is runner_mod.WeissEnv


def test_enable_replay_sampling_forwards_to_pool():
    calls: list[dict[str, object]] = []

    class _DummyPool:
        envs_len = 2
        action_space = 8

        def enable_replay_sampling(
            self,
            sample_rate: float,
            out_dir: str | None = None,
            compress: bool = False,
            include_trigger_card_id: bool = False,
            visibility_mode: str | None = None,
            store_actions: bool = True,
        ) -> None:
            calls.append(
                {
                    "sample_rate": sample_rate,
                    "out_dir": out_dir,
                    "compress": compress,
                    "include_trigger_card_id": include_trigger_card_id,
                    "visibility_mode": visibility_mode,
                    "store_actions": store_actions,
                }
            )

    sim = runner_mod.WeissEnv(
        pool=_DummyPool(),
        out=object(),
        reset_method="reset_into",
        step_method="step_into",
        has_mask=True,
        embedded_legal_ids=False,
        legal_repr="mask_u8",
        ids_safety=None,
        runtime_mode="eval_debug",
        control_seat=None,
        effective={"mode": "inspect"},
        spec_fn=lambda: {"spec_hash": int(weiss_sim.SPEC_HASH)},
    )

    sim.enable_replay_sampling(
        sample_rate=0.25,
        out_dir="tmp/replays",
        compress=True,
        include_trigger_card_id=True,
        visibility_mode="full",
        store_actions=False,
    )
    assert calls == [
        {
            "sample_rate": 0.25,
            "out_dir": "tmp/replays",
            "compress": True,
            "include_trigger_card_id": True,
            "visibility_mode": "full",
            "store_actions": False,
        }
    ]

    sim.close()
    with pytest.raises(weiss_sim.WeissSimError, match="closed"):
        sim.enable_replay_sampling(sample_rate=0.10)


@pytest.mark.parametrize(("policy",), [("raise",), ("replace",), ("terminate",)])
def test_error_policy_mapping(policy: str):
    with weiss_sim.make(num_envs=2, seed=77, card_pool="all", error_policy=policy) as sim:
        cfg = sim.effective_config()
        assert cfg["error_policy"] == policy


def test_error_policy_unknown_value_rejected():
    with pytest.raises(weiss_sim.ConfigConflictError, match="error_policy must be one of"):
        weiss_sim.make(num_envs=2, seed=77, card_pool="all", error_policy="strict")


def test_make_seed_none_entropy_and_explicit_seed_determinism():
    with (
        weiss_sim.make(num_envs=2, seed=None, card_pool="all") as entropy_a,
        weiss_sim.make(num_envs=2, seed=None, card_pool="all") as entropy_b,
    ):
        cfg_a = entropy_a.effective_config()
        cfg_b = entropy_b.effective_config()
        assert cfg_a["seed_source"] == "entropy"
        assert cfg_b["seed_source"] == "entropy"
        assert int(cfg_a["seed"]) != int(cfg_b["seed"])

    with (
        weiss_sim.make(num_envs=2, seed=123, card_pool="all") as det_a,
        weiss_sim.make(num_envs=2, seed=123, card_pool="all") as det_b,
    ):
        cfg_a = det_a.effective_config()
        cfg_b = det_b.effective_config()
        assert cfg_a["seed_source"] == "user"
        assert cfg_b["seed_source"] == "user"
        assert int(cfg_a["seed"]) == 123
        assert int(cfg_b["seed"]) == 123

        reset_a = det_a.reset()
        reset_b = det_b.reset()
        _assert_reset_batches_equal(reset_a, reset_b)

        actions_a = _first_legal_actions(reset_a, 2)
        actions_b = _first_legal_actions(reset_b, 2)
        assert np.array_equal(actions_a, actions_b)

        step_a = det_a.step(actions_a)
        step_b = det_b.step(actions_b)
        _assert_step_batches_equal(step_a, step_b)


def test_reset_seed_none_keeps_rng_state():
    with (
        weiss_sim.make(num_envs=2, seed=555, card_pool="all") as sim_a,
        weiss_sim.make(num_envs=2, seed=555, card_pool="all") as sim_b,
    ):
        reset_a = sim_a.reset(seed=None)
        reset_b = sim_b.reset()
        _assert_reset_batches_equal(reset_a, reset_b)

        step_a = sim_a.step(_first_legal_actions(reset_a, 2))
        step_b = sim_b.step(_first_legal_actions(reset_b, 2))
        _assert_step_batches_equal(step_a, step_b)

        followup_a = sim_a.reset(seed=None)
        followup_b = sim_b.reset()
        _assert_reset_batches_equal(followup_a, followup_b)


@pytest.mark.parametrize(("factory_name", "expects_mask"), [("fast", False), ("inspect", True)])
def test_batch_legal_properties_and_legal_view_behavior(factory_name: str, expects_mask: bool):
    factory = getattr(weiss_sim, factory_name)
    with factory(num_envs=2, seed=123, card_pool="all") as sim:
        reset = sim.reset()
        legal = reset.legal
        assert sim.latest_batch is reset
        assert sim.legal is legal
        assert reset.legal is legal

        assert reset.legal_ids is not None
        assert reset.legal_offsets is not None
        assert legal.legal_ids is reset.legal_ids
        assert legal.legal_offsets is reset.legal_offsets

        if expects_mask:
            assert reset.legal_mask is not None
            assert legal.mask is reset.legal_mask
        else:
            assert reset.legal_mask is None
            dense = legal.mask
            assert dense is not None
            assert dense.shape[0] == 2

        for i in range(2):
            start = int(reset.legal_offsets[i])
            end = int(reset.legal_offsets[i + 1])
            expected_ids = reset.legal_ids[start:end]
            assert np.array_equal(legal.ids(i), expected_ids)
            if expected_ids.size:
                assert legal.contains(i, int(expected_ids[0]))
        assert np.array_equal(legal.first_legal(), _first_legal_actions(reset, 2))

        step = sim.step(_first_legal_actions(reset, 2))
        assert sim.latest_batch is step
        assert sim.legal is step.legal
        assert sim.legal is not legal
        assert step.legal_ids is not None
        assert step.legal_offsets is not None
        if expects_mask:
            assert step.legal_mask is not None
        else:
            assert step.legal_mask is None


def test_step_argmax_and_sample_logits_fast_path():
    with weiss_sim.make(mode="inspect", num_envs=4, seed=321, card_pool="all") as sim:
        batch = sim.reset()
        logits = np.random.default_rng(11).standard_normal(
            (sim.num_envs, sim.action_space_n), dtype=np.float32
        )
        step_select, actions_select = sim.step_argmax_logits(logits)
        assert actions_select.shape == (4,)
        for i in range(4):
            assert batch.legal.contains(i, int(actions_select[i]))

        logits_2 = np.random.default_rng(12).standard_normal(
            (sim.num_envs, sim.action_space_n), dtype=np.float32
        )
        step_sample, actions_sample = sim.step_sample_logits(logits_2, seed=99)
        assert actions_sample.shape == (4,)
        for i in range(4):
            assert step_select.legal.contains(i, int(actions_sample[i])) or bool(
                step_select.done[i]
            )


def test_step_first_legal_and_step_uniform_legal_helpers():
    with weiss_sim.make(mode="inspect", num_envs=4, seed=111, card_pool="all") as sim:
        reset = sim.reset()
        expected_first = reset.legal.first_legal()
        step_first, actions_first = sim.step_first_legal()
        assert np.array_equal(actions_first, expected_first)
        assert sim.latest_batch is step_first

        legal_ids_before_random = [
            np.asarray(step_first.legal.ids(env_i), dtype=np.uint32).copy()
            for env_i in range(sim.num_envs)
        ]
        step_random, actions_random = sim.step_uniform_legal(seed=19)
        assert actions_random.shape == (4,)
        for env_i in range(4):
            ids = legal_ids_before_random[env_i]
            if ids.size == 0:
                assert int(actions_random[env_i]) == int(weiss_sim.PASS_ACTION_ID)
                continue
            assert np.any(ids == np.uint32(actions_random[env_i]))
        assert sim.latest_batch is step_random


def test_legal_actions_first_helper_and_default_action():
    with weiss_sim.make(mode="inspect", num_envs=2, seed=222, card_pool="all") as sim:
        reset = sim.reset()
        first = reset.legal.first_legal()
        assert first.shape == (2,)
        for i in range(2):
            ids_i = reset.legal.ids(i)
            expected = weiss_sim.PASS_ACTION_ID if ids_i.size == 0 else int(ids_i[0])
            assert int(first[i]) == expected

    empty = weiss_sim.LegalActions(
        legal_ids=np.array([], dtype=np.uint16),
        legal_offsets=np.array([0, 0], dtype=np.uint32),
        legal_mask_raw=None,
    )
    assert np.array_equal(
        empty.first_legal(),
        np.array([weiss_sim.PASS_ACTION_ID], dtype=np.uint32),
    )
    assert np.array_equal(empty.first_legal(default_action=17), np.array([17], dtype=np.uint32))
    assert np.array_equal(empty.choose("first", default_action=17), np.array([17], dtype=np.uint32))


def test_legal_actions_choose_helper():
    with weiss_sim.make(mode="inspect", num_envs=2, seed=333, card_pool="all") as sim:
        reset = sim.reset()
        legal = reset.legal
        assert np.array_equal(legal.choose("first"), legal.first_legal())
        assert np.array_equal(legal.choose("uniform", seed=7), legal.sample_uniform(seed=7))

        logits = np.random.default_rng(33).standard_normal(
            (sim.num_envs, sim.action_space_n), dtype=np.float32
        )
        assert np.array_equal(legal.choose("argmax", logits=logits), legal.argmax_logits(logits))
        assert np.array_equal(
            legal.choose("sample", logits=logits, seed=9),
            legal.sample_logits(logits, seed=9),
        )
        expected_argmax = legal.argmax_logits(logits)
        assert np.array_equal(
            legal.sample_logits(logits, seed=9, temperature=0.0),
            expected_argmax,
        )
        assert np.array_equal(
            legal.sample_logits(logits, seed=999, temperature=0.0),
            expected_argmax,
        )
        assert np.array_equal(
            legal.choose("sample", logits=logits, seed=9, temperature=0.0),
            expected_argmax,
        )
        assert np.array_equal(
            legal.choose("sample", logits=logits, seed=12345, temperature=0.0),
            expected_argmax,
        )
        with pytest.raises(ValueError, match="temperature must be >= 0"):
            legal.sample_logits(logits, temperature=-0.001)

        with pytest.raises(ValueError, match="logits is required"):
            legal.choose("argmax")
        with pytest.raises(ValueError, match="strategy must be one of"):
            legal.choose("unknown")

    empty = weiss_sim.LegalActions(
        legal_ids=np.array([], dtype=np.uint16),
        legal_offsets=np.array([0, 0], dtype=np.uint32),
        legal_mask_raw=None,
    )
    logits = np.zeros((1, 4), dtype=np.float32)
    assert np.array_equal(
        empty.choose("argmax", logits=logits, default_action=17),
        np.array([17], dtype=np.uint32),
    )
    assert np.array_equal(
        empty.choose("sample", logits=logits, seed=1, default_action=17),
        np.array([17], dtype=np.uint32),
    )


def test_step_batch_needs_reset_property():
    step = weiss_sim.StepBatch(
        obs=np.zeros((2, 1), dtype=np.int32),
        to_play_seat=np.array([0, 1], dtype=np.int8),
        starting_seat=np.array([0, 1], dtype=np.uint8),
        episode_seed=np.array([1, 2], dtype=np.uint64),
        episode_index=np.array([0, 0], dtype=np.uint32),
        env_index=np.array([0, 1], dtype=np.uint32),
        episode_key=np.array([11, 22], dtype=np.uint64),
        decision_id=np.array([5, 6], dtype=np.uint32),
        engine_status=np.array([0, 7], dtype=np.uint8),
        spec_hash=np.array([weiss_sim.SPEC_HASH, weiss_sim.SPEC_HASH], dtype=np.uint64),
        reward=np.zeros((2,), dtype=np.float32),
        terminated=np.array([True, False], dtype=np.bool_),
        truncated=np.array([False, False], dtype=np.bool_),
        terminal_during_internal_opponent=np.array([False, False], dtype=np.bool_),
        decision_count=np.array([1, 1], dtype=np.uint32),
        tick_count=np.array([2, 2], dtype=np.uint32),
        no_progress_count=np.array([0, 0], dtype=np.uint32),
        main_move_action=np.array([False, True], dtype=np.bool_),
        main_pass_action=np.array([True, False], dtype=np.bool_),
    )
    assert np.array_equal(step.done, np.array([True, False], dtype=np.bool_))
    assert np.array_equal(step.needs_reset, np.array([True, True], dtype=np.bool_))
    assert np.array_equal(step.done_indices, np.array([0], dtype=np.int64))
    assert np.array_equal(step.error_indices, np.array([1], dtype=np.int64))
    assert np.array_equal(step.needs_reset_indices, np.array([0, 1], dtype=np.int64))


def test_step_auto_helper_actions_and_resets():
    with weiss_sim.make(mode="inspect", num_envs=4, seed=444, card_pool="all") as sim:
        reset = sim.reset()
        expected = reset.legal.first_legal()
        step, actions, reset_batch = sim.step_auto(policy="first")
        assert np.array_equal(actions, expected)
        assert isinstance(step, weiss_sim.StepBatch)
        assert reset_batch is None
        assert sim.latest_batch is not step
        assert np.array_equal(sim.latest_batch.obs, step.obs)

        manual_actions = step.legal.first_legal()
        step_manual, actions_manual, _ = sim.step_auto(
            actions=manual_actions,
            policy="random",
            seed=99,
        )
        assert np.array_equal(actions_manual, manual_actions)
        assert isinstance(step_manual, weiss_sim.StepBatch)

    with weiss_sim.make(
        mode="inspect",
        num_envs=4,
        seed=445,
        error_policy="terminate",
        card_pool="all",
    ) as sim:
        sim.reset()
        invalid_actions = np.full((4,), weiss_sim.ACTION_SPACE_SIZE + 7, dtype=np.uint32)
        saw_terminal = False
        for _ in range(16):
            step, _, reset_batch = sim.step_auto(actions=invalid_actions)
            if np.any(step.done):
                saw_terminal = True
                assert reset_batch is not None
                assert sim.latest_batch is reset_batch
                break
        assert saw_terminal

    with weiss_sim.make(
        mode="inspect",
        num_envs=4,
        seed=446,
        error_policy="terminate",
        card_pool="all",
    ) as sim:
        sim.reset()
        invalid_actions = np.full((4,), weiss_sim.ACTION_SPACE_SIZE + 11, dtype=np.uint32)
        saw_terminal = False
        for _ in range(16):
            step, _, reset_batch = sim.step_auto(
                actions=invalid_actions,
                reset_done=False,
                reset_engine_errors=False,
            )
            if np.any(step.done):
                saw_terminal = True
                assert reset_batch is None
                assert sim.latest_batch is not step
                assert np.array_equal(sim.latest_batch.obs, step.obs)
                break
        assert saw_terminal


def test_rollout_helper_with_policy_and_callback():
    with weiss_sim.make(mode="inspect", num_envs=2, seed=447, card_pool="all") as sim:
        sim.reset()
        steps = sim.rollout(steps=3, policy="first")
        assert len(steps) == 3
        assert all(isinstance(step, weiss_sim.StepBatch) for step in steps)
        assert sim.latest_batch is not steps[-1]
        assert np.array_equal(sim.latest_batch.obs, steps[-1].obs)

    calls = 0

    def callback(batch):
        nonlocal calls
        calls += 1
        return batch.legal.first_legal()

    with weiss_sim.make(mode="inspect", num_envs=2, seed=448, card_pool="all") as sim:
        sim.reset()
        steps = sim.rollout(steps=2, policy=callback)
        assert len(steps) == 2
        assert calls == 2

    with weiss_sim.make(
        mode="inspect",
        num_envs=2,
        seed=449,
        max_decisions=1,
        card_pool="all",
    ) as sim:
        sim.reset()
        steps = sim.rollout(steps=3, policy="first", auto_reset=True)
        assert len(steps) == 3
        assert not isinstance(sim.latest_batch, weiss_sim.StepBatch)


def test_render_smoke():
    with weiss_sim.make(mode="inspect", num_envs=1, seed=7, card_pool="all") as sim:
        sim.reset()
        rendered = sim.render()
        assert isinstance(rendered, str)
        assert rendered
        assert "WeissEnv[0]" in rendered


def test_decode_action_smoke():
    with weiss_sim.make(mode="inspect", num_envs=1, seed=8, card_pool="all") as sim:
        batch = sim.reset()
        legal = batch.legal.ids(0)
        action_id = int(legal[0]) if legal.size else int(weiss_sim.PASS_ACTION_ID)
        desc = sim.decode_action(action_id)
        assert isinstance(desc, dict)
        assert "family" in desc
        assert "params" in desc
        assert isinstance(desc["params"], list)


def test_reset_done_and_reset_indices_partial_helpers():
    with weiss_sim.make(mode="inspect", num_envs=4, seed=404, card_pool="all") as sim:
        reset = sim.reset()
        step = sim.step(_first_legal_actions(reset, 4))

        done_mask = np.asarray(step.done, dtype=np.bool_)
        if not done_mask.any():
            done_mask = np.array([True, False, False, False], dtype=np.bool_)
        reset_done = sim.reset_done(done_mask)
        assert reset_done.obs.shape == (4, weiss_sim.OBS_LEN)
        assert sim.latest_batch is reset_done

        reset_indices = sim.reset_indices([1, 3])
        assert reset_indices.obs.shape == (4, weiss_sim.OBS_LEN)
        assert sim.latest_batch is reset_indices


def test_as_single_env_adapter_contract():
    with weiss_sim.make(mode="inspect", num_envs=1, seed=7, card_pool="all").as_single_env() as env:
        obs = env.reset()
        assert obs.shape == (weiss_sim.OBS_LEN,)

        action = env.legal.sample_uniform(seed=5)
        assert isinstance(action, int)
        assert env.legal.contains(action)

        obs2, reward, terminated, truncated, info = env.step(action)
        assert obs2.shape == (weiss_sim.OBS_LEN,)
        assert isinstance(reward, float)
        assert isinstance(terminated, bool)
        assert isinstance(truncated, bool)
        assert "legal_ids" in info


def test_as_single_env_requires_num_envs_one():
    with weiss_sim.make(mode="inspect", num_envs=2, seed=7, card_pool="all") as env:
        with pytest.raises(weiss_sim.WeissSimError, match="num_envs == 1"):
            env.as_single_env()


@pytest.mark.parametrize("mode", ["inspect", "fast"])
def test_as_gym_adapter_contract_if_installed(mode: str):
    if importlib.util.find_spec("gymnasium") is None and importlib.util.find_spec("gym") is None:
        pytest.skip("gymnasium/gym not installed")

    with weiss_sim.make(mode=mode, num_envs=2, seed=42, card_pool="all") as env:
        gym_env = env.as_gym()
        obs, info = gym_env.reset(seed=42)
        assert obs.shape == (2, weiss_sim.OBS_LEN)
        assert "to_play_seat" in info

        masks = gym_env.action_masks()
        assert masks is not None
        assert masks.shape == (2, weiss_sim.ACTION_SPACE_SIZE)

        actions = env.legal.sample_uniform(seed=9)
        obs2, reward, terminated, truncated, info2 = gym_env.step(actions)
        assert obs2.shape == (2, weiss_sim.OBS_LEN)
        assert reward.shape == (2,)
        assert terminated.shape == (2,)
        assert truncated.shape == (2,)
        assert "decision_id" in info2


def test_legal_actions_mask_recomputes_after_buffer_mutation():
    legal = weiss_sim.LegalActions(
        legal_ids=np.array([0, 2], dtype=np.uint16),
        legal_offsets=np.array([0, 2], dtype=np.uint32),
        legal_mask_raw=None,
    )

    mask_before = legal.mask
    assert mask_before is not None
    assert np.array_equal(np.flatnonzero(mask_before[0]), np.array([0, 2]))

    legal.legal_ids[:] = np.array([1, 3], dtype=np.uint16)
    mask_after = legal.mask
    assert mask_after is not None
    assert np.array_equal(np.flatnonzero(mask_after[0]), np.array([1, 3]))


def test_termination_truncation_exclusive_and_timeout_reward_zero():
    with weiss_sim.make(
        mode="inspect",
        num_envs=4,
        seed=7,
        max_decisions=1,
        card_pool="all",
    ) as sim:
        reset = sim.reset()
        step = sim.step(_first_legal_actions(reset, 4))
        assert not np.any(np.logical_and(step.terminated, step.truncated))
        if np.any(step.truncated):
            assert np.allclose(step.reward[step.truncated], 0.0)


def test_strict_profile_conflicting_curriculum_rejected():
    with pytest.raises(weiss_sim.ConfigConflictError):
        weiss_sim.make(
            deck="preset:starter_v1",
            rules_profile="strict",
            curriculum={"enable_approx_effects": True},
        )


def test_ids_safety_only_allowed_with_ids_u16():
    with pytest.raises(weiss_sim.ConfigConflictError):
        weiss_sim.make(
            legal_repr="both",
            ids_safety="checked",
            card_pool="all",
        )


def test_parsed_only_rejects_mismatched_external_db(tmp_path: Path):
    default_wsdb_path = (
        Path(__file__).resolve().parents[1] / "weiss_sim" / "data" / "default_cards.wsdb"
    )
    bad_wsdb = tmp_path / "bad.wsdb"
    bad_wsdb.write_bytes(default_wsdb_path.read_bytes() + b"\x00")
    with pytest.raises(weiss_sim.DbMismatchError) as exc_info:
        weiss_sim.make(
            deck="preset:starter_v1",
            db_path=str(bad_wsdb),
            card_pool="parsed_only",
        )
    err = exc_info.value
    assert err.expected_db_sha256
    assert err.actual_db_sha256
    assert err.expected_db_sha256 != err.actual_db_sha256
    assert "Remediation:" in str(err)


def test_cards_namespace_search_get_presets_and_resolve_deck(tmp_path: Path):
    card = weiss_sim.cards.get(1)
    assert card.id == 1
    assert isinstance(card.card_no, str)

    by_card_no = weiss_sim.cards.get(card.card_no)
    assert by_card_no.id == card.id

    results = weiss_sim.cards.search(card.card_no, limit=5)
    assert results
    assert any(r.id == card.id for r in results)

    names = weiss_sim.cards.presets()
    assert "starter_v1" in names

    seq_ids = weiss_sim.cards.resolve_deck(
        [1] * 50,
        rules_profile="approx",
        card_pool="all",
    )
    assert seq_ids == [1] * 50

    map_ids = weiss_sim.cards.resolve_deck(
        {"1": 50},
        rules_profile="approx",
        card_pool="all",
    )
    assert map_ids == [1] * 50

    preset_ids = weiss_sim.cards.resolve_deck(
        "starter_v1",
        rules_profile="approx",
        card_pool="all",
    )
    assert len(preset_ids) == 50

    file_path = tmp_path / "deck.json"
    file_path.write_text(json.dumps({"1": 50}), encoding="utf-8")
    file_ids = weiss_sim.cards.resolve_deck(
        f"file:{file_path}",
        rules_profile="approx",
        card_pool="all",
    )
    assert file_ids == [1] * 50

    inferred_file_ids = weiss_sim.cards.resolve_deck(
        str(file_path),
        rules_profile="approx",
        card_pool="all",
    )
    assert inferred_file_ids == [1] * 50

    details = weiss_sim.cards.describe_deck(
        {"1": 50},
        rules_profile="approx",
        card_pool="all",
    )
    assert details["ids"] == [1] * 50
    assert len(details["cards"]) == 50
    first_card = details["cards"][0]
    assert first_card["id"] == 1
    assert isinstance(first_card["card_no"], str)
    assert details["counts"] == [
        {
            "id": 1,
            "card_no": first_card["card_no"],
            "name": first_card["name"],
            "card_type": first_card["card_type"],
            "card_set": first_card["card_set"],
            "strict_ok": first_card["strict_ok"],
            "approx_ok": first_card["approx_ok"],
            "count": 50,
        }
    ]

    suggestions = weiss_sim.cards.suggest(f"{card.card_no}-typo", limit=3)
    assert suggestions

    report = weiss_sim.cards.validate_deck(
        "starter_v1",
        rules_profile="approx",
        card_pool="all",
    )
    assert report.ok
    assert not report.errors
    assert len(report.resolved_ids) == 50

    builder = weiss_sim.cards.builder(initial="starter_v1")
    assert isinstance(builder, weiss_sim.DeckBuilder)
    assert builder.total_cards() == 50

    exported = weiss_sim.cards.export_deck(
        "starter_v1",
        rules_profile="approx",
        card_pool="all",
    )
    assert exported["format"] == "wsim_deck_v1"

    deck_path = tmp_path / "starter_export.json"
    saved_path = weiss_sim.cards.save_deck(
        deck_path,
        "starter_v1",
        rules_profile="approx",
        card_pool="all",
    )
    assert Path(saved_path).exists()
    loaded_payload = weiss_sim.cards.load_deck(deck_path)
    loaded_ids = weiss_sim.cards.resolve_deck(
        loaded_payload,
        rules_profile="approx",
        card_pool="all",
    )
    assert len(loaded_ids) == 50


def test_deck_resolve_rejects_invalid_length_and_unknown_id():
    with pytest.raises(weiss_sim.DeckValidationError):
        weiss_sim.cards.resolve_deck(
            [1] * 49,
            rules_profile="approx",
            card_pool="all",
        )

    with pytest.raises(weiss_sim.DeckValidationError):
        weiss_sim.cards.resolve_deck(
            [99999999] * 50,
            rules_profile="approx",
            card_pool="all",
        )

    with pytest.raises(weiss_sim.DeckValidationError):
        weiss_sim.cards.resolve_deck(
            [17] + [1] * 49,
            rules_profile="approx",
            card_pool="all",
        )


def test_deck_resolve_db_probe_does_not_depend_on_starter_cards(monkeypatch, tmp_path: Path):
    default_wsdb_path = (
        Path(__file__).resolve().parents[1] / "weiss_sim" / "data" / "default_cards.wsdb"
    )
    custom_wsdb = tmp_path / "custom.wsdb"
    custom_wsdb.write_bytes(default_wsdb_path.read_bytes())

    def fake_validate_deck_issues(
        *,
        db_path=None,
        deck_lists=None,
        **_kwargs,
    ):
        assert db_path == str(custom_wsdb)
        assert deck_lists is not None
        return []

    monkeypatch.setattr(decks_mod.EnvPool, "validate_deck_issues", fake_validate_deck_issues)

    deck = [17] + [1] * 49
    resolved = decks_mod.resolve_deck(
        deck,
        rules_profile="approx",
        card_pool="all",
        db_path=str(custom_wsdb),
    )
    assert resolved == deck


def test_deck_resolve_db_probe_surfaces_non_membership_errors(monkeypatch):
    def fake_validate_deck_issues(**_kwargs):
        raise RuntimeError("Failed to decode card db payload")

    monkeypatch.setattr(decks_mod.EnvPool, "validate_deck_issues", fake_validate_deck_issues)

    with pytest.raises(
        weiss_sim.DeckValidationError, match="failed to validate deck against selected DB"
    ):
        decks_mod.resolve_deck(
            [1] * 50,
            rules_profile="approx",
            card_pool="all",
        )


def test_auto_sizing_deterministic(monkeypatch):
    monkeypatch.setattr(api_mod.os, "cpu_count", lambda: 20)
    with weiss_sim.fast(num_envs="auto", num_threads="auto", card_pool="all") as sim:
        cfg = sim.effective_config()
        assert int(cfg["num_threads"]) == 16
        assert int(cfg["num_envs"]) == 64

    with weiss_sim.fast(num_envs=5, num_threads="auto", card_pool="all") as sim:
        cfg = sim.effective_config()
        assert int(cfg["num_threads"]) == 5
        assert int(cfg["num_envs"]) == 5


def test_effective_config_and_spec_export_contract():
    with weiss_sim.inspect(num_envs=2, seed=99, card_pool="all") as sim:
        cfg = sim.effective_config()
        assert cfg["mode"] == "inspect"
        assert cfg["runtime_mode"] == "eval_debug"
        assert cfg["rules_profile"] == "strict"
        assert cfg["card_pool"] == "all"
        assert cfg["legal_repr"] == "both"
        assert cfg["obs_dtype"] == "i32"
        assert isinstance(cfg["curriculum"], dict)
        assert cfg["curriculum"]["enable_approx_effects"] is False
        assert isinstance(cfg["db"], dict)
        assert {"db_sha256", "catalog_db_sha256", "matches_catalog"} <= set(cfg["db"].keys())
        assert cfg["reward_timeout_policy"]["timeout_uses_terminal_draw_reward"] is False
        assert cfg["reward_timeout_policy"]["timeout_uses_terminal_timeout_reward"] is True
        assert cfg["reward_timeout_policy"]["terminal_timeout_effective_value"] == 0.0
        assert set(cfg["resolved_decks"].keys()) == {"player", "opponent"}
        for seat in ("player", "opponent"):
            deck_info = cfg["resolved_decks"][seat]
            assert len(deck_info["ids"]) == 50
            assert len(deck_info["cards"]) == 50
            assert isinstance(deck_info["counts"], list)
        assert cfg["spec_hash"] == int(weiss_sim.SPEC_HASH)
        assert cfg["end_condition_policy"] == {
            "simultaneous_loss": "Draw",
            "allow_draw_on_simultaneous_loss": True,
        }

        spec = sim.spec()
        exported = weiss_sim.export_spec_bundle()
        assert spec["spec_hash"] == exported["spec_hash"] == weiss_sim.SPEC_HASH
        assert "observation" in spec and "action" in spec

    info = weiss_sim.db_info()
    assert bool(info["matches_catalog"]) is True


def test_observation_visibility_default_and_override():
    with weiss_sim.inspect(num_envs=2, seed=99, card_pool="all") as sim:
        cfg = sim.effective_config()
        assert cfg["observation_visibility"] == "public"
        assert cfg["reveal_opponent_hand_stock_counts"] is False
        assert cfg["curriculum"]["memory_is_public"] is False

    with weiss_sim.inspect(
        num_envs=2,
        seed=99,
        card_pool="all",
        observation_visibility="full",
    ) as sim:
        cfg = sim.effective_config()
        assert cfg["observation_visibility"] == "full"


def test_reveal_opponent_hand_stock_counts_top_level_override():
    with weiss_sim.inspect(
        num_envs=2,
        seed=42,
        card_pool="all",
        curriculum={"reveal_opponent_hand_stock_counts": False},
        reveal_opponent_hand_stock_counts=True,
    ) as sim:
        cfg = sim.effective_config()
        assert cfg["reveal_opponent_hand_stock_counts"] is True
        assert cfg["curriculum"]["reveal_opponent_hand_stock_counts"] is True


def test_reward_json_dict_supported_in_high_level_api():
    reward_cfg = {
        "terminal_win": 2.0,
        "terminal_loss": -2.0,
        "terminal_draw": 0.0,
        "enable_shaping": True,
        "damage_reward": 0.05,
    }
    with weiss_sim.inspect(num_envs=2, seed=17, card_pool="all", reward_json=reward_cfg) as sim:
        cfg = sim.effective_config()
        assert cfg["reward"] == {
            "terminal_win": 2.0,
            "terminal_loss": -2.0,
            "terminal_draw": 0.0,
            "terminal_timeout": 0.0,
            "enable_shaping": True,
            "damage_reward": 0.05,
            "level_reward": 0.0,
            "board_reward": 0.0,
            "no_progress_penalty": 0.0,
        }


def test_reward_json_empty_string_rejected():
    with pytest.raises(weiss_sim.ConfigConflictError):
        weiss_sim.make(card_pool="all", reward_json="")


def test_end_condition_policy_override_supported_in_high_level_api():
    with weiss_sim.inspect(
        num_envs=2,
        seed=88,
        card_pool="all",
        end_condition_policy={
            "simultaneous_loss": "non_active_player_wins",
            "allow_draw_on_simultaneous_loss": False,
        },
    ) as sim:
        cfg = sim.effective_config()
        assert cfg["end_condition_policy"] == {
            "simultaneous_loss": "NonActivePlayerWins",
            "allow_draw_on_simultaneous_loss": False,
        }


def test_end_condition_policy_invalid_simultaneous_loss_rejected():
    with pytest.raises(weiss_sim.ConfigConflictError):
        weiss_sim.make(
            card_pool="all",
            end_condition_policy={"simultaneous_loss": "who_knows"},
        )


def test_typed_override_dataclasses_supported():
    with weiss_sim.inspect(
        num_envs=2,
        seed=91,
        card_pool="all",
        curriculum=weiss_sim.CurriculumOverrides(
            memory_is_public=True,
            reveal_opponent_hand_stock_counts=True,
        ),
        end_condition_policy=weiss_sim.EndConditionOverrides(
            simultaneous_loss="active_player_wins",
            allow_draw_on_simultaneous_loss=False,
        ),
    ) as sim:
        cfg = sim.effective_config()
        assert cfg["curriculum"]["memory_is_public"] is True
        assert cfg["curriculum"]["reveal_opponent_hand_stock_counts"] is True
        assert cfg["end_condition_policy"] == {
            "simultaneous_loss": "ActivePlayerWins",
            "allow_draw_on_simultaneous_loss": False,
        }


def test_seat_action_helpers_for_switching_and_manual_control():
    with weiss_sim.inspect(num_envs=4, seed=123, card_pool="all") as sim:
        reset = sim.reset()
        assert np.array_equal(sim.current_to_play_seat(), reset.to_play_seat)

        legal = _first_legal_actions(reset, 4)
        seat0_actions = np.full((4,), weiss_sim.PASS_ACTION_ID, dtype=np.uint32)
        seat1_actions = np.full((4,), weiss_sim.PASS_ACTION_ID, dtype=np.uint32)
        seat0_mask = reset.to_play_seat == 0
        seat1_mask = reset.to_play_seat == 1
        seat0_actions[seat0_mask] = legal[seat0_mask]
        seat1_actions[seat1_mask] = legal[seat1_mask]

        merged = sim.merge_actions_by_seat(seat0_actions, seat1_actions)
        assert np.array_equal(merged, legal)

        step = sim.step_by_seat(seat0_actions, seat1_actions)
        _assert_common_step_contract(step, num_envs=4, obs_dtype=np.int32)
        assert np.array_equal(sim.current_to_play_seat(), step.to_play_seat)


def test_legal_id_contract_checked_every_step_in_eval_debug(monkeypatch):
    call_count = 0

    def wrapped(ids, offsets, num_envs, action_space):
        nonlocal call_count
        call_count += 1
        return None

    monkeypatch.setattr(runner_mod, "_validate_legal_ids_contract", wrapped)

    with weiss_sim.inspect(num_envs=2, seed=123, card_pool="all") as sim:
        reset = sim.reset()
        assert call_count == 1
        step_1 = sim.step(_first_legal_actions(reset, 2))
        assert call_count == 2
        sim.step(_first_legal_actions(step_1, 2))
        assert call_count == 3


def test_legal_id_contract_spotcheck_in_speed(monkeypatch):
    call_count = 0

    def wrapped(ids, offsets, num_envs, action_space):
        nonlocal call_count
        call_count += 1
        return None

    monkeypatch.setattr(runner_mod, "_validate_legal_ids_contract", wrapped)

    with weiss_sim.fast(num_envs=2, seed=321, card_pool="all") as sim:
        reset = sim.reset()
        assert call_count == 1
        step_1 = sim.step(_first_legal_actions(reset, 2))
        assert call_count == 2
        step_2 = sim.step(_first_legal_actions(step_1, 2))
        assert call_count == 2
        sim._step_count = 4096
        sim.step(_first_legal_actions(step_2, 2))
        assert call_count == 3


def test_legal_id_contract_holds_across_eval_rollout():
    rng = np.random.default_rng(12345)
    with weiss_sim.inspect(num_envs=32, seed=7, card_pool="all") as sim:
        batch = sim.reset()
        assert batch.legal_ids is not None
        assert batch.legal_offsets is not None
        _assert_legal_ids_strictly_sorted(batch.legal_ids, batch.legal_offsets, 32)
        for _ in range(256):
            actions = np.empty((32,), dtype=np.uint32)
            for i in range(32):
                start = int(batch.legal_offsets[i])
                end = int(batch.legal_offsets[i + 1])
                if end <= start:
                    actions[i] = weiss_sim.PASS_ACTION_ID
                else:
                    pick = int(rng.integers(start, end))
                    actions[i] = int(batch.legal_ids[pick])
            batch = sim.step(actions)
            assert batch.legal_ids is not None
            assert batch.legal_offsets is not None
            _assert_legal_ids_strictly_sorted(batch.legal_ids, batch.legal_offsets, 32)


def test_terminal_during_internal_opponent_behavior():
    with weiss_sim.make(
        mode="inspect",
        num_envs=4,
        seed=5,
        max_decisions=1,
        control_seat=0,
        card_pool="all",
    ) as sim:
        reset = sim.reset()
        to_play_before = reset.to_play_seat.copy()
        step = sim.step(_first_legal_actions(reset, 4))
        done = np.logical_or(step.terminated, step.truncated)
        expected = np.logical_and(done, to_play_before != 0)
        assert np.array_equal(step.terminal_during_internal_opponent, expected)

    with weiss_sim.make(
        mode="inspect",
        num_envs=4,
        seed=5,
        max_decisions=1,
        control_seat=None,
        card_pool="all",
    ) as sim:
        reset = sim.reset()
        step = sim.step(_first_legal_actions(reset, 4))
        assert not np.any(step.terminal_during_internal_opponent)
