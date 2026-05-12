from __future__ import annotations

from dataclasses import dataclass
from typing import Literal

from .weiss_sim import (
    BatchOutMinimal,
    BatchOutMinimalI16,
    BatchOutMinimalI16LegalIds,
    BatchOutMinimalI16LegalIdsNoMeta,
    BatchOutMinimalNoMask,
    BatchOutTrajectory,
    BatchOutTrajectoryI16,
    BatchOutTrajectoryI16LegalIds,
    BatchOutTrajectoryNoMask,
)

LayoutName = Literal["mask", "nomask", "i16", "i16_legal_ids", "i16_legal_ids_nometa"]


@dataclass(frozen=True)
class LayoutSpec:
    suffix: str
    out_cls: type
    trajectory_out_cls: type
    has_masks: bool
    has_legal_ids: bool
    use_i16: bool


LAYOUT_SPECS: dict[LayoutName, LayoutSpec] = {
    "mask": LayoutSpec(
        suffix="",
        out_cls=BatchOutMinimal,
        trajectory_out_cls=BatchOutTrajectory,
        has_masks=True,
        has_legal_ids=False,
        use_i16=False,
    ),
    "nomask": LayoutSpec(
        suffix="_nomask",
        out_cls=BatchOutMinimalNoMask,
        trajectory_out_cls=BatchOutTrajectoryNoMask,
        has_masks=False,
        has_legal_ids=False,
        use_i16=False,
    ),
    "i16": LayoutSpec(
        suffix="_i16",
        out_cls=BatchOutMinimalI16,
        trajectory_out_cls=BatchOutTrajectoryI16,
        has_masks=True,
        has_legal_ids=False,
        use_i16=True,
    ),
    "i16_legal_ids": LayoutSpec(
        suffix="_i16_legal_ids",
        out_cls=BatchOutMinimalI16LegalIds,
        trajectory_out_cls=BatchOutTrajectoryI16LegalIds,
        has_masks=False,
        has_legal_ids=True,
        use_i16=True,
    ),
    "i16_legal_ids_nometa": LayoutSpec(
        suffix="_i16_legal_ids_nometa",
        out_cls=BatchOutMinimalI16LegalIdsNoMeta,
        trajectory_out_cls=BatchOutTrajectoryI16LegalIds,
        has_masks=False,
        has_legal_ids=True,
        use_i16=True,
    ),
}

LAYOUT_FLAGS: dict[LayoutName, tuple[bool, bool, bool]] = {
    "mask": (True, False, False),
    "nomask": (False, False, False),
    "i16": (True, True, False),
    "i16_legal_ids": (False, True, True),
    "i16_legal_ids_nometa": (False, True, True),
}

FLAGS_TO_LAYOUT: dict[tuple[bool, bool, bool], LayoutName] = {
    flags: layout for layout, flags in LAYOUT_FLAGS.items()
}
FLAGS_TO_LAYOUT[(False, True, True)] = "i16_legal_ids"


def normalize_layout(layout: str) -> LayoutName:
    layout_norm = str(layout).lower().strip()
    if layout_norm not in LAYOUT_SPECS:
        allowed = ", ".join(LAYOUT_SPECS)
        raise ValueError(f"unknown layout '{layout}' (expected {allowed})")
    return layout_norm  # type: ignore[return-value]


def pool_method_name(base_name: str, spec: LayoutSpec) -> str:
    if spec.suffix:
        return f"{base_name}{spec.suffix}"
    return base_name
