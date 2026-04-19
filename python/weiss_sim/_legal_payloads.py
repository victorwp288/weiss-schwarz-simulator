from __future__ import annotations

from typing import Any, Callable

import numpy as np

LegalIdsIntoFn = Callable[[np.ndarray, np.ndarray], int]
LegalActionMetaIntoFn = Callable[[np.ndarray], int]


def materialize_legal_ids_u16(
    *,
    embedded_legal_ids: bool,
    out: Any,
    legal_ids_buffer: np.ndarray,
    legal_offsets_buffer: np.ndarray,
    legal_action_ids_into: LegalIdsIntoFn,
) -> tuple[np.ndarray, np.ndarray]:
    """Return legal ids/offsets as uint16/uint32 views without unnecessary copies."""
    if embedded_legal_ids:
        return (
            np.asarray(out.legal_ids, dtype=np.uint16),
            np.asarray(out.legal_offsets, dtype=np.uint32),
        )

    count = int(legal_action_ids_into(legal_ids_buffer, legal_offsets_buffer))
    return legal_ids_buffer[:count], legal_offsets_buffer


def materialize_legal_action_meta_u16(
    *,
    embedded_legal_ids: bool,
    out: Any,
    legal_action_meta_buffer: np.ndarray,
    used_rows: int,
    legal_action_meta_into: LegalActionMetaIntoFn | None,
) -> np.ndarray | None:
    """Return packed legal-action metadata aligned 1:1 with the used legal-id prefix."""
    if used_rows < 0:
        raise ValueError("used_rows must be >= 0")
    if embedded_legal_ids:
        raw = getattr(out, "legal_action_meta", None)
        if raw is None:
            return None
        raw_arr = np.asarray(raw, dtype=np.uint16)
        if raw_arr.shape[0] < used_rows:
            raise ValueError(
                "embedded legal_action_meta buffer is smaller than used legal-id rows "
                f"(got {raw_arr.shape[0]}, need at least {used_rows})"
            )
        return raw_arr[:used_rows]

    if legal_action_meta_into is None:
        return None
    count = int(legal_action_meta_into(legal_action_meta_buffer))
    if count != used_rows:
        raise ValueError(
            "legal_action_meta rows must match used legal-id rows "
            f"(got {count}, expected {used_rows})"
        )
    return legal_action_meta_buffer[:count]


def cast_legal_ids(ids: np.ndarray, *, as_uint32: bool) -> np.ndarray:
    dtype = np.uint32 if as_uint32 else np.uint16
    return ids.astype(dtype, copy=False)


def cast_legal_offsets(offsets: np.ndarray) -> np.ndarray:
    return offsets.astype(np.uint32, copy=False)


def cast_legal_action_meta(meta: np.ndarray | None) -> np.ndarray | None:
    if meta is None:
        return None
    return meta.astype(np.uint16, copy=False)
