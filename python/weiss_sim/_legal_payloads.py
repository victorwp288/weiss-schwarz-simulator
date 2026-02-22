from __future__ import annotations

from typing import Any, Callable

import numpy as np

LegalIdsIntoFn = Callable[[np.ndarray, np.ndarray], int]


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


def cast_legal_ids(ids: np.ndarray, *, as_uint32: bool) -> np.ndarray:
    dtype = np.uint32 if as_uint32 else np.uint16
    return ids.astype(dtype, copy=False)


def cast_legal_offsets(offsets: np.ndarray) -> np.ndarray:
    return offsets.astype(np.uint32, copy=False)
