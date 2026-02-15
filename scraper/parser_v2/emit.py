from __future__ import annotations

import re
from typing import Any, Dict, Optional

from .conditions import (
    build_conditions,
    infer_effect_optional,
    infer_target_limit,
    infer_targets,
    infer_timing,
    kind_from_tag,
)
from .cost import default_cost
from .models import ParseContext, RuleMatch
from .registry import CompiledRule


def _effect_for_rule(
    rule: CompiledRule,
    body_text: str,
    groups: Dict[str, str],
) -> list[Any]:
    if rule.effect_mode == "draw_count":
        draw_count = 1
        raw = groups.get("draw_count") or groups.get("count")
        if raw is not None:
            try:
                draw_count = int(raw)
            except ValueError:
                draw_count = 1
        return [{"Draw": {"count": draw_count}}]

    if rule.effect_mode == "deal_damage_cancelable_true":
        amount = 1
        raw = groups.get("damage") or groups.get("amount")
        if raw is not None:
            try:
                amount = int(raw)
            except ValueError:
                amount = 1
        return [{"DealDamage": {"amount": amount, "cancelable": True}}]

    if rule.effect_mode == "move_to_hand":
        return ["MoveToHand"]

    if rule.effect_mode == "move_to_waiting_room":
        return ["MoveToWaitingRoom"]

    if rule.effect_mode == "add_power_self":
        amount = 0
        raw = groups.get("amount")
        if raw is not None:
            try:
                amount = int(raw)
            except ValueError:
                amount = 0
        return [{"AddPower": {"amount": amount, "duration_turn": False}}]

    if rule.effect_mode == "brainstorm_draw_approx":
        reveal_count = 4
        if groups.get("reveal_count"):
            try:
                reveal_count = int(groups["reveal_count"])
            except ValueError:
                reveal_count = 4
        else:
            match = re.search(r"\b(?:flip over|reveal)\s+(\d+)\s+cards?\b", body_text, re.I)
            if match:
                reveal_count = int(match.group(1))
        return [
            {
                "Brainstorm": {
                    "reveal_count": reveal_count,
                    "per_climax": 1,
                    "mode": "Draw",
                }
            }
        ]

    if rule.effect_mode == "cannot_side_attack":
        duration_turn = bool(re.search(r"until end of turn", body_text, re.I))
        return [{"CannotSideAttack": {"duration_turn": duration_turn}}]

    return [{"Draw": {"count": 0}}]


def emit_ability_def(
    context: ParseContext,
    rule: CompiledRule,
    groups: Dict[str, str],
    parsed_cost: Optional[Dict[str, Any]] = None,
    cost_supported: bool = True,
) -> Dict[str, Any]:
    body = context.line.body
    conditions_extra: Dict[str, Any] = {}
    if not cost_supported:
        conditions_extra["cost_parse_supported"] = False
    rule_match = RuleMatch(
        rule_id=rule.id,
        mode=rule.mode,
        priority=rule.priority,
        pattern=rule.pattern,
        metadata=dict(rule.metadata),
        groups=groups,
    )

    targets_override = rule.metadata.get("targets")
    if isinstance(targets_override, list) and all(
        isinstance(item, str) for item in targets_override
    ):
        targets = [str(item) for item in targets_override]
    else:
        targets = infer_targets(body)

    target_limit_override = rule.metadata.get("target_limit")
    if isinstance(target_limit_override, int):
        target_limit = target_limit_override
    else:
        target_limit = infer_target_limit(body)

    target_card_type = rule.metadata.get("target_card_type")
    if not isinstance(target_card_type, str):
        target_card_type = None

    return {
        "kind": rule.kind or kind_from_tag(context.line.tag),
        "timing": rule.timing or infer_timing(context.line.tag, body),
        "effects": _effect_for_rule(rule, body, groups),
        "effect_optional": infer_effect_optional(body),
        "targets": targets,
        "cost": parsed_cost or default_cost(),
        "conditions": build_conditions(rule_match.mode, rule_match.rule_id, extra=conditions_extra),
        "target_card_type": target_card_type,
        "target_trait": None,
        "target_level_max": None,
        "target_cost_max": None,
        "target_limit": target_limit,
    }
