from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path
from typing import Literal, Mapping, Sequence


RulesProfile = Literal["strict", "approx"]
RuntimeMode = Literal["speed", "eval_debug"]
CardPoolMode = Literal["parsed_only", "all"]
LegalRepr = Literal["ids_u16", "ids_u32", "mask_u8", "both"]
ObsDType = Literal["i16", "i32"]
IdsSafety = Literal["checked", "unsafe"]
NumLike = int | Literal["auto"]
ThreadsLike = int | Literal["auto"] | None
ErrorPolicy = Literal["strict", "lenient_terminate", "lenient_noop"]
ObservationVisibility = Literal["public", "full"]
SimultaneousLossPolicy = Literal["draw", "active_player_wins", "non_active_player_wins"]

DeckInput = Sequence[int] | Mapping[int | str, int] | str | Path


@dataclass(slots=True)
class CurriculumOverrides:
    allowed_card_sets: list[str] | None = None
    allow_character: bool | None = None
    allow_event: bool | None = None
    allow_climax: bool | None = None
    enable_clock_phase: bool | None = None
    enable_climax_phase: bool | None = None
    enable_side_attacks: bool | None = None
    enable_direct_attacks: bool | None = None
    enable_counters: bool | None = None
    enable_triggers: bool | None = None
    enable_trigger_soul: bool | None = None
    enable_trigger_draw: bool | None = None
    enable_trigger_shot: bool | None = None
    enable_trigger_bounce: bool | None = None
    enable_trigger_treasure: bool | None = None
    enable_trigger_gate: bool | None = None
    enable_trigger_standby: bool | None = None
    enable_on_reverse_triggers: bool | None = None
    enable_backup: bool | None = None
    enable_encore: bool | None = None
    enable_refresh_penalty: bool | None = None
    enable_level_up_choice: bool | None = None
    enable_activated_abilities: bool | None = None
    enable_continuous_modifiers: bool | None = None
    enable_approx_effects: bool | None = None
    enable_priority_windows: bool | None = None
    enable_visibility_policies: bool | None = None
    use_alternate_end_conditions: bool | None = None
    priority_autopick_single_action: bool | None = None
    priority_allow_pass: bool | None = None
    strict_priority_mode: bool | None = None
    reduced_stage_mode: bool | None = None
    enforce_color_requirement: bool | None = None
    enforce_cost_requirement: bool | None = None
    allow_concede: bool | None = None
    reveal_opponent_hand_stock_counts: bool | None = None
    memory_is_public: bool | None = None

    def to_dict(self) -> dict[str, object]:
        payload: dict[str, object] = {}
        for key, value in self.__dict__.items():
            if value is None:
                continue
            payload[key] = value
        return payload


@dataclass(slots=True)
class EndConditionOverrides:
    simultaneous_loss: SimultaneousLossPolicy | None = None
    allow_draw_on_simultaneous_loss: bool | None = None

    def to_dict(self) -> dict[str, object]:
        payload: dict[str, object] = {}
        for key, value in self.__dict__.items():
            if value is None:
                continue
            payload[key] = value
        return payload
