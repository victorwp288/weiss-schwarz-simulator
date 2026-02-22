from __future__ import annotations

import warnings

import numpy as np
import pytest
import weiss_sim
import weiss_sim.runner as runner_mod
import weiss_sim.types as types_mod
from tests.support import (
    assert_reset_batches_equal as _assert_reset_batches_equal,
    assert_step_batches_equal as _assert_step_batches_equal,
    first_legal_actions as _first_legal_actions,
)


def _assert_actions_compatible(legal, actions: np.ndarray) -> None:
    for env_i in range(legal.num_envs):
        ids = legal.ids(env_i)
        action = int(actions[env_i])
        if ids.size == 0:
            assert action == int(weiss_sim.PASS_ACTION_ID)
            continue
        assert legal.contains(env_i, action)


def test_step_argmax_logits_default_fast_path_compatibility():
    with (
        weiss_sim.inspect(num_envs=4, seed=1618, card_pool="all") as sim_fast,
        weiss_sim.inspect(num_envs=4, seed=1618, card_pool="all") as sim_legacy,
    ):
        reset_fast = sim_fast.reset()
        reset_legacy = sim_legacy.reset()
        _assert_reset_batches_equal(reset_fast, reset_legacy)

        logits = np.random.default_rng(17).standard_normal(
            (sim_fast.num_envs, sim_fast.action_space_n), dtype=np.float32
        )

        step_fast, actions_fast = sim_fast.step_argmax_logits(logits)
        actions_legacy = reset_legacy.legal.argmax_logits(logits, illegal_value=-1e9)
        step_legacy = sim_legacy.step(actions_legacy)

        assert np.array_equal(actions_fast, actions_legacy)
        _assert_step_batches_equal(step_fast, step_legacy)


def test_step_argmax_logits_default_fast_path_does_not_hit_legacy_argmax(monkeypatch):
    def _fail_legacy_argmax(_self, _logits, illegal_value=-1e9):
        raise AssertionError(
            "default step_argmax_logits should bypass LegalActions.argmax_logits; "
            f"got illegal_value={illegal_value}"
        )

    monkeypatch.setattr(types_mod.LegalActions, "argmax_logits", _fail_legacy_argmax)

    with weiss_sim.inspect(num_envs=4, seed=17, card_pool="all") as sim:
        reset = sim.reset()
        logits = np.random.default_rng(21).standard_normal(
            (sim.num_envs, sim.action_space_n), dtype=np.float32
        )
        _step, actions = sim.step_argmax_logits(logits)
        _assert_actions_compatible(reset.legal, actions)


def test_step_argmax_logits_non_default_illegal_value_warns_once_and_uses_compat(monkeypatch):
    calls: list[float] = []
    original_mask = runner_mod.WeissEnv._apply_illegal_value_compatibility_mask

    def _mask_with_record(self, logits, *, illegal_value):
        calls.append(float(illegal_value))
        return original_mask(self, logits, illegal_value=illegal_value)

    def _fail_legacy_argmax(_self, _logits, illegal_value=-1e9):
        raise AssertionError(
            "non-default illegal_value should use compatibility masking before Rust fast-path, "
            f"not legacy argmax_logits (illegal_value={illegal_value})"
        )

    monkeypatch.setattr(
        runner_mod.WeissEnv, "_apply_illegal_value_compatibility_mask", _mask_with_record
    )
    monkeypatch.setattr(types_mod.LegalActions, "argmax_logits", _fail_legacy_argmax)

    with weiss_sim.inspect(num_envs=3, seed=123, card_pool="all") as sim:
        sim.reset()
        logits_a = np.random.default_rng(22).standard_normal(
            (sim.num_envs, sim.action_space_n), dtype=np.float32
        )
        legal_before_a = sim.latest_batch.legal
        with warnings.catch_warnings(record=True) as caught_a:
            warnings.simplefilter("always")
            _step_a, actions_a = sim.step_argmax_logits(logits_a, illegal_value=-321.0)
        compat_warnings_a = [
            item for item in caught_a if "illegal_value" in str(item.message).lower()
        ]
        assert len(compat_warnings_a) == 1
        _assert_actions_compatible(legal_before_a, actions_a)

        logits_b = np.random.default_rng(23).standard_normal(
            (sim.num_envs, sim.action_space_n), dtype=np.float32
        )
        legal_before_b = sim.latest_batch.legal
        with warnings.catch_warnings(record=True) as caught_b:
            warnings.simplefilter("always")
            _step_b, actions_b = sim.step_argmax_logits(logits_b, illegal_value=-321.0)
        compat_warnings_b = [
            item for item in caught_b if "illegal_value" in str(item.message).lower()
        ]
        assert not compat_warnings_b
        _assert_actions_compatible(legal_before_b, actions_b)

    assert calls == [-321.0, -321.0]


def test_step_sample_logits_temperature_below_zero_rejected():
    with weiss_sim.inspect(num_envs=2, seed=9, card_pool="all") as sim:
        sim.reset()
        logits = np.random.default_rng(24).standard_normal(
            (sim.num_envs, sim.action_space_n), dtype=np.float32
        )
        with pytest.raises((ValueError, weiss_sim.WeissSimError), match="temperature"):
            sim.step_sample_logits(logits, seed=7, temperature=-0.01)


def test_step_sample_logits_temperature_zero_matches_argmax_deterministically():
    with (
        weiss_sim.inspect(num_envs=4, seed=222, card_pool="all") as sim_a,
        weiss_sim.inspect(num_envs=4, seed=222, card_pool="all") as sim_b,
        weiss_sim.inspect(num_envs=4, seed=222, card_pool="all") as sim_ref,
    ):
        reset_a = sim_a.reset()
        reset_b = sim_b.reset()
        reset_ref = sim_ref.reset()
        _assert_reset_batches_equal(reset_a, reset_b)
        _assert_reset_batches_equal(reset_a, reset_ref)

        logits = np.random.default_rng(25).standard_normal(
            (sim_a.num_envs, sim_a.action_space_n), dtype=np.float32
        )
        expected_actions = reset_ref.legal.argmax_logits(logits)

        step_a, actions_a = sim_a.step_sample_logits(logits, seed=1234, temperature=0.0)
        step_b, actions_b = sim_b.step_sample_logits(logits, seed=9876, temperature=0.0)
        step_ref = sim_ref.step(expected_actions)

        assert np.array_equal(actions_a, expected_actions)
        assert np.array_equal(actions_b, expected_actions)
        _assert_step_batches_equal(step_a, step_ref)
        _assert_step_batches_equal(step_b, step_ref)


def test_make_config_bounds_rejected():
    cases = (
        ({"num_envs": 0}, "num_envs must be"),
        ({"num_envs": -1}, "num_envs must be"),
        ({"num_threads": 0}, "num_threads must be"),
        ({"num_threads": -3}, "num_threads must be"),
        ({"max_decisions": 0}, "max_decisions and max_ticks must both be > 0"),
        ({"max_decisions": -7}, "max_decisions and max_ticks must both be > 0"),
        ({"max_ticks": 0}, "max_decisions and max_ticks must both be > 0"),
        ({"max_ticks": -11}, "max_decisions and max_ticks must both be > 0"),
    )
    for kwargs, message in cases:
        with pytest.raises(weiss_sim.ConfigConflictError, match=message):
            weiss_sim.make(mode="inspect", card_pool="all", **kwargs)


def test_auto_reset_on_engine_errors_continuity_without_errors():
    with (
        weiss_sim.inspect(num_envs=4, seed=909, card_pool="all") as sim_auto,
        weiss_sim.inspect(num_envs=4, seed=909, card_pool="all") as sim_ref,
    ):
        batch_auto = sim_auto.reset()
        batch_ref = sim_ref.reset()
        _assert_reset_batches_equal(batch_auto, batch_ref)
        assert int(sim_auto.pool.engine_error_reset_count()) == 0

        for _ in range(8):
            actions = _first_legal_actions(batch_auto)
            step_auto = sim_auto.step(actions)
            step_ref = sim_ref.step(actions)
            _assert_step_batches_equal(step_auto, step_ref)

            reset_count, reset_batch = sim_auto.auto_reset_on_engine_errors()
            assert reset_count == 0
            assert reset_batch is None
            assert int(sim_auto.pool.engine_error_reset_count()) == 0
            batch_auto = step_auto
