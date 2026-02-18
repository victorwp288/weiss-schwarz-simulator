from __future__ import annotations

import json
import os
from dataclasses import asdict, is_dataclass
from pathlib import Path
from typing import Literal

from .catalog import db_info as _db_info
from .config_types import (
    CardPoolMode,
    CurriculumOverrides,
    DeckInput,
    EndConditionOverrides,
    ErrorPolicy,
    IdsSafety,
    LegalRepr,
    NumLike,
    ObservationVisibility,
    ObsDType,
    RulesProfile,
    RuntimeMode,
    ThreadsLike,
)
from .decks import resolve_match_decks
from .errors import ConfigConflictError
from .runner import SimRunner
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


def _normalize_runtime_mode(value: RuntimeMode | str) -> RuntimeMode:
    token = str(value).strip().lower()
    if token not in {"speed", "eval_debug"}:
        raise ConfigConflictError(f"runtime_mode must be one of speed/eval_debug (got {value!r})")
    return token  # type: ignore[return-value]


def _normalize_rules_profile(value: RulesProfile | str) -> RulesProfile:
    token = str(value).strip().lower()
    if token not in {"strict", "approx"}:
        raise ConfigConflictError(f"rules_profile must be strict or approx (got {value!r})")
    return token  # type: ignore[return-value]


def _normalize_card_pool(value: CardPoolMode | str) -> CardPoolMode:
    token = str(value).strip().lower()
    if token not in {"parsed_only", "all"}:
        raise ConfigConflictError(f"card_pool must be parsed_only or all (got {value!r})")
    return token  # type: ignore[return-value]


def _normalize_legal_repr(value: LegalRepr | str | None) -> LegalRepr | None:
    if value is None:
        return None
    token = str(value).strip().lower()
    if token not in {"ids_u16", "ids_u32", "mask_u8", "both"}:
        raise ConfigConflictError(
            f"legal_repr must be one of ids_u16/ids_u32/mask_u8/both (got {value!r})"
        )
    return token  # type: ignore[return-value]


def _normalize_obs_dtype(value: ObsDType | str | None) -> ObsDType | None:
    if value is None:
        return None
    token = str(value).strip().lower()
    if token not in {"i16", "i32"}:
        raise ConfigConflictError(f"obs_dtype must be one of i16/i32 (got {value!r})")
    return token  # type: ignore[return-value]


def _normalize_ids_safety(value: IdsSafety | str | None) -> IdsSafety | None:
    if value is None:
        return None
    token = str(value).strip().lower()
    if token not in {"checked", "unsafe"}:
        raise ConfigConflictError(f"ids_safety must be one of checked/unsafe (got {value!r})")
    return token  # type: ignore[return-value]


def _normalize_error_policy(value: ErrorPolicy | str) -> ErrorPolicy:
    token = str(value).strip().lower()
    if token not in {"strict", "lenient_terminate", "lenient_noop"}:
        raise ConfigConflictError(
            f"error_policy must be one of strict/lenient_terminate/lenient_noop (got {value!r})"
        )
    return token  # type: ignore[return-value]


def _normalize_observation_visibility(
    value: ObservationVisibility | str,
) -> ObservationVisibility:
    token = str(value).strip().lower()
    if token not in {"public", "full"}:
        raise ConfigConflictError(
            f"observation_visibility must be one of public/full (got {value!r})"
        )
    return token  # type: ignore[return-value]


def _normalize_reward_json(value: str | dict[str, object] | None) -> str | None:
    if value is None:
        return None
    if isinstance(value, str):
        if not value.strip():
            raise ConfigConflictError("reward_json must be non-empty when provided as a string")
        return value
    if isinstance(value, dict):
        return json.dumps(value)
    raise ConfigConflictError("reward_json must be a JSON string, dict, or None")


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
    value: EndConditionOverrides | dict[str, object] | str | None,
) -> str | None:
    if value is None:
        return None
    payload: dict[str, object]
    if isinstance(value, str):
        if not value.strip():
            raise ConfigConflictError(
                "end_condition_policy must be non-empty when provided as a JSON string"
            )
        try:
            decoded = json.loads(value)
        except json.JSONDecodeError as exc:
            raise ConfigConflictError(f"end_condition_policy JSON parse error: {exc}") from exc
        if not isinstance(decoded, dict):
            raise ConfigConflictError("end_condition_policy JSON must decode to an object")
        payload = dict(decoded)
    elif isinstance(value, EndConditionOverrides):
        payload = value.to_dict()
    elif is_dataclass(value):
        payload = {k: v for k, v in asdict(value).items() if v is not None}
    elif isinstance(value, dict):
        payload = dict(value)
    else:
        raise ConfigConflictError(
            "end_condition_policy must be EndConditionOverrides, dict, JSON string, dataclass, or None"
        )

    unknown = sorted(set(payload.keys()) - _KNOWN_END_CONDITION_KEYS)
    if unknown:
        raise ConfigConflictError(f"unknown end_condition_policy fields: {', '.join(unknown)}")

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


def _resolve_threads_and_envs(num_envs: NumLike, num_threads: ThreadsLike) -> tuple[int, int]:
    cpu = max(1, int(os.cpu_count() or 1))
    auto_threads = min(16, cpu)
    if isinstance(num_threads, int):
        if num_threads <= 0:
            raise ConfigConflictError(f"num_threads must be > 0 (got {num_threads})")
        resolved_threads = num_threads
    elif num_threads in ("auto", None):
        resolved_threads = auto_threads
    else:
        raise ConfigConflictError(f"num_threads must be int, 'auto', or None (got {num_threads!r})")

    if isinstance(num_envs, int):
        if num_envs <= 0:
            raise ConfigConflictError(f"num_envs must be > 0 (got {num_envs})")
        resolved_envs = num_envs
    elif num_envs == "auto":
        resolved_envs = min(128, max(32, 4 * resolved_threads))
    else:
        raise ConfigConflictError(f"num_envs must be int or 'auto' (got {num_envs!r})")

    resolved_threads = min(resolved_threads, resolved_envs)
    return resolved_envs, resolved_threads


def _normalize_curriculum(
    curriculum: CurriculumOverrides | dict[str, object] | None, rules_profile: RulesProfile
) -> dict[str, object]:
    payload: dict[str, object]
    if curriculum is None:
        payload = {}
    elif isinstance(curriculum, CurriculumOverrides):
        payload = curriculum.to_dict()
    elif is_dataclass(curriculum):
        payload = {k: v for k, v in asdict(curriculum).items() if v is not None}
    elif isinstance(curriculum, dict):
        payload = dict(curriculum)
    else:
        raise ConfigConflictError(
            "curriculum must be CurriculumOverrides, dict, dataclass, or None"
        )

    unknown = sorted(set(payload.keys()) - _KNOWN_CURRICULUM_KEYS)
    if unknown:
        raise ConfigConflictError(f"unknown curriculum fields: {', '.join(unknown)}")

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


def _resolve_mode_defaults(
    runtime_mode: RuntimeMode,
    legal_repr: LegalRepr | None,
    obs_dtype: ObsDType | None,
    ids_safety: IdsSafety | None,
) -> tuple[LegalRepr, ObsDType, IdsSafety | None]:
    if runtime_mode == "speed":
        default_legal_repr: LegalRepr = "ids_u16"
        default_obs_dtype: ObsDType = "i16"
        default_ids_safety: IdsSafety | None = "checked"
    else:
        default_legal_repr = "both"
        default_obs_dtype = "i32"
        default_ids_safety = None

    effective_legal_repr = legal_repr or default_legal_repr
    effective_obs_dtype = obs_dtype or default_obs_dtype
    explicit_ids_safety = ids_safety
    if effective_legal_repr == "ids_u16":
        effective_ids_safety = explicit_ids_safety or default_ids_safety or "checked"
    else:
        if explicit_ids_safety is not None:
            raise ConfigConflictError("ids_safety is only valid when legal_repr='ids_u16'")
        effective_ids_safety = None

    return effective_legal_repr, effective_obs_dtype, effective_ids_safety


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
    return {
        "policy_version": POLICY_VERSION,
        "spec_hash": SPEC_HASH,
        "observation": json.loads(observation_spec_json()),
        "action": json.loads(action_spec_json()),
    }


def db_info(db_path: str | Path | None = None) -> dict[str, object]:
    return _db_info(db_path=db_path)


def create(
    *,
    deck: DeckInput | None = None,
    opponent_deck: DeckInput | None = None,
    db_path: str | None = None,
    rules_profile: RulesProfile = "strict",
    runtime_mode: RuntimeMode = "speed",
    card_pool: CardPoolMode = "parsed_only",
    curriculum: CurriculumOverrides | dict[str, object] | None = None,
    reward_json: str | dict[str, object] | None = None,
    end_condition_policy: EndConditionOverrides | dict[str, object] | str | None = None,
    observation_visibility: ObservationVisibility = "public",
    reveal_opponent_hand_stock_counts: bool | None = None,
    legal_repr: LegalRepr | None = None,
    obs_dtype: ObsDType | None = None,
    ids_safety: IdsSafety | None = None,
    num_envs: NumLike = "auto",
    num_threads: ThreadsLike = "auto",
    seed: int = 0,
    max_decisions: int = 2000,
    max_ticks: int = 100_000,
    error_policy: ErrorPolicy = "lenient_terminate",
    control_seat: Literal[0, 1] | None = None,
) -> SimRunner:
    runtime_mode = _normalize_runtime_mode(runtime_mode)
    rules_profile = _normalize_rules_profile(rules_profile)
    card_pool = _normalize_card_pool(card_pool)
    observation_visibility = _normalize_observation_visibility(observation_visibility)
    legal_repr = _normalize_legal_repr(legal_repr)
    obs_dtype = _normalize_obs_dtype(obs_dtype)
    ids_safety = _normalize_ids_safety(ids_safety)
    error_policy = _normalize_error_policy(error_policy)
    reward_json_payload = _normalize_reward_json(reward_json)
    end_condition_policy_json = _normalize_end_condition_policy_json(end_condition_policy)
    if control_seat not in (None, 0, 1):
        raise ConfigConflictError("control_seat must be one of None, 0, or 1")
    if max_decisions <= 0 or max_ticks <= 0:
        raise ConfigConflictError("max_decisions and max_ticks must both be > 0")

    if deck is None:
        deck = "preset:starter_v1"
    curriculum_payload = _normalize_curriculum(curriculum, rules_profile)
    # Public observations should not expose memory-zone identities unless explicitly requested.
    if observation_visibility == "public" and "memory_is_public" not in curriculum_payload:
        curriculum_payload["memory_is_public"] = False
    if reveal_opponent_hand_stock_counts is not None:
        curriculum_payload["reveal_opponent_hand_stock_counts"] = bool(
            reveal_opponent_hand_stock_counts
        )
    deck_a, deck_b = resolve_match_decks(
        deck,
        opponent_deck,
        rules_profile=rules_profile,
        card_pool=card_pool,
        db_path=db_path,
    )
    resolved_num_envs, resolved_num_threads = _resolve_threads_and_envs(num_envs, num_threads)
    legal_repr, obs_dtype, ids_safety = _resolve_mode_defaults(
        runtime_mode, legal_repr, obs_dtype, ids_safety
    )
    constructor_output_masks = legal_repr in {"mask_u8", "both"}

    constructor = EnvPool.new_rl_train if runtime_mode == "speed" else EnvPool.new_rl_eval
    pool = constructor(
        resolved_num_envs,
        db_path=db_path,
        deck_lists=[deck_a, deck_b],
        deck_ids=[0, 1],
        max_decisions=max_decisions,
        max_ticks=max_ticks,
        seed=int(seed),
        curriculum_json=json.dumps(curriculum_payload),
        reward_json=reward_json_payload,
        end_condition_policy_json=end_condition_policy_json,
        error_policy=error_policy,
        observation_visibility=observation_visibility,
        num_threads=resolved_num_threads,
        output_masks=constructor_output_masks,
    )

    out, reset_method, step_method, has_mask, embedded_ids, needs_runtime_ids = _select_out_mode(
        pool, legal_repr, obs_dtype
    )
    if not has_mask:
        pool.set_output_mask_enabled(False)
        pool.set_output_mask_bits_enabled(False)
    if obs_dtype == "i16":
        pool.set_i16_clamp_enabled(True)
    db_hash_info = db_info(db_path=db_path)
    reward_effective: object | None = None
    if reward_json_payload is not None:
        try:
            reward_effective = json.loads(reward_json_payload)
        except json.JSONDecodeError:
            reward_effective = reward_json_payload
    end_condition_policy_effective: object
    if end_condition_policy_json is None:
        end_condition_policy_effective = {
            "simultaneous_loss": "Draw",
            "allow_draw_on_simultaneous_loss": True,
        }
    else:
        try:
            end_condition_policy_effective = json.loads(end_condition_policy_json)
        except json.JSONDecodeError:
            end_condition_policy_effective = end_condition_policy_json

    effective = {
        "runtime_mode": runtime_mode,
        "rules_profile": rules_profile,
        "card_pool": card_pool,
        "observation_visibility": observation_visibility,
        "legal_repr": legal_repr,
        "obs_dtype": obs_dtype,
        "ids_safety": ids_safety,
        "num_envs": resolved_num_envs,
        "num_threads": resolved_num_threads,
        "seed": int(seed),
        "max_decisions": int(max_decisions),
        "max_ticks": int(max_ticks),
        "error_policy": error_policy,
        "reward": reward_effective,
        "end_condition_policy": end_condition_policy_effective,
        "curriculum": curriculum_payload,
        "db": db_hash_info,
        "reward_timeout_policy": {
            "timeout_uses_terminal_draw_reward": True,
            "terminal_draw_expected_zero_sum": True,
            "terminal_draw_expected_value": 0.0,
        },
        "spec_hash": int(SPEC_HASH),
        "action_space": int(ACTION_SPACE_SIZE),
        "control_seat": control_seat,
        "reveal_opponent_hand_stock_counts": bool(
            curriculum_payload.get("reveal_opponent_hand_stock_counts", False)
        ),
        "needs_runtime_legal_ids": needs_runtime_ids,
    }

    return SimRunner(
        pool=pool,
        out=out,
        reset_method=reset_method,
        step_method=step_method,
        has_mask=has_mask,
        embedded_legal_ids=embedded_ids,
        legal_repr=legal_repr,
        ids_safety=ids_safety,
        runtime_mode=runtime_mode,
        control_seat=control_seat,
        effective=effective,
        spec_fn=export_spec_bundle,
    )


def train(**kwargs) -> SimRunner:
    if "runtime_mode" in kwargs:
        raise ConfigConflictError("train() does not accept runtime_mode; it is fixed to 'speed'")
    return create(runtime_mode="speed", **kwargs)


def evaluate(**kwargs) -> SimRunner:
    if "runtime_mode" in kwargs:
        raise ConfigConflictError(
            "evaluate() does not accept runtime_mode; it is fixed to 'eval_debug'"
        )
    return create(runtime_mode="eval_debug", **kwargs)
