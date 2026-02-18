from __future__ import annotations

import json
from pathlib import Path

import numpy as np
import pytest
import weiss_sim
import weiss_sim.api as api_mod
import weiss_sim.runner as runner_mod


def _first_legal_actions_from_ids(
    legal_ids: np.ndarray, legal_offsets: np.ndarray, num_envs: int
) -> np.ndarray:
    actions = np.empty((num_envs,), dtype=np.uint32)
    for i in range(num_envs):
        start = int(legal_offsets[i])
        end = int(legal_offsets[i + 1])
        if end <= start:
            actions[i] = weiss_sim.PASS_ACTION_ID
        else:
            actions[i] = int(legal_ids[start])
    return actions


def _first_legal_actions(batch, num_envs: int) -> np.ndarray:
    if batch.legal_ids is not None and batch.legal_offsets is not None:
        return _first_legal_actions_from_ids(batch.legal_ids, batch.legal_offsets, num_envs)
    if batch.legal_mask is not None:
        actions = np.empty((num_envs,), dtype=np.uint32)
        for i in range(num_envs):
            legal = np.flatnonzero(batch.legal_mask[i])
            actions[i] = weiss_sim.PASS_ACTION_ID if legal.size == 0 else int(legal[0])
        return actions
    raise AssertionError("batch does not expose legal ids or legal mask")


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


@pytest.mark.parametrize(
    ("legal_repr", "obs_dtype", "expects_mask", "expects_ids", "ids_dtype"),
    [
        ("ids_u16", "i16", False, True, np.uint16),
        ("ids_u32", "i32", False, True, np.uint32),
        ("mask_u8", "i32", True, False, None),
        ("both", "i32", True, True, np.uint32),
    ],
)
def test_create_output_contract_by_legal_repr(
    legal_repr, obs_dtype, expects_mask, expects_ids, ids_dtype
):
    sim = weiss_sim.create(
        runtime_mode="eval_debug",
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


def test_train_and_evaluate_zero_config_reset_step_contract():
    with weiss_sim.train(num_envs=2, seed=123) as train_sim:
        train_reset = train_sim.reset()
        train_actions = _first_legal_actions(train_reset, 2)
        train_step = train_sim.step(train_actions)
        _assert_common_step_contract(train_step, num_envs=2, obs_dtype=np.int16)

    with weiss_sim.evaluate(num_envs=2, seed=123) as eval_sim:
        eval_reset = eval_sim.reset()
        eval_actions = _first_legal_actions(eval_reset, 2)
        eval_step = eval_sim.step(eval_actions)
        _assert_common_step_contract(eval_step, num_envs=2, obs_dtype=np.int32)
        assert eval_reset.legal_mask is not None
        assert eval_reset.legal_ids is not None
        assert eval_step.legal_mask is not None
        assert eval_step.legal_ids is not None


def test_termination_truncation_exclusive_and_timeout_reward_zero():
    with weiss_sim.create(
        runtime_mode="eval_debug",
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
        weiss_sim.create(
            deck="preset:starter_v1",
            rules_profile="strict",
            curriculum={"enable_approx_effects": True},
        )


def test_ids_safety_only_allowed_with_ids_u16():
    with pytest.raises(weiss_sim.ConfigConflictError):
        weiss_sim.create(
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
        weiss_sim.create(
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


def test_auto_sizing_deterministic(monkeypatch):
    monkeypatch.setattr(api_mod.os, "cpu_count", lambda: 20)
    with weiss_sim.train(num_envs="auto", num_threads="auto", card_pool="all") as sim:
        cfg = sim.effective_config()
        assert int(cfg["num_threads"]) == 16
        assert int(cfg["num_envs"]) == 64

    with weiss_sim.train(num_envs=5, num_threads="auto", card_pool="all") as sim:
        cfg = sim.effective_config()
        assert int(cfg["num_threads"]) == 5
        assert int(cfg["num_envs"]) == 5


def test_effective_config_and_spec_export_contract():
    with weiss_sim.evaluate(num_envs=2, seed=99, card_pool="all") as sim:
        cfg = sim.effective_config()
        assert cfg["runtime_mode"] == "eval_debug"
        assert cfg["rules_profile"] == "approx"
        assert cfg["card_pool"] == "all"
        assert cfg["legal_repr"] == "both"
        assert cfg["obs_dtype"] == "i32"
        assert isinstance(cfg["curriculum"], dict)
        assert isinstance(cfg["db"], dict)
        assert {"db_sha256", "catalog_db_sha256", "matches_catalog"} <= set(cfg["db"].keys())
        assert cfg["reward_timeout_policy"]["timeout_uses_terminal_draw_reward"] is True
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
    with weiss_sim.evaluate(num_envs=2, seed=99, card_pool="all") as sim:
        cfg = sim.effective_config()
        assert cfg["observation_visibility"] == "public"
        assert cfg["reveal_opponent_hand_stock_counts"] is False
        assert cfg["curriculum"]["memory_is_public"] is False

    with weiss_sim.evaluate(
        num_envs=2,
        seed=99,
        card_pool="all",
        observation_visibility="full",
    ) as sim:
        cfg = sim.effective_config()
        assert cfg["observation_visibility"] == "full"


def test_reveal_opponent_hand_stock_counts_top_level_override():
    with weiss_sim.evaluate(
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
    with weiss_sim.evaluate(num_envs=2, seed=17, card_pool="all", reward_json=reward_cfg) as sim:
        cfg = sim.effective_config()
        assert cfg["reward"] == reward_cfg


def test_reward_json_empty_string_rejected():
    with pytest.raises(weiss_sim.ConfigConflictError):
        weiss_sim.create(card_pool="all", reward_json="")


def test_end_condition_policy_override_supported_in_high_level_api():
    with weiss_sim.evaluate(
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
        weiss_sim.create(
            card_pool="all",
            end_condition_policy={"simultaneous_loss": "who_knows"},
        )


def test_seat_action_helpers_for_switching_and_manual_control():
    with weiss_sim.evaluate(num_envs=4, seed=123, card_pool="all") as sim:
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

    with weiss_sim.evaluate(num_envs=2, seed=123, card_pool="all") as sim:
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

    with weiss_sim.train(num_envs=2, seed=321, card_pool="all") as sim:
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
    with weiss_sim.evaluate(num_envs=32, seed=7, card_pool="all") as sim:
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
    with weiss_sim.create(
        runtime_mode="eval_debug",
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

    with weiss_sim.create(
        runtime_mode="eval_debug",
        num_envs=4,
        seed=5,
        max_decisions=1,
        control_seat=None,
        card_pool="all",
    ) as sim:
        reset = sim.reset()
        step = sim.step(_first_legal_actions(reset, 4))
        assert not np.any(step.terminal_during_internal_opponent)
