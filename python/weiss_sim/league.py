from __future__ import annotations

from collections import defaultdict
from dataclasses import dataclass
from itertools import combinations
from typing import Any, Iterable, Mapping, Sequence

from .types import StepBatch

_U64_MASK = (1 << 64) - 1


@dataclass(slots=True, frozen=True)
class MatchRecord:
    seat0_agent: str
    seat1_agent: str
    winner: int | None
    terminated: bool
    truncated: bool
    reward_seat0: float
    decision_count: int
    tick_count: int
    episode_seed: int
    episode_key: int
    starting_seat: int = 0


@dataclass(slots=True, frozen=True)
class AgentSummary:
    matches: int
    wins: int
    losses: int
    draws: int
    truncated: int
    win_rate: float


@dataclass(slots=True, frozen=True)
class FirstPlayerBiasSummary:
    matches: int
    decided: int
    first_player_wins: int
    second_player_wins: int
    draws: int
    truncated: int
    first_player_win_rate: float


@dataclass(slots=True, frozen=True)
class ClockGreedSummary:
    decision_samples: int
    clock_decisions: int
    clock_actions_taken: int
    clock_passes: int
    clock_action_rate: float
    clock_events: int
    clock_events_followed_by_draw: int
    clock_followed_by_draw_rate: float
    self_effect_damage_intents: int
    self_effect_damage_committed: int
    self_effect_damage_followed_by_draw: int
    self_effect_damage_followed_by_draw_rate: float


def _splitmix64_next(state: int) -> tuple[int, int]:
    """Small deterministic RNG primitive used for schedule sampling."""
    state = (state + 0x9E3779B97F4A7C15) & _U64_MASK
    z = state
    z = (z ^ (z >> 30)) * 0xBF58476D1CE4E5B9 & _U64_MASK
    z = (z ^ (z >> 27)) * 0x94D049BB133111EB & _U64_MASK
    z = (z ^ (z >> 31)) & _U64_MASK
    return state, z


def _sample_index(state: int, upper: int) -> tuple[int, int]:
    if upper <= 0:
        raise ValueError(f"upper must be > 0 (got {upper})")
    state, rnd = _splitmix64_next(state)
    # Multiply-high maps 64-bit random values to [0, upper) without division.
    return state, int((rnd * upper) >> 64)


def _normalize_agents(agent_ids: Sequence[str]) -> list[str]:
    out = [str(agent_id) for agent_id in agent_ids]
    if len(set(out)) != len(out):
        raise ValueError("agent_ids must be unique")
    return out


def _normalize_per_env_agents(
    agents: str | Sequence[str], *, num_envs: int, name: str
) -> list[str]:
    if isinstance(agents, str):
        return [agents] * num_envs
    out = [str(agent_id) for agent_id in agents]
    if len(out) != num_envs:
        raise ValueError(f"{name} must have length {num_envs}, got {len(out)}")
    return out


def round_robin_schedule(
    agent_ids: Sequence[str], *, double_round: bool = True
) -> list[tuple[str, str]]:
    agents = _normalize_agents(agent_ids)
    if len(agents) < 2:
        return []
    schedule = [(a, b) for a, b in combinations(agents, 2)]
    if double_round:
        schedule.extend((b, a) for a, b in schedule.copy())
    return schedule


def sample_population_schedule(
    agent_ids: Sequence[str],
    num_matches: int,
    *,
    seed: int = 0,
    allow_mirror: bool = False,
) -> list[tuple[str, str]]:
    if num_matches < 0:
        raise ValueError(f"num_matches must be >= 0 (got {num_matches})")
    agents = _normalize_agents(agent_ids)
    if not agents:
        return []
    if len(agents) == 1 and not allow_mirror:
        raise ValueError("allow_mirror=False requires at least two agents")

    state = (int(seed) ^ 0xD1B54A32D192ED03) & _U64_MASK
    size = len(agents)
    out: list[tuple[str, str]] = []
    for _ in range(num_matches):
        state, left_idx = _sample_index(state, size)
        state, right_idx = _sample_index(state, size)
        seat0 = agents[left_idx]
        seat1 = agents[right_idx]
        if not allow_mirror:
            while seat1 == seat0:
                state, right_idx = _sample_index(state, size)
                seat1 = agents[right_idx]
        out.append((seat0, seat1))
    return out


def records_from_step(
    step: StepBatch,
    *,
    seat0_agents: str | Sequence[str],
    seat1_agents: str | Sequence[str],
) -> list[MatchRecord]:
    num_envs = int(step.reward.shape[0])
    seat0 = _normalize_per_env_agents(seat0_agents, num_envs=num_envs, name="seat0_agents")
    seat1 = _normalize_per_env_agents(seat1_agents, num_envs=num_envs, name="seat1_agents")

    out: list[MatchRecord] = []
    for env_idx in range(num_envs):
        reward = float(step.reward[env_idx])
        terminated = bool(step.terminated[env_idx])
        truncated = bool(step.truncated[env_idx])
        winner: int | None = None
        if terminated:
            if reward > 0:
                winner = 0
            elif reward < 0:
                winner = 1
        out.append(
            MatchRecord(
                seat0_agent=seat0[env_idx],
                seat1_agent=seat1[env_idx],
                winner=winner,
                terminated=terminated,
                truncated=truncated,
                reward_seat0=reward,
                decision_count=int(step.decision_count[env_idx]),
                tick_count=int(step.tick_count[env_idx]),
                episode_seed=int(step.episode_seed[env_idx]),
                episode_key=int(step.episode_key[env_idx]),
                starting_seat=int(step.starting_seat[env_idx]),
            )
        )
    return out


def summarize_records(records: Iterable[MatchRecord]) -> dict[str, AgentSummary]:
    raw: dict[str, dict[str, int]] = defaultdict(
        lambda: {
            "matches": 0,
            "wins": 0,
            "losses": 0,
            "draws": 0,
            "truncated": 0,
        }
    )
    for record in records:
        if not record.terminated and not record.truncated:
            continue
        left = raw[record.seat0_agent]
        right = raw[record.seat1_agent]
        left["matches"] += 1
        right["matches"] += 1
        if record.truncated:
            left["truncated"] += 1
            right["truncated"] += 1
            continue
        if record.winner == 0:
            left["wins"] += 1
            right["losses"] += 1
            continue
        if record.winner == 1:
            right["wins"] += 1
            left["losses"] += 1
            continue
        left["draws"] += 1
        right["draws"] += 1

    out: dict[str, AgentSummary] = {}
    for agent_id, values in raw.items():
        matches = int(values["matches"])
        wins = int(values["wins"])
        out[agent_id] = AgentSummary(
            matches=matches,
            wins=wins,
            losses=int(values["losses"]),
            draws=int(values["draws"]),
            truncated=int(values["truncated"]),
            win_rate=(float(wins) / float(matches)) if matches > 0 else 0.0,
        )
    return out


def rank_agents(summary: dict[str, AgentSummary]) -> list[tuple[str, AgentSummary]]:
    return sorted(
        summary.items(),
        key=lambda item: (
            item[1].win_rate,
            item[1].wins,
            -item[1].losses,
            item[1].matches,
            item[0],
        ),
        reverse=True,
    )


def summarize_first_player_bias(records: Iterable[MatchRecord]) -> FirstPlayerBiasSummary:
    matches = 0
    decided = 0
    first_player_wins = 0
    second_player_wins = 0
    draws = 0
    truncated = 0
    for record in records:
        if not record.terminated and not record.truncated:
            continue
        matches += 1
        if record.truncated:
            truncated += 1
            continue
        if record.winner is None:
            draws += 1
            continue
        decided += 1
        if int(record.winner) == int(record.starting_seat):
            first_player_wins += 1
        else:
            second_player_wins += 1
    return FirstPlayerBiasSummary(
        matches=matches,
        decided=decided,
        first_player_wins=first_player_wins,
        second_player_wins=second_player_wins,
        draws=draws,
        truncated=truncated,
        first_player_win_rate=(float(first_player_wins) / float(decided) if decided > 0 else 0.0),
    )


def _normalize_replay_variant(entry: Any) -> tuple[str, Mapping[str, Any]]:
    if isinstance(entry, str):
        return entry, {}
    if isinstance(entry, Mapping) and len(entry) == 1:
        variant, payload = next(iter(entry.items()))
        if isinstance(payload, Mapping):
            return str(variant), payload
        return str(variant), {}
    raise ValueError(f"invalid replay entry format: {entry!r}")


def summarize_clock_greed_from_replay(
    replay_data: Mapping[str, Any],
    *,
    actor: int | None = None,
    draw_window_events: int = 8,
) -> ClockGreedSummary:
    if draw_window_events < 1:
        raise ValueError(f"draw_window_events must be >= 1 (got {draw_window_events})")
    body = replay_data.get("body")
    if not isinstance(body, Mapping):
        raise ValueError("replay_data must include a mapping body")
    actions_raw = body.get("actions")
    steps_raw = body.get("steps")
    events_raw = body.get("events")
    if not isinstance(actions_raw, Sequence) or not isinstance(steps_raw, Sequence):
        raise ValueError("replay body must include sequence actions and steps")
    if len(actions_raw) != len(steps_raw):
        raise ValueError(
            f"replay actions/steps length mismatch ({len(actions_raw)} != {len(steps_raw)})"
        )
    if events_raw is None:
        events_raw = []
    if not isinstance(events_raw, Sequence):
        raise ValueError("replay body events must be a sequence when present")

    decision_samples = 0
    clock_decisions = 0
    clock_actions_taken = 0
    clock_passes = 0
    for step_entry, action_entry in zip(steps_raw, actions_raw):
        if not isinstance(step_entry, Mapping):
            continue
        actor_value = step_entry.get("actor", -1)
        try:
            step_actor = int(actor_value)
        except (TypeError, ValueError):
            continue
        if actor is not None and step_actor != int(actor):
            continue
        decision_samples += 1
        if str(step_entry.get("decision_kind")) != "Clock":
            continue
        clock_decisions += 1
        action_variant, _ = _normalize_replay_variant(action_entry)
        if action_variant == "Clock":
            clock_actions_taken += 1
        elif action_variant == "Pass":
            clock_passes += 1

    parsed_events: list[tuple[str, Mapping[str, Any]]] = []
    for event_entry in events_raw:
        parsed_events.append(_normalize_replay_variant(event_entry))

    clock_events = 0
    clock_events_followed_by_draw = 0
    self_effect_damage_intents = 0
    self_effect_damage_committed = 0
    self_effect_damage_followed_by_draw = 0
    effect_intent_player: dict[int, int] = {}
    for idx, (variant, payload) in enumerate(parsed_events):
        if variant == "Clock":
            player = int(payload.get("player", -1))
            if actor is not None and player != int(actor):
                continue
            clock_events += 1
            window = parsed_events[idx + 1 : idx + 1 + draw_window_events]
            if any(
                w_variant == "Draw" and int(w_payload.get("player", -1)) == player
                for w_variant, w_payload in window
            ):
                clock_events_followed_by_draw += 1
            continue
        if variant == "DamageIntent":
            source_player = int(payload.get("source_player", -1))
            target = int(payload.get("target", -1))
            amount = int(payload.get("amount", 0))
            damage_type = str(payload.get("damage_type"))
            if (
                source_player == target
                and amount > 0
                and damage_type == "Effect"
                and (actor is None or source_player == int(actor))
            ):
                self_effect_damage_intents += 1
                event_id = int(payload.get("event_id", -1))
                if event_id >= 0:
                    effect_intent_player[event_id] = source_player
            continue
        if variant == "DamageCommitted":
            event_id = int(payload.get("event_id", -1))
            target = int(payload.get("target", -1))
            expected_player = effect_intent_player.get(event_id)
            if expected_player is None or expected_player != target:
                continue
            self_effect_damage_committed += 1
            window = parsed_events[idx + 1 : idx + 1 + draw_window_events]
            if any(
                w_variant == "Draw" and int(w_payload.get("player", -1)) == target
                for w_variant, w_payload in window
            ):
                self_effect_damage_followed_by_draw += 1

    return ClockGreedSummary(
        decision_samples=decision_samples,
        clock_decisions=clock_decisions,
        clock_actions_taken=clock_actions_taken,
        clock_passes=clock_passes,
        clock_action_rate=(
            float(clock_actions_taken) / float(clock_decisions) if clock_decisions > 0 else 0.0
        ),
        clock_events=clock_events,
        clock_events_followed_by_draw=clock_events_followed_by_draw,
        clock_followed_by_draw_rate=(
            float(clock_events_followed_by_draw) / float(clock_events) if clock_events > 0 else 0.0
        ),
        self_effect_damage_intents=self_effect_damage_intents,
        self_effect_damage_committed=self_effect_damage_committed,
        self_effect_damage_followed_by_draw=self_effect_damage_followed_by_draw,
        self_effect_damage_followed_by_draw_rate=(
            float(self_effect_damage_followed_by_draw) / float(self_effect_damage_committed)
            if self_effect_damage_committed > 0
            else 0.0
        ),
    )
