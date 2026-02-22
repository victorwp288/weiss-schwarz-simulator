from __future__ import annotations

import json
import os
from collections.abc import Mapping
from dataclasses import asdict, dataclass, is_dataclass
from pathlib import Path
from typing import Literal, overload

from ._config_norm import (
    normalize_card_pool,
    normalize_error_policy,
    normalize_ids_safety,
    normalize_legal_repr,
    normalize_mode,
    normalize_observation_visibility,
    normalize_obs_dtype,
    normalize_rules_profile,
    resolve_mode_defaults,
    resolve_seed,
    resolve_threads_and_envs,
)
from .catalog import db_info as _db_info
from .config_types import (
    CardPoolMode,
    CurriculumOverrides,
    DeckInput,
    EndConditionOverrides,
    IdsSafety,
    LegalRepr,
    NumLike,
    ObservationVisibility,
    ObsDType,
    RulesProfile,
    ThreadsLike,
)
from .decks import resolve_match_decks, summarize_resolved_deck
from .errors import ConfigConflictError
from .runner import WeissEnv
from .weiss_sim import (
    ACTION_SPACE_SIZE,
    POLICY_VERSION,
    SPEC_HASH,
    BatchOutMinimal,
    BatchOutMinimalI16,
    BatchOutMinimalI16LegalIds,
    BatchOutMinimalNoMask,
    EnvPool,
    action_spec_json,
    observation_spec_json,
)

_KNOWN_CURRICULUM_KEYS = set(CurriculumOverrides.__annotations__.keys())
_KNOWN_END_CONDITION_KEYS = {"simultaneous_loss", "allow_draw_on_simultaneous_loss"}


def _coerce_optional_object_payload(
    value: object | None,
    *,
    field_name: str,
    typed_name: str,
    typed_cls: type[object],
    allow_json_string: bool,
) -> dict[str, object] | None:
    if value is None:
        return None

    if allow_json_string and isinstance(value, str):
        if not value.strip():
            raise ConfigConflictError(
                f"{field_name} must be non-empty when provided as a JSON string"
            )
        try:
            decoded = json.loads(value)
        except json.JSONDecodeError as exc:
            raise ConfigConflictError(f"{field_name} JSON parse error: {exc}") from exc
        if not isinstance(decoded, dict):
            raise ConfigConflictError(f"{field_name} JSON must decode to an object")
        return dict(decoded)

    if isinstance(value, typed_cls):
        return value.to_dict()  # type: ignore[attr-defined]
    if is_dataclass(value) and not isinstance(value, type):
        return {k: v for k, v in asdict(value).items() if v is not None}
    if isinstance(value, Mapping):
        return dict(value)

    accepted = f"{typed_name}, mapping, dataclass, or None"
    if allow_json_string:
        accepted = f"{typed_name}, mapping, JSON string, dataclass, or None"
    raise ConfigConflictError(f"{field_name} must be {accepted}")


def _reject_unknown_fields(
    payload: dict[str, object], *, known_keys: set[str], field_name: str
) -> None:
    unknown = sorted(set(payload.keys()) - known_keys)
    if unknown:
        raise ConfigConflictError(f"unknown {field_name} fields: {', '.join(unknown)}")


def _normalize_reward_json(value: str | Mapping[str, object] | None) -> str | None:
    if value is None:
        return None
    if isinstance(value, str):
        if not value.strip():
            raise ConfigConflictError("reward_json must be non-empty when provided as a string")
        return value
    if isinstance(value, Mapping):
        return json.dumps(value)
    raise ConfigConflictError("reward_json must be a JSON string, mapping, or None")


def _normalize_simultaneous_loss_policy(value: object) -> str:
    token = str(value).strip()
    token_lower = token.lower()
    if token in {"Draw"} or token_lower == "draw":
        return "Draw"
    if token in {"ActivePlayerWins"} or token_lower in {"active_player_wins", "activeplayerwins"}:
        return "ActivePlayerWins"
    if token in {"NonActivePlayerWins"} or token_lower in {
        "non_active_player_wins",
        "nonactiveplayerwins",
    }:
        return "NonActivePlayerWins"
    raise ConfigConflictError(
        "end_condition_policy.simultaneous_loss must be draw/active_player_wins/non_active_player_wins"
    )


def _normalize_end_condition_policy_json(
    value: EndConditionOverrides | Mapping[str, object] | str | None,
) -> str | None:
    payload = _coerce_optional_object_payload(
        value,
        field_name="end_condition_policy",
        typed_name="EndConditionOverrides",
        typed_cls=EndConditionOverrides,
        allow_json_string=True,
    )
    if payload is None:
        return None
    _reject_unknown_fields(
        payload, known_keys=_KNOWN_END_CONDITION_KEYS, field_name="end_condition_policy"
    )

    normalized: dict[str, object] = {}
    if "simultaneous_loss" in payload:
        normalized["simultaneous_loss"] = _normalize_simultaneous_loss_policy(
            payload["simultaneous_loss"]
        )
    if "allow_draw_on_simultaneous_loss" in payload:
        normalized["allow_draw_on_simultaneous_loss"] = bool(
            payload["allow_draw_on_simultaneous_loss"]
        )
    return json.dumps(normalized)


def _normalize_curriculum(
    curriculum: CurriculumOverrides | Mapping[str, object] | None, rules_profile: RulesProfile
) -> dict[str, object]:
    payload = _coerce_optional_object_payload(
        curriculum,
        field_name="curriculum",
        typed_name="CurriculumOverrides",
        typed_cls=CurriculumOverrides,
        allow_json_string=False,
    )
    if payload is None:
        payload = {}

    _reject_unknown_fields(payload, known_keys=_KNOWN_CURRICULUM_KEYS, field_name="curriculum")

    if "enable_approx_effects" in payload:
        requested = bool(payload["enable_approx_effects"])
        if rules_profile == "strict" and requested:
            raise ConfigConflictError(
                "rules_profile='strict' conflicts with curriculum.enable_approx_effects=True"
            )
        payload["enable_approx_effects"] = requested
    else:
        payload["enable_approx_effects"] = bool(rules_profile == "approx")
    return payload


def _select_out_mode(
    pool: EnvPool, legal_repr: LegalRepr, obs_dtype: ObsDType
) -> tuple[object, str, str, bool, bool, bool]:
    if legal_repr in {"ids_u16", "ids_u32"}:
        if obs_dtype == "i16":
            return (
                BatchOutMinimalI16LegalIds(pool.envs_len),
                "reset_into_i16_legal_ids",
                "step_into_i16_legal_ids",
                False,
                True,
                False,
            )
        return (
            BatchOutMinimalNoMask(pool.envs_len),
            "reset_into_nomask",
            "step_into_nomask",
            False,
            False,
            True,
        )
    if legal_repr == "mask_u8":
        if obs_dtype == "i16":
            return (
                BatchOutMinimalI16(pool.envs_len),
                "reset_into_i16",
                "step_into_i16",
                True,
                False,
                False,
            )
        return (
            BatchOutMinimal(pool.envs_len),
            "reset_into",
            "step_into",
            True,
            False,
            False,
        )
    # legal_repr == "both"
    if obs_dtype == "i16":
        return (
            BatchOutMinimalI16(pool.envs_len),
            "reset_into_i16",
            "step_into_i16",
            True,
            False,
            True,
        )
    return (
        BatchOutMinimal(pool.envs_len),
        "reset_into",
        "step_into",
        True,
        False,
        True,
    )


def export_spec_bundle() -> dict[str, object]:
    """Export the current observation/action specs and compatibility hashes."""
    return {
        "policy_version": POLICY_VERSION,
        "spec_hash": SPEC_HASH,
        "observation": json.loads(observation_spec_json()),
        "action": json.loads(action_spec_json()),
    }


def db_info(db_path: str | Path | None = None) -> dict[str, object]:
    """Return hash/compatibility metadata for the selected card database."""
    return _db_info(db_path=db_path)


@dataclass(slots=True)
class _MakeNormalized:
    mode: Literal["fast", "inspect"]
    runtime_mode: str
    deck: DeckInput
    opponent_deck: DeckInput | None
    db_path: str | None
    rules_profile: RulesProfile
    card_pool: CardPoolMode
    curriculum_payload: dict[str, object]
    reward_json_payload: str | None
    end_condition_policy_json: str | None
    observation_visibility: ObservationVisibility
    legal_repr: LegalRepr
    obs_dtype: ObsDType
    ids_safety: IdsSafety | None
    num_envs: NumLike
    num_threads: ThreadsLike
    seed_value: int
    seed_source: str
    max_decisions: int
    max_ticks: int
    error_policy: Literal["raise", "replace", "terminate"]
    control_seat: Literal[0, 1] | None


@dataclass(slots=True)
class _MakeResolved:
    normalized: _MakeNormalized
    deck_a: list[int]
    deck_b: list[int]
    resolved_decks: dict[str, object]
    resolved_num_envs: int
    resolved_num_threads: int
    legal_repr: LegalRepr
    obs_dtype: ObsDType
    ids_safety: IdsSafety | None


@dataclass(slots=True)
class _LayoutSelection:
    out: object
    reset_method: str
    step_method: str
    has_mask: bool
    embedded_ids: bool
    needs_runtime_ids: bool


def _make_stage_normalize(
    *,
    mode: Literal["fast", "inspect"],
    deck: DeckInput | None,
    opponent_deck: DeckInput | None,
    db_path: str | None,
    rules_profile: RulesProfile,
    card_pool: CardPoolMode,
    curriculum: CurriculumOverrides | Mapping[str, object] | None,
    reward_json: str | Mapping[str, object] | None,
    end_condition_policy: EndConditionOverrides | Mapping[str, object] | str | None,
    observation_visibility: ObservationVisibility,
    reveal_opponent_hand_stock_counts: bool | None,
    legal_repr: LegalRepr | None,
    obs_dtype: ObsDType | None,
    ids_safety: IdsSafety | None,
    num_envs: NumLike,
    num_threads: ThreadsLike,
    seed: int | None,
    max_decisions: int,
    max_ticks: int,
    error_policy: Literal["raise", "replace", "terminate"],
    control_seat: Literal[0, 1] | None,
) -> _MakeNormalized:
    normalized_mode, runtime_mode = normalize_mode(mode)
    normalized_rules_profile = normalize_rules_profile(rules_profile)
    normalized_card_pool = normalize_card_pool(card_pool)
    normalized_observation_visibility = normalize_observation_visibility(observation_visibility)
    normalized_legal_repr = normalize_legal_repr(legal_repr)
    normalized_obs_dtype = normalize_obs_dtype(obs_dtype)
    normalized_ids_safety = normalize_ids_safety(ids_safety)
    normalized_error_policy = normalize_error_policy(error_policy)
    reward_json_payload = _normalize_reward_json(reward_json)
    end_condition_policy_json = _normalize_end_condition_policy_json(end_condition_policy)
    seed_value, seed_source = resolve_seed(seed, urandom_fn=os.urandom)

    if control_seat not in (None, 0, 1):
        raise ConfigConflictError("control_seat must be one of None, 0, or 1")
    if max_decisions <= 0 or max_ticks <= 0:
        raise ConfigConflictError("max_decisions and max_ticks must both be > 0")

    resolved_deck = deck if deck is not None else "preset:starter_v1"
    curriculum_payload = _normalize_curriculum(curriculum, normalized_rules_profile)
    if (
        normalized_observation_visibility == "public"
        and "memory_is_public" not in curriculum_payload
    ):
        curriculum_payload["memory_is_public"] = False
    if reveal_opponent_hand_stock_counts is not None:
        curriculum_payload["reveal_opponent_hand_stock_counts"] = bool(
            reveal_opponent_hand_stock_counts
        )

    return _MakeNormalized(
        mode=normalized_mode,
        runtime_mode=runtime_mode,
        deck=resolved_deck,
        opponent_deck=opponent_deck,
        db_path=db_path,
        rules_profile=normalized_rules_profile,
        card_pool=normalized_card_pool,
        curriculum_payload=curriculum_payload,
        reward_json_payload=reward_json_payload,
        end_condition_policy_json=end_condition_policy_json,
        observation_visibility=normalized_observation_visibility,
        legal_repr=normalized_legal_repr,
        obs_dtype=normalized_obs_dtype,
        ids_safety=normalized_ids_safety,
        num_envs=num_envs,
        num_threads=num_threads,
        seed_value=seed_value,
        seed_source=seed_source,
        max_decisions=max_decisions,
        max_ticks=max_ticks,
        error_policy=normalized_error_policy,
        control_seat=control_seat,
    )


def _make_stage_resolve_decks(normalized: _MakeNormalized) -> _MakeResolved:
    deck_a, deck_b = resolve_match_decks(
        normalized.deck,
        normalized.opponent_deck,
        rules_profile=normalized.rules_profile,
        card_pool=normalized.card_pool,
        db_path=normalized.db_path,
    )
    resolved_decks = {
        "player": summarize_resolved_deck(deck_a),
        "opponent": summarize_resolved_deck(deck_b),
    }
    resolved_num_envs, resolved_num_threads = resolve_threads_and_envs(
        normalized.num_envs,
        normalized.num_threads,
        cpu_count_fn=os.cpu_count,
    )
    legal_repr, obs_dtype, ids_safety = resolve_mode_defaults(
        normalized.runtime_mode,
        normalized.legal_repr,
        normalized.obs_dtype,
        normalized.ids_safety,
    )
    return _MakeResolved(
        normalized=normalized,
        deck_a=deck_a,
        deck_b=deck_b,
        resolved_decks=resolved_decks,
        resolved_num_envs=resolved_num_envs,
        resolved_num_threads=resolved_num_threads,
        legal_repr=legal_repr,
        obs_dtype=obs_dtype,
        ids_safety=ids_safety,
    )


def _make_stage_build_pool(resolved: _MakeResolved) -> EnvPool:
    constructor = (
        EnvPool.new_rl_train if resolved.normalized.runtime_mode == "speed" else EnvPool.new_rl_eval
    )
    constructor_output_masks = resolved.legal_repr in {"mask_u8", "both"}
    return constructor(
        resolved.resolved_num_envs,
        db_path=resolved.normalized.db_path,
        deck_lists=[resolved.deck_a, resolved.deck_b],
        deck_ids=[0, 1],
        max_decisions=resolved.normalized.max_decisions,
        max_ticks=resolved.normalized.max_ticks,
        seed=resolved.normalized.seed_value,
        curriculum_json=json.dumps(resolved.normalized.curriculum_payload),
        reward_json=resolved.normalized.reward_json_payload,
        end_condition_policy_json=resolved.normalized.end_condition_policy_json,
        error_policy=resolved.normalized.error_policy,
        observation_visibility=resolved.normalized.observation_visibility,
        num_threads=resolved.resolved_num_threads,
        output_masks=constructor_output_masks,
    )


def _make_stage_select_layout(
    pool: EnvPool, *, legal_repr: LegalRepr, obs_dtype: ObsDType
) -> _LayoutSelection:
    out, reset_method, step_method, has_mask, embedded_ids, needs_runtime_ids = _select_out_mode(
        pool, legal_repr, obs_dtype
    )
    if not has_mask:
        pool.set_output_mask_enabled(False)
        pool.set_output_mask_bits_enabled(False)
    if obs_dtype == "i16":
        pool.set_i16_clamp_enabled(True)
    return _LayoutSelection(
        out=out,
        reset_method=reset_method,
        step_method=step_method,
        has_mask=has_mask,
        embedded_ids=embedded_ids,
        needs_runtime_ids=needs_runtime_ids,
    )


def _make_stage_build_effective_config(
    resolved: _MakeResolved, layout: _LayoutSelection
) -> dict[str, object]:
    db_hash_info = db_info(db_path=resolved.normalized.db_path)

    reward_effective: object | None = None
    if resolved.normalized.reward_json_payload is not None:
        try:
            reward_effective = json.loads(resolved.normalized.reward_json_payload)
        except json.JSONDecodeError:
            reward_effective = resolved.normalized.reward_json_payload

    end_condition_policy_effective: object
    if resolved.normalized.end_condition_policy_json is None:
        end_condition_policy_effective = {
            "simultaneous_loss": "Draw",
            "allow_draw_on_simultaneous_loss": True,
        }
    else:
        try:
            end_condition_policy_effective = json.loads(
                resolved.normalized.end_condition_policy_json
            )
        except json.JSONDecodeError:
            end_condition_policy_effective = resolved.normalized.end_condition_policy_json

    return {
        "mode": resolved.normalized.mode,
        "runtime_mode": resolved.normalized.runtime_mode,
        "internal_runtime_mode": resolved.normalized.runtime_mode,
        "rules_profile": resolved.normalized.rules_profile,
        "card_pool": resolved.normalized.card_pool,
        "observation_visibility": resolved.normalized.observation_visibility,
        "legal_repr": resolved.legal_repr,
        "obs_dtype": resolved.obs_dtype,
        "ids_safety": resolved.ids_safety,
        "num_envs": resolved.resolved_num_envs,
        "num_threads": resolved.resolved_num_threads,
        "seed": resolved.normalized.seed_value,
        "seed_source": resolved.normalized.seed_source,
        "max_decisions": int(resolved.normalized.max_decisions),
        "max_ticks": int(resolved.normalized.max_ticks),
        "error_policy": resolved.normalized.error_policy,
        "reward": reward_effective,
        "end_condition_policy": end_condition_policy_effective,
        "curriculum": resolved.normalized.curriculum_payload,
        "db": db_hash_info,
        "reward_timeout_policy": {
            "timeout_uses_terminal_draw_reward": True,
            "terminal_draw_expected_zero_sum": True,
            "terminal_draw_expected_value": 0.0,
        },
        "resolved_decks": resolved.resolved_decks,
        "spec_hash": int(SPEC_HASH),
        "action_space": int(ACTION_SPACE_SIZE),
        "control_seat": resolved.normalized.control_seat,
        "reveal_opponent_hand_stock_counts": bool(
            resolved.normalized.curriculum_payload.get("reveal_opponent_hand_stock_counts", False)
        ),
        "needs_runtime_legal_ids": layout.needs_runtime_ids,
    }


def _make_stage_create_env(
    *,
    pool: EnvPool,
    resolved: _MakeResolved,
    layout: _LayoutSelection,
    effective: dict[str, object],
) -> WeissEnv:
    return WeissEnv(
        pool=pool,
        out=layout.out,
        reset_method=layout.reset_method,
        step_method=layout.step_method,
        has_mask=layout.has_mask,
        embedded_legal_ids=layout.embedded_ids,
        legal_repr=resolved.legal_repr,
        ids_safety=resolved.ids_safety,
        runtime_mode=resolved.normalized.runtime_mode,
        control_seat=resolved.normalized.control_seat,
        effective=effective,
        spec_fn=export_spec_bundle,
    )


def make(
    *,
    mode: Literal["fast", "inspect"] = "fast",
    deck: DeckInput | None = None,
    opponent_deck: DeckInput | None = None,
    db_path: str | None = None,
    rules_profile: RulesProfile = "strict",
    card_pool: CardPoolMode = "parsed_only",
    curriculum: CurriculumOverrides | Mapping[str, object] | None = None,
    reward_json: str | Mapping[str, object] | None = None,
    end_condition_policy: EndConditionOverrides | Mapping[str, object] | str | None = None,
    observation_visibility: ObservationVisibility = "public",
    reveal_opponent_hand_stock_counts: bool | None = None,
    legal_repr: LegalRepr | None = None,
    obs_dtype: ObsDType | None = None,
    ids_safety: IdsSafety | None = None,
    num_envs: NumLike = 1,
    num_threads: ThreadsLike = "auto",
    seed: int | None = None,
    max_decisions: int = 2000,
    max_ticks: int = 100_000,
    error_policy: Literal["raise", "replace", "terminate"] = "replace",
    control_seat: Literal[0, 1] | None = None,
) -> WeissEnv:
    """Create a high-level `WeissEnv` for batched reset/step loops.

    `make()` is the preferred entry point for most Python integrations. It wraps
    the Rust `EnvPool` and returns a `WeissEnv` with stable observation/action
    contracts and ergonomic legality helpers via `batch.legal`.
    """
    normalized = _make_stage_normalize(
        mode=mode,
        deck=deck,
        opponent_deck=opponent_deck,
        db_path=db_path,
        rules_profile=rules_profile,
        card_pool=card_pool,
        curriculum=curriculum,
        reward_json=reward_json,
        end_condition_policy=end_condition_policy,
        observation_visibility=observation_visibility,
        reveal_opponent_hand_stock_counts=reveal_opponent_hand_stock_counts,
        legal_repr=legal_repr,
        obs_dtype=obs_dtype,
        ids_safety=ids_safety,
        num_envs=num_envs,
        num_threads=num_threads,
        seed=seed,
        max_decisions=max_decisions,
        max_ticks=max_ticks,
        error_policy=error_policy,
        control_seat=control_seat,
    )
    resolved = _make_stage_resolve_decks(normalized)
    pool = _make_stage_build_pool(resolved)
    layout = _make_stage_select_layout(
        pool,
        legal_repr=resolved.legal_repr,
        obs_dtype=resolved.obs_dtype,
    )
    effective = _make_stage_build_effective_config(resolved, layout)
    return _make_stage_create_env(pool=pool, resolved=resolved, layout=layout, effective=effective)


@overload
def fast(
    *,
    deck: DeckInput | None = None,
    opponent_deck: DeckInput | None = None,
    db_path: str | None = None,
    rules_profile: RulesProfile = "strict",
    card_pool: CardPoolMode = "parsed_only",
    curriculum: CurriculumOverrides | Mapping[str, object] | None = None,
    reward_json: str | Mapping[str, object] | None = None,
    end_condition_policy: EndConditionOverrides | Mapping[str, object] | str | None = None,
    observation_visibility: ObservationVisibility = "public",
    reveal_opponent_hand_stock_counts: bool | None = None,
    legal_repr: LegalRepr | None = None,
    obs_dtype: ObsDType | None = None,
    ids_safety: IdsSafety | None = None,
    num_envs: NumLike = 1,
    num_threads: ThreadsLike = "auto",
    seed: int | None = None,
    max_decisions: int = 2000,
    max_ticks: int = 100_000,
    error_policy: Literal["raise", "replace", "terminate"] = "replace",
    control_seat: Literal[0, 1] | None = None,
) -> WeissEnv: ...


def fast(**kwargs: object) -> WeissEnv:
    """Shortcut for `make(mode="fast", ...)`."""
    if "mode" in kwargs:
        raise ConfigConflictError("fast() does not accept mode; it is fixed to 'fast'")
    return make(mode="fast", **kwargs)


@overload
def inspect(
    *,
    deck: DeckInput | None = None,
    opponent_deck: DeckInput | None = None,
    db_path: str | None = None,
    rules_profile: RulesProfile = "strict",
    card_pool: CardPoolMode = "parsed_only",
    curriculum: CurriculumOverrides | Mapping[str, object] | None = None,
    reward_json: str | Mapping[str, object] | None = None,
    end_condition_policy: EndConditionOverrides | Mapping[str, object] | str | None = None,
    observation_visibility: ObservationVisibility = "public",
    reveal_opponent_hand_stock_counts: bool | None = None,
    legal_repr: LegalRepr | None = None,
    obs_dtype: ObsDType | None = None,
    ids_safety: IdsSafety | None = None,
    num_envs: NumLike = 1,
    num_threads: ThreadsLike = "auto",
    seed: int | None = None,
    max_decisions: int = 2000,
    max_ticks: int = 100_000,
    error_policy: Literal["raise", "replace", "terminate"] = "replace",
    control_seat: Literal[0, 1] | None = None,
) -> WeissEnv: ...


def inspect(**kwargs: object) -> WeissEnv:
    """Shortcut for `make(mode="inspect", ...)`."""
    if "mode" in kwargs:
        raise ConfigConflictError("inspect() does not accept mode; it is fixed to 'inspect'")
    return make(mode="inspect", **kwargs)
