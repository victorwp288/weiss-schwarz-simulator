from __future__ import annotations

import os
from typing import Callable, Literal

from .config_types import (
    CardPoolMode,
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
from .errors import ConfigConflictError

_MODE_TO_RUNTIME_MODE: dict[str, RuntimeMode] = {"fast": "speed", "inspect": "eval_debug"}
MAX_NUM_ENVS = 2048
MAX_NUM_THREADS = 256


def normalize_mode(
    value: Literal["fast", "inspect"] | str,
    *,
    error_cls: type[Exception] = ConfigConflictError,
) -> tuple[str, RuntimeMode]:
    token = str(value).strip().lower()
    runtime_mode = _MODE_TO_RUNTIME_MODE.get(token)
    if runtime_mode is None:
        raise error_cls(f"mode must be one of fast/inspect (got {value!r})")
    return token, runtime_mode


def normalize_rules_profile(
    value: RulesProfile | str,
    *,
    error_cls: type[Exception] = ConfigConflictError,
) -> RulesProfile:
    token = str(value).strip().lower()
    if token not in {"strict", "approx"}:
        raise error_cls(f"rules_profile must be strict or approx (got {value!r})")
    return token  # type: ignore[return-value]


def normalize_card_pool(
    value: CardPoolMode | str,
    *,
    error_cls: type[Exception] = ConfigConflictError,
) -> CardPoolMode:
    token = str(value).strip().lower()
    if token not in {"parsed_only", "all"}:
        raise error_cls(f"card_pool must be parsed_only or all (got {value!r})")
    return token  # type: ignore[return-value]


def normalize_legal_repr(
    value: LegalRepr | str | None,
    *,
    error_cls: type[Exception] = ConfigConflictError,
) -> LegalRepr | None:
    if value is None:
        return None
    token = str(value).strip().lower()
    if token not in {"ids_u16", "ids_u32", "mask_u8", "both"}:
        raise error_cls(f"legal_repr must be one of ids_u16/ids_u32/mask_u8/both (got {value!r})")
    return token  # type: ignore[return-value]


def normalize_obs_dtype(
    value: ObsDType | str | None,
    *,
    error_cls: type[Exception] = ConfigConflictError,
) -> ObsDType | None:
    if value is None:
        return None
    token = str(value).strip().lower()
    if token not in {"i16", "i32"}:
        raise error_cls(f"obs_dtype must be one of i16/i32 (got {value!r})")
    return token  # type: ignore[return-value]


def normalize_ids_safety(
    value: IdsSafety | str | None,
    *,
    error_cls: type[Exception] = ConfigConflictError,
) -> IdsSafety | None:
    if value is None:
        return None
    token = str(value).strip().lower()
    if token not in {"checked", "unsafe"}:
        raise error_cls(f"ids_safety must be one of checked/unsafe (got {value!r})")
    return token  # type: ignore[return-value]


def normalize_error_policy(
    value: Literal["raise", "replace", "terminate"] | str,
    *,
    error_cls: type[Exception] = ConfigConflictError,
) -> ErrorPolicy:
    token = str(value).strip().lower()
    if token not in {"raise", "replace", "terminate"}:
        raise error_cls(f"error_policy must be one of raise/replace/terminate (got {value!r})")
    return token  # type: ignore[return-value]


def normalize_observation_visibility(
    value: ObservationVisibility | str,
    *,
    error_cls: type[Exception] = ConfigConflictError,
) -> ObservationVisibility:
    token = str(value).strip().lower()
    if token not in {"public", "full"}:
        raise error_cls(f"observation_visibility must be one of public/full (got {value!r})")
    return token  # type: ignore[return-value]


def resolve_seed(
    seed: int | None,
    *,
    urandom_fn: Callable[[int], bytes] = os.urandom,
    error_cls: type[Exception] = ConfigConflictError,
) -> tuple[int, str]:
    if seed is None:
        return int.from_bytes(urandom_fn(8), "little"), "entropy"
    try:
        return int(seed), "user"
    except (TypeError, ValueError) as exc:
        raise error_cls(f"seed must be an int or None (got {seed!r})") from exc


def resolve_threads_and_envs(
    num_envs: NumLike,
    num_threads: ThreadsLike,
    *,
    cpu_count_fn: Callable[[], int | None] = os.cpu_count,
    error_cls: type[Exception] = ConfigConflictError,
) -> tuple[int, int]:
    cpu = max(1, int(cpu_count_fn() or 1))
    auto_threads = min(16, cpu)
    if isinstance(num_threads, int):
        if num_threads <= 0 or num_threads > MAX_NUM_THREADS:
            raise error_cls(
                f"num_threads must be in [1, {MAX_NUM_THREADS}] when set explicitly "
                f"(got {num_threads})"
            )
        resolved_threads = num_threads
    elif num_threads in ("auto", None):
        resolved_threads = auto_threads
    else:
        raise error_cls(
            f"num_threads must be an int in [1, {MAX_NUM_THREADS}], 'auto', or None "
            f"(got {num_threads!r})"
        )

    if isinstance(num_envs, int):
        if num_envs <= 0 or num_envs > MAX_NUM_ENVS:
            raise error_cls(
                f"num_envs must be in [1, {MAX_NUM_ENVS}] when set explicitly (got {num_envs})"
            )
        resolved_envs = num_envs
    elif num_envs == "auto":
        resolved_envs = min(MAX_NUM_ENVS, min(128, max(32, 4 * resolved_threads)))
    else:
        raise error_cls(
            f"num_envs must be an int in [1, {MAX_NUM_ENVS}] or 'auto' (got {num_envs!r})"
        )

    resolved_threads = min(resolved_threads, resolved_envs)
    return resolved_envs, resolved_threads


def resolve_mode_defaults(
    runtime_mode: RuntimeMode,
    legal_repr: LegalRepr | None,
    obs_dtype: ObsDType | None,
    ids_safety: IdsSafety | None,
    *,
    error_cls: type[Exception] = ConfigConflictError,
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
            raise error_cls("ids_safety is only valid when legal_repr='ids_u16'")
        effective_ids_safety = None

    return effective_legal_repr, effective_obs_dtype, effective_ids_safety
