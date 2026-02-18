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
from .cost import COUNT_TOKEN_RE, default_cost, parse_count_token
from .models import ParseContext, RuleMatch
from .registry import CompiledRule


def _parse_int(raw: Optional[str], default: int = 0) -> int:
    if raw is None:
        return default
    try:
        return int(raw)
    except ValueError:
        return default


def _parse_count(raw: Optional[str], default: int = 0) -> int:
    if raw is None:
        return default
    parsed = parse_count_token(raw)
    return default if parsed is None else parsed


def _zone_count_level_sum(min_total: int) -> Dict[str, Any]:
    return {
        "side": "SelfSide",
        "zone": "Level",
        "cmp": "AtLeastLevelSum",
        "value": int(min_total),
        "card_ids": [],
    }


def _conditional_add_power(
    amount: int,
    *,
    turn: Optional[str] = None,
    zone_count: Optional[Dict[str, Any]] = None,
    require_source_marker: bool = False,
    per_source_marker: bool = False,
    per_zone_count: bool = False,
    exclude_source: bool = False,
) -> Dict[str, Any]:
    return {
        "ConditionalAddPower": {
            "amount": int(amount),
            "turn": turn,
            "zone_count": zone_count,
            "require_source_marker": require_source_marker,
            "per_source_marker": per_source_marker,
            "per_zone_count": per_zone_count,
            "exclude_source": exclude_source,
            "target_ids": [],
        }
    }


def _conditional_add_soul(
    amount: int,
    *,
    turn: Optional[str] = None,
    zone_count: Optional[Dict[str, Any]] = None,
    require_source_marker: bool = False,
    per_source_marker: bool = False,
    per_zone_count: bool = False,
    exclude_source: bool = False,
) -> Dict[str, Any]:
    return {
        "ConditionalAddSoul": {
            "amount": int(amount),
            "turn": turn,
            "zone_count": zone_count,
            "require_source_marker": require_source_marker,
            "per_source_marker": per_source_marker,
            "per_zone_count": per_zone_count,
            "exclude_source": exclude_source,
            "target_ids": [],
        }
    }


def _conditional_cannot_side_attack(
    *,
    turn: Optional[str] = None,
    zone_count: Optional[Dict[str, Any]] = None,
    require_source_marker: bool = False,
    exclude_source: bool = False,
) -> Dict[str, Any]:
    return {
        "ConditionalCannotSideAttack": {
            "turn": turn,
            "zone_count": zone_count,
            "require_source_marker": require_source_marker,
            "exclude_source": exclude_source,
        }
    }


def _strip_nested_quote_prefix(text: str) -> str:
    cleaned = text.strip()
    cleaned = cleaned.replace("“", '"').replace("”", '"').replace("’", "'")
    cleaned = re.sub(r"^【(?:AUTO|CONT|ACT)】\s*", "", cleaned, flags=re.I)
    return re.sub(r"\s+", " ", cleaned).strip()


def _parse_flatten_effect(quoted_text: str) -> Optional[Any]:
    nested = _strip_nested_quote_prefix(quoted_text)
    if not nested:
        return None

    match = re.match(r"^This card gets\s*([+-]?\d+)\s*power\.?$", nested, re.I)
    if match:
        return {"AddPower": {"amount": int(match.group(1)), "duration_turn": False}}

    match = re.match(r"^This card gets\s*([+-]?\d+)\s*soul\.?$", nested, re.I)
    if match:
        return {"AddSoul": {"amount": int(match.group(1)), "duration_turn": False}}

    match = re.match(r"^This card gets\s*([+-]?\d+)\s*level\.?$", nested, re.I)
    if match:
        return {"AddLevel": {"amount": int(match.group(1)), "duration_turn": False}}

    if re.match(r"^This card cannot side attack\.?$", nested, re.I):
        return {"CannotSideAttack": {"duration_turn": False}}

    if re.match(r"^This card cannot frontal attack\.?$", nested, re.I):
        return {"CannotFrontalAttack": {"duration_turn": False}}

    if re.match(
        r"^This card cannot be chosen by your opponent's effects\.?$",
        nested,
        re.I,
    ):
        return {"CannotBeChosenByOpponentEffects": {"duration_turn": False}}

    if re.match(
        r"^This card cannot move to another position (?:of|on) the stage(?: and cannot be returned to hand)?\.?$",
        nested,
        re.I,
    ):
        return {"CannotMoveStagePosition": {"duration_turn": False}}

    if re.match(
        r'^During this card\'s battle, all players cannot play "Backup" from (?:their )?hands?\.?$',
        nested,
        re.I,
    ) or re.match(
        r'^During this card\'s battle, all players cannot play "Backup" from hand\.?$',
        nested,
        re.I,
    ):
        return {"CannotPlayBackupFromHand": {"duration_turn": False}}

    match = re.match(r"^Encore\s*\[\((\d+)\)\]\.?$", nested, re.I)
    if match:
        return {"EncoreStockCost": {"cost": int(match.group(1)), "duration_turn": False}}

    return None


def _build_granted_ability(effect: Any) -> Dict[str, Any]:
    effects: list[Any]
    if isinstance(effect, list):
        effects = effect
    else:
        effects = [effect]
    return {
        "kind": "Continuous",
        "timing": None,
        "effects": effects,
        "effect_optional": [],
        "targets": ["This"],
        "cost": default_cost(),
        "conditions": {},
        "target_card_type": None,
        "target_trait": None,
        "target_level_max": None,
        "target_cost_max": None,
        "target_limit": None,
    }


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

    if rule.effect_mode == "noop_draw_exact":
        return [{"Draw": {"count": 0}}]

    if rule.effect_mode == "look_top_card_top_or_waiting_room":
        return ["LookTopCardTopOrWaitingRoom"]

    if rule.effect_mode == "move_self_to_memory":
        return ["MoveToMemory"]

    if rule.effect_mode == "move_self_to_deck_bottom":
        return ["MoveToDeckBottom"]

    if rule.effect_mode == "terminal_result_from_group":
        result = (groups.get("result") or "").strip().lower()
        if result == "win":
            return [{"SetTerminalOutcome": {"outcome": "WinSelf"}}]
        if result == "lose":
            return [{"SetTerminalOutcome": {"outcome": "WinOpponent"}}]
        return [{"Draw": {"count": 0}}]

    if rule.effect_mode == "facing_opponent_quoted_restriction":
        nested = _parse_flatten_effect(groups.get("quoted") or "")
        if isinstance(nested, dict):
            add_soul = nested.get("AddSoul")
            if isinstance(add_soul, dict):
                return [{"FacingOpponentAddSoul": {"amount": int(add_soul.get("amount", 0))}}]
            if "CannotMoveStagePosition" in nested:
                return ["FacingOpponentCannotMoveStagePosition"]
            return [nested]
        return [{"Draw": {"count": 0}}]

    if rule.effect_mode == "on_play_power_following_quote":
        amount = _parse_int(groups.get("amount"), default=0)
        granted_effects: list[Any] = [{"AddPower": {"amount": amount, "duration_turn": False}}]
        nested = _parse_flatten_effect(groups.get("quoted") or "")
        if nested is not None:
            granted_effects.append(nested)
        return [
            {
                "GrantAbilityDef": {
                    "ability": _build_granted_ability(granted_effects),
                    "duration": "UntilEndOfOpponentsNextTurn",
                }
            }
        ]

    if rule.effect_mode == "marker_power_exact":
        power = _parse_int(groups.get("power"), default=0)
        return [
            _conditional_add_power(
                power,
                require_source_marker=True,
            )
        ]

    if rule.effect_mode == "marker_power_soul_exact":
        power = _parse_int(groups.get("power"), default=0)
        soul = _parse_int(groups.get("soul"), default=0)
        return [
            _conditional_add_power(
                power,
                require_source_marker=True,
            ),
            _conditional_add_soul(
                soul,
                require_source_marker=True,
            ),
        ]

    if rule.effect_mode == "marker_power_following_exact":
        power = _parse_int(groups.get("power"), default=0)
        effects: list[Any] = [
            _conditional_add_power(
                power,
                require_source_marker=True,
            )
        ]
        nested = _parse_flatten_effect(groups.get("quoted") or "")
        if isinstance(nested, dict) and "CannotSideAttack" in nested:
            effects.append(
                _conditional_cannot_side_attack(
                    require_source_marker=True,
                )
            )
        return effects

    if rule.effect_mode == "experience_power_with_optional_following_exact":
        min_total = _parse_count(groups.get("level_total"), default=0)
        power = _parse_int(groups.get("power"), default=0)
        turn = "SelfTurn" if groups.get("self_turn") else None
        zone_count = _zone_count_level_sum(min_total)
        effects: list[Any] = [
            _conditional_add_power(
                power,
                turn=turn,
                zone_count=zone_count,
            )
        ]
        nested = _parse_flatten_effect(groups.get("quoted") or "")
        if isinstance(nested, dict) and "CannotSideAttack" in nested:
            effects.append(
                _conditional_cannot_side_attack(
                    turn=turn,
                    zone_count=zone_count,
                )
            )
        return effects

    if rule.effect_mode == "climax_placed_buff_or_following_exact":
        effects: list[Any] = []
        if groups.get("power"):
            effects.append(
                {
                    "AddPower": {
                        "amount": _parse_int(groups.get("power"), default=0),
                        "duration_turn": True,
                    }
                }
            )
        if groups.get("soul"):
            effects.append(
                {
                    "AddSoul": {
                        "amount": _parse_int(groups.get("soul"), default=0),
                        "duration_turn": True,
                    }
                }
            )
        nested = _parse_flatten_effect(groups.get("quoted") or "")
        if nested is not None:
            effects.append(
                {
                    "GrantAbilityDef": {
                        "ability": _build_granted_ability(nested),
                        "duration": "UntilEndOfTurn",
                    }
                }
            )
        if effects:
            return effects
        return [{"Draw": {"count": 0}}]

    if rule.effect_mode == "paid_on_play_search_or_salvage_generic_exact":
        lower = body_text.lower()
        if "return it to your hand" in lower or "return them to your hand" in lower:
            return ["MoveToHand"]
        if "search your deck" in lower or "look at up to" in lower:
            return ["MoveToHand"]
        return [{"Draw": {"count": 0}}]

    if rule.effect_mode == "experience_generic_exact":
        fallback = re.search(
            rf"total level of (?:the )?cards in your level is ({COUNT_TOKEN_RE}) or higher,\s*this card gets \+(\d+) power",
            body_text,
            re.I,
        )
        if fallback:
            min_total = _parse_count(fallback.group(1), default=0)
            power = _parse_int(fallback.group(2), default=0)
            turn = (
                "SelfTurn" if re.search(r"experience during your turn", body_text, re.I) else None
            )
            return [
                _conditional_add_power(
                    power,
                    turn=turn,
                    zone_count=_zone_count_level_sum(min_total),
                )
            ]
        return [{"Draw": {"count": 0}}]

    if rule.effect_mode == "climax_placed_buff_generic_exact":
        match = re.search(r"gets \+(\d+) power", body_text, re.I)
        if match:
            return [{"AddPower": {"amount": int(match.group(1)), "duration_turn": True}}]
        return [{"Draw": {"count": 0}}]

    if rule.effect_mode == "on_reverse_self_move_generic_exact":
        lower = body_text.lower()
        if "put this card at the bottom of your deck" in lower:
            return ["MoveToDeckBottom"]
        if "put this card into your memory" in lower:
            return ["MoveToMemory"]
        return [{"Draw": {"count": 0}}]

    if rule.effect_mode == "add_power_generic_exact":
        amount = _parse_int(groups.get("amount"), default=0)
        if amount == 0:
            match = re.search(r"\+(\d+)\s*power", body_text, re.I)
            if match:
                amount = int(match.group(1))
        if amount == 0:
            return [{"Draw": {"count": 0}}]
        duration_turn = bool(
            re.search(
                r"until (?:the end of your opponent's next turn|end of turn)", body_text, re.I
            )
        )
        return [{"AddPower": {"amount": amount, "duration_turn": duration_turn}}]

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
    target_limit_group = rule.metadata.get("target_limit_group")
    if isinstance(target_limit_group, str):
        grouped_limit = parse_count_token(groups.get(target_limit_group, ""))
        if grouped_limit is not None:
            target_limit = grouped_limit

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
