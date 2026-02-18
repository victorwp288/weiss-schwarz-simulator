from __future__ import annotations

import numpy as np
import pytest
import weiss_sim


def _dummy_step_batch(
    rewards: list[float],
    terminated: list[bool],
    truncated: list[bool],
    starting_seat: list[int] | None = None,
) -> weiss_sim.StepBatch:
    num_envs = len(rewards)
    starting = (
        np.asarray(starting_seat, dtype=np.uint8)
        if starting_seat is not None
        else np.zeros((num_envs,), dtype=np.uint8)
    )
    return weiss_sim.StepBatch(
        obs=np.zeros((num_envs, weiss_sim.OBS_LEN), dtype=np.int32),
        to_play_seat=np.zeros((num_envs,), dtype=np.int8),
        starting_seat=starting,
        episode_seed=np.arange(num_envs, dtype=np.uint64),
        episode_index=np.zeros((num_envs,), dtype=np.uint32),
        env_index=np.arange(num_envs, dtype=np.uint32),
        episode_key=np.arange(100, 100 + num_envs, dtype=np.uint64),
        decision_id=np.zeros((num_envs,), dtype=np.uint32),
        engine_status=np.zeros((num_envs,), dtype=np.uint8),
        spec_hash=np.full((num_envs,), np.uint64(weiss_sim.SPEC_HASH), dtype=np.uint64),
        reward=np.asarray(rewards, dtype=np.float32),
        terminated=np.asarray(terminated, dtype=np.bool_),
        truncated=np.asarray(truncated, dtype=np.bool_),
        terminal_during_internal_opponent=np.zeros((num_envs,), dtype=np.bool_),
        decision_count=np.arange(1, 1 + num_envs, dtype=np.uint32),
        tick_count=np.arange(10, 10 + num_envs, dtype=np.uint32),
    )


def test_round_robin_schedule_supports_double_round():
    schedule = weiss_sim.round_robin_schedule(["A", "B", "C"])
    assert len(schedule) == 6
    assert ("A", "B") in schedule
    assert ("B", "A") in schedule
    assert ("A", "C") in schedule
    assert ("C", "A") in schedule
    assert ("B", "C") in schedule
    assert ("C", "B") in schedule

    one_way = weiss_sim.round_robin_schedule(["A", "B", "C"], double_round=False)
    assert len(one_way) == 3
    assert set(one_way) == {("A", "B"), ("A", "C"), ("B", "C")}


def test_sample_population_schedule_is_seeded_and_valid():
    sample_a = weiss_sim.sample_population_schedule(["A", "B", "C"], 8, seed=17)
    sample_b = weiss_sim.sample_population_schedule(["A", "B", "C"], 8, seed=17)
    assert sample_a == sample_b
    assert sample_a == [
        ("A", "C"),
        ("C", "B"),
        ("B", "A"),
        ("A", "C"),
        ("C", "B"),
        ("B", "A"),
        ("C", "B"),
        ("C", "B"),
    ]
    assert len(sample_a) == 8
    assert all(left != right for left, right in sample_a)

    with pytest.raises(ValueError):
        weiss_sim.sample_population_schedule(["solo"], 1, allow_mirror=False)


def test_records_and_summary_helpers():
    step = _dummy_step_batch(
        rewards=[1.0, -1.0, 0.0, 0.0],
        terminated=[True, True, True, False],
        truncated=[False, False, False, True],
    )
    records = weiss_sim.records_from_step(
        step,
        seat0_agents=["A", "A", "B", "B"],
        seat1_agents=["B", "B", "A", "A"],
    )
    assert len(records) == 4
    assert records[0].winner == 0
    assert records[1].winner == 1
    assert records[2].winner is None
    assert records[3].truncated is True
    assert records[0].starting_seat == 0

    summary = weiss_sim.summarize_records(records)
    assert summary["A"].matches == 4
    assert summary["B"].matches == 4
    assert summary["A"].wins == 1
    assert summary["B"].wins == 1
    assert summary["A"].losses == 1
    assert summary["B"].losses == 1
    assert summary["A"].draws == 1
    assert summary["B"].draws == 1
    assert summary["A"].truncated == 1
    assert summary["B"].truncated == 1

    ranked = weiss_sim.rank_agents(summary)
    assert {agent_id for agent_id, _ in ranked} == {"A", "B"}


def test_summarize_first_player_bias():
    step = _dummy_step_batch(
        rewards=[1.0, -1.0, 0.0, 0.0, 1.0],
        terminated=[True, True, True, False, True],
        truncated=[False, False, False, True, False],
        starting_seat=[0, 0, 1, 1, 1],
    )
    records = weiss_sim.records_from_step(
        step,
        seat0_agents=["A"] * 5,
        seat1_agents=["B"] * 5,
    )
    summary = weiss_sim.summarize_first_player_bias(records)
    assert summary.matches == 5
    assert summary.decided == 3
    assert summary.first_player_wins == 1
    assert summary.second_player_wins == 2
    assert summary.draws == 1
    assert summary.truncated == 1
    assert summary.first_player_win_rate == pytest.approx(1.0 / 3.0)


def test_summarize_clock_greed_from_replay():
    replay_data = {
        "body": {
            "actions": [{"Clock": {"hand_index": 255}}, "Pass", {"Clock": {"hand_index": 255}}],
            "steps": [
                {
                    "actor": 0,
                    "decision_kind": "Clock",
                    "illegal_action": False,
                    "engine_error": False,
                },
                {
                    "actor": 0,
                    "decision_kind": "Clock",
                    "illegal_action": False,
                    "engine_error": False,
                },
                {
                    "actor": 1,
                    "decision_kind": "Clock",
                    "illegal_action": False,
                    "engine_error": False,
                },
            ],
            "events": [
                {"Clock": {"player": 0, "card": 1}},
                {"Draw": {"player": 0, "card": 0}},
                {"Clock": {"player": 0, "card": 2}},
                {
                    "DamageIntent": {
                        "event_id": 9,
                        "source_player": 0,
                        "source_slot": None,
                        "target": 0,
                        "amount": 1,
                        "damage_type": "Effect",
                        "cancelable": False,
                    }
                },
                {
                    "DamageCommitted": {
                        "event_id": 9,
                        "target": 0,
                        "card": 1,
                        "damage_type": "Effect",
                    }
                },
                {"Draw": {"player": 0, "card": 0}},
                {
                    "DamageIntent": {
                        "event_id": 10,
                        "source_player": 0,
                        "source_slot": None,
                        "target": 0,
                        "amount": 1,
                        "damage_type": "Effect",
                        "cancelable": False,
                    }
                },
                {
                    "DamageCommitted": {
                        "event_id": 10,
                        "target": 0,
                        "card": 1,
                        "damage_type": "Effect",
                    }
                },
                {"Clock": {"player": 1, "card": 3}},
                {"Draw": {"player": 1, "card": 0}},
            ],
        }
    }
    metrics = weiss_sim.summarize_clock_greed_from_replay(
        replay_data, actor=0, draw_window_events=2
    )
    assert metrics.decision_samples == 2
    assert metrics.clock_decisions == 2
    assert metrics.clock_actions_taken == 1
    assert metrics.clock_passes == 1
    assert metrics.clock_action_rate == pytest.approx(0.5)
    assert metrics.clock_events == 2
    assert metrics.clock_events_followed_by_draw == 1
    assert metrics.clock_followed_by_draw_rate == pytest.approx(0.5)
    assert metrics.self_effect_damage_intents == 2
    assert metrics.self_effect_damage_committed == 2
    assert metrics.self_effect_damage_followed_by_draw == 1
    assert metrics.self_effect_damage_followed_by_draw_rate == pytest.approx(0.5)
