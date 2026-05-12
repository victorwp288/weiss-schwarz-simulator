import re
from typing import Any, Dict, List, Optional, Tuple

try:
    from scraper.parser_v2.engine import PARSER_VERSION_V2, parse_line as parse_line_v2
    from scraper.parser_v2.normalize import normalize_ability_line
except ModuleNotFoundError:
    from parser_v2.engine import PARSER_VERSION_V2, parse_line as parse_line_v2
    from parser_v2.normalize import normalize_ability_line

try:
    from scraper.ability_common import (
        APPROX_PROFILE_APPROX,
        APPROX_PROFILE_CLI_CHOICES,
        APPROX_PROFILE_STRICT,
        APPROX_PROFILES,
        COUNT_TOKEN_RE,
        RULE_MODE_APPROX,
        RULE_MODE_EXACT,
        AbilityParseStats,
        ParseRule,
        _ability_def,
        _template,
        ability_signature,
        normalize_approx_profile,
        parse_count_token,
        resolve_name_fragment_ids,
    )
    from scraper.ability_cost import cost_is_empty, parse_cost
    from scraper.ability_rules import ACT_RULES, AUTO_RULES, CONT_RULES
except ModuleNotFoundError:
    from ability_common import (
        APPROX_PROFILE_APPROX,
        APPROX_PROFILE_CLI_CHOICES,
        APPROX_PROFILE_STRICT,
        APPROX_PROFILES,
        COUNT_TOKEN_RE,
        RULE_MODE_APPROX,
        RULE_MODE_EXACT,
        AbilityParseStats,
        ParseRule,
        _ability_def,
        _template,
        ability_signature,
        normalize_approx_profile,
        parse_count_token,
        resolve_name_fragment_ids,
    )
    from ability_cost import cost_is_empty, parse_cost
    from ability_rules import ACT_RULES, AUTO_RULES, CONT_RULES

PARSER_VERSIONS = {PARSER_VERSION_V2}

__all__ = [
    "ACT_RULES",
    "APPROX_PROFILE_APPROX",
    "APPROX_PROFILE_CLI_CHOICES",
    "APPROX_PROFILE_STRICT",
    "APPROX_PROFILES",
    "AUTO_RULES",
    "CONT_RULES",
    "PARSER_VERSIONS",
    "RULE_MODE_APPROX",
    "RULE_MODE_EXACT",
    "AbilityParseStats",
    "ParseRule",
    "ability_signature",
    "cost_is_empty",
    "normalize_ability_line",
    "normalize_approx_profile",
    "parse_abilities",
    "parse_cost",
    "parse_count_token",
    "resolve_name_fragment_ids",
]


def parse_abilities(
    text: str,
    card_type: str,
    stats: AbilityParseStats,
    name_to_ids: Optional[Dict[str, List[int]]] = None,
    trait_map: Optional[Dict[str, int]] = None,
    approx_profile: str = APPROX_PROFILE_STRICT,
    trait_to_ids: Optional[Dict[str, List[int]]] = None,
    source_card_id: Optional[int] = None,
    parser_version: Optional[str] = None,
    emit_parse_trace: bool = False,
    _nested_parse_depth: int = 0,
) -> Tuple[List[Any], List[Any], bool]:
    approx_profile = normalize_approx_profile(approx_profile)
    allow_approx_effects = approx_profile == APPROX_PROFILE_APPROX
    abilities: List[Any] = []
    ability_defs: List[Any] = []
    counter_timing = False

    if not text:
        return abilities, ability_defs, counter_timing

    def rule_enabled(rule: ParseRule) -> bool:
        if rule.mode == RULE_MODE_EXACT:
            return True
        return approx_profile == APPROX_PROFILE_APPROX

    def with_approx_condition(conditions: Optional[Dict[str, Any]] = None) -> Dict[str, Any]:
        out: Dict[str, Any] = dict(conditions or {})
        out["requires_approx_effects"] = True
        return out

    def parse_turn_condition_prefix(
        text_in: str,
    ) -> Tuple[Optional[str], str]:
        lower = text_in.lower()
        if lower.startswith("during your turn, "):
            return "SelfTurn", text_in[len("During your turn, ") :].strip()
        if lower.startswith("during your opponent's turn, "):
            return "OpponentTurn", text_in[len("During your opponent's turn, ") :].strip()
        return None, text_in

    def strip_activation_cap_prefix(text_in: str, enabled: bool) -> str:
        if not enabled:
            return text_in
        out = text_in.strip()
        while True:
            match = re.match(
                rf"^This ability activates up to ({COUNT_TOKEN_RE}) time(?:s)? per turn\.\s*",
                out,
                re.I,
            )
            if not match:
                return out
            out = out[match.end() :].strip()

    def parse_cxcombo_climax_condition(clause: str) -> Optional[Dict[str, Any]]:
        lower = clause.lower()
        side = None
        area_phrase = ""
        if "your opponent's climax area" in lower:
            side = "Opponent"
            area_phrase = "your opponent's climax area"
        elif "your climax area" in lower:
            side = "SelfSide"
            area_phrase = "your climax area"
        if side is None:
            return None

        quoted_names = [name.strip() for name in re.findall(r'"([^"]+)"', clause) if name.strip()]
        card_ids: List[int] = []
        if quoted_names and name_to_ids:
            seen_ids = set()
            for name in quoted_names:
                for card_id in name_to_ids.get(name, []):
                    if card_id in seen_ids:
                        continue
                    seen_ids.add(card_id)
                    card_ids.append(card_id)

        if not quoted_names:
            # Ensure this is a "there is a climax" style condition, not arbitrary text that
            # merely contains the phrase "climax area".
            generic_check = lower.replace(area_phrase, " ")
            if "climax" not in generic_check:
                return None

        return {
            "climax_area": {
                "side": side,
                "card_ids": card_ids,
            }
        }

    def strip_cxcombo_condition_prefix(
        text_in: str, enabled: bool
    ) -> Tuple[str, Optional[Dict[str, Any]]]:
        if not enabled:
            return text_in, None
        out = text_in.strip()
        stripped_climax = False
        parsed_conditions: Optional[Dict[str, Any]] = None
        while True:
            changed = False
            climax = re.match(
                r"^if\s+([^,]*\bin your(?: opponent's)? climax area\b[^,]*),\s*",
                out,
                re.I,
            )
            if climax:
                cond = parse_cxcombo_climax_condition(climax.group(1).strip())
                if cond is not None:
                    parsed_conditions = cond
                out = out[climax.end() :].strip()
                stripped_climax = True
                changed = True
            elif stripped_climax:
                conj = re.match(r"^and\s+[^,]+,\s*", out, re.I)
                if conj:
                    out = out[conj.end() :].strip()
                    changed = True
            if not changed:
                break
        return out, parsed_conditions

    def parse_following_flatten_effect(
        nested_text: str,
        duration_turn: bool,
    ) -> Optional[Any]:
        nested = (
            nested_text.strip()
            .replace("“", '"')
            .replace("”", '"')
            .replace("’", "'")
            .replace("［", "[")
            .replace("］", "]")
        )
        nested = re.sub(r"^【(?:AUTO|CONT|ACT)】\s*", "", nested, flags=re.I)
        nested = re.sub(r"\s+", " ", nested).strip()

        match = re.match(
            r"^This card gets\s*([+-]?\d+)\s*power(?: until (?:the end of your opponent's next turn|end of turn))?\.?$",
            nested,
            re.I,
        )
        if match:
            return _template("AddPower", amount=int(match.group(1)), duration_turn=duration_turn)

        match = re.match(
            r"^This card gets\s*([+-]?\d+)\s*soul(?: until (?:the end of your opponent's next turn|end of turn))?\.?$",
            nested,
            re.I,
        )
        if match:
            return _template("AddSoul", amount=int(match.group(1)), duration_turn=duration_turn)

        match = re.match(
            r"^This card gets\s*([+-]?\d+)\s*level(?: until (?:the end of your opponent's next turn|end of turn))?\.?$",
            nested,
            re.I,
        )
        if match:
            return _template("AddLevel", amount=int(match.group(1)), duration_turn=duration_turn)

        if re.match(r"^This card cannot side attack\.?$", nested, re.I):
            return _template("CannotSideAttack", duration_turn=duration_turn)

        if re.match(r"^This card cannot frontal attack\.?$", nested, re.I):
            return _template("CannotFrontalAttack", duration_turn=duration_turn)

        if re.match(
            r"^This card cannot become 【REVERSE】(?: by your opponent's effects?| by (?:the )?【AUTO】 effects? of your opponent's characters?)?\.?$",
            nested,
            re.I,
        ):
            return _template("CannotBecomeReverse", duration_turn=duration_turn)

        if re.match(
            r"^This card cannot be chosen by your opponent's effects\.?$",
            nested,
            re.I,
        ):
            return _template(
                "CannotBeChosenByOpponentEffects",
                duration_turn=duration_turn,
            )

        if re.match(
            r"^This card cannot move to another position (?:of|on) the stage(?: and cannot be returned to hand)?\.?$",
            nested,
            re.I,
        ):
            return _template("CannotMoveStagePosition", duration_turn=duration_turn)

        if re.match(
            r"^This card cannot (?:【STAND】|stand) during your stand phase\.?$",
            nested,
            re.I,
        ):
            return _template("CannotStandDuringStandPhase", duration_turn=duration_turn)

        match = re.match(
            r"^(?:【AUTO】\s*)?Encore\s*\[\((\d+)\)\]\.?$",
            nested,
            re.I,
        )
        if match:
            return _template(
                "EncoreStockCost",
                cost=int(match.group(1)),
                duration_turn=duration_turn,
            )

        if re.match(
            r"^When this card's battle opponent becomes\s*【REVERSE】,\s*(?:you may )?put that character into (?:your opponent's|their) memory\.?$",
            nested,
            re.I,
        ):
            return _template("BattleOpponentMoveToMemoryOnReverse", duration_turn=duration_turn)

        match = re.match(
            r"^Look at the top card of your deck, and put it on the top of your deck or into your waiting room\.?$",
            nested,
            re.I,
        )
        if match:
            return _template("LookTopCardTopOrWaitingRoom")

        match = re.match(
            r"^Look at the top card of your deck, and put it on the top or at the bottom of your deck\.?$",
            nested,
            re.I,
        )
        if match:
            return _template("LookTopCardTopOrBottom")

        match = re.match(
            rf"^Look at up to ({COUNT_TOKEN_RE}) cards? from the top of your deck, and put them on the top of your deck in any order\.?$",
            nested,
            re.I,
        )
        if match:
            count = parse_count_token(match.group(1))
            if count is not None:
                return _template("LookTopDeckReorder", count=count)

        return None

    def parse_nested_ability_def_exact(nested_text: str) -> Optional[Dict[str, Any]]:
        if _nested_parse_depth >= 1:
            return None
        nested_line = (
            nested_text.strip()
            .replace("“", '"')
            .replace("”", '"')
            .replace("’", "'")
            .replace("［", "[")
            .replace("］", "]")
        )
        if not nested_line.startswith("【"):
            return None

        nested_stats = AbilityParseStats()
        nested_templates, nested_defs, _ = parse_abilities(
            nested_line,
            card_type,
            nested_stats,
            name_to_ids=name_to_ids,
            trait_map=trait_map,
            approx_profile=APPROX_PROFILE_STRICT,
            trait_to_ids=trait_to_ids,
            source_card_id=None,
            parser_version=parser_version,
            emit_parse_trace=False,
            _nested_parse_depth=_nested_parse_depth + 1,
        )
        if nested_stats.unsupported_signatures:
            return None
        if nested_templates:
            return None
        if len(nested_defs) != 1:
            return None
        return nested_defs[0]

    def parse_trigger_icon_condition(text_in: str) -> Optional[str]:
        icon_patterns = (
            ("Treasure", (r"\[\[treasure\.gif\]\]", r"\[treasure\]", r"\[icon:treasure\]")),
            ("Gate", (r"\[\[gate\.gif\]\]", r"\[gate\]", r"\[icon:gate\]")),
            ("Choice", (r"\[\[choice\.gif\]\]", r"\[choice\]", r"\[icon:choice\]")),
            ("Shot", (r"\[\[shot\.gif\]\]", r"\[shot\]", r"\[icon:shot\]")),
            ("Soul", (r"\[\[soul\.gif\]\]", r"\[soul\]", r"\[icon:soul\]")),
            ("Standby", (r"\[\[standby\.gif\]\]", r"\[standby\]", r"\[icon:standby\]")),
            ("Pool", (r"\[\[pool\.gif\]\]", r"\[pool\]", r"\[icon:pool\]")),
            ("Draw", (r"\[\[draw\.gif\]\]", r"\[draw\]", r"\[icon:draw\]")),
            (
                "Bounce",
                (
                    r"\[\[return\.gif\]\]",
                    r"\[\[bounce\.gif\]\]",
                    r"\[return\]",
                    r"\[bounce\]",
                    r"\[icon:return\]",
                    r"\[icon:bounce\]",
                ),
            ),
        )
        for icon_name, patterns in icon_patterns:
            if any(re.search(pattern, text_in, re.I) for pattern in patterns):
                return icon_name
        return None

    def parse_following_granted_ability_def(nested_text: str) -> Optional[Dict[str, Any]]:
        nested = (
            nested_text.strip()
            .replace("“", '"')
            .replace("”", '"')
            .replace("’", "'")
            .replace("［", "[")
            .replace("］", "]")
        )
        nested = re.sub(r"^【(?:AUTO|CONT|ACT)】\s*", "", nested, flags=re.I)
        nested = re.sub(r"\s+", " ", nested).strip()

        match = re.match(
            r"^When this card attacks, this card gets\s*([+-]?\d+)\s*power until end of turn\.?$",
            nested,
            re.I,
        )
        if match:
            return _ability_def(
                "Auto",
                "AttackDeclaration",
                effects=[_template("AddPower", amount=int(match.group(1)), duration_turn=True)],
                targets=["This"],
            )

        match = re.match(
            r"^When this card attacks, this card gets\s*([+-]?\d+)\s*soul until end of turn\.?$",
            nested,
            re.I,
        )
        if match:
            return _ability_def(
                "Auto",
                "AttackDeclaration",
                effects=[_template("AddSoul", amount=int(match.group(1)), duration_turn=True)],
                targets=["This"],
            )

        trigger_check_paid_salvage = re.match(
            rf"^\[(.+)\]\s*When this card\'s trigger check reveals a climax(?: with(?: [^,]+)? in its trigger icon)?, you may pay the cost\. If you do, choose (up to )?({COUNT_TOKEN_RE}) (.+?) in your waiting room, and return (?:it|them) to your hand\.?$",
            nested,
            re.I,
        )
        if trigger_check_paid_salvage:
            choose_count = parse_count_token(trigger_check_paid_salvage.group(3))
            selector_constraints = parse_generic_selector_constraints(
                trigger_check_paid_salvage.group(4).strip()
            )
            parsed_cost, cost_supported, _ = parse_cost(
                f"【AUTO】 [{trigger_check_paid_salvage.group(1).strip()}] cost context"
            )
            icon_condition = parse_trigger_icon_condition(nested)
            if "trigger icon" in nested.lower() and icon_condition is None:
                return None
            if (
                choose_count is not None
                and selector_constraints is not None
                and cost_supported
                and not cost_is_empty(parsed_cost)
            ):
                conditions: Dict[str, Any] = {"trigger_check_revealed_climax": True}
                if icon_condition is not None:
                    conditions["trigger_check_revealed_icon"] = icon_condition
                return _ability_def(
                    "Auto",
                    "TriggerResolution",
                    effects=[_template("MoveToHand")],
                    targets=["SelfWaitingRoom"],
                    cost=parsed_cost,
                    effect_optional=[True],
                    conditions=conditions,
                    target_card_type=selector_constraints["target_card_type"],
                    target_trait=selector_constraints["target_trait"],
                    target_level_max=selector_constraints["target_level_max"],
                    target_cost_max=selector_constraints["target_cost_max"],
                    target_card_ids=selector_constraints["target_card_ids"],
                    target_limit=choose_count,
                )

        trigger_check_paid_stock_power = re.match(
            rf"^\[(.+)\]\s*When this card\'s trigger check reveals a climax(?: with(?: [^,]+)? in its trigger icon)?, you may pay the cost\. If you do, put the top card of your deck into your stock, choose (up to )?({COUNT_TOKEN_RE}) of your characters(?: in battle)?, and that character gets \+([+-]?\d+) power until end of turn\.?$",
            nested,
            re.I,
        )
        if trigger_check_paid_stock_power:
            choose_count = parse_count_token(trigger_check_paid_stock_power.group(3))
            parsed_cost, cost_supported, _ = parse_cost(
                f"【AUTO】 [{trigger_check_paid_stock_power.group(1).strip()}] cost context"
            )
            icon_condition = parse_trigger_icon_condition(nested)
            if "trigger icon" in nested.lower() and icon_condition is None:
                return None
            if (
                choose_count is not None
                and choose_count >= 1
                and cost_supported
                and not cost_is_empty(parsed_cost)
            ):
                conditions = {"trigger_check_revealed_climax": True}
                if icon_condition is not None:
                    conditions["trigger_check_revealed_icon"] = icon_condition
                return _ability_def(
                    "Auto",
                    "TriggerResolution",
                    effects=[
                        _template("MoveToStock"),
                        _template(
                            "AddPower",
                            amount=int(trigger_check_paid_stock_power.group(4)),
                            duration_turn=True,
                        ),
                    ],
                    targets=["SelfDeckTop", "SelfStage"],
                    cost=parsed_cost,
                    effect_optional=[True],
                    conditions=conditions,
                    target_limit=choose_count,
                )

        encore_grant = re.match(
            r"^(?:【AUTO】\s*)?Encore\s*\[(.+)\]\.?$",
            nested,
            re.I,
        )
        if encore_grant:
            parsed_cost, cost_supported, _ = parse_cost(
                f"【AUTO】 [{encore_grant.group(1).strip()}] cost context"
            )
            if cost_supported and not cost_is_empty(parsed_cost):
                return _ability_def(
                    "Auto",
                    "Encore",
                    effects=[_template("Draw", count=0)],
                    targets=[],
                    cost=parsed_cost,
                )

        if re.match(
            r"^When this card's battle opponent becomes\s*【REVERSE】,\s*(?:you may )?put that character into (?:your opponent's|their) memory\.?$",
            nested,
            re.I,
        ):
            return _ability_def(
                "Auto",
                "BattleOpponentReverse",
                effects=[
                    _template(
                        "BattleOpponentMoveIf",
                        destination="Memory",
                        prelude=None,
                        max_level=None,
                        max_cost=None,
                        level_gt_opponent_level=False,
                    )
                ],
                targets=[],
            )

        battle_reverse_topdeck_stock = re.match(
            r"^When this card's battle opponent becomes\s*【REVERSE】,\s*(you may )?put the top card of your deck into your stock\.?$",
            nested,
            re.I,
        )
        if battle_reverse_topdeck_stock:
            optional = bool(battle_reverse_topdeck_stock.group(1))
            return _ability_def(
                "Auto",
                "BattleOpponentReverse",
                effects=[_template("MoveToStock")],
                targets=["SelfDeckTop"],
                effect_optional=[True] if optional else [],
                target_limit=1,
            )

        battle_reverse_bottom_deck = re.match(
            r"^When this card's battle opponent becomes\s*【REVERSE】,\s*(you may )?put that character (?:on|at) the bottom of (?:your opponent's|their|his or her) deck\.?$",
            nested,
            re.I,
        )
        if battle_reverse_bottom_deck:
            optional = bool(battle_reverse_bottom_deck.group(1))
            return _ability_def(
                "Auto",
                "BattleOpponentReverse",
                effects=[
                    _template(
                        "BattleOpponentMoveIf",
                        destination="DeckBottom",
                        prelude=None,
                        max_level=None,
                        max_cost=None,
                        level_gt_opponent_level=False,
                    )
                ],
                targets=[],
                effect_optional=[True] if optional else [],
            )

        battle_reverse_clock = re.match(
            r"^When this card's battle opponent becomes\s*【REVERSE】,\s*(you may )?(?:put the top card of (?:your opponent's|their|his or her) clock into (?:their|his or her) waiting room\. If you do, )?put that character into (?:your opponent's|their|his or her) clock\.?$",
            nested,
            re.I,
        )
        if battle_reverse_clock:
            optional = bool(battle_reverse_clock.group(1))
            prelude = (
                "OpponentClockTopToWaitingRoom"
                if "top card of" in nested.lower() and "clock into" in nested.lower()
                else None
            )
            return _ability_def(
                "Auto",
                "BattleOpponentReverse",
                effects=[
                    _template(
                        "BattleOpponentMoveIf",
                        destination="Clock",
                        prelude=prelude,
                        max_level=None,
                        max_cost=None,
                        level_gt_opponent_level=False,
                    )
                ],
                targets=[],
                effect_optional=[True] if optional else [],
            )

        battle_reverse_draw = re.match(
            r"^When this card's battle opponent becomes\s*【REVERSE】,\s*(you may )?draw (?:a|1) card\.?$",
            nested,
            re.I,
        )
        if battle_reverse_draw:
            optional = bool(battle_reverse_draw.group(1))
            return _ability_def(
                "Auto",
                "BattleOpponentReverse",
                effects=[_template("Draw", count=1)],
                targets=[],
                effect_optional=[True] if optional else [],
            )

        battle_reverse_damage = re.match(
            rf"^When this card's battle opponent becomes\s*【REVERSE】,\s*(you may )?deal ({COUNT_TOKEN_RE}) damage to your opponent\.?(?:\s*\([^)]*\))?$",
            nested,
            re.I,
        )
        if battle_reverse_damage:
            amount = parse_count_token(battle_reverse_damage.group(2))
            if amount is not None:
                optional = bool(battle_reverse_damage.group(1))
                cancelable = "cannot be canceled" not in nested.lower()
                return _ability_def(
                    "Auto",
                    "BattleOpponentReverse",
                    effects=[_template("DealDamage", amount=amount, cancelable=cancelable)],
                    targets=[],
                    effect_optional=[True] if optional else [],
                )

        battle_reverse_salvage = re.match(
            rf"^When this card's battle opponent becomes\s*【REVERSE】,\s*(you may )?choose (up to )?({COUNT_TOKEN_RE}) (.+?) in your waiting room, and return (?:it|them) to your hand\.?$",
            nested,
            re.I,
        )
        if battle_reverse_salvage:
            choose_count = parse_count_token(battle_reverse_salvage.group(3))
            selector_constraints = parse_generic_selector_constraints(
                battle_reverse_salvage.group(4).strip()
            )
            if choose_count is not None and selector_constraints is not None:
                optional = bool(battle_reverse_salvage.group(1)) or bool(
                    battle_reverse_salvage.group(2)
                )
                return _ability_def(
                    "Auto",
                    "BattleOpponentReverse",
                    effects=[_template("MoveToHand")],
                    targets=["SelfWaitingRoom"],
                    effect_optional=[True] if optional else [],
                    target_card_type=selector_constraints["target_card_type"],
                    target_trait=selector_constraints["target_trait"],
                    target_level_max=selector_constraints["target_level_max"],
                    target_cost_max=selector_constraints["target_cost_max"],
                    target_card_ids=selector_constraints["target_card_ids"],
                    target_limit=choose_count,
                )

        battle_reverse_search = re.match(
            rf"^When this card's battle opponent becomes\s*【REVERSE】,\s*(you may )?search your deck for up to ({COUNT_TOKEN_RE}) (.+?), reveal (?:it|them) to your opponent, put (?:it|them) into your hand, and shuffle your deck(?: afterwards)?\.?$",
            nested,
            re.I,
        )
        if battle_reverse_search:
            choose_count = parse_count_token(battle_reverse_search.group(2))
            selector_constraints = parse_generic_selector_constraints(
                battle_reverse_search.group(3).strip()
            )
            if choose_count is not None and selector_constraints is not None:
                optional = bool(battle_reverse_search.group(1))
                return _ability_def(
                    "Auto",
                    "BattleOpponentReverse",
                    effects=[_template("MoveToHand")],
                    targets=["SelfDeckTop"],
                    effect_optional=[True] if optional else [],
                    target_card_type=selector_constraints["target_card_type"],
                    target_trait=selector_constraints["target_trait"],
                    target_level_max=selector_constraints["target_level_max"],
                    target_cost_max=selector_constraints["target_cost_max"],
                    target_card_ids=selector_constraints["target_card_ids"],
                    target_limit=choose_count,
                )

        attack_trigger_checks = re.match(
            rf"^\[\(({COUNT_TOKEN_RE})\)\] When this card attacks, you may pay the cost\. If you do, during that attack, perform a trigger check ({COUNT_TOKEN_RE}) times? on the trigger step\.?$",
            nested,
            re.I,
        )
        if attack_trigger_checks:
            pay_cost = parse_count_token(attack_trigger_checks.group(1))
            trigger_count = parse_count_token(attack_trigger_checks.group(2))
            if pay_cost is not None and trigger_count is not None:
                return _ability_def(
                    "Auto",
                    "AttackDeclaration",
                    effects=[_template("SetTriggerCheckCount", count=trigger_count)],
                    targets=[],
                    cost={"stock": pay_cost},
                    effect_optional=[True],
                )

        if re.match(r"^This card cannot side attack\.?$", nested, re.I):
            return _ability_def(
                "Continuous",
                None,
                effects=[_template("CannotSideAttack", duration_turn=False)],
                targets=["This"],
            )

        if re.match(r"^This card cannot frontal attack\.?$", nested, re.I):
            return _ability_def(
                "Continuous",
                None,
                effects=[_template("CannotFrontalAttack", duration_turn=False)],
                targets=["This"],
            )

        if re.match(
            r"^This card cannot be chosen by your opponent's effects\.?$",
            nested,
            re.I,
        ):
            return _ability_def(
                "Continuous",
                None,
                effects=[_template("CannotBeChosenByOpponentEffects", duration_turn=False)],
                targets=["This"],
            )

        if re.match(
            r"^This card cannot become 【REVERSE】(?: by your opponent's effects?| by (?:the )?【AUTO】 effects? of your opponent's characters?)?\.?$",
            nested,
            re.I,
        ):
            return _ability_def(
                "Continuous",
                None,
                effects=[_template("CannotBecomeReverse", duration_turn=False)],
                targets=["This"],
            )

        if re.match(
            r"^This card cannot move to another position (?:of|on) the stage(?: and cannot be returned to hand)?\.?$",
            nested,
            re.I,
        ):
            return _ability_def(
                "Continuous",
                None,
                effects=[_template("CannotMoveStagePosition", duration_turn=False)],
                targets=["This"],
            )

        if re.match(
            r"^This card cannot (?:【STAND】|stand) during your stand phase\.?$",
            nested,
            re.I,
        ):
            return _ability_def(
                "Continuous",
                None,
                effects=[_template("CannotStandDuringStandPhase", duration_turn=False)],
                targets=["This"],
            )

        nested_exact = parse_nested_ability_def_exact(nested)
        if nested_exact is not None:
            return nested_exact

        return None

    def parse_following_effect_or_grant(
        nested_text: str,
        *,
        duration_turn: bool,
        grant_duration: Optional[str],
    ) -> Optional[Dict[str, Any]]:
        nested_effect = parse_following_flatten_effect(nested_text, duration_turn=duration_turn)
        if nested_effect is not None:
            if grant_duration and grant_duration != "UntilEndOfTurn":
                granted = _ability_def(
                    "Continuous",
                    None,
                    effects=[nested_effect],
                    targets=["This"],
                )
                return _template("GrantAbilityDef", ability=granted, duration=grant_duration)
            return nested_effect
        if grant_duration is None:
            return None
        nested_ability = parse_following_granted_ability_def(nested_text)
        if nested_ability is None:
            return None
        return _template(
            "GrantAbilityDef",
            ability=nested_ability,
            duration=grant_duration,
        )

    def parse_following_ability_grant(effect_text: str) -> Optional[Dict[str, Any]]:
        def grant_duration_from_phrase(duration_phrase: str) -> str:
            lowered = duration_phrase.lower()
            if "opponent's next turn" in lowered or "end of the next turn" in lowered:
                return "UntilEndOfOpponentsNextTurn"
            return "UntilEndOfTurn"

        duration_phrase_re = (
            r"(?:the end of your opponent's next turn|end of turn|of turn|the end of the next turn)"
        )

        def approx_noop(
            optional: bool, target_limit: Optional[int] = None
        ) -> Optional[Dict[str, Any]]:
            if not allow_approx_effects:
                return None
            return {
                "effect": _template("Draw", count=0),
                "effects": [_template("Draw", count=0)],
                "targets": [],
                "optional": optional,
                "target_limit": target_limit,
                "approx": True,
            }

        match = re.match(
            rf'^this card (?:gets|gains) \+([+-]?\d+) power and the following ability until ({duration_phrase_re})\.\s*"(.+)"\.?$',
            effect_text,
            re.I,
        )
        if match:
            duration_phrase = match.group(2)
            nested_effect = parse_following_effect_or_grant(
                match.group(3),
                duration_turn=True,
                grant_duration=grant_duration_from_phrase(duration_phrase),
            )
            if nested_effect is None:
                return approx_noop(False)
            add_power = _template("AddPower", amount=int(match.group(1)), duration_turn=True)
            return {
                "effect": add_power,
                "effects": [add_power, nested_effect],
                "targets": ["This", "This"],
                "optional": False,
                "target_limit": None,
                "approx": False,
            }

        match = re.match(
            rf'^this card (?:gets|gains) the following ability until ({duration_phrase_re})\.\s*"(.+)"\.?$',
            effect_text,
            re.I,
        )
        if match:
            duration_phrase = match.group(1)
            nested_text = match.group(2)
            nested_effect = parse_following_effect_or_grant(
                nested_text,
                duration_turn=True,
                grant_duration=grant_duration_from_phrase(duration_phrase),
            )
            if nested_effect is None:
                return approx_noop(False)
            return {
                "effect": nested_effect,
                "effects": [nested_effect],
                "targets": ["This"],
                "optional": False,
                "target_limit": None,
                "approx": False,
            }

        match = re.match(
            rf'^choose (up to )?({COUNT_TOKEN_RE}) of your (?:other )?characters(?: in battle)?, and that character gets the following ability until ({duration_phrase_re})\.\s*"(.+)"\.?$',
            effect_text,
            re.I,
        )
        if match:
            choose_count = parse_count_token(match.group(2))
            optional = bool(match.group(1))
            if choose_count is None:
                return None
            duration_phrase = match.group(3)
            nested_text = match.group(4)
            nested_effect = parse_following_effect_or_grant(
                nested_text,
                duration_turn=True,
                grant_duration=grant_duration_from_phrase(duration_phrase),
            )
            if nested_effect is None:
                return approx_noop(optional, target_limit=choose_count)
            return {
                "effect": nested_effect,
                "effects": [nested_effect],
                "targets": ["SelfStage"],
                "optional": optional,
                "target_limit": choose_count,
                "approx": False,
            }

        match = re.match(
            rf'^choose (up to )?({COUNT_TOKEN_RE}) of your opponent\'s characters(?: in battle)?, and that character gets the following ability until ({duration_phrase_re})\.\s*"(.+)"\.?$',
            effect_text,
            re.I,
        )
        if match:
            choose_count = parse_count_token(match.group(2))
            optional = bool(match.group(1))
            if choose_count is None:
                return None
            duration_phrase = match.group(3)
            nested_text = match.group(4)
            nested_effect = parse_following_effect_or_grant(
                nested_text,
                duration_turn=True,
                grant_duration=grant_duration_from_phrase(duration_phrase),
            )
            if nested_effect is None:
                return approx_noop(optional, target_limit=choose_count)
            return {
                "effect": nested_effect,
                "effects": [nested_effect],
                "targets": ["OppStage"],
                "optional": optional,
                "target_limit": choose_count,
                "approx": False,
            }

        match = re.match(
            r'^all of your characters get the following ability until end of turn\.\s*"(.+)"\.?$',
            effect_text,
            re.I,
        )
        if match:
            nested_text = match.group(1)
            nested_effect = parse_following_effect_or_grant(
                nested_text,
                duration_turn=True,
                grant_duration="UntilEndOfTurn",
            )
            if nested_effect is None:
                return approx_noop(False)
            return {
                "effect": nested_effect,
                "effects": [nested_effect],
                "targets": ["SelfStage"],
                "optional": False,
                "target_limit": None,
                "approx": False,
            }

        match = re.match(
            r'^all of your characters get the following ability\.\s*"(.+)"\.?$',
            effect_text,
            re.I,
        )
        if match:
            nested_effect = parse_following_effect_or_grant(
                match.group(1),
                duration_turn=False,
                grant_duration=None,
            )
            if nested_effect is None:
                return approx_noop(False)
            return {
                "effect": nested_effect,
                "effects": [nested_effect],
                "targets": ["SelfStage"],
                "optional": False,
                "target_limit": None,
                "approx": False,
            }

        # High-frequency quoted-effect grants that are not flattened exactly yet.
        if allow_approx_effects and re.search(
            r'(^all of your opponent\'s characters get\s*"[^"]+"\.?$)|(^the character facing this card gets\s*"[^"]+"\.?$)',
            effect_text,
            re.I,
        ):
            return approx_noop(False)

        # Generic fallback for remaining quoted following-ability text in approx profile.
        if allow_approx_effects and (
            "following ability" in effect_text.lower()
            or (re.search(r'"[^"]+"', effect_text) and re.search(r"\bgets\b", effect_text, re.I))
        ):
            optional = bool(re.search(r"\byou may\b|\bup to\b", effect_text, re.I))
            return approx_noop(optional)

        return None

    def parse_on_play_topdeck_search(effect_text: str) -> Optional[Dict[str, Any]]:
        match = re.match(
            rf"^look at up to ({COUNT_TOKEN_RE}) cards? from the top of your deck, choose up to ({COUNT_TOKEN_RE}) (.+?) from among them, (?:reveal (?:it|them) to your opponent, )?put (?:it|them) into your hand, and put the rest into your waiting room\.?$",
            effect_text,
            re.I,
        ) or re.match(
            rf"^look at up to ({COUNT_TOKEN_RE}) cards? from the top of your deck, choose up to ({COUNT_TOKEN_RE}) (.+?) from among them, and put (?:it|them) into your hand\. put the rest into your waiting room\.?$",
            effect_text,
            re.I,
        )
        if not match:
            return None

        look_count = parse_count_token(match.group(1))
        choose_count = parse_count_token(match.group(2))
        if look_count is None or choose_count is None or choose_count != 1:
            return {}

        selector = match.group(3).strip()
        selector_lower = selector.lower()
        target_card_ids = resolve_stage_selector_card_ids(selector) or []
        if re.search(r"\bor\b", selector_lower) and not re.search(
            r"\bor (?:lower|higher)\b", selector_lower
        ):
            if not target_card_ids:
                return {}
        if ('"' in selector or "named " in selector_lower) and not target_card_ids:
            return {}

        card_type_hint = None
        if "character" in selector_lower:
            card_type_hint = "Character"
        elif "event" in selector_lower:
            card_type_hint = "Event"
        elif "climax" in selector_lower:
            card_type_hint = "Climax"

        trait_id = None
        trait_matches = re.findall(r"《([^》]+)》", selector)
        if trait_matches:
            if len(trait_matches) == 1:
                trait_name = trait_matches[0].strip()
                trait_id = (trait_map or {}).get(trait_name)
                if trait_id is None and not target_card_ids:
                    return {}
            elif not target_card_ids:
                return {}

        target_level_max = None
        level_match = re.search(r"level (\d+) or lower", selector_lower)
        if level_match:
            target_level_max = int(level_match.group(1))
        elif " level " in f" {selector_lower} ":
            return {}

        target_cost_max = None
        cost_match = re.search(r"cost (\d+) or lower", selector_lower)
        if cost_match:
            target_cost_max = int(cost_match.group(1))
        elif " cost " in f" {selector_lower} ":
            return {}

        return {
            "look_count": look_count,
            "card_type_hint": card_type_hint,
            "trait_id": trait_id,
            "target_level_max": target_level_max,
            "target_cost_max": target_cost_max,
            "target_card_ids": target_card_ids,
        }

    def parse_card_type_hint(selector_text: str) -> Optional[str]:
        selector_lower = selector_text.lower()
        if "character" in selector_lower:
            return "Character"
        if "event" in selector_lower:
            return "Event"
        if "climax" in selector_lower:
            return "Climax"
        return None

    def resolve_selector_name_fragment_ids(selector_text: str) -> Optional[List[int]]:
        selector = selector_text.strip()
        if not selector:
            return None
        quoted = [frag.strip() for frag in re.findall(r'"([^"]+)"', selector) if frag.strip()]
        if not quoted:
            return None
        ids: set[int] = set()
        for frag in quoted:
            ids.update(resolve_name_fragment_ids(name_to_ids, frag))
        if not ids:
            return None
        return sorted(ids)

    def resolve_exact_quoted_name_ids(selector_text: str) -> List[int]:
        selector = selector_text.strip()
        if not selector:
            return []
        ids: set[int] = set()
        for quoted in re.findall(r'"([^"]+)"', selector):
            name = quoted.strip()
            if not name:
                continue
            ids.update((name_to_ids or {}).get(name, []))
        return sorted(ids)

    def resolve_stage_selector_card_ids(selector_text: str) -> Optional[List[int]]:
        selector = selector_text.strip()

        # Trait selectors.
        trait_names = [trait.strip() for trait in re.findall(r"《([^》]+)》", selector)]
        if trait_names:
            ids: set[int] = set()
            for trait_name in trait_names:
                ids.update((trait_to_ids or {}).get(trait_name, []))
            if not ids:
                return None
            return sorted(ids)

        # Name-fragment selectors.
        frag_match = re.search(
            r'with\s+"([^"]+)"\s+in\s+(?:its|their)\s+card\s+name',
            selector,
            re.I,
        )
        if frag_match and name_to_ids:
            fragment = frag_match.group(1).strip().casefold()
            ids: set[int] = set()
            for card_name, matched_ids in name_to_ids.items():
                if fragment and fragment in card_name.casefold():
                    ids.update(matched_ids)
            if not ids:
                return None
            return sorted(ids)

        # Generic quoted-name selectors.
        if re.search(r'\bwith\s+"[^"]+"\b', selector, re.I):
            quoted_ids = resolve_selector_name_fragment_ids(selector)
            if quoted_ids:
                return quoted_ids

        # Direct exact-name selectors (e.g., "Card Name").
        exact_ids = resolve_exact_quoted_name_ids(selector)
        if exact_ids:
            return exact_ids

        return None

    def resolve_single_trait_selector(selector_text: str) -> Optional[int]:
        selector = selector_text.strip()
        if not selector:
            return None
        if re.search(r"\bor\b", selector, re.I):
            return None
        traits = [trait.strip() for trait in re.findall(r"《([^》]+)》", selector)]
        if len(traits) != 1:
            return None
        return (trait_map or {}).get(traits[0])

    def all_known_card_ids() -> List[int]:
        ids: set[int] = set()
        for matched_ids in (name_to_ids or {}).values():
            for card_id in matched_ids:
                ids.add(card_id)
        return sorted(ids)

    def build_all_characters_trait_zone_count(
        trait_names: List[str],
    ) -> Optional[Dict[str, Any]]:
        if not trait_names:
            return None
        trait_card_ids: set[int] = set()
        for trait_name in trait_names:
            for card_id in (trait_to_ids or {}).get(trait_name, []):
                trait_card_ids.add(card_id)
        if not trait_card_ids:
            return None
        known_ids = set(all_known_card_ids())
        if not known_ids:
            return None
        non_matching_ids = sorted(known_ids - trait_card_ids)
        return {
            "side": "SelfSide",
            "zone": "Stage",
            "cmp": "AtMost",
            "value": 0,
            "card_ids": non_matching_ids,
        }

    def parse_generic_selector_constraints(selector_text: str) -> Optional[Dict[str, Any]]:
        selector = selector_text.strip()
        if not selector:
            return None
        selector_lower = selector.lower()

        target_card_ids = resolve_stage_selector_card_ids(selector) or []
        if re.search(r"\bor\b", selector_lower) and not re.search(
            r"\bor (?:lower|higher)\b", selector_lower
        ):
            if not target_card_ids:
                return None
        if ('"' in selector or "named " in selector_lower) and not target_card_ids:
            return None

        trait_id = resolve_single_trait_selector(selector)
        trait_matches = re.findall(r"《([^》]+)》", selector)
        if trait_matches and trait_id is None and not target_card_ids:
            return None

        target_level_max: Optional[int] = None
        level_match = re.search(r"level (\d+) or lower", selector_lower)
        if level_match:
            target_level_max = int(level_match.group(1))
        elif re.search(r"level\s+x", selector_lower):
            return None

        target_cost_max: Optional[int] = None
        cost_match = re.search(r"cost (\d+) or lower", selector_lower)
        if cost_match:
            target_cost_max = int(cost_match.group(1))
        elif " cost " in f" {selector_lower} ":
            return None

        card_type_hint = parse_card_type_hint(selector)
        if (
            card_type_hint is None
            and trait_id is None
            and target_level_max is None
            and target_cost_max is None
            and not target_card_ids
        ):
            return None

        return {
            "target_card_type": card_type_hint,
            "target_trait": trait_id,
            "target_level_max": target_level_max,
            "target_cost_max": target_cost_max,
            "target_card_ids": target_card_ids,
        }

    def infer_auto_timing_from_remainder(text_in: str) -> Optional[str]:
        lower = text_in.strip().lower()
        if lower.startswith(
            "when this card is placed on the stage from your hand"
        ) or lower.startswith("when this card is placed on stage from your hand"):
            return "OnPlay"
        if lower.startswith("when this card attacks") or lower.startswith(
            "when this card direct attacks"
        ):
            return "AttackDeclaration"
        if lower.startswith("when your other ") or lower.startswith("when another of your "):
            if " attacks" in lower:
                return "OtherAttackDeclaration"
        if lower.startswith("when this card's battle opponent becomes 【reverse】"):
            return "BattleOpponentReverse"
        if lower.startswith("when this card becomes 【reverse】"):
            return "OnReverse"
        if lower.startswith("when your climax is placed on your climax area") or lower.startswith(
            "when a climax is placed on your climax area"
        ):
            return "AfterClimaxPhase"
        if lower.startswith("at the beginning of your climax phase"):
            return "BeginClimaxPhase"
        if lower.startswith("at the beginning of your opponent's attack phase"):
            return "BeginAttackPhase"
        if lower.startswith("at the beginning of your main phase"):
            return "BeginMainPhase"
        if lower.startswith("at the beginning of your opponent's draw phase"):
            return "BeginDrawPhase"
        if lower.startswith("when you use an 【act】"):
            return "UseAct"
        return None

    def looks_like_search_or_salvage_text(text_in: str) -> bool:
        return bool(
            re.search(
                r"(search your deck|look at up to|return (?:it|them) to your hand|choose (?:up to )?.+ in your waiting room, and return)",
                text_in,
                re.I,
            )
        )

    def normalize_trace_value(value: Any) -> Any:
        if value is None or isinstance(value, (str, int, float, bool)):
            return value
        if isinstance(value, dict):
            return {str(k): normalize_trace_value(v) for k, v in value.items()}
        if isinstance(value, (list, tuple, set)):
            return [normalize_trace_value(v) for v in value]
        return str(value)

    def try_parser_v2_fallback(line_in: str) -> bool:
        if parser_version != PARSER_VERSION_V2:
            return False
        outcome = parse_line_v2(
            line_in,
            card_type=card_type,
            source_card_id=source_card_id,
            emit_trace=emit_parse_trace,
            allow_approx_rules=allow_approx_effects,
        )
        if not outcome.matched or outcome.ability_def is None:
            return False
        ability_defs.append(outcome.ability_def)
        stats.parsed_lines += 1
        matched_rule_id = outcome.rule_match.rule_id if outcome.rule_match else "unknown"
        stats.emitted_defs[f"ParserV2Fallback.{matched_rule_id}"] += 1
        if emit_parse_trace:
            stats.parse_traces.append(
                {
                    "source_card_id": source_card_id,
                    "line": line_in,
                    "matched_rule_id": matched_rule_id,
                    "trace": normalize_trace_value(outcome.trace),
                }
            )
        return True

    def mark_unsupported(line_in: str, allow_parser_v2: bool = True) -> None:
        if allow_parser_v2 and try_parser_v2_fallback(line_in):
            return
        stats.unsupported_signatures[ability_signature(line_in)] += 1

    lines = [line.strip() for line in text.split("\n") if line.strip()]
    for line in lines:
        line = normalize_ability_line(line)
        if not line.startswith("【"):
            continue
        has_cxcombo_tag = "【CXCOMBO】" in line.upper()
        stats.total_lines += 1

        if "【COUNTER】" in line and re.search(r"Backup\s+\d+", line, re.IGNORECASE):
            match = re.search(r"Backup\s+(\d+)", line, re.IGNORECASE)
            if match:
                power = int(match.group(1))
                abilities.append(_template("CounterBackup", power=power))
                counter_timing = True
                stats.parsed_lines += 1
                stats.emitted_templates["CounterBackup"] += 1
                continue
        if allow_approx_effects and re.match(
            r"^【COUNTER】\s*Put this card into your memory\.?$", line, re.I
        ):
            ability_defs.append(
                _ability_def(
                    "Auto",
                    "Counter",
                    effects=[_template("Draw", count=0)],
                    targets=[],
                    conditions=with_approx_condition(),
                )
            )
            counter_timing = True
            stats.parsed_lines += 1
            stats.emitted_defs["Counter.MoveSelfToMemory.ApproxNoop"] += 1
            continue
        if allow_approx_effects and re.match(
            r"^【COUNTER】\s*If you do not have a 《[^》]+》 character, this card cannot be played from your hand\..+$",
            line,
            re.I,
        ):
            ability_defs.append(
                _ability_def(
                    "Auto",
                    "Counter",
                    effects=[_template("Draw", count=0)],
                    targets=[],
                    conditions=with_approx_condition(),
                )
            )
            counter_timing = True
            stats.parsed_lines += 1
            stats.emitted_defs["Counter.ConditionalLookTop.ApproxNoop"] += 1
            continue

        cost, cost_supported, line_clean = parse_cost(line)
        line_for_fallback = re.sub(r"【CXCOMBO】\s*", "", line, flags=re.I).strip()

        change_exact = re.match(
            r'^【AUTO】\s*Change\s+\[[^\]]+\]\s+At the beginning of your climax phase,(?:\s*if [^,]+,)?\s*you may pay the cost\. If you do, choose (?:up to )?(?:1|one)?\s*(?:a card named\s*)?"([^"]+)" in your waiting room, and put it (?:on|in) the stage position that this card was (?:on|in)\.(?:\s*\([^)]*\))?$',
            line_for_fallback,
            re.I,
        )
        if change_exact and cost_supported:
            target_name = change_exact.group(1).strip()
            target_ids = sorted(name_to_ids.get(target_name, [])) if name_to_ids else []
            if not target_ids:
                mark_unsupported(line)
                continue
            ability_defs.append(
                _ability_def(
                    "Auto",
                    "BeginClimaxPhase",
                    effects=[
                        _template(
                            "MoveWaitingRoomCardToSourceSlot",
                            target_ids=target_ids,
                        ),
                    ],
                    targets=["SelfWaitingRoom"],
                    cost=cost,
                    effect_optional=[True],
                    target_limit=1,
                )
            )
            stats.parsed_lines += 1
            stats.emitted_defs["Auto.Change.NamedWaitingRoom.BeginClimaxPhase"] += 1
            continue

        change_draw_exact = re.match(
            r'^【AUTO】\s*Change\s+\[[^\]]+\]\s+At the beginning of your draw phase,(?:\s*if [^,]+,)?\s*you may pay the cost\. If you do, choose (?:up to )?(?:1|one)?\s*(?:a card named\s*)?"([^"]+)" in your waiting room, and put it (?:on|in) the stage position that this card was (?:on|in)\.(?:\s*\([^)]*\))?$',
            line_for_fallback,
            re.I,
        )
        if change_draw_exact and cost_supported:
            target_name = change_draw_exact.group(1).strip()
            target_ids = sorted(name_to_ids.get(target_name, [])) if name_to_ids else []
            if not target_ids:
                mark_unsupported(line)
                continue
            ability_defs.append(
                _ability_def(
                    "Auto",
                    "BeginDrawPhase",
                    effects=[
                        _template(
                            "MoveWaitingRoomCardToSourceSlot",
                            target_ids=target_ids,
                        ),
                    ],
                    targets=["SelfWaitingRoom"],
                    cost=cost,
                    effect_optional=[True],
                    target_limit=1,
                )
            )
            stats.parsed_lines += 1
            stats.emitted_defs["Auto.Change.NamedWaitingRoom.BeginDrawPhase"] += 1
            continue

        change_encore_exact = re.match(
            r'^【AUTO】\s*Change\s+\[[^\]]+\]\s+At the beginning of your encore step,(?:\s*if this card is 【REST】,\s*)?\s*you may pay the cost\. If you do, choose (?:up to )?(?:1|one)?\s*(?:a card named\s*)?"([^"]+)" in your waiting room, and put it (?:on|in) the stage position that this card was (?:on|in)\.(?:\s*\([^)]*\))?$',
            line_for_fallback,
            re.I,
        )
        if change_encore_exact and cost_supported:
            target_name = change_encore_exact.group(1).strip()
            target_ids = sorted(name_to_ids.get(target_name, [])) if name_to_ids else []
            if not target_ids:
                mark_unsupported(line)
                continue
            ability_defs.append(
                _ability_def(
                    "Auto",
                    "BeginEncoreStep",
                    effects=[
                        _template(
                            "MoveWaitingRoomCardToSourceSlot",
                            target_ids=target_ids,
                        ),
                    ],
                    targets=["SelfWaitingRoom"],
                    cost=cost,
                    effect_optional=[True],
                    target_limit=1,
                )
            )
            stats.parsed_lines += 1
            stats.emitted_defs["Auto.Change.NamedWaitingRoom.BeginEncoreStep"] += 1
            continue

        change_generic_named_waiting_room = re.match(
            r"^【AUTO】\s*Change\s+\[[^\]]+\]\s+At the beginning of (your climax phase|your draw phase|your encore step),(?:\s*if [^,]+,\s*)?\s*you may pay the cost\. If you do, choose (?:up to )?(?:1|one)?\s*(.+?)\s+in your waiting room, and put it (?:on|in) the stage position that this card was (?:on|in)\.(?:\s*\([^)]*\))?$",
            line_for_fallback,
            re.I,
        )
        if change_generic_named_waiting_room and cost_supported:
            timing_text = change_generic_named_waiting_room.group(1).strip().lower()
            timing = "BeginClimaxPhase"
            if "draw phase" in timing_text:
                timing = "BeginDrawPhase"
            elif "encore step" in timing_text:
                timing = "BeginEncoreStep"

            selector = change_generic_named_waiting_room.group(2).replace('""', '"').strip()
            target_ids = resolve_selector_name_fragment_ids(selector)
            if not target_ids:
                mark_unsupported(line)
                continue
            ability_defs.append(
                _ability_def(
                    "Auto",
                    timing,
                    effects=[
                        _template(
                            "MoveWaitingRoomCardToSourceSlot",
                            target_ids=target_ids,
                        ),
                    ],
                    targets=["SelfWaitingRoom"],
                    cost=cost,
                    effect_optional=[True],
                    target_limit=1,
                )
            )
            stats.parsed_lines += 1
            stats.emitted_defs["Auto.Change.NamedWaitingRoom.GenericTiming"] += 1
            continue

        paid_on_play_or_leave_salvage = re.match(
            rf"^【AUTO】\s*\[[^\]]+\]\s*When this card is placed on the stage from your hand or put into your waiting room from the stage, you may pay the cost\. If you do, choose (?:up to )?({COUNT_TOKEN_RE}) character in your waiting room, and return (?:it|them) to your hand\.?$",
            line_for_fallback,
            re.I,
        )
        if paid_on_play_or_leave_salvage and cost_supported:
            count = parse_count_token(paid_on_play_or_leave_salvage.group(1))
            if count is None or cost_is_empty(cost):
                mark_unsupported(line)
                continue
            ability_defs.append(
                _ability_def(
                    "Auto",
                    None,
                    effects=[_template("MoveToHand")],
                    targets=["SelfWaitingRoom"],
                    cost=cost,
                    effect_optional=[True],
                    target_card_type="Character",
                    target_limit=count,
                )
            )
            stats.parsed_lines += 1
            stats.emitted_defs["Auto.SalvageWaitingRoom.OnPlayOrLeaveStage.Paid"] += 1
            continue

        paid_reverse_burn_turn = re.match(
            rf"^【AUTO】\s*\[[^\]]+\]\s*During your turn, when this card's battle opponent becomes 【REVERSE】, you may pay the cost\. If you do, deal ({COUNT_TOKEN_RE}) damage to your opponent\. \(Damage may be canceled\)\.?$",
            line_for_fallback,
            re.I,
        )
        if paid_reverse_burn_turn and cost_supported:
            amount = parse_count_token(paid_reverse_burn_turn.group(1))
            if amount is None or cost_is_empty(cost):
                mark_unsupported(line)
                continue
            ability_defs.append(
                _ability_def(
                    "Auto",
                    "BattleOpponentReverse",
                    effects=[_template("DealDamage", amount=amount, cancelable=True)],
                    targets=[],
                    cost=cost,
                    conditions={"turn": "SelfTurn"},
                )
            )
            stats.parsed_lines += 1
            stats.emitted_defs["Auto.DealDamage.BattleOpponentReverse.Paid.SelfTurn"] += 1
            continue

        paid_draw_on_damage_canceled = re.match(
            rf"^【AUTO】\s*\[[^\]]+\]\s*When damage dealt by this card is canceled, you may pay the cost\. If you do, draw ({COUNT_TOKEN_RE}) card(?:s)?\.?$",
            line_for_fallback,
            re.I,
        )
        if paid_draw_on_damage_canceled and cost_supported:
            draw_count = parse_count_token(paid_draw_on_damage_canceled.group(1))
            if draw_count is None or cost_is_empty(cost):
                mark_unsupported(line)
                continue
            ability_defs.append(
                _ability_def(
                    "Auto",
                    "DamageDealtCanceled",
                    effects=[_template("Draw", count=draw_count)],
                    targets=[],
                    cost=cost,
                    effect_optional=[True],
                )
            )
            stats.parsed_lines += 1
            stats.emitted_defs["Auto.Draw.OnDamageDealtCanceled.Paid"] += 1
            continue

        on_play_or_act_heal_clock = re.match(
            r'^【AUTO】\s*When this card is placed on the stage from your hand or by the 【ACT】 effect of "[^"]+", you may put the top card of your clock into your waiting room\.?$',
            line_for_fallback,
            re.I,
        )
        if on_play_or_act_heal_clock:
            ability_defs.append(
                _ability_def(
                    "Auto",
                    "OnPlay",
                    effects=[_template("MoveToWaitingRoom")],
                    targets=["SelfClock"],
                    effect_optional=[True],
                    target_limit=1,
                )
            )
            stats.parsed_lines += 1
            stats.emitted_defs["Auto.MoveTopClockToWaitingRoom.OnPlayOrAct"] += 1
            continue

        attack_named_climax_salvage = re.match(
            rf'^【AUTO】\s*When this card attacks, if a card named "([^"]+)" is in your climax area, you may choose(?: up to)?(?: ({COUNT_TOKEN_RE}|a|an|one))? character in your waiting room, and return (?:it|them) to your hand\.?$',
            line_for_fallback,
            re.I,
        )
        if attack_named_climax_salvage:
            choose_token = attack_named_climax_salvage.group(2) or "1"
            choose_count = parse_count_token(choose_token)
            if choose_count is None:
                mark_unsupported(line)
                continue
            climax_name = attack_named_climax_salvage.group(1).strip()
            named_ids = sorted(name_to_ids.get(climax_name, [])) if name_to_ids else []
            ability_defs.append(
                _ability_def(
                    "Auto",
                    "AttackDeclaration",
                    effects=[_template("MoveToHand")],
                    targets=["SelfWaitingRoom"],
                    target_limit=choose_count,
                    target_card_type="Character",
                    effect_optional=[True],
                    conditions={"climax_area": {"side": "SelfSide", "card_ids": named_ids}},
                )
            )
            stats.parsed_lines += 1
            stats.emitted_defs["Auto.Salvage.IfNamedClimax.AttackDeclaration"] += 1
            continue

        paid_self_memory_inline = re.match(
            r"^【AUTO】\s*\[[^\]]+\]\s*When this card becomes 【REVERSE】 in battle, you may pay the cost\. If you do, put this card into your memory\.?$",
            line_for_fallback,
            re.I,
        )
        if paid_self_memory_inline and cost_supported:
            if cost_is_empty(cost):
                mark_unsupported(line)
                continue
            ability_defs.append(
                _ability_def(
                    "Auto",
                    "OnReverse",
                    effects=[_template("MoveToMemory")],
                    targets=["This"],
                    cost=cost,
                    effect_optional=[True],
                )
            )
            stats.parsed_lines += 1
            stats.emitted_defs["Auto.MoveSelfToMemory.OnReverse.Paid"] += 1
            continue

        if allow_approx_effects and re.match(
            r"^【AUTO】\s*Change\s+\[[^\]]+\]\s+At the beginning of your climax phase,\s+.+$",
            line_for_fallback,
            re.I,
        ):
            ability_defs.append(
                _ability_def(
                    "Auto",
                    "BeginClimaxPhase",
                    effects=[_template("Draw", count=0)],
                    targets=[],
                    conditions=with_approx_condition(),
                    effect_optional=[True],
                )
            )
            stats.parsed_lines += 1
            stats.emitted_defs["Auto.Change.BeginClimaxPhase.ApproxNoop"] += 1
            continue

        if not cost_supported:
            change_exact = re.match(
                r'^【AUTO】\s*Change\s+\[([^\]]+)\]\s+At the beginning of your climax phase,(?:\s*if [^,]+,)?\s*you may pay the cost\. If you do, choose (?:up to )?(?:1|one)?\s*(?:a card named\s*)?"([^"]+)" in your waiting room, and put it (?:on|in) the stage position that this card was (?:on|in)\.(?:\s*\([^)]*\))?$',
                line_for_fallback,
                re.I,
            )
            if change_exact:
                raw_cost_segment = change_exact.group(1).strip()
                move_effect: Optional[Any] = None
                move_patterns = [
                    (r"put this card into your waiting room", _template("MoveToWaitingRoom")),
                    (r"return this card to your hand", _template("MoveToHand")),
                    (r"put this card into your stock", _template("MoveToStock")),
                    (r"put this card into your clock", _template("MoveToClock")),
                    (r"put this card at the bottom of your deck", _template("MoveToDeckBottom")),
                ]
                remaining_cost = raw_cost_segment
                for pattern, effect_template in move_patterns:
                    if re.search(pattern, remaining_cost, re.I):
                        move_effect = effect_template
                        remaining_cost = re.sub(pattern, " ", remaining_cost, flags=re.I)
                        break
                if move_effect is None:
                    mark_unsupported(line)
                    continue

                parsed_cost, parsed_cost_supported, _ = parse_cost(
                    f"【AUTO】 [{remaining_cost.strip()}] cost context."
                )
                if not parsed_cost_supported:
                    mark_unsupported(line)
                    continue
                target_name = change_exact.group(2).strip()
                target_ids = sorted(name_to_ids.get(target_name, [])) if name_to_ids else []
                if not target_ids:
                    mark_unsupported(line)
                    continue
                ability_defs.append(
                    _ability_def(
                        "Auto",
                        "BeginClimaxPhase",
                        effects=[
                            move_effect,
                            _template(
                                "MoveWaitingRoomCardToSourceSlot",
                                target_ids=target_ids,
                            ),
                        ],
                        targets=["This", "SelfWaitingRoom"],
                        cost=parsed_cost,
                        effect_optional=[True],
                        target_limit=1,
                    )
                )
                stats.parsed_lines += 1
                stats.emitted_defs["Auto.Change.NamedWaitingRoom.BeginClimaxPhase"] += 1
                continue

            if allow_approx_effects:
                if re.match(
                    r"^【AUTO】\s*Change\s+\[[^\]]+\]\s+At the beginning of your climax phase,\s+.+$",
                    line_for_fallback,
                    re.I,
                ):
                    ability_defs.append(
                        _ability_def(
                            "Auto",
                            "BeginClimaxPhase",
                            effects=[_template("Draw", count=0)],
                            targets=[],
                            conditions=with_approx_condition(),
                            effect_optional=[True],
                        )
                    )
                    stats.parsed_lines += 1
                    stats.emitted_defs["Auto.Change.BeginClimaxPhase.ApproxNoop"] += 1
                    continue
                # Cost strings with unsupported stage-sacrifice clauses are common for
                # on-play search/salvage lines. Keep strict mode unsupported, but
                # provide deterministic RL-only no-op coverage.
                if re.match(
                    r"^【AUTO】\s*\[[^\]]+\]\s*When this card is placed on (?:the )?stage from your hand(?: or [^,]+)?,\s*you may pay the cost\. If you do,\s*(?:search|look at|choose).+$",
                    line_for_fallback,
                    re.I,
                ):
                    ability_defs.append(
                        _ability_def(
                            "Auto",
                            "OnPlay",
                            effects=[_template("Draw", count=0)],
                            targets=[],
                            conditions=with_approx_condition(),
                            effect_optional=[True],
                        )
                    )
                    stats.parsed_lines += 1
                    stats.emitted_defs[
                        "Auto.OnPlay.PaidSearchSalvage.UnsupportedCost.ApproxNoop"
                    ] += 1
                    continue
                if re.match(
                    r'^【AUTO】\s*\[[^\]]+\]\s*When you use this card\'s "[^"]+",\s*you may pay the cost\. If you do,\s*choose .+ of your opponent\'s characters with level higher than your opponent\'s level, and put it into your waiting room\.?$',
                    line_for_fallback,
                    re.I,
                ):
                    ability_defs.append(
                        _ability_def(
                            "Auto",
                            "UseAct",
                            effects=[_template("Draw", count=0)],
                            targets=[],
                            conditions=with_approx_condition(),
                            effect_optional=[True],
                        )
                    )
                    stats.parsed_lines += 1
                    stats.emitted_defs[
                        "Auto.UseThisCard.LevelHigherThanOpponent.ToWaitingRoom.ApproxNoop"
                    ] += 1
                    continue
                if re.match(
                    r"^【AUTO】\s*\[Return [^\]]+ from your waiting room to your deck & Shuffle your deck\]\s*When this card is placed on (?:the )?stage from your hand, you may pay the cost\. If you do, this card's soul does not decrease by side attacking until end of turn\.?$",
                    line_for_fallback,
                    re.I,
                ):
                    ability_defs.append(
                        _ability_def(
                            "Auto",
                            "OnPlay",
                            effects=[_template("Draw", count=0)],
                            targets=[],
                            conditions=with_approx_condition(),
                            effect_optional=[True],
                        )
                    )
                    stats.parsed_lines += 1
                    stats.emitted_defs["Auto.NoSideSoulLoss.OnPlay.Paid.ApproxNoop"] += 1
                    continue
                if re.search(
                    r'^【AUTO】\s*\[[^\]]+\]\s*When you use this card\'s "[^"]+",',
                    line_for_fallback,
                    re.I,
                ):
                    ability_defs.append(
                        _ability_def(
                            "Auto",
                            "UseAct",
                            effects=[_template("Draw", count=0)],
                            targets=[],
                            conditions=with_approx_condition(),
                            effect_optional=[True],
                        )
                    )
                    stats.parsed_lines += 1
                    stats.emitted_defs["Auto.UseThisCard.UnsupportedCost.ApproxNoop"] += 1
                    continue
            mark_unsupported(line)
            continue
        line_clean = re.sub(r"【CXCOMBO】\s*", "", line_clean, flags=re.I).strip()

        if line_clean.startswith("【CONT】"):
            remainder = line_clean[len("【CONT】") :].strip()

            match = re.match(
                r"^(?:\[ICON:LINK\]\s*)?Super Dimension Venus\.?$",
                remainder,
                re.I,
            )
            if match:
                # Link keyword helper text has no direct runtime effect in the current engine model.
                stats.parsed_lines += 1
                stats.emitted_defs["Continuous.Keyword.Link.SuperDimensionVenus"] += 1
                continue

            if "【" in remainder and "following ability" not in remainder.lower():
                allowed_embedded_icon = (
                    re.match(
                        r'^All of your opponent\'s characters get\s*"[^"]+"\.?$',
                        remainder,
                        re.I,
                    )
                    or re.match(
                        r'^All of your characters get the following ability\.\s*"[^"]+"\.?$',
                        remainder,
                        re.I,
                    )
                    or re.match(
                        r'^All of your other characters get the following ability\.\s*"[^"]+"\.?$',
                        remainder,
                        re.I,
                    )
                    or re.match(
                        r'^The character facing this card gets\s*"[^"]+"\.?$',
                        remainder,
                        re.I,
                    )
                    or re.match(
                        r'^All of your other "[^"]+" get \+[+-]?\d+ power and "[^"]+"\.?$',
                        remainder,
                        re.I,
                    )
                    or re.match(
                        r"^If the character facing this card is cost \d+ or lower, this card cannot become 【REVERSE】\.?$",
                        remainder,
                        re.I,
                    )
                    or re.match(
                        r'^If you do not have another character with "[^"]+" in its card name, this card cannot 【STAND】 during your stand phase\.?$',
                        remainder,
                        re.I,
                    )
                )
                if not allowed_embedded_icon:
                    mark_unsupported(line)
                    continue

            match = re.match(
                r"^You can put any number of cards with the same card name as this card into your deck\.?$",
                remainder,
                re.I,
            )
            if match:
                # Deck-construction rule text has no runtime battle effect.
                stats.parsed_lines += 1
                stats.emitted_defs["Continuous.DeckConstruction.SameNameRule"] += 1
                continue

            match = re.match(
                rf"^You can put up to ({COUNT_TOKEN_RE}) cards with the same card name as this card into your deck\.?$",
                remainder,
                re.I,
            )
            if match:
                # Deck-construction rule text has no runtime battle effect.
                if parse_count_token(match.group(1)) is None:
                    mark_unsupported(line)
                    continue
                stats.parsed_lines += 1
                stats.emitted_defs["Continuous.DeckConstruction.SameNameRule"] += 1
                continue

            match = re.match(
                rf"^You can put up to ({COUNT_TOKEN_RE}) cards with the same card name as this card in your deck\.?$",
                remainder,
                re.I,
            )
            if match:
                # Deck-construction rule text has no runtime battle effect.
                if parse_count_token(match.group(1)) is None:
                    mark_unsupported(line)
                    continue
                stats.parsed_lines += 1
                stats.emitted_defs["Continuous.DeckConstruction.SameNameRule"] += 1
                continue

            match = re.match(
                rf'^You can put up to ({COUNT_TOKEN_RE}) cards with the same card name as this card and cards named "([^"]+)" in your deck\.?$',
                remainder,
                re.I,
            )
            if match:
                if parse_count_token(match.group(1)) is None:
                    mark_unsupported(line)
                    continue
                # Deck-construction rule text has no runtime battle effect.
                stats.parsed_lines += 1
                stats.emitted_defs["Continuous.DeckConstruction.SameNameRule"] += 1
                continue

            match = re.match(
                r"^This card gets ([+-]?\d+) level while (?:on|in) the stage\.?$",
                remainder,
                re.I,
            )
            if match:
                ability_defs.append(
                    _ability_def(
                        "Continuous",
                        None,
                        effects=[
                            _template(
                                "AddLevel",
                                amount=int(match.group(1)),
                                duration_turn=False,
                            )
                        ],
                        targets=["This"],
                    )
                )
                stats.parsed_lines += 1
                stats.emitted_defs["Continuous.AddLevel.OnStage"] += 1
                continue

            match = re.match(
                rf"^(?:If your waiting room has ({COUNT_TOKEN_RE}) or less climax(?: cards?)?|If the number of climax in your waiting room is ({COUNT_TOKEN_RE}) or less), this card gets -(\d+) level while in your hand\.?$",
                remainder,
                re.I,
            )
            if match:
                threshold_token = match.group(1) or match.group(2)
                threshold = parse_count_token(threshold_token)
                if threshold is None:
                    mark_unsupported(line)
                    continue
                ability_defs.append(
                    _ability_def(
                        "Continuous",
                        None,
                        effects=[],
                        targets=[],
                        conditions={
                            "hand_level_delta": -int(match.group(3)),
                            "self_waiting_room_climax_at_most": threshold,
                        },
                    )
                )
                stats.parsed_lines += 1
                stats.emitted_defs["Continuous.HandLevelDelta.WaitingRoomClimaxAtMost"] += 1
                continue

            match = re.match(
                r'^If (?:"([^"]+)"|a card named "([^"]+)") is in your clock, this card gets -(\d+) level while in your hand\.?$',
                remainder,
                re.I,
            )
            if match:
                card_name = (match.group(1) or match.group(2) or "").strip()
                target_ids = sorted(set((name_to_ids or {}).get(card_name, [])))
                if not target_ids:
                    mark_unsupported(line)
                    continue
                ability_defs.append(
                    _ability_def(
                        "Continuous",
                        None,
                        effects=[],
                        targets=[],
                        conditions={
                            "hand_level_delta": -int(match.group(3)),
                            "self_clock_card_ids_any": target_ids,
                        },
                    )
                )
                stats.parsed_lines += 1
                stats.emitted_defs["Continuous.HandLevelDelta.ClockContainsNamed"] += 1
                continue

            match = re.match(
                r"^If your opponent has a level (\d+) or higher character, this card gets -(\d+) level while in your hand\.?$",
                remainder,
                re.I,
            )
            if match:
                ability_defs.append(
                    _ability_def(
                        "Continuous",
                        None,
                        effects=[],
                        targets=[],
                        conditions={
                            "hand_level_delta": -int(match.group(2)),
                            "opponent_stage_has_level_at_least": int(match.group(1)),
                        },
                    )
                )
                stats.parsed_lines += 1
                stats.emitted_defs["Continuous.HandLevelDelta.OpponentStageHasLevelAtLeast"] += 1
                continue

            match = re.match(
                r"^This card can be played from your hand without fulfilling color requirements\.?$",
                remainder,
                re.I,
            )
            if match:
                ability_defs.append(
                    _ability_def(
                        "Continuous",
                        None,
                        effects=[],
                        targets=[],
                        conditions={"ignore_color_requirement": True},
                    )
                )
                stats.parsed_lines += 1
                stats.emitted_defs["Continuous.IgnoreColorRequirement"] += 1
                continue
            match = re.match(
                r"^All of your characters get \+(\d+) power and \+(\d+) soul\.(?:\s*\(.+\))?$",
                remainder,
                re.I,
            )
            if match:
                ability_defs.append(
                    _ability_def(
                        "Continuous",
                        None,
                        effects=[
                            _template("AddPower", amount=int(match.group(1)), duration_turn=False),
                            _template("AddSoul", amount=int(match.group(2)), duration_turn=False),
                        ],
                        targets=["SelfStage", "SelfStage"],
                    )
                )
                stats.parsed_lines += 1
                stats.emitted_defs["Continuous.AddPowerSoul.AllCharacters"] += 1
                continue

            match = re.match(
                r"^All of your characters get \+(\d+) soul\.?$",
                remainder,
                re.I,
            )
            if match:
                ability_defs.append(
                    _ability_def(
                        "Continuous",
                        None,
                        effects=[
                            _template("AddSoul", amount=int(match.group(1)), duration_turn=False)
                        ],
                        targets=["SelfStage"],
                    )
                )
                stats.parsed_lines += 1
                stats.emitted_defs["Continuous.AddSoul.AllCharacters"] += 1
                continue

            match = re.match(
                r'^Memory If "([^"]+)" is in your memory, this card gets \+([+-]?\d+) power\.?$',
                remainder,
                re.I,
            )
            if match:
                named = match.group(1).strip()
                named_ids = sorted(set((name_to_ids or {}).get(named, [])))
                if not named_ids:
                    if allow_approx_effects:
                        ability_defs.append(
                            _ability_def(
                                "Continuous",
                                None,
                                effects=[
                                    _template(
                                        "AddPower",
                                        amount=int(match.group(2)),
                                        duration_turn=False,
                                    )
                                ],
                                targets=["This"],
                                conditions=with_approx_condition(),
                            )
                        )
                        stats.parsed_lines += 1
                        stats.emitted_defs["Continuous.MemoryPower.Named.Approx"] += 1
                    else:
                        mark_unsupported(line)
                    continue
                ability_defs.append(
                    _ability_def(
                        "Continuous",
                        None,
                        effects=[
                            _template(
                                "AddPower",
                                amount=int(match.group(2)),
                                duration_turn=False,
                            )
                        ],
                        targets=["This"],
                        conditions={"self_memory_card_ids_any": named_ids},
                    )
                )
                stats.parsed_lines += 1
                stats.emitted_defs["Continuous.MemoryPower.Named"] += 1
                continue

            match = re.match(
                r"^Assist All of your (?:other )?characters in front of this card gets? \+(\d+) power\.?$",
                remainder,
                re.I,
            )
            if match:
                ability_defs.append(
                    _ability_def(
                        "Continuous",
                        None,
                        effects=[
                            _template("AddPower", amount=int(match.group(1)), duration_turn=False)
                        ],
                        targets=["SelfFrontRow"],
                    )
                )
                stats.parsed_lines += 1
                stats.emitted_defs["Continuous.AssistFrontPower"] += 1
                continue

            match = (
                re.match(
                    r"^Assist All of your (?:other )?characters in front of this card gets? \+X power\. X is equal to that character's level ×(\d+)\.?$",
                    remainder,
                    re.I,
                )
                or re.match(
                    r"^Assist All of your (?:other )?characters in front of this card gets? \+X power\. X is equal to the level of that character ×(\d+)\.?$",
                    remainder,
                    re.I,
                )
                or re.match(
                    r"^Assist All of your (?:other )?characters in front of this card gets? \+X power\. X is equal to (\d+) multiplied by that character's level\.?$",
                    remainder,
                    re.I,
                )
            )
            if match:
                ability_defs.append(
                    _ability_def(
                        "Continuous",
                        None,
                        effects=[
                            _template(
                                "AddPowerByLevel",
                                multiplier=int(match.group(1)),
                                duration_turn=False,
                            )
                        ],
                        targets=["SelfFrontRow"],
                    )
                )
                stats.parsed_lines += 1
                stats.emitted_defs["Continuous.AssistFrontPower.ByLevel"] += 1
                continue

            match = (
                re.match(
                    r"^Assist All of your (?:other )?《([^》]+)》 characters in front of this card gets? \+X power\. X is equal to that character's level ×(\d+)\.?$",
                    remainder,
                    re.I,
                )
                or re.match(
                    r"^Assist All of your (?:other )?《([^》]+)》 characters in front of this card gets? \+X power\. X is equal to the level of that character ×(\d+)\.?$",
                    remainder,
                    re.I,
                )
                or re.match(
                    r"^Assist All of your (?:other )?《([^》]+)》 characters in front of this card gets? \+X power\. X is equal to (\d+) multiplied by that character's level\.?$",
                    remainder,
                    re.I,
                )
            )
            if match:
                if re.match(
                    r"^Assist All of your (?:other )?《([^》]+)》 characters in front of this card gets? \+X power\. X is equal to (\d+) multiplied by that character's level\.?$",
                    remainder,
                    re.I,
                ):
                    trait_name = match.group(1).strip()
                    multiplier = int(match.group(2))
                else:
                    trait_name = match.group(1).strip()
                    multiplier = int(match.group(2))
                trait_id = (trait_map or {}).get(trait_name)
                if trait_id is None:
                    mark_unsupported(line)
                    continue
                ability_defs.append(
                    _ability_def(
                        "Continuous",
                        None,
                        effects=[
                            _template(
                                "AddPowerByLevel",
                                multiplier=multiplier,
                                duration_turn=False,
                            )
                        ],
                        targets=["SelfFrontRow"],
                        target_trait=trait_id,
                    )
                )
                stats.parsed_lines += 1
                stats.emitted_defs["Continuous.AssistFrontPower.Trait.ByLevel"] += 1
                continue

            match = re.match(
                r"^Assist All of your (?:other )?characters in front of this card gets? \+([+-]?\d+) level and \+([+-]?\d+) power\.?$",
                remainder,
                re.I,
            )
            if match:
                ability_defs.append(
                    _ability_def(
                        "Continuous",
                        None,
                        effects=[
                            _template(
                                "AddLevel",
                                amount=int(match.group(1)),
                                duration_turn=False,
                            ),
                            _template(
                                "AddPower",
                                amount=int(match.group(2)),
                                duration_turn=False,
                            ),
                        ],
                        targets=["SelfFrontRow", "SelfFrontRow"],
                    )
                )
                stats.parsed_lines += 1
                stats.emitted_defs["Continuous.AssistFrontLevelPower"] += 1
                continue

            match = re.match(
                r"^Assist All of your (?:other )?《([^》]+)》 characters in front of this card gets? \+([+-]?\d+) level and \+([+-]?\d+) power\.?$",
                remainder,
                re.I,
            )
            if match:
                trait_name = match.group(1).strip()
                trait_id = (trait_map or {}).get(trait_name)
                if trait_id is None:
                    mark_unsupported(line)
                    continue
                ability_defs.append(
                    _ability_def(
                        "Continuous",
                        None,
                        effects=[
                            _template(
                                "AddLevel",
                                amount=int(match.group(2)),
                                duration_turn=False,
                            ),
                            _template(
                                "AddPower",
                                amount=int(match.group(3)),
                                duration_turn=False,
                            ),
                        ],
                        targets=["SelfFrontRow", "SelfFrontRow"],
                        target_trait=trait_id,
                    )
                )
                stats.parsed_lines += 1
                stats.emitted_defs["Continuous.AssistFrontLevelPower.Trait"] += 1
                continue

            match = re.match(
                r'^Assist All of your (?:other )?characters in front of this card gets? \+X power\. X is equal to (\d+) multiplied by the number of characters you have with "([^"]+)"\.?$',
                remainder,
                re.I,
            ) or re.match(
                r'^Assist All of your (?:other )?characters in front of this card gets? \+X power\. X is equal to the number of characters you have with "([^"]+)" ×(\d+)\.?$',
                remainder,
                re.I,
            )
            if match:
                if re.match(
                    r'^Assist All of your (?:other )?characters in front of this card gets? \+X power\. X is equal to (\d+) multiplied by the number of characters you have with "([^"]+)"\.?$',
                    remainder,
                    re.I,
                ):
                    amount = int(match.group(1))
                    fragment = match.group(2).strip()
                else:
                    fragment = match.group(1).strip()
                    amount = int(match.group(2))
                target_ids = resolve_name_fragment_ids(name_to_ids, fragment)
                if not target_ids:
                    mark_unsupported(line)
                    continue
                ability_defs.append(
                    _ability_def(
                        "Continuous",
                        None,
                        effects=[
                            _template(
                                "ConditionalAddPower",
                                amount=amount,
                                turn=None,
                                zone_count={
                                    "side": "SelfSide",
                                    "zone": "Stage",
                                    "cmp": "AtLeast",
                                    "value": 0,
                                    "card_ids": target_ids,
                                },
                                require_source_marker=False,
                                per_source_marker=False,
                                per_zone_count=True,
                                exclude_source=False,
                                target_ids=[],
                            )
                        ],
                        targets=["SelfFrontRow"],
                    )
                )
                stats.parsed_lines += 1
                stats.emitted_defs["Continuous.AssistFrontPower.PerNameFragmentCount"] += 1
                continue

            match = re.match(
                r"^Assist All of your level (\d+) or higher characters in front of this card gets? \+(\d+) power\.?$",
                remainder,
                re.I,
            )
            if match:
                ability_defs.append(
                    _ability_def(
                        "Continuous",
                        None,
                        effects=[
                            _template(
                                "AddPowerIfTargetLevelAtLeast",
                                amount=int(match.group(2)),
                                min_level=int(match.group(1)),
                                duration_turn=False,
                            )
                        ],
                        targets=["SelfFrontRow"],
                    )
                )
                stats.parsed_lines += 1
                stats.emitted_defs["Continuous.AssistFrontPower.LevelMin"] += 1
                continue

            match = re.match(
                r"^Assist All of your level (\d+) or lower characters in front of this card gets? \+(\d+) power\.?$",
                remainder,
                re.I,
            )
            if match:
                ability_defs.append(
                    _ability_def(
                        "Continuous",
                        None,
                        effects=[
                            _template("AddPower", amount=int(match.group(2)), duration_turn=False)
                        ],
                        targets=["SelfFrontRow"],
                        target_level_max=int(match.group(1)),
                    )
                )
                stats.parsed_lines += 1
                stats.emitted_defs["Continuous.AssistFrontPower.LevelMax"] += 1
                continue

            match = re.match(r"^This card cannot side attack\.?$", remainder, re.I)
            if match:
                ability_defs.append(
                    _ability_def(
                        "Continuous",
                        None,
                        effects=[_template("CannotSideAttack", duration_turn=False)],
                        targets=["This"],
                    )
                )
                stats.parsed_lines += 1
                stats.emitted_defs["Continuous.CannotSideAttack"] += 1
                continue

            match = re.match(
                r"^This card cannot be chosen by your opponent's effects\.?$",
                remainder,
                re.I,
            )
            if match:
                ability_defs.append(
                    _ability_def(
                        "Continuous",
                        None,
                        effects=[_template("CannotBeChosenByOpponentEffects", duration_turn=False)],
                        targets=["This"],
                    )
                )
                stats.parsed_lines += 1
                stats.emitted_defs["Continuous.CannotBeChosenByOpponentEffects"] += 1
                continue

            match = re.match(
                r'^You cannot play event(?:s| cards?)? or "Backup" from your hand\.?$',
                remainder,
                re.I,
            )
            if match:
                ability_defs.append(
                    _ability_def(
                        "Continuous",
                        None,
                        effects=[
                            _template("CannotPlayEventsFromHand", duration_turn=False),
                            _template("CannotPlayBackupFromHand", duration_turn=False),
                        ],
                        targets=["This", "This"],
                    )
                )
                stats.parsed_lines += 1
                stats.emitted_defs["Continuous.CannotPlayEventAndBackupFromHand"] += 1
                continue

            turn_condition, conditional_body = parse_turn_condition_prefix(remainder)

            handled_cont_rule = False
            for rule in CONT_RULES:
                match = rule.pattern.match(conditional_body)
                if not match:
                    continue
                if not rule_enabled(rule):
                    continue
                if rule.id == "Continuous.ConditionalPower.PerOtherTraitCount":
                    trait_name = match.group(2).strip()
                    trait_id = (trait_map or {}).get(trait_name)
                    trait_ids = sorted(set((trait_to_ids or {}).get(trait_name, [])))
                    if trait_id is None or not trait_ids:
                        mark_unsupported(line)
                        handled_cont_rule = True
                        break
                    amount = int(match.group(1))
                    effects = [
                        _template(
                            "ConditionalAddPower",
                            amount=amount,
                            turn=turn_condition,
                            zone_count={
                                "side": "SelfSide",
                                "zone": "Stage",
                                "cmp": "AtLeast",
                                "value": 0,
                                "card_ids": trait_ids,
                            },
                            require_source_marker=False,
                            per_source_marker=False,
                            per_zone_count=True,
                            exclude_source=False,
                            target_ids=[],
                        )
                    ]
                    if source_card_id is not None and source_card_id in trait_ids:
                        effects.append(
                            _template(
                                "AddPower",
                                amount=-amount,
                                duration_turn=False,
                            )
                        )
                    ability_defs.append(
                        _ability_def(
                            "Continuous",
                            None,
                            effects=effects,
                            targets=["This"],
                            conditions=(
                                with_approx_condition() if rule.mode == RULE_MODE_APPROX else None
                            ),
                        )
                    )
                    stats.parsed_lines += 1
                    stats.emitted_defs[rule.id] += 1
                    handled_cont_rule = True
                    break
                if rule.id == "Continuous.ConditionalPower.MiddleCenter.Self":
                    ability_defs.append(
                        _ability_def(
                            "Continuous",
                            None,
                            effects=[
                                _template(
                                    "ConditionalAddPower",
                                    amount=int(match.group(1)),
                                    turn=turn_condition,
                                    zone_count=None,
                                    require_source_marker=False,
                                    per_source_marker=False,
                                    per_zone_count=False,
                                    exclude_source=False,
                                    target_ids=[],
                                )
                            ],
                            targets=["This"],
                            conditions=(
                                with_approx_condition() if rule.mode == RULE_MODE_APPROX else None
                            ),
                        )
                    )
                    stats.parsed_lines += 1
                    stats.emitted_defs[rule.id] += 1
                    handled_cont_rule = True
                    break
                if rule.id == "Continuous.ConditionalSoul.MiddleCenter.Self":
                    if turn_condition is not None:
                        mark_unsupported(line)
                        handled_cont_rule = True
                        break
                    ability_defs.append(
                        _ability_def(
                            "Continuous",
                            None,
                            effects=[
                                _template(
                                    "AddSoulIfMiddleCenter",
                                    amount=int(match.group(1)),
                                )
                            ],
                            targets=[],
                            conditions=(
                                with_approx_condition() if rule.mode == RULE_MODE_APPROX else None
                            ),
                        )
                    )
                    stats.parsed_lines += 1
                    stats.emitted_defs[rule.id] += 1
                    handled_cont_rule = True
                    break
                if rule.id == "Continuous.ConditionalPower.IfHasOtherTrait":
                    min_count = parse_count_token(match.group(1))
                    trait_name = match.group(2).strip()
                    trait_id = (trait_map or {}).get(trait_name)
                    trait_ids = sorted(set((trait_to_ids or {}).get(trait_name, [])))
                    if min_count is None or trait_id is None or not trait_ids:
                        mark_unsupported(line)
                        handled_cont_rule = True
                        break
                    threshold = min_count + (
                        1 if source_card_id is not None and source_card_id in trait_ids else 0
                    )
                    ability_defs.append(
                        _ability_def(
                            "Continuous",
                            None,
                            effects=[
                                _template(
                                    "ConditionalAddPower",
                                    amount=int(match.group(3)),
                                    turn=turn_condition,
                                    zone_count={
                                        "side": "SelfSide",
                                        "zone": "Stage",
                                        "cmp": "AtLeast",
                                        "value": threshold,
                                        "card_ids": trait_ids,
                                    },
                                    require_source_marker=False,
                                    per_source_marker=False,
                                    per_zone_count=False,
                                    exclude_source=False,
                                    target_ids=[],
                                )
                            ],
                            targets=["This"],
                            conditions=(
                                with_approx_condition() if rule.mode == RULE_MODE_APPROX else None
                            ),
                            target_trait=trait_id,
                        )
                    )
                    stats.parsed_lines += 1
                    stats.emitted_defs[rule.id] += 1
                    handled_cont_rule = True
                    break
            if handled_cont_rule:
                continue

            match = re.match(
                r"^If the character facing this card is cost (\d+) or lower, this card cannot become 【REVERSE】\.?$",
                conditional_body,
                re.I,
            )
            if match:
                ability_defs.append(
                    _ability_def(
                        "Continuous",
                        None,
                        effects=[
                            _template(
                                "SelfCannotBecomeReverseIfFacingOpponent",
                                max_level=None,
                                max_cost=int(match.group(1)),
                                level_gt_source_level=False,
                            )
                        ],
                        targets=[],
                    )
                )
                stats.parsed_lines += 1
                stats.emitted_defs["Continuous.CannotBecomeReverse.IfFacingOpponentCostAtMost"] += 1
                continue

            match = re.match(
                r"^If the character facing this card is a higher level than this card, this card cannot frontal attack\.?$",
                conditional_body,
                re.I,
            )
            if match:
                ability_defs.append(
                    _ability_def(
                        "Continuous",
                        None,
                        effects=[_template("SelfCannotFrontalAttackIfFacingOpponentHigherLevel")],
                        targets=[],
                    )
                )
                stats.parsed_lines += 1
                stats.emitted_defs[
                    "Continuous.CannotFrontalAttack.IfFacingOpponentHigherLevel"
                ] += 1
                continue

            match = re.match(
                r"^The character facing this card cannot move to another position (?:of|on) the stage\.?$",
                conditional_body,
                re.I,
            )
            if match:
                ability_defs.append(
                    _ability_def(
                        "Continuous",
                        None,
                        effects=[_template("FacingOpponentCannotMoveStagePosition")],
                        targets=[],
                    )
                )
                stats.parsed_lines += 1
                stats.emitted_defs["Continuous.FacingOpponentCannotMoveStagePosition"] += 1
                continue

            match = re.match(
                r"^This card gets \+(\d+) power for each of your other 《([^》]+)》 or 《([^》]+)》 characters\.?$",
                conditional_body,
                re.I,
            )
            if match:
                amount = int(match.group(1))
                trait_a = match.group(2).strip()
                trait_b = match.group(3).strip()
                trait_ids = sorted(
                    {
                        *list((trait_to_ids or {}).get(trait_a, [])),
                        *list((trait_to_ids or {}).get(trait_b, [])),
                    }
                )
                if not trait_ids:
                    mark_unsupported(line)
                    continue
                effects = [
                    _template(
                        "ConditionalAddPower",
                        amount=amount,
                        turn=turn_condition,
                        zone_count={
                            "side": "SelfSide",
                            "zone": "Stage",
                            "cmp": "AtLeast",
                            "value": 0,
                            "card_ids": trait_ids,
                        },
                        require_source_marker=False,
                        per_source_marker=False,
                        per_zone_count=True,
                        exclude_source=False,
                        target_ids=[],
                    )
                ]
                if source_card_id is not None and source_card_id in trait_ids:
                    effects.append(
                        _template(
                            "AddPower",
                            amount=-amount,
                            duration_turn=False,
                        )
                    )
                ability_defs.append(
                    _ability_def(
                        "Continuous",
                        None,
                        effects=effects,
                        targets=["This"],
                    )
                )
                stats.parsed_lines += 1
                stats.emitted_defs["Continuous.ConditionalPower.PerOtherDualTraitCount"] += 1
                continue

            match = re.match(
                rf"^Experience If the total level of the cards in your level is ({COUNT_TOKEN_RE}) or higher, this card gets ([+-]?\d+) power\.?$",
                conditional_body,
                re.I,
            )
            if match:
                min_level_total = parse_count_token(match.group(1))
                if min_level_total is None:
                    mark_unsupported(line)
                    continue
                ability_defs.append(
                    _ability_def(
                        "Continuous",
                        None,
                        effects=[
                            _template(
                                "ConditionalAddPower",
                                amount=int(match.group(2)),
                                turn=turn_condition,
                                zone_count={
                                    "side": "SelfSide",
                                    "zone": "LevelTotal",
                                    "cmp": "AtLeast",
                                    "value": min_level_total,
                                },
                                require_source_marker=False,
                                per_source_marker=False,
                                per_zone_count=False,
                                exclude_source=False,
                                target_ids=[],
                            )
                        ],
                        targets=["This"],
                    )
                )
                stats.parsed_lines += 1
                stats.emitted_defs["Continuous.ExperiencePower.LevelTotal"] += 1
                continue

            match = re.match(
                rf"^Experience During your turn, if the total level of the cards in your level is ({COUNT_TOKEN_RE}) or higher, this card gets ([+-]?\d+) power\.?$",
                conditional_body,
                re.I,
            )
            if match:
                min_level_total = parse_count_token(match.group(1))
                if min_level_total is None:
                    mark_unsupported(line)
                    continue
                ability_defs.append(
                    _ability_def(
                        "Continuous",
                        None,
                        effects=[
                            _template(
                                "ConditionalAddPower",
                                amount=int(match.group(2)),
                                turn="SelfTurn",
                                zone_count={
                                    "side": "SelfSide",
                                    "zone": "LevelTotal",
                                    "cmp": "AtLeast",
                                    "value": min_level_total,
                                },
                                require_source_marker=False,
                                per_source_marker=False,
                                per_zone_count=False,
                                exclude_source=False,
                                target_ids=[],
                            )
                        ],
                        targets=["This"],
                    )
                )
                stats.parsed_lines += 1
                stats.emitted_defs["Continuous.ExperiencePower.SelfTurn.LevelTotal"] += 1
                continue

            match = re.match(
                rf"^If the number of cards in your (stock|hand) is ({COUNT_TOKEN_RE}) or (more|less), this card gets ([+-]?\d+) power\.?$",
                conditional_body,
                re.I,
            )
            if match:
                threshold = parse_count_token(match.group(2))
                if threshold is None:
                    mark_unsupported(line)
                    continue
                zone_name = "Stock" if match.group(1).lower() == "stock" else "Hand"
                cmp_name = "AtLeast" if match.group(3).lower() == "more" else "AtMost"
                ability_defs.append(
                    _ability_def(
                        "Continuous",
                        None,
                        effects=[
                            _template(
                                "ConditionalAddPower",
                                amount=int(match.group(4)),
                                turn=turn_condition,
                                zone_count={
                                    "side": "SelfSide",
                                    "zone": zone_name,
                                    "cmp": cmp_name,
                                    "value": threshold,
                                },
                                require_source_marker=False,
                                per_source_marker=False,
                                per_zone_count=False,
                                exclude_source=False,
                                target_ids=[],
                            )
                        ],
                        targets=["This"],
                    )
                )
                stats.parsed_lines += 1
                stats.emitted_defs[f"Continuous.ConditionalPower.{zone_name}{cmp_name}"] += 1
                continue

            match = re.match(
                rf"^If you have ({COUNT_TOKEN_RE}) or more other characters, this card gets ([+-]?\d+) power\.?$",
                conditional_body,
                re.I,
            )
            if match:
                other_count = parse_count_token(match.group(1))
                if other_count is None:
                    mark_unsupported(line)
                    continue
                ability_defs.append(
                    _ability_def(
                        "Continuous",
                        None,
                        effects=[
                            _template(
                                "ConditionalAddPower",
                                amount=int(match.group(2)),
                                turn=turn_condition,
                                zone_count={
                                    "side": "SelfSide",
                                    "zone": "Stage",
                                    "cmp": "AtLeast",
                                    "value": other_count + 1,
                                },
                                require_source_marker=False,
                                per_source_marker=False,
                                per_zone_count=False,
                                exclude_source=False,
                                target_ids=[],
                            )
                        ],
                        targets=["This"],
                    )
                )
                stats.parsed_lines += 1
                stats.emitted_defs["Continuous.ConditionalPower.IfHasOtherCharacters"] += 1
                continue

            match = re.match(
                rf"^If you have ({COUNT_TOKEN_RE}) or less other characters, this card gets ([+-]?\d+) power\.?$",
                conditional_body,
                re.I,
            ) or re.match(
                rf"^If the number of (?:your )?other characters(?: you have)? is ({COUNT_TOKEN_RE}) or less, this card gets ([+-]?\d+) power\.?$",
                conditional_body,
                re.I,
            )
            if match:
                other_count = parse_count_token(match.group(1))
                if other_count is None:
                    mark_unsupported(line)
                    continue
                ability_defs.append(
                    _ability_def(
                        "Continuous",
                        None,
                        effects=[
                            _template(
                                "ConditionalAddPower",
                                amount=int(match.group(2)),
                                turn=turn_condition,
                                zone_count={
                                    "side": "SelfSide",
                                    "zone": "Stage",
                                    "cmp": "AtMost",
                                    "value": other_count + 1,
                                },
                                require_source_marker=False,
                                per_source_marker=False,
                                per_zone_count=False,
                                exclude_source=False,
                                target_ids=[],
                            )
                        ],
                        targets=["This"],
                    )
                )
                stats.parsed_lines += 1
                stats.emitted_defs["Continuous.ConditionalPower.IfHasAtMostOtherCharacters"] += 1
                continue

            match = re.match(
                rf"^If the number of (?:your )?other 《([^》]+)》 characters(?: you have)? is ({COUNT_TOKEN_RE}) or more, this card gets ([+-]?\d+) power\.?$",
                conditional_body,
                re.I,
            )
            if match:
                trait_name = match.group(1).strip()
                min_other_count = parse_count_token(match.group(2))
                trait_ids = sorted(set((trait_to_ids or {}).get(trait_name, [])))
                if min_other_count is None or not trait_ids:
                    mark_unsupported(line)
                    continue
                threshold = min_other_count + (
                    1 if source_card_id is not None and source_card_id in trait_ids else 0
                )
                ability_defs.append(
                    _ability_def(
                        "Continuous",
                        None,
                        effects=[
                            _template(
                                "ConditionalAddPower",
                                amount=int(match.group(3)),
                                turn=turn_condition,
                                zone_count={
                                    "side": "SelfSide",
                                    "zone": "Stage",
                                    "cmp": "AtLeast",
                                    "value": threshold,
                                    "card_ids": trait_ids,
                                },
                                require_source_marker=False,
                                per_source_marker=False,
                                per_zone_count=False,
                                exclude_source=False,
                                target_ids=[],
                            )
                        ],
                        targets=["This"],
                    )
                )
                stats.parsed_lines += 1
                stats.emitted_defs["Continuous.ConditionalPower.IfHasOtherTrait.CountForm"] += 1
                continue

            match = re.match(
                rf"^If (?:you have|the number of (?:your )?other) 《([^》]+)》 characters(?: you have)? is ({COUNT_TOKEN_RE}) or less, this card gets ([+-]?\d+) power\.?$",
                conditional_body,
                re.I,
            ) or re.match(
                rf"^If you have ({COUNT_TOKEN_RE}) or less other 《([^》]+)》 characters, this card gets ([+-]?\d+) power\.?$",
                conditional_body,
                re.I,
            )
            if match:
                if re.match(
                    rf"^If you have ({COUNT_TOKEN_RE}) or less other 《([^》]+)》 characters, this card gets ([+-]?\d+) power\.?$",
                    conditional_body,
                    re.I,
                ):
                    max_other_count = parse_count_token(match.group(1))
                    trait_name = match.group(2).strip()
                    amount = int(match.group(3))
                else:
                    trait_name = match.group(1).strip()
                    max_other_count = parse_count_token(match.group(2))
                    amount = int(match.group(3))
                trait_ids = sorted(set((trait_to_ids or {}).get(trait_name, [])))
                if max_other_count is None or not trait_ids:
                    mark_unsupported(line)
                    continue
                threshold = max_other_count + (
                    1 if source_card_id is not None and source_card_id in trait_ids else 0
                )
                ability_defs.append(
                    _ability_def(
                        "Continuous",
                        None,
                        effects=[
                            _template(
                                "ConditionalAddPower",
                                amount=amount,
                                turn=turn_condition,
                                zone_count={
                                    "side": "SelfSide",
                                    "zone": "Stage",
                                    "cmp": "AtMost",
                                    "value": threshold,
                                    "card_ids": trait_ids,
                                },
                                require_source_marker=False,
                                per_source_marker=False,
                                per_zone_count=False,
                                exclude_source=False,
                                target_ids=[],
                            )
                        ],
                        targets=["This"],
                    )
                )
                stats.parsed_lines += 1
                stats.emitted_defs["Continuous.ConditionalPower.IfHasAtMostOtherTrait"] += 1
                continue

            match = re.match(
                rf"^If the number of (?:your )?other 《([^》]+)》 or 《([^》]+)》 characters(?: you have)? is ({COUNT_TOKEN_RE}) or more, this card gets ([+-]?\d+) power\.?$",
                conditional_body,
                re.I,
            )
            if match:
                trait_a = match.group(1).strip()
                trait_b = match.group(2).strip()
                min_other_count = parse_count_token(match.group(3))
                trait_ids = sorted(
                    {
                        *list((trait_to_ids or {}).get(trait_a, [])),
                        *list((trait_to_ids or {}).get(trait_b, [])),
                    }
                )
                if min_other_count is None or not trait_ids:
                    mark_unsupported(line)
                    continue
                threshold = min_other_count + (
                    1 if source_card_id is not None and source_card_id in trait_ids else 0
                )
                ability_defs.append(
                    _ability_def(
                        "Continuous",
                        None,
                        effects=[
                            _template(
                                "ConditionalAddPower",
                                amount=int(match.group(4)),
                                turn=turn_condition,
                                zone_count={
                                    "side": "SelfSide",
                                    "zone": "Stage",
                                    "cmp": "AtLeast",
                                    "value": threshold,
                                    "card_ids": trait_ids,
                                },
                                require_source_marker=False,
                                per_source_marker=False,
                                per_zone_count=False,
                                exclude_source=False,
                                target_ids=[],
                            )
                        ],
                        targets=["This"],
                    )
                )
                stats.parsed_lines += 1
                stats.emitted_defs["Continuous.ConditionalPower.IfHasOtherDualTrait.CountForm"] += 1
                continue

            match = re.match(
                r'^If you have another "([^"]+)", this card gets \+(\d+) power\.?$',
                conditional_body,
                re.I,
            )
            if match:
                target_ids = sorted(set((name_to_ids or {}).get(match.group(1).strip(), [])))
                if not target_ids:
                    mark_unsupported(line)
                    continue
                threshold = 2 if source_card_id is not None and source_card_id in target_ids else 1
                ability_defs.append(
                    _ability_def(
                        "Continuous",
                        None,
                        effects=[
                            _template(
                                "ConditionalAddPower",
                                amount=int(match.group(2)),
                                turn=turn_condition,
                                zone_count={
                                    "side": "SelfSide",
                                    "zone": "Stage",
                                    "cmp": "AtLeast",
                                    "value": threshold,
                                    "card_ids": target_ids,
                                },
                                require_source_marker=False,
                                per_source_marker=False,
                                per_zone_count=False,
                                exclude_source=False,
                                target_ids=[],
                            )
                        ],
                        targets=["This"],
                    )
                )
                stats.parsed_lines += 1
                stats.emitted_defs["Continuous.ConditionalPower.AnotherNamed"] += 1
                continue

            match = re.match(
                r'^If you have another character with "([^"]+)" in its card name, this card gets \+(\d+) power\.?$',
                conditional_body,
                re.I,
            )
            if match:
                fragment = match.group(1).strip()
                target_ids = resolve_name_fragment_ids(name_to_ids, fragment)
                if not target_ids:
                    mark_unsupported(line)
                    continue
                threshold = 2 if source_card_id is not None and source_card_id in target_ids else 1
                ability_defs.append(
                    _ability_def(
                        "Continuous",
                        None,
                        effects=[
                            _template(
                                "ConditionalAddPower",
                                amount=int(match.group(2)),
                                turn=turn_condition,
                                zone_count={
                                    "side": "SelfSide",
                                    "zone": "Stage",
                                    "cmp": "AtLeast",
                                    "value": threshold,
                                    "card_ids": target_ids,
                                },
                                require_source_marker=False,
                                per_source_marker=False,
                                per_zone_count=False,
                                exclude_source=False,
                                target_ids=[],
                            )
                        ],
                        targets=["This"],
                    )
                )
                stats.parsed_lines += 1
                stats.emitted_defs["Continuous.ConditionalPower.AnotherNameFragment"] += 1
                continue

            match = re.match(
                r"^If you do not have another character, this card gets \+(\d+) level and \+(\d+) power\.?$",
                conditional_body,
                re.I,
            )
            if match:
                level_amount = int(match.group(1))
                power_amount = int(match.group(2))
                zone_condition = {
                    "side": "SelfSide",
                    "zone": "Stage",
                    "cmp": "AtMost",
                    "value": 1,
                    "card_ids": [],
                }
                ability_defs.append(
                    _ability_def(
                        "Continuous",
                        None,
                        effects=[
                            _template(
                                "ConditionalAddLevel",
                                amount=level_amount,
                                turn=turn_condition,
                                zone_count=zone_condition,
                                require_source_marker=False,
                                per_source_marker=False,
                                per_zone_count=False,
                                exclude_source=False,
                                target_ids=[],
                            ),
                            _template(
                                "ConditionalAddPower",
                                amount=power_amount,
                                turn=turn_condition,
                                zone_count=zone_condition,
                                require_source_marker=False,
                                per_source_marker=False,
                                per_zone_count=False,
                                exclude_source=False,
                                target_ids=[],
                            ),
                        ],
                        targets=["This", "This"],
                    )
                )
                stats.parsed_lines += 1
                stats.emitted_defs["Continuous.ConditionalLevelPower.NoOtherCharacter"] += 1
                continue

            match = re.match(
                r"^This card gets \+(\d+) level and \+(\d+) power for each marker underneath (?:this card|it)\.?$",
                conditional_body,
                re.I,
            )
            if match:
                level_amount = int(match.group(1))
                power_amount = int(match.group(2))
                ability_defs.append(
                    _ability_def(
                        "Continuous",
                        None,
                        effects=[
                            _template(
                                "ConditionalAddLevel",
                                amount=level_amount,
                                turn=turn_condition,
                                zone_count=None,
                                require_source_marker=False,
                                per_source_marker=True,
                                exclude_source=False,
                                target_ids=[],
                            ),
                            _template(
                                "ConditionalAddPower",
                                amount=power_amount,
                                turn=turn_condition,
                                zone_count=None,
                                require_source_marker=False,
                                per_source_marker=True,
                                exclude_source=False,
                                target_ids=[],
                            ),
                        ],
                        targets=["This", "This"],
                    )
                )
                stats.parsed_lines += 1
                stats.emitted_defs["Continuous.ConditionalLevelPower.PerSourceMarker"] += 1
                continue

            match = re.match(
                r"^This card gets \+(\d+) power for each marker underneath (?:this card|it)\.?$",
                conditional_body,
                re.I,
            )
            if match:
                ability_defs.append(
                    _ability_def(
                        "Continuous",
                        None,
                        effects=[
                            _template(
                                "ConditionalAddPower",
                                amount=int(match.group(1)),
                                turn=turn_condition,
                                zone_count=None,
                                require_source_marker=False,
                                per_source_marker=True,
                                exclude_source=False,
                                target_ids=[],
                            )
                        ],
                        targets=["This"],
                    )
                )
                stats.parsed_lines += 1
                stats.emitted_defs["Continuous.ConditionalPower.PerSourceMarker"] += 1
                continue

            match = re.match(
                r"^If there is (?:a |an )?(?:face up |green |red )?marker underneath this card, this card gets \+(\d+) level and \+(\d+) power\.?$",
                conditional_body,
                re.I,
            ) or re.match(
                r"^If this card has a marker underneath it, this card gets \+(\d+) level and \+(\d+) power\.?$",
                conditional_body,
                re.I,
            )
            if match:
                level_amount = int(match.group(1))
                power_amount = int(match.group(2))
                ability_defs.append(
                    _ability_def(
                        "Continuous",
                        None,
                        effects=[
                            _template(
                                "ConditionalAddLevel",
                                amount=level_amount,
                                turn=turn_condition,
                                zone_count=None,
                                require_source_marker=True,
                                per_source_marker=False,
                                exclude_source=False,
                                target_ids=[],
                            ),
                            _template(
                                "ConditionalAddPower",
                                amount=power_amount,
                                turn=turn_condition,
                                zone_count=None,
                                require_source_marker=True,
                                per_source_marker=False,
                                exclude_source=False,
                                target_ids=[],
                            ),
                        ],
                        targets=["This", "This"],
                    )
                )
                stats.parsed_lines += 1
                stats.emitted_defs["Continuous.ConditionalLevelPower.RequireSourceMarker"] += 1
                continue

            match = re.match(
                r"^If there is (?:a |an )?(?:face up |green |red )?marker underneath this card, this card gets \+(\d+) power\.?$",
                conditional_body,
                re.I,
            ) or re.match(
                r"^If this card has a marker underneath it, this card gets \+(\d+) power\.?$",
                conditional_body,
                re.I,
            )
            if match:
                ability_defs.append(
                    _ability_def(
                        "Continuous",
                        None,
                        effects=[
                            _template(
                                "ConditionalAddPower",
                                amount=int(match.group(1)),
                                turn=turn_condition,
                                zone_count=None,
                                require_source_marker=True,
                                per_source_marker=False,
                                exclude_source=False,
                                target_ids=[],
                            )
                        ],
                        targets=["This"],
                    )
                )
                stats.parsed_lines += 1
                stats.emitted_defs["Continuous.ConditionalPower.RequireSourceMarker"] += 1
                continue

            match = re.match(
                r"^All of your other characters get \+(\d+) power\.?$",
                conditional_body,
                re.I,
            )
            if match:
                ability_defs.append(
                    _ability_def(
                        "Continuous",
                        None,
                        effects=[
                            _template(
                                "ConditionalAddPower",
                                amount=int(match.group(1)),
                                turn=turn_condition,
                                zone_count=None,
                                require_source_marker=False,
                                per_source_marker=False,
                                exclude_source=True,
                                target_ids=[],
                            )
                        ],
                        targets=["SelfStage"],
                    )
                )
                stats.parsed_lines += 1
                stats.emitted_defs["Continuous.ConditionalPower.AllOtherCharacters"] += 1
                continue

            match = re.match(
                r"^All of your other 《([^》]+)》 characters get \+(\d+) power\.?$",
                conditional_body,
                re.I,
            )
            if match:
                trait_name = match.group(1).strip()
                trait_id = (trait_map or {}).get(trait_name)
                if trait_id is None:
                    mark_unsupported(line)
                    continue
                ability_defs.append(
                    _ability_def(
                        "Continuous",
                        None,
                        effects=[
                            _template(
                                "ConditionalAddPower",
                                amount=int(match.group(2)),
                                turn=turn_condition,
                                zone_count=None,
                                require_source_marker=False,
                                per_source_marker=False,
                                exclude_source=True,
                                target_ids=[],
                            )
                        ],
                        targets=["SelfStage"],
                        target_trait=trait_id,
                    )
                )
                stats.parsed_lines += 1
                stats.emitted_defs["Continuous.ConditionalPower.AllOtherTrait"] += 1
                continue

            match = re.match(
                r"^All of your other 《([^》]+)》 or 《([^》]+)》 characters get \+(\d+) power\.?$",
                conditional_body,
                re.I,
            )
            if match:
                trait_a = match.group(1).strip()
                trait_b = match.group(2).strip()
                trait_ids = sorted(
                    {
                        *list((trait_to_ids or {}).get(trait_a, [])),
                        *list((trait_to_ids or {}).get(trait_b, [])),
                    }
                )
                if not trait_ids:
                    mark_unsupported(line)
                    continue
                ability_defs.append(
                    _ability_def(
                        "Continuous",
                        None,
                        effects=[
                            _template(
                                "ConditionalAddPower",
                                amount=int(match.group(3)),
                                turn=turn_condition,
                                zone_count=None,
                                require_source_marker=False,
                                per_source_marker=False,
                                per_zone_count=False,
                                exclude_source=True,
                                target_ids=trait_ids,
                            )
                        ],
                        targets=["SelfStage"],
                    )
                )
                stats.parsed_lines += 1
                stats.emitted_defs["Continuous.ConditionalPower.AllOtherDualTrait"] += 1
                continue

            match = re.match(
                r'^All of your other cards named "([^"]+)" get ([+-]?\d+) power\.?$',
                conditional_body,
                re.I,
            )
            if match:
                target_ids = sorted(set((name_to_ids or {}).get(match.group(1).strip(), [])))
                if not target_ids:
                    mark_unsupported(line)
                    continue
                ability_defs.append(
                    _ability_def(
                        "Continuous",
                        None,
                        effects=[
                            _template(
                                "ConditionalAddPower",
                                amount=int(match.group(2)),
                                turn=turn_condition,
                                zone_count=None,
                                require_source_marker=False,
                                per_source_marker=False,
                                per_zone_count=False,
                                exclude_source=True,
                                target_ids=target_ids,
                            )
                        ],
                        targets=["SelfStage"],
                    )
                )
                stats.parsed_lines += 1
                stats.emitted_defs["Continuous.ConditionalPower.AllOtherNamed.CardsNamed"] += 1
                continue

            match = re.match(
                r'^All of your other "([^"]+)" get \+(\d+) power\.?$',
                conditional_body,
                re.I,
            )
            if match:
                target_ids: List[int] = []
                if name_to_ids:
                    seen_ids = set()
                    for card_id in name_to_ids.get(match.group(1).strip(), []):
                        if card_id in seen_ids:
                            continue
                        seen_ids.add(card_id)
                        target_ids.append(card_id)
                if not target_ids:
                    mark_unsupported(line)
                    continue
                ability_defs.append(
                    _ability_def(
                        "Continuous",
                        None,
                        effects=[
                            _template(
                                "ConditionalAddPower",
                                amount=int(match.group(2)),
                                turn=turn_condition,
                                zone_count=None,
                                require_source_marker=False,
                                per_source_marker=False,
                                exclude_source=True,
                                target_ids=target_ids,
                            )
                        ],
                        targets=["SelfStage"],
                    )
                )
                stats.parsed_lines += 1
                stats.emitted_defs["Continuous.ConditionalPower.AllOtherNamed"] += 1
                continue

            match = re.match(
                r'^All of your other "([^"]+)" and "([^"]+)" get \+([+-]?\d+) power\.?$',
                conditional_body,
                re.I,
            )
            if match:
                first_ids = resolve_name_fragment_ids(name_to_ids, match.group(1).strip())
                second_ids = resolve_name_fragment_ids(name_to_ids, match.group(2).strip())
                target_ids = sorted(set(first_ids + second_ids))
                if not target_ids:
                    mark_unsupported(line)
                    continue
                ability_defs.append(
                    _ability_def(
                        "Continuous",
                        None,
                        effects=[
                            _template(
                                "ConditionalAddPower",
                                amount=int(match.group(3)),
                                turn=turn_condition,
                                zone_count=None,
                                require_source_marker=False,
                                per_source_marker=False,
                                per_zone_count=False,
                                exclude_source=True,
                                target_ids=target_ids,
                            )
                        ],
                        targets=["SelfStage"],
                    )
                )
                stats.parsed_lines += 1
                stats.emitted_defs["Continuous.ConditionalPower.AllOtherTwoNamed"] += 1
                continue

            match = re.match(
                r'^All of your other characters with "([^"]+)" in (?:its|their) card name get ([+-]?\d+) power\.?$',
                conditional_body,
                re.I,
            )
            if match:
                fragment = match.group(1).strip()
                target_ids = resolve_name_fragment_ids(name_to_ids, fragment)
                if not target_ids:
                    mark_unsupported(line)
                    continue
                ability_defs.append(
                    _ability_def(
                        "Continuous",
                        None,
                        effects=[
                            _template(
                                "ConditionalAddPower",
                                amount=int(match.group(2)),
                                turn=turn_condition,
                                zone_count=None,
                                require_source_marker=False,
                                per_source_marker=False,
                                per_zone_count=False,
                                exclude_source=True,
                                target_ids=target_ids,
                            )
                        ],
                        targets=["SelfStage"],
                    )
                )
                stats.parsed_lines += 1
                stats.emitted_defs["Continuous.ConditionalPower.AllOtherNameFragment"] += 1
                continue

            match = re.match(
                r'^All of your other characters with "([^"]+)" or "([^"]+)" in (?:its|their) card name get ([+-]?\d+) power\.?$',
                conditional_body,
                re.I,
            )
            if match:
                first_ids = resolve_name_fragment_ids(name_to_ids, match.group(1).strip())
                second_ids = resolve_name_fragment_ids(name_to_ids, match.group(2).strip())
                target_ids = sorted(set(first_ids + second_ids))
                if not target_ids:
                    mark_unsupported(line)
                    continue
                ability_defs.append(
                    _ability_def(
                        "Continuous",
                        None,
                        effects=[
                            _template(
                                "ConditionalAddPower",
                                amount=int(match.group(3)),
                                turn=turn_condition,
                                zone_count=None,
                                require_source_marker=False,
                                per_source_marker=False,
                                per_zone_count=False,
                                exclude_source=True,
                                target_ids=target_ids,
                            )
                        ],
                        targets=["SelfStage"],
                    )
                )
                stats.parsed_lines += 1
                stats.emitted_defs["Continuous.ConditionalPower.AllOtherNameFragmentDual"] += 1
                continue

            match = re.match(
                r'^All of your other "([^"]+)" get the following ability\.\s*"This card gets \+(\d+) power\.?"\.?$',
                conditional_body,
                re.I,
            )
            if match:
                target_ids = resolve_name_fragment_ids(name_to_ids, match.group(1).strip())
                if not target_ids:
                    mark_unsupported(line)
                    continue
                ability_defs.append(
                    _ability_def(
                        "Continuous",
                        None,
                        effects=[
                            _template(
                                "ConditionalAddPower",
                                amount=int(match.group(2)),
                                turn=turn_condition,
                                zone_count=None,
                                require_source_marker=False,
                                per_source_marker=False,
                                per_zone_count=False,
                                exclude_source=True,
                                target_ids=target_ids,
                            )
                        ],
                        targets=["SelfStage"],
                    )
                )
                stats.parsed_lines += 1
                stats.emitted_defs[
                    "Continuous.ConditionalPower.AllOtherNameFragment.FollowingAbility"
                ] += 1
                continue

            match = re.match(
                r'^All of your other "([^"]+)" get the following ability\.\s*"[^"]+"\.?$',
                conditional_body,
                re.I,
            )
            if match and allow_approx_effects:
                ability_defs.append(
                    _ability_def(
                        "Continuous",
                        None,
                        effects=[_template("Draw", count=0)],
                        targets=[],
                        conditions=with_approx_condition(),
                    )
                )
                stats.parsed_lines += 1
                stats.emitted_defs["Continuous.FollowingAbility.AllOtherNamed.ApproxNoop"] += 1
                continue

            match = re.match(
                r'^All of your other "([^"]+)" get \+(\d+) power and the following ability\.\s*"[^"]+"\.?$',
                conditional_body,
                re.I,
            )
            if match and allow_approx_effects:
                target_ids = resolve_name_fragment_ids(name_to_ids, match.group(1).strip())
                if not target_ids:
                    mark_unsupported(line)
                    continue
                ability_defs.append(
                    _ability_def(
                        "Continuous",
                        None,
                        effects=[
                            _template(
                                "ConditionalAddPower",
                                amount=int(match.group(2)),
                                turn=turn_condition,
                                zone_count=None,
                                require_source_marker=False,
                                per_source_marker=False,
                                per_zone_count=False,
                                exclude_source=True,
                                target_ids=target_ids,
                            )
                        ],
                        targets=["SelfStage"],
                        conditions=with_approx_condition(),
                    )
                )
                stats.parsed_lines += 1
                stats.emitted_defs[
                    "Continuous.ConditionalPower.AllOtherNamed.FollowingAbility.Approx"
                ] += 1
                continue

            match = re.match(
                r'^All of your other characters get the following ability\.\s*"This card gets \+(\d+) power\.?"\.?$',
                conditional_body,
                re.I,
            )
            if match:
                ability_defs.append(
                    _ability_def(
                        "Continuous",
                        None,
                        effects=[
                            _template(
                                "ConditionalAddPower",
                                amount=int(match.group(1)),
                                turn=turn_condition,
                                zone_count=None,
                                require_source_marker=False,
                                per_source_marker=False,
                                per_zone_count=False,
                                exclude_source=True,
                                target_ids=[],
                            )
                        ],
                        targets=["SelfStage"],
                    )
                )
                stats.parsed_lines += 1
                stats.emitted_defs["Continuous.ConditionalPower.AllOtherFollowingAbility"] += 1
                continue

            match = re.match(
                r'^All of your other characters get the following ability\.\s*"【CONT】 This card cannot side attack\.?"\.?$',
                conditional_body,
                re.I,
            )
            if match:
                ability_defs.append(
                    _ability_def(
                        "Continuous",
                        None,
                        effects=[
                            _template(
                                "ConditionalCannotSideAttack",
                                turn=turn_condition,
                                zone_count=None,
                                require_source_marker=False,
                                exclude_source=True,
                            )
                        ],
                        targets=["SelfStage"],
                    )
                )
                stats.parsed_lines += 1
                stats.emitted_defs[
                    "Continuous.ConditionalCannotSideAttack.AllOtherFollowingAbility"
                ] += 1
                continue

            match = re.match(
                r'^All of your other characters get the following ability\.\s*"[^"]+"\.?$',
                conditional_body,
                re.I,
            )
            if match and allow_approx_effects:
                ability_defs.append(
                    _ability_def(
                        "Continuous",
                        None,
                        effects=[_template("Draw", count=0)],
                        targets=[],
                        conditions=with_approx_condition(),
                    )
                )
                stats.parsed_lines += 1
                stats.emitted_defs["Continuous.FollowingAbility.AllOther.ApproxNoop"] += 1
                continue

            match = re.match(
                r'^All of your opponent\'s characters get\s*"This card gets ([+-]?\d+) power\.?"\.?$',
                conditional_body,
                re.I,
            )
            if match:
                ability_defs.append(
                    _ability_def(
                        "Continuous",
                        None,
                        effects=[
                            _template(
                                "ConditionalAddPower",
                                amount=int(match.group(1)),
                                turn=turn_condition,
                                zone_count=None,
                                require_source_marker=False,
                                per_source_marker=False,
                                per_zone_count=False,
                                exclude_source=False,
                                target_ids=[],
                            )
                        ],
                        targets=["OppStage"],
                    )
                )
                stats.parsed_lines += 1
                stats.emitted_defs["Continuous.ConditionalPower.AllOpponentFollowingAbility"] += 1
                continue

            match = re.match(
                r'^All of your opponent\'s characters get\s*"【AUTO】 Encore \[\((\d+)\)\]"\.?$',
                conditional_body,
                re.I,
            )
            if match:
                ability_defs.append(
                    _ability_def(
                        "Continuous",
                        None,
                        effects=[
                            _template(
                                "EncoreStockCost",
                                cost=int(match.group(1)),
                                duration_turn=False,
                            )
                        ],
                        targets=["OppStage"],
                    )
                )
                stats.parsed_lines += 1
                stats.emitted_defs["Continuous.EncoreStockCost.AllOpponent"] += 1
                continue

            match = re.match(
                r'^All of your opponent\'s characters get\s*"([^"]+)"\.?$',
                conditional_body,
                re.I,
            )
            if match:
                nested_effect = parse_following_flatten_effect(match.group(1), duration_turn=False)
                if nested_effect is not None:
                    ability_defs.append(
                        _ability_def(
                            "Continuous",
                            None,
                            effects=[nested_effect],
                            targets=["OppStage"],
                        )
                    )
                    stats.parsed_lines += 1
                    stats.emitted_defs["Continuous.FollowingAbility.AllOpponent.Flattened"] += 1
                    continue
                if allow_approx_effects:
                    ability_defs.append(
                        _ability_def(
                            "Continuous",
                            None,
                            effects=[_template("Draw", count=0)],
                            targets=[],
                            conditions=with_approx_condition(),
                        )
                    )
                    stats.parsed_lines += 1
                    stats.emitted_defs["Continuous.FollowingAbility.AllOpponent.ApproxNoop"] += 1
                    continue

            match = re.match(
                rf"^If the character facing this card is level ({COUNT_TOKEN_RE}) or higher, this card gets \+([+-]?\d+) power\.?$",
                conditional_body,
                re.I,
            )
            if match:
                min_level = parse_count_token(match.group(1))
                if min_level is None:
                    mark_unsupported(line)
                    continue
                ability_defs.append(
                    _ability_def(
                        "Continuous",
                        None,
                        effects=[
                            _template(
                                "AddPowerIfBattleOpponentLevelAtLeast",
                                amount=int(match.group(2)),
                                min_level=min_level,
                                duration_turn=False,
                            )
                        ],
                        targets=["This"],
                    )
                )
                stats.parsed_lines += 1
                stats.emitted_defs["Continuous.AddPower.IfFacingOpponentLevelAtLeast"] += 1
                continue

            match = re.match(
                r"^The character facing this card gets ([+-]?\d+) soul\.?$",
                conditional_body,
                re.I,
            )
            if match:
                ability_defs.append(
                    _ability_def(
                        "Continuous",
                        None,
                        effects=[_template("FacingOpponentAddSoul", amount=int(match.group(1)))],
                        targets=[],
                    )
                )
                stats.parsed_lines += 1
                stats.emitted_defs["Continuous.FacingOpponentSoul"] += 1
                continue

            match = re.match(
                r'^All of your characters get the following ability\.\s*"(.+)"\.?$',
                conditional_body,
                re.I,
            )
            if match:
                nested_effect = parse_following_flatten_effect(match.group(1), duration_turn=False)
                if nested_effect is not None:
                    ability_defs.append(
                        _ability_def(
                            "Continuous",
                            None,
                            effects=[nested_effect],
                            targets=["SelfStage"],
                        )
                    )
                    stats.parsed_lines += 1
                    stats.emitted_defs["Continuous.FollowingAbility.AllSelf.Flattened"] += 1
                    continue
                if allow_approx_effects:
                    ability_defs.append(
                        _ability_def(
                            "Continuous",
                            None,
                            effects=[_template("Draw", count=0)],
                            targets=[],
                            conditions=with_approx_condition(),
                        )
                    )
                    stats.parsed_lines += 1
                    stats.emitted_defs["Continuous.FollowingAbility.AllSelf.ApproxNoop"] += 1
                    continue

            match = re.match(
                r"^Your other character in the middle position of (?:your|the) center stage gets \+(\d+) power\.?$",
                conditional_body,
                re.I,
            )
            if match:
                ability_defs.append(
                    _ability_def(
                        "Continuous",
                        None,
                        effects=[
                            _template(
                                "ConditionalAddPower",
                                amount=int(match.group(1)),
                                turn=turn_condition,
                                zone_count=None,
                                require_source_marker=False,
                                per_source_marker=False,
                                per_zone_count=False,
                                exclude_source=True,
                                target_ids=[],
                            )
                        ],
                        targets=[_template("SelfStageSlot", slot=1)],
                    )
                )
                stats.parsed_lines += 1
                stats.emitted_defs["Continuous.ConditionalPower.MiddleCenter.Other"] += 1
                continue

            match = re.match(
                r"^This card gets \+(\d+) power for each character in your(?: opponent's)? back stage\.?$",
                conditional_body,
                re.I,
            )
            if match:
                side = (
                    "Opponent"
                    if re.search(r"opponent's back stage", conditional_body, re.I)
                    else "SelfSide"
                )
                ability_defs.append(
                    _ability_def(
                        "Continuous",
                        None,
                        effects=[
                            _template(
                                "ConditionalAddPower",
                                amount=int(match.group(1)),
                                turn=turn_condition,
                                zone_count={
                                    "side": side,
                                    "zone": "BackStage",
                                    "cmp": "AtLeast",
                                    "value": 0,
                                },
                                require_source_marker=False,
                                per_source_marker=False,
                                per_zone_count=True,
                                exclude_source=False,
                                target_ids=[],
                            )
                        ],
                        targets=["This"],
                    )
                )
                stats.parsed_lines += 1
                stats.emitted_defs["Continuous.ConditionalPower.PerBackStageCount"] += 1
                continue

            match = re.match(
                r"^This card gets \+([+-]?\d+) power for each other 《([^》]+)》 character in your back stage\.?$",
                conditional_body,
                re.I,
            )
            if match:
                amount = int(match.group(1))
                trait_name = match.group(2).strip()
                trait_ids = sorted(set((trait_to_ids or {}).get(trait_name, [])))
                if not trait_ids:
                    mark_unsupported(line)
                    continue
                ability_defs.append(
                    _ability_def(
                        "Continuous",
                        None,
                        effects=[
                            _template(
                                "ConditionalAddPower",
                                amount=amount,
                                turn=turn_condition,
                                zone_count={
                                    "side": "SelfSide",
                                    "zone": "BackStage",
                                    "cmp": "AtLeast",
                                    "value": 0,
                                    "card_ids": trait_ids,
                                },
                                require_source_marker=False,
                                per_source_marker=False,
                                per_zone_count=True,
                                exclude_source=True,
                                target_ids=[],
                            )
                        ],
                        targets=["This"],
                    )
                )
                stats.parsed_lines += 1
                stats.emitted_defs["Continuous.ConditionalPower.PerOtherTraitInBackStage"] += 1
                continue

            match = re.match(
                r'^This card gets \+(\d+) power for each of your other cards named "([^"]+)"\.?$',
                conditional_body,
                re.I,
            )
            if match:
                amount = int(match.group(1))
                target_ids = sorted(set((name_to_ids or {}).get(match.group(2).strip(), [])))
                if not target_ids:
                    mark_unsupported(line)
                    continue
                effects: List[Any] = [
                    _template(
                        "ConditionalAddPower",
                        amount=amount,
                        turn=turn_condition,
                        zone_count={
                            "side": "SelfSide",
                            "zone": "Stage",
                            "cmp": "AtLeast",
                            "value": 0,
                            "card_ids": target_ids,
                        },
                        require_source_marker=False,
                        per_source_marker=False,
                        per_zone_count=True,
                        exclude_source=False,
                        target_ids=[],
                    )
                ]
                if source_card_id is not None and source_card_id in target_ids:
                    effects.append(
                        _template(
                            "AddPower",
                            amount=-amount,
                            duration_turn=False,
                        )
                    )
                ability_defs.append(
                    _ability_def(
                        "Continuous",
                        None,
                        effects=effects,
                        targets=["This"],
                    )
                )
                stats.parsed_lines += 1
                stats.emitted_defs["Continuous.ConditionalPower.PerOtherNamedCount"] += 1
                continue

            match = re.match(
                r'^This card gets \+(\d+) power for each other "([^"]+)" in your center stage\.?$',
                conditional_body,
                re.I,
            )
            if match:
                amount = int(match.group(1))
                target_ids = resolve_name_fragment_ids(name_to_ids, match.group(2).strip())
                if not target_ids:
                    mark_unsupported(line)
                    continue
                effects: List[Any] = [
                    _template(
                        "ConditionalAddPower",
                        amount=amount,
                        turn=turn_condition,
                        zone_count={
                            "side": "SelfSide",
                            "zone": "FrontStage",
                            "cmp": "AtLeast",
                            "value": 0,
                            "card_ids": target_ids,
                        },
                        require_source_marker=False,
                        per_source_marker=False,
                        per_zone_count=True,
                        exclude_source=False,
                        target_ids=[],
                    )
                ]
                if source_card_id is not None and source_card_id in target_ids:
                    effects.append(
                        _template(
                            "AddPower",
                            amount=-amount,
                            duration_turn=False,
                        )
                    )
                ability_defs.append(
                    _ability_def(
                        "Continuous",
                        None,
                        effects=effects,
                        targets=["This"],
                    )
                )
                stats.parsed_lines += 1
                stats.emitted_defs["Continuous.ConditionalPower.PerOtherNameInCenter"] += 1
                continue

            match = re.match(
                r"^This card gets \+(\d+) power for each climax in your waiting room\.?$",
                conditional_body,
                re.I,
            )
            if match:
                ability_defs.append(
                    _ability_def(
                        "Continuous",
                        None,
                        effects=[
                            _template(
                                "ConditionalAddPower",
                                amount=int(match.group(1)),
                                turn=turn_condition,
                                zone_count={
                                    "side": "SelfSide",
                                    "zone": "WaitingRoomClimax",
                                    "cmp": "AtLeast",
                                    "value": 0,
                                },
                                require_source_marker=False,
                                per_source_marker=False,
                                per_zone_count=True,
                                exclude_source=False,
                                target_ids=[],
                            )
                        ],
                        targets=["This"],
                    )
                )
                stats.parsed_lines += 1
                stats.emitted_defs["Continuous.ConditionalPower.PerWaitingRoomClimax"] += 1
                continue

            match = re.match(
                rf"^If your (stock|waiting room|hand|stage|back stage) has ({COUNT_TOKEN_RE}) or (less|more) cards?, this card gets \+(\d+) power\.?$",
                conditional_body,
                re.I,
            )
            if match:
                zone_text = match.group(1).strip().lower()
                zone_map = {
                    "stock": "Stock",
                    "waiting room": "WaitingRoom",
                    "hand": "Hand",
                    "stage": "Stage",
                    "back stage": "BackStage",
                }
                zone = zone_map.get(zone_text)
                threshold = parse_count_token(match.group(2))
                if zone is None or threshold is None:
                    mark_unsupported(line)
                    continue
                cmp = "AtMost" if match.group(3).strip().lower() == "less" else "AtLeast"
                ability_defs.append(
                    _ability_def(
                        "Continuous",
                        None,
                        effects=[
                            _template(
                                "ConditionalAddPower",
                                amount=int(match.group(4)),
                                turn=turn_condition,
                                zone_count={
                                    "side": "SelfSide",
                                    "zone": zone,
                                    "cmp": cmp,
                                    "value": threshold,
                                },
                                require_source_marker=False,
                                per_source_marker=False,
                                per_zone_count=False,
                                exclude_source=False,
                                target_ids=[],
                            )
                        ],
                        targets=["This"],
                    )
                )
                stats.parsed_lines += 1
                stats.emitted_defs["Continuous.ConditionalPower.ZoneCount"] += 1
                continue

            match = re.match(
                rf"^If your waiting room has ({COUNT_TOKEN_RE}) or (less|more) climax(?: cards?)?, this card gets \+(\d+) power\.?$",
                conditional_body,
                re.I,
            )
            if match:
                threshold = parse_count_token(match.group(1))
                if threshold is None:
                    mark_unsupported(line)
                    continue
                cmp = "AtMost" if match.group(2).strip().lower() == "less" else "AtLeast"
                ability_defs.append(
                    _ability_def(
                        "Continuous",
                        None,
                        effects=[
                            _template(
                                "ConditionalAddPower",
                                amount=int(match.group(3)),
                                turn=turn_condition,
                                zone_count={
                                    "side": "SelfSide",
                                    "zone": "WaitingRoomClimax",
                                    "cmp": cmp,
                                    "value": threshold,
                                },
                                require_source_marker=False,
                                per_source_marker=False,
                                per_zone_count=False,
                                exclude_source=False,
                                target_ids=[],
                            )
                        ],
                        targets=["This"],
                    )
                )
                stats.parsed_lines += 1
                stats.emitted_defs["Continuous.ConditionalPower.WaitingRoomClimaxCount"] += 1
                continue

            match = re.match(
                r"^If you have another 《([^》]+)》 character, this card gets ([+-]?\d+) power\.?$",
                conditional_body,
                re.I,
            )
            if match:
                trait_name = match.group(1).strip()
                trait_id = (trait_map or {}).get(trait_name)
                trait_ids = sorted(set((trait_to_ids or {}).get(trait_name, [])))
                if trait_id is None or not trait_ids:
                    mark_unsupported(line)
                    continue
                threshold = 2 if source_card_id is not None and source_card_id in trait_ids else 1
                ability_defs.append(
                    _ability_def(
                        "Continuous",
                        None,
                        effects=[
                            _template(
                                "ConditionalAddPower",
                                amount=int(match.group(2)),
                                turn=turn_condition,
                                zone_count={
                                    "side": "SelfSide",
                                    "zone": "Stage",
                                    "cmp": "AtLeast",
                                    "value": threshold,
                                    "card_ids": trait_ids,
                                },
                                require_source_marker=False,
                                per_source_marker=False,
                                per_zone_count=False,
                                exclude_source=False,
                                target_ids=[],
                            )
                        ],
                        targets=["This"],
                        target_trait=trait_id,
                    )
                )
                stats.parsed_lines += 1
                stats.emitted_defs["Continuous.ConditionalPower.IfHasAnotherTrait"] += 1
                continue

            match = re.match(
                r"^If you have another 《([^》]+)》 or 《([^》]+)》 character, this card gets ([+-]?\d+) power\.?$",
                conditional_body,
                re.I,
            )
            if match:
                trait_a = match.group(1).strip()
                trait_b = match.group(2).strip()
                trait_ids = sorted(
                    {
                        *list((trait_to_ids or {}).get(trait_a, [])),
                        *list((trait_to_ids or {}).get(trait_b, [])),
                    }
                )
                if not trait_ids:
                    mark_unsupported(line)
                    continue
                threshold = 2 if source_card_id is not None and source_card_id in trait_ids else 1
                ability_defs.append(
                    _ability_def(
                        "Continuous",
                        None,
                        effects=[
                            _template(
                                "ConditionalAddPower",
                                amount=int(match.group(3)),
                                turn=turn_condition,
                                zone_count={
                                    "side": "SelfSide",
                                    "zone": "Stage",
                                    "cmp": "AtLeast",
                                    "value": threshold,
                                    "card_ids": trait_ids,
                                },
                                require_source_marker=False,
                                per_source_marker=False,
                                per_zone_count=False,
                                exclude_source=False,
                                target_ids=[],
                            )
                        ],
                        targets=["This"],
                    )
                )
                stats.parsed_lines += 1
                stats.emitted_defs["Continuous.ConditionalPower.IfHasAnotherDualTrait"] += 1
                continue

            match = re.match(
                rf"^If you have ({COUNT_TOKEN_RE}) or more other 《([^》]+)》 or 《([^》]+)》 characters, this card gets ([+-]?\d+) power\.?$",
                conditional_body,
                re.I,
            )
            if match:
                min_count = parse_count_token(match.group(1))
                trait_a = match.group(2).strip()
                trait_b = match.group(3).strip()
                trait_ids = sorted(
                    {
                        *list((trait_to_ids or {}).get(trait_a, [])),
                        *list((trait_to_ids or {}).get(trait_b, [])),
                    }
                )
                if min_count is None or not trait_ids:
                    mark_unsupported(line)
                    continue
                threshold = min_count + (
                    1 if source_card_id is not None and source_card_id in trait_ids else 0
                )
                ability_defs.append(
                    _ability_def(
                        "Continuous",
                        None,
                        effects=[
                            _template(
                                "ConditionalAddPower",
                                amount=int(match.group(4)),
                                turn=turn_condition,
                                zone_count={
                                    "side": "SelfSide",
                                    "zone": "Stage",
                                    "cmp": "AtLeast",
                                    "value": threshold,
                                    "card_ids": trait_ids,
                                },
                                require_source_marker=False,
                                per_source_marker=False,
                                per_zone_count=False,
                                exclude_source=False,
                                target_ids=[],
                            )
                        ],
                        targets=["This"],
                    )
                )
                stats.parsed_lines += 1
                stats.emitted_defs["Continuous.ConditionalPower.IfHasCountOtherDualTrait"] += 1
                continue

            match = re.match(
                rf'^If you have ({COUNT_TOKEN_RE}) or more other 《([^》]+)》 characters, all of your other "([^"]+)" and "([^"]+)" get \+([+-]?\d+) power\.?$',
                conditional_body,
                re.I,
            )
            if match:
                min_count = parse_count_token(match.group(1))
                trait_name = match.group(2).strip()
                trait_ids = sorted(set((trait_to_ids or {}).get(trait_name, [])))
                first_ids = resolve_name_fragment_ids(name_to_ids, match.group(3).strip())
                second_ids = resolve_name_fragment_ids(name_to_ids, match.group(4).strip())
                target_ids = sorted(set(first_ids + second_ids))
                if min_count is None or not trait_ids or not target_ids:
                    mark_unsupported(line)
                    continue
                threshold = min_count + (
                    1 if source_card_id is not None and source_card_id in trait_ids else 0
                )
                ability_defs.append(
                    _ability_def(
                        "Continuous",
                        None,
                        effects=[
                            _template(
                                "ConditionalAddPower",
                                amount=int(match.group(5)),
                                turn=turn_condition,
                                zone_count={
                                    "side": "SelfSide",
                                    "zone": "Stage",
                                    "cmp": "AtLeast",
                                    "value": threshold,
                                    "card_ids": trait_ids,
                                },
                                require_source_marker=False,
                                per_source_marker=False,
                                per_zone_count=False,
                                exclude_source=True,
                                target_ids=target_ids,
                            )
                        ],
                        targets=["SelfStage"],
                    )
                )
                stats.parsed_lines += 1
                stats.emitted_defs[
                    "Continuous.ConditionalPower.IfHasCountTrait.AllOtherTwoNamed"
                ] += 1
                continue

            match = re.match(
                r'^If you have another "([^"]+)" in your back stage, this card gets ([+-]?\d+) power\.?$',
                conditional_body,
                re.I,
            )
            if match:
                name_ids = sorted(set((name_to_ids or {}).get(match.group(1).strip(), [])))
                if not name_ids:
                    mark_unsupported(line)
                    continue
                threshold = 2 if source_card_id is not None and source_card_id in name_ids else 1
                ability_defs.append(
                    _ability_def(
                        "Continuous",
                        None,
                        effects=[
                            _template(
                                "ConditionalAddPower",
                                amount=int(match.group(2)),
                                turn=turn_condition,
                                zone_count={
                                    "side": "SelfSide",
                                    "zone": "BackStage",
                                    "cmp": "AtLeast",
                                    "value": threshold,
                                    "card_ids": name_ids,
                                },
                                require_source_marker=False,
                                per_source_marker=False,
                                per_zone_count=False,
                                exclude_source=False,
                                target_ids=[],
                            )
                        ],
                        targets=["This"],
                    )
                )
                stats.parsed_lines += 1
                stats.emitted_defs["Continuous.ConditionalPower.IfHasAnotherNamedBackStage"] += 1
                continue

            match = re.match(
                r'^If you have another card named "([^"]+)", this card gets ([+-]?\d+) power\.?$',
                conditional_body,
                re.I,
            )
            if match:
                name_ids = sorted(set((name_to_ids or {}).get(match.group(1).strip(), [])))
                if not name_ids:
                    mark_unsupported(line)
                    continue
                threshold = 2 if source_card_id is not None and source_card_id in name_ids else 1
                ability_defs.append(
                    _ability_def(
                        "Continuous",
                        None,
                        effects=[
                            _template(
                                "ConditionalAddPower",
                                amount=int(match.group(2)),
                                turn=turn_condition,
                                zone_count={
                                    "side": "SelfSide",
                                    "zone": "Stage",
                                    "cmp": "AtLeast",
                                    "value": threshold,
                                    "card_ids": name_ids,
                                },
                                require_source_marker=False,
                                per_source_marker=False,
                                per_zone_count=False,
                                exclude_source=False,
                                target_ids=[],
                            )
                        ],
                        targets=["This"],
                    )
                )
                stats.parsed_lines += 1
                stats.emitted_defs["Continuous.ConditionalPower.IfHasAnotherCardNamed"] += 1
                continue

            match = re.match(
                r'^If you have another "([^"]+)" in your center stage, this card gets ([+-]?\d+) power\.?$',
                conditional_body,
                re.I,
            )
            if match and name_to_ids:
                name = match.group(1).strip()
                name_ids = sorted(set((name_to_ids or {}).get(name, [])))
                if not name_ids:
                    mark_unsupported(line)
                    continue
                threshold = 2 if source_card_id is not None and source_card_id in name_ids else 1
                ability_defs.append(
                    _ability_def(
                        "Continuous",
                        None,
                        effects=[
                            _template(
                                "ConditionalAddPower",
                                amount=int(match.group(2)),
                                turn=turn_condition,
                                zone_count={
                                    "side": "SelfSide",
                                    "zone": "Stage",
                                    "cmp": "AtLeast",
                                    "value": threshold,
                                    "card_ids": name_ids,
                                },
                                require_source_marker=False,
                                per_source_marker=False,
                                per_zone_count=False,
                                exclude_source=False,
                                target_ids=[],
                            )
                        ],
                        targets=["This"],
                    )
                )
                stats.parsed_lines += 1
                stats.emitted_defs["Continuous.ConditionalPower.IfHasAnotherNamedCenter"] += 1
                continue

            match = re.match(
                r"^If you do not have any other characters, this card gets \+([+-]?\d+) power\.?$",
                conditional_body,
                re.I,
            )
            if match:
                ability_defs.append(
                    _ability_def(
                        "Continuous",
                        None,
                        effects=[
                            _template(
                                "ConditionalAddPower",
                                amount=int(match.group(1)),
                                turn=turn_condition,
                                zone_count={
                                    "side": "SelfSide",
                                    "zone": "Stage",
                                    "cmp": "AtMost",
                                    "value": 1,
                                },
                                require_source_marker=False,
                                per_source_marker=False,
                                per_zone_count=False,
                                exclude_source=False,
                                target_ids=[],
                            )
                        ],
                        targets=["This"],
                    )
                )
                stats.parsed_lines += 1
                stats.emitted_defs["Continuous.ConditionalPower.IfNoOtherCharacters"] += 1
                continue

            match = re.match(
                rf"^If you have ({COUNT_TOKEN_RE}) or more cards in your hand, this card gets \+([+-]?\d+) power\.?$",
                conditional_body,
                re.I,
            )
            if match:
                threshold = parse_count_token(match.group(1))
                if threshold is None:
                    mark_unsupported(line)
                    continue
                ability_defs.append(
                    _ability_def(
                        "Continuous",
                        None,
                        effects=[
                            _template(
                                "ConditionalAddPower",
                                amount=int(match.group(2)),
                                turn=turn_condition,
                                zone_count={
                                    "side": "SelfSide",
                                    "zone": "Hand",
                                    "cmp": "AtLeast",
                                    "value": threshold,
                                },
                                require_source_marker=False,
                                per_source_marker=False,
                                per_zone_count=False,
                                exclude_source=False,
                                target_ids=[],
                            )
                        ],
                        targets=["This"],
                    )
                )
                stats.parsed_lines += 1
                stats.emitted_defs["Continuous.ConditionalPower.IfHandCountAtLeast"] += 1
                continue

            match = re.match(
                rf"^If your clock has ({COUNT_TOKEN_RE}) or more cards, this card gets \+([+-]?\d+) power\.?$",
                conditional_body,
                re.I,
            )
            if match:
                threshold = parse_count_token(match.group(1))
                if threshold is None:
                    mark_unsupported(line)
                    continue
                if allow_approx_effects:
                    ability_defs.append(
                        _ability_def(
                            "Continuous",
                            None,
                            effects=[_template("Draw", count=0)],
                            targets=[],
                            conditions=with_approx_condition(),
                        )
                    )
                    stats.parsed_lines += 1
                    stats.emitted_defs[
                        "Continuous.ConditionalPower.IfClockCountAtLeast.ApproxNoop"
                    ] += 1
                    continue
                mark_unsupported(line)
                continue

            match = re.match(
                rf"^If your stock has ({COUNT_TOKEN_RE}) or less cards, this card gets \+([+-]?\d+) level and \+([+-]?\d+) power\.?$",
                conditional_body,
                re.I,
            )
            if match:
                threshold = parse_count_token(match.group(1))
                if threshold is None:
                    mark_unsupported(line)
                    continue
                zone_count = {
                    "side": "SelfSide",
                    "zone": "Stock",
                    "cmp": "AtMost",
                    "value": threshold,
                }
                ability_defs.append(
                    _ability_def(
                        "Continuous",
                        None,
                        effects=[
                            _template(
                                "ConditionalAddLevel",
                                amount=int(match.group(2)),
                                turn=turn_condition,
                                zone_count=zone_count,
                                require_source_marker=False,
                                per_source_marker=False,
                                per_zone_count=False,
                                exclude_source=False,
                                target_ids=[],
                            ),
                            _template(
                                "ConditionalAddPower",
                                amount=int(match.group(3)),
                                turn=turn_condition,
                                zone_count=zone_count,
                                require_source_marker=False,
                                per_source_marker=False,
                                per_zone_count=False,
                                exclude_source=False,
                                target_ids=[],
                            ),
                        ],
                        targets=["This", "This"],
                    )
                )
                stats.parsed_lines += 1
                stats.emitted_defs["Continuous.ConditionalLevelPower.IfStockCountAtMost"] += 1
                continue

            match = re.match(r"^This card gets \+(\d+) power\.?$", conditional_body, re.I)
            if match and turn_condition is not None:
                ability_defs.append(
                    _ability_def(
                        "Continuous",
                        None,
                        effects=[
                            _template(
                                "ConditionalAddPower",
                                amount=int(match.group(1)),
                                turn=turn_condition,
                                zone_count=None,
                                require_source_marker=False,
                                per_source_marker=False,
                                exclude_source=False,
                                target_ids=[],
                            )
                        ],
                        targets=["This"],
                    )
                )
                stats.parsed_lines += 1
                stats.emitted_defs["Continuous.ConditionalPower.Turn"] += 1
                continue

            match = re.match(r"^This card gets \+(\d+) power\.?$", remainder, re.I)
            if match:
                abilities.append(_template("ContinuousPower", amount=int(match.group(1))))
                stats.parsed_lines += 1
                stats.emitted_templates["ContinuousPower"] += 1
                continue

            all_chars_trait_power_exact = re.match(
                r"^If all of your characters are ((?:《[^》]+》(?: or )?)+), this card gets \+([+-]?\d+) power\.?$",
                conditional_body,
                re.I,
            )
            if all_chars_trait_power_exact:
                trait_names = [
                    trait.strip()
                    for trait in re.findall(r"《([^》]+)》", all_chars_trait_power_exact.group(1))
                    if trait.strip()
                ]
                zone_count = build_all_characters_trait_zone_count(trait_names)
                if zone_count is None:
                    mark_unsupported(line)
                    continue
                ability_defs.append(
                    _ability_def(
                        "Continuous",
                        None,
                        effects=[
                            _template(
                                "ConditionalAddPower",
                                amount=int(all_chars_trait_power_exact.group(2)),
                                turn=turn_condition,
                                zone_count=zone_count,
                                require_source_marker=False,
                                per_source_marker=False,
                                per_zone_count=False,
                                exclude_source=False,
                                target_ids=[],
                            )
                        ],
                        targets=["This"],
                    )
                )
                stats.parsed_lines += 1
                stats.emitted_defs["Continuous.AllCharactersTraitPower.Exact"] += 1
                continue

            all_chars_trait_level_power_exact = re.match(
                r"^If all of your characters are ((?:《[^》]+》(?: or )?)+), this card gets \+([+-]?\d+) level and \+([+-]?\d+) power\.?$",
                conditional_body,
                re.I,
            )
            if all_chars_trait_level_power_exact:
                trait_names = [
                    trait.strip()
                    for trait in re.findall(
                        r"《([^》]+)》", all_chars_trait_level_power_exact.group(1)
                    )
                    if trait.strip()
                ]
                zone_count = build_all_characters_trait_zone_count(trait_names)
                if zone_count is None:
                    mark_unsupported(line)
                    continue
                ability_defs.append(
                    _ability_def(
                        "Continuous",
                        None,
                        effects=[
                            _template(
                                "ConditionalAddLevel",
                                amount=int(all_chars_trait_level_power_exact.group(2)),
                                turn=turn_condition,
                                zone_count=zone_count,
                                require_source_marker=False,
                                per_source_marker=False,
                                per_zone_count=False,
                                exclude_source=False,
                                target_ids=[],
                            ),
                            _template(
                                "ConditionalAddPower",
                                amount=int(all_chars_trait_level_power_exact.group(3)),
                                turn=turn_condition,
                                zone_count=zone_count,
                                require_source_marker=False,
                                per_source_marker=False,
                                per_zone_count=False,
                                exclude_source=False,
                                target_ids=[],
                            ),
                        ],
                        targets=["This", "This"],
                    )
                )
                stats.parsed_lines += 1
                stats.emitted_defs["Continuous.AllTraitLevelPower.Exact"] += 1
                continue

            facing_gets_quoted_exact = re.match(
                r'^The character facing this card gets\s*"([^"]+)"\.?$',
                conditional_body,
                re.I,
            )
            if facing_gets_quoted_exact:
                nested_effect = parse_following_flatten_effect(
                    facing_gets_quoted_exact.group(1), duration_turn=False
                )
                if nested_effect is not None:
                    nested_id = nested_effect.get("id")
                    if nested_id == "AddSoul":
                        ability_defs.append(
                            _ability_def(
                                "Continuous",
                                None,
                                effects=[
                                    _template(
                                        "FacingOpponentAddSoul",
                                        amount=int(nested_effect.get("amount", 0)),
                                    )
                                ],
                                targets=[],
                            )
                        )
                        stats.parsed_lines += 1
                        stats.emitted_defs["Continuous.FacingOpponent.QuotedSoul"] += 1
                        continue
                    if nested_id == "CannotMoveStagePosition":
                        ability_defs.append(
                            _ability_def(
                                "Continuous",
                                None,
                                effects=[_template("FacingOpponentCannotMoveStagePosition")],
                                targets=[],
                            )
                        )
                        stats.parsed_lines += 1
                        stats.emitted_defs[
                            "Continuous.FacingOpponent.QuotedCannotMoveStagePosition"
                        ] += 1
                        continue

                if allow_approx_effects:
                    ability_defs.append(
                        _ability_def(
                            "Continuous",
                            None,
                            effects=[_template("Draw", count=0)],
                            targets=[],
                            conditions=with_approx_condition(),
                        )
                    )
                    stats.parsed_lines += 1
                    stats.emitted_defs["Continuous.FacingOpponent.QuotedAbility.ApproxNoop"] += 1
                    continue

            hand_level_if_trait_count_exact = re.match(
                rf"^If you have ({COUNT_TOKEN_RE}) or more ((?:《[^》]+》(?: or )?)+) characters, this card gets -({COUNT_TOKEN_RE}) level while in your hand\.?$",
                conditional_body,
                re.I,
            )
            if hand_level_if_trait_count_exact:
                threshold = parse_count_token(hand_level_if_trait_count_exact.group(1))
                level_delta = parse_count_token(hand_level_if_trait_count_exact.group(3))
                trait_names = [
                    trait.strip()
                    for trait in re.findall(
                        r"《([^》]+)》", hand_level_if_trait_count_exact.group(2)
                    )
                    if trait.strip()
                ]
                trait_ids: List[int] = []
                seen_trait_ids = set()
                unresolved_trait = False
                for trait_name in trait_names:
                    trait_id = (trait_map or {}).get(trait_name)
                    if trait_id is None:
                        unresolved_trait = True
                        break
                    if trait_id in seen_trait_ids:
                        continue
                    seen_trait_ids.add(trait_id)
                    trait_ids.append(trait_id)
                if threshold is None or level_delta is None or unresolved_trait or not trait_ids:
                    if try_parser_v2_fallback(line):
                        continue
                    mark_unsupported(line)
                    continue
                ability_defs.append(
                    _ability_def(
                        "Continuous",
                        None,
                        effects=[],
                        targets=[],
                        conditions={
                            "hand_level_delta": -int(level_delta),
                            "zone_count": {
                                "side": "SelfSide",
                                "zone": "Stage",
                                "cmp": "AtLeast",
                                "value": int(threshold),
                                "card_ids": trait_ids,
                            },
                        },
                    )
                )
                stats.parsed_lines += 1
                stats.emitted_defs["Continuous.HandLevelDelta.TraitCountAtLeast"] += 1
                continue

            hand_level_if_trait_count_alt_exact = re.match(
                rf"^If the number of ((?:《[^》]+》(?: or )?)+) characters you have is ({COUNT_TOKEN_RE}) or more, this card gets -({COUNT_TOKEN_RE}) level while in your hand\.?$",
                conditional_body,
                re.I,
            )
            if hand_level_if_trait_count_alt_exact:
                threshold = parse_count_token(hand_level_if_trait_count_alt_exact.group(2))
                level_delta = parse_count_token(hand_level_if_trait_count_alt_exact.group(3))
                trait_names = [
                    trait.strip()
                    for trait in re.findall(
                        r"《([^》]+)》", hand_level_if_trait_count_alt_exact.group(1)
                    )
                    if trait.strip()
                ]
                trait_ids: List[int] = []
                seen_trait_ids = set()
                unresolved_trait = False
                for trait_name in trait_names:
                    trait_id = (trait_map or {}).get(trait_name)
                    if trait_id is None:
                        unresolved_trait = True
                        break
                    if trait_id in seen_trait_ids:
                        continue
                    seen_trait_ids.add(trait_id)
                    trait_ids.append(trait_id)
                if threshold is None or level_delta is None or unresolved_trait or not trait_ids:
                    if try_parser_v2_fallback(line):
                        continue
                    mark_unsupported(line)
                    continue
                ability_defs.append(
                    _ability_def(
                        "Continuous",
                        None,
                        effects=[],
                        targets=[],
                        conditions={
                            "hand_level_delta": -int(level_delta),
                            "zone_count": {
                                "side": "SelfSide",
                                "zone": "Stage",
                                "cmp": "AtLeast",
                                "value": int(threshold),
                                "card_ids": trait_ids,
                            },
                        },
                    )
                )
                stats.parsed_lines += 1
                stats.emitted_defs["Continuous.HandLevelDelta.TraitCountAtLeastAlt"] += 1
                continue

            hand_level_if_named_count_in_wr_exact = re.match(
                rf'^If you have ({COUNT_TOKEN_RE}) or more "([^"]+)" in your waiting room, this card gets -({COUNT_TOKEN_RE}) level while in your hand\.?$',
                conditional_body,
                re.I,
            )
            if hand_level_if_named_count_in_wr_exact:
                threshold = parse_count_token(hand_level_if_named_count_in_wr_exact.group(1))
                level_delta = parse_count_token(hand_level_if_named_count_in_wr_exact.group(3))
                named_ids = sorted(
                    set(
                        (name_to_ids or {}).get(
                            hand_level_if_named_count_in_wr_exact.group(2).strip(), []
                        )
                    )
                )
                if threshold is None or level_delta is None or not named_ids:
                    if try_parser_v2_fallback(line):
                        continue
                    mark_unsupported(line)
                    continue
                ability_defs.append(
                    _ability_def(
                        "Continuous",
                        None,
                        effects=[],
                        targets=[],
                        conditions={
                            "hand_level_delta": -int(level_delta),
                            "zone_count": {
                                "side": "SelfSide",
                                "zone": "WaitingRoom",
                                "cmp": "AtLeast",
                                "value": int(threshold),
                                "card_ids": named_ids,
                            },
                        },
                    )
                )
                stats.parsed_lines += 1
                stats.emitted_defs["Continuous.HandLevelDelta.NamedCountInWaitingRoom"] += 1
                continue

            hand_level_if_wr_climax_more_exact = re.match(
                rf"^If your waiting room has ({COUNT_TOKEN_RE}) or more climax, this card gets -({COUNT_TOKEN_RE}) level while in your hand\.?$",
                conditional_body,
                re.I,
            )
            if hand_level_if_wr_climax_more_exact:
                threshold = parse_count_token(hand_level_if_wr_climax_more_exact.group(1))
                level_delta = parse_count_token(hand_level_if_wr_climax_more_exact.group(2))
                if threshold is None or level_delta is None:
                    if try_parser_v2_fallback(line):
                        continue
                    mark_unsupported(line)
                    continue
                ability_defs.append(
                    _ability_def(
                        "Continuous",
                        None,
                        effects=[],
                        targets=[],
                        conditions={
                            "hand_level_delta": -int(level_delta),
                            "zone_count": {
                                "side": "SelfSide",
                                "zone": "WaitingRoomClimax",
                                "cmp": "AtLeast",
                                "value": int(threshold),
                                "card_ids": [],
                            },
                        },
                    )
                )
                stats.parsed_lines += 1
                stats.emitted_defs["Continuous.HandLevelDelta.WaitingRoomClimaxAtLeast"] += 1
                continue

            hand_level_if_named_in_clock_exact = re.match(
                rf'^If a card named "([^"]+)" is in your clock, this card gets -({COUNT_TOKEN_RE}) level while in your hand\.?$',
                conditional_body,
                re.I,
            )
            if hand_level_if_named_in_clock_exact:
                level_delta = parse_count_token(hand_level_if_named_in_clock_exact.group(2))
                named_ids = sorted(
                    set(
                        (name_to_ids or {}).get(
                            hand_level_if_named_in_clock_exact.group(1).strip(), []
                        )
                    )
                )
                if level_delta is None or not named_ids:
                    if try_parser_v2_fallback(line):
                        continue
                    mark_unsupported(line)
                    continue
                ability_defs.append(
                    _ability_def(
                        "Continuous",
                        None,
                        effects=[],
                        targets=[],
                        conditions={
                            "hand_level_delta": -int(level_delta),
                            "self_clock_card_ids_any": named_ids,
                        },
                    )
                )
                stats.parsed_lines += 1
                stats.emitted_defs["Continuous.HandLevelDelta.ClockContainsNamed.Exact"] += 1
                continue

            cannot_stand_without_named_exact = re.match(
                r'^If you do not have another character with "([^"]+)" in (?:its )?card name, this card cannot 【STAND】 during your stand phase\.?$',
                conditional_body,
                re.I,
            )
            if cannot_stand_without_named_exact:
                target_ids = resolve_name_fragment_ids(
                    name_to_ids, cannot_stand_without_named_exact.group(1).strip()
                )
                if not target_ids:
                    if try_parser_v2_fallback(line):
                        continue
                    mark_unsupported(line)
                    continue
                # "another" excludes this card when it matches the same name fragment.
                at_most = 0
                if source_card_id is not None and source_card_id in target_ids:
                    at_most = 1
                ability_defs.append(
                    _ability_def(
                        "Continuous",
                        None,
                        effects=[_template("CannotStandDuringStandPhase", duration_turn=False)],
                        targets=["This"],
                        conditions={
                            "zone_count": {
                                "side": "SelfSide",
                                "zone": "Stage",
                                "cmp": "AtMost",
                                "value": at_most,
                                "card_ids": target_ids,
                            }
                        },
                    )
                )
                stats.parsed_lines += 1
                stats.emitted_defs["Continuous.CannotStandWithoutNamed"] += 1
                continue

            if allow_approx_effects:
                all_other_named_power_and_quoted = re.match(
                    r'^All of your other "([^"]+)" get \+([+-]?\d+) power and "[^"]+"\.?$',
                    conditional_body,
                    re.I,
                )
                if all_other_named_power_and_quoted:
                    target_ids = resolve_name_fragment_ids(
                        name_to_ids, all_other_named_power_and_quoted.group(1).strip()
                    )
                    if target_ids:
                        ability_defs.append(
                            _ability_def(
                                "Continuous",
                                None,
                                effects=[
                                    _template(
                                        "ConditionalAddPower",
                                        amount=int(all_other_named_power_and_quoted.group(2)),
                                        turn=turn_condition,
                                        zone_count=None,
                                        require_source_marker=False,
                                        per_source_marker=False,
                                        per_zone_count=False,
                                        exclude_source=True,
                                        target_ids=target_ids,
                                    )
                                ],
                                targets=["SelfStage"],
                                conditions=with_approx_condition(),
                            )
                        )
                        stats.parsed_lines += 1
                        stats.emitted_defs[
                            "Continuous.ConditionalPower.AllOtherNamed.QuotedAbility.Approx"
                        ] += 1
                        continue

                facing_level_power = re.match(
                    r"^If the character facing this card is level (\d+) or higher, this card gets \+([+-]?\d+) power\.?$",
                    conditional_body,
                    re.I,
                )
                if facing_level_power:
                    ability_defs.append(
                        _ability_def(
                            "Continuous",
                            None,
                            effects=[_template("Draw", count=0)],
                            targets=[],
                            conditions=with_approx_condition(),
                        )
                    )
                    stats.parsed_lines += 1
                    stats.emitted_defs["Continuous.FacingLevelAtLeastPower.ApproxNoop"] += 1
                    continue

                facing_cost_level_power = re.match(
                    rf"^If the character facing this card is cost ({COUNT_TOKEN_RE}) or lower, this card gets \+([+-]?\d+) level and \+([+-]?\d+) power\.?$",
                    conditional_body,
                    re.I,
                )
                if facing_cost_level_power:
                    ability_defs.append(
                        _ability_def(
                            "Continuous",
                            None,
                            effects=[_template("Draw", count=0)],
                            targets=[],
                            conditions=with_approx_condition(),
                        )
                    )
                    stats.parsed_lines += 1
                    stats.emitted_defs["Continuous.FacingCostAtMostLevelPower.ApproxNoop"] += 1
                    continue

                colors_on_stage_power = re.match(
                    rf"^If the number of colors of your characters on stage is ({COUNT_TOKEN_RE}) or more, this card gets \+([+-]?\d+) power\.?$",
                    conditional_body,
                    re.I,
                ) or re.match(
                    rf"^If your characters on stage have ({COUNT_TOKEN_RE}) or more colors in total, this card gets \+([+-]?\d+) power\.?$",
                    conditional_body,
                    re.I,
                )
                if colors_on_stage_power:
                    ability_defs.append(
                        _ability_def(
                            "Continuous",
                            None,
                            effects=[_template("Draw", count=0)],
                            targets=[],
                            conditions=with_approx_condition(),
                        )
                    )
                    stats.parsed_lines += 1
                    stats.emitted_defs["Continuous.StageColorCountPower.ApproxNoop"] += 1
                    continue

                marker_power_soul = re.match(
                    r"^If there is a marker underneath this card, this card gets \+([+-]?\d+) power and \+([+-]?\d+) soul\.?$",
                    conditional_body,
                    re.I,
                )
                if marker_power_soul:
                    ability_defs.append(
                        _ability_def(
                            "Continuous",
                            None,
                            effects=[_template("Draw", count=0)],
                            targets=[],
                            conditions=with_approx_condition(),
                        )
                    )
                    stats.parsed_lines += 1
                    stats.emitted_defs["Continuous.MarkerPowerSoul.ApproxNoop"] += 1
                    continue

                card_name_alias = re.match(
                    r'^If this card is in the stage, this card\'s card name will also be regarded as "[^"]+"\.?$',
                    conditional_body,
                    re.I,
                )
                if card_name_alias:
                    ability_defs.append(
                        _ability_def(
                            "Continuous",
                            None,
                            effects=[_template("Draw", count=0)],
                            targets=[],
                            conditions=with_approx_condition(),
                        )
                    )
                    stats.parsed_lines += 1
                    stats.emitted_defs["Continuous.CardNameAliasOnStage.ApproxNoop"] += 1
                    continue

                facing_level_multiplied = re.match(
                    rf"^This card gets \+X power\. X is equal to ({COUNT_TOKEN_RE}) multiplied by the level of the character facing this card\.?$",
                    conditional_body,
                    re.I,
                )
                if facing_level_multiplied:
                    ability_defs.append(
                        _ability_def(
                            "Continuous",
                            None,
                            effects=[_template("Draw", count=0)],
                            targets=[],
                            conditions=with_approx_condition(),
                        )
                    )
                    stats.parsed_lines += 1
                    stats.emitted_defs["Continuous.FacingOpponentLevelScaledPower.ApproxNoop"] += 1
                    continue

                all_chars_trait_power = re.match(
                    r"^If all of your characters are ((?:《[^》]+》(?: or )?)+), this card gets \+([+-]?\d+) power\.?$",
                    conditional_body,
                    re.I,
                )
                if all_chars_trait_power:
                    trait_names = [
                        trait.strip()
                        for trait in re.findall(r"《([^》]+)》", all_chars_trait_power.group(1))
                        if trait.strip()
                    ]
                    zone_count = build_all_characters_trait_zone_count(trait_names)
                    if zone_count is None:
                        mark_unsupported(line)
                        continue
                    ability_defs.append(
                        _ability_def(
                            "Continuous",
                            None,
                            effects=[
                                _template(
                                    "ConditionalAddPower",
                                    amount=int(all_chars_trait_power.group(2)),
                                    turn=turn_condition,
                                    zone_count=zone_count,
                                    require_source_marker=False,
                                    per_source_marker=False,
                                    per_zone_count=False,
                                    exclude_source=False,
                                    target_ids=[],
                                )
                            ],
                            targets=["This"],
                        )
                    )
                    stats.parsed_lines += 1
                    stats.emitted_defs["Continuous.AllCharsTraitPower.Exact"] += 1
                    continue

                all_chars_trait_level_power = re.match(
                    r"^If all of your characters are ((?:《[^》]+》(?: or )?)+), this card gets \+([+-]?\d+) level and \+([+-]?\d+) power\.?$",
                    conditional_body,
                    re.I,
                )
                if all_chars_trait_level_power:
                    trait_names = [
                        trait.strip()
                        for trait in re.findall(
                            r"《([^》]+)》", all_chars_trait_level_power.group(1)
                        )
                        if trait.strip()
                    ]
                    zone_count = build_all_characters_trait_zone_count(trait_names)
                    if zone_count is None:
                        mark_unsupported(line)
                        continue
                    ability_defs.append(
                        _ability_def(
                            "Continuous",
                            None,
                            effects=[
                                _template(
                                    "ConditionalAddLevel",
                                    amount=int(all_chars_trait_level_power.group(2)),
                                    turn=turn_condition,
                                    zone_count=zone_count,
                                    require_source_marker=False,
                                    per_source_marker=False,
                                    per_zone_count=False,
                                    exclude_source=False,
                                    target_ids=[],
                                ),
                                _template(
                                    "ConditionalAddPower",
                                    amount=int(all_chars_trait_level_power.group(3)),
                                    turn=turn_condition,
                                    zone_count=zone_count,
                                    require_source_marker=False,
                                    per_source_marker=False,
                                    per_zone_count=False,
                                    exclude_source=False,
                                    target_ids=[],
                                ),
                            ],
                            targets=["This", "This"],
                        )
                    )
                    stats.parsed_lines += 1
                    stats.emitted_defs["Continuous.AllTraitLevelPower.Exact"] += 1
                    continue

                no_markers_power_loss = re.match(
                    r"^If there are no markers underneath this card, this card gets -([+-]?\d+) power\.?$",
                    conditional_body,
                    re.I,
                )
                if no_markers_power_loss:
                    ability_defs.append(
                        _ability_def(
                            "Continuous",
                            None,
                            effects=[_template("Draw", count=0)],
                            targets=[],
                            conditions=with_approx_condition(),
                        )
                    )
                    stats.parsed_lines += 1
                    stats.emitted_defs["Continuous.NoMarkerPowerLoss.ApproxNoop"] += 1
                    continue

                center_trait_team_power = re.match(
                    r"^If this card is in your center stage, all of your 《[^》]+》 characters get \+([+-]?\d+) power\.?$",
                    conditional_body,
                    re.I,
                )
                if center_trait_team_power:
                    ability_defs.append(
                        _ability_def(
                            "Continuous",
                            None,
                            effects=[_template("Draw", count=0)],
                            targets=[],
                            conditions=with_approx_condition(),
                        )
                    )
                    stats.parsed_lines += 1
                    stats.emitted_defs["Continuous.CenterTraitTeamPower.ApproxNoop"] += 1
                    continue

                battle_no_backup = re.match(
                    r'^During this card\'s battle, all players cannot play "[^"]+" from (?:their )?hands?\.?$',
                    conditional_body,
                    re.I,
                )
                if battle_no_backup:
                    ability_defs.append(
                        _ability_def(
                            "Continuous",
                            None,
                            effects=[_template("Draw", count=0)],
                            targets=[],
                            conditions=with_approx_condition(),
                        )
                    )
                    stats.parsed_lines += 1
                    stats.emitted_defs["Continuous.BattleNoBackup.ApproxNoop"] += 1
                    continue

                battle_no_backup_opponent = re.match(
                    r'^During this card\'s battle, your opponent cannot play "[^"]+" from hand\.?$',
                    conditional_body,
                    re.I,
                )
                if battle_no_backup_opponent:
                    ability_defs.append(
                        _ability_def(
                            "Continuous",
                            None,
                            effects=[_template("Draw", count=0)],
                            targets=[],
                            conditions=with_approx_condition(),
                        )
                    )
                    stats.parsed_lines += 1
                    stats.emitted_defs["Continuous.BattleNoBackupOpponent.ApproxNoop"] += 1
                    continue

                middle_center_level_power = re.match(
                    r"^If this card is in the middle position of your center stage, this card gets \+([+-]?\d+) level and \+([+-]?\d+) power\.?$",
                    conditional_body,
                    re.I,
                )
                if middle_center_level_power:
                    ability_defs.append(
                        _ability_def(
                            "Continuous",
                            None,
                            effects=[_template("Draw", count=0)],
                            targets=[],
                            conditions=with_approx_condition(),
                        )
                    )
                    stats.parsed_lines += 1
                    stats.emitted_defs["Continuous.MiddleCenterLevelPower.ApproxNoop"] += 1
                    continue

                center_count_power = re.match(
                    rf"^If you have ({COUNT_TOKEN_RE}) or less other characters in your center stage, this card gets \+([+-]?\d+) power\.?$",
                    conditional_body,
                    re.I,
                )
                if center_count_power:
                    ability_defs.append(
                        _ability_def(
                            "Continuous",
                            None,
                            effects=[_template("Draw", count=0)],
                            targets=[],
                            conditions=with_approx_condition(),
                        )
                    )
                    stats.parsed_lines += 1
                    stats.emitted_defs["Continuous.CenterStageOtherCountPower.ApproxNoop"] += 1
                    continue

                all_other_named_soul = re.match(
                    r'^All of your other "[^"]+" get \+([+-]?\d+) soul\.?$',
                    conditional_body,
                    re.I,
                )
                if all_other_named_soul:
                    ability_defs.append(
                        _ability_def(
                            "Continuous",
                            None,
                            effects=[_template("Draw", count=0)],
                            targets=[],
                            conditions=with_approx_condition(),
                        )
                    )
                    stats.parsed_lines += 1
                    stats.emitted_defs["Continuous.AllOtherNamedSoul.ApproxNoop"] += 1
                    continue

                self_soul_per_other_named = re.match(
                    r'^This card gets \+([+-]?\d+) soul for each of your other "[^"]+"\.?$',
                    conditional_body,
                    re.I,
                )
                if self_soul_per_other_named:
                    ability_defs.append(
                        _ability_def(
                            "Continuous",
                            None,
                            effects=[_template("Draw", count=0)],
                            targets=[],
                            conditions=with_approx_condition(),
                        )
                    )
                    stats.parsed_lines += 1
                    stats.emitted_defs["Continuous.SelfSoulPerOtherNamed.ApproxNoop"] += 1
                    continue

                experience_power_following = re.match(
                    rf'^Experience During your turn, if the total level of the cards in your level is ({COUNT_TOKEN_RE}) or higher, this card gets \+([+-]?\d+) power and the following ability\.\s*"[^"]+"\.?$',
                    conditional_body,
                    re.I,
                )
                if experience_power_following:
                    ability_defs.append(
                        _ability_def(
                            "Continuous",
                            None,
                            effects=[_template("Draw", count=0)],
                            targets=[],
                            conditions=with_approx_condition(),
                        )
                    )
                    stats.parsed_lines += 1
                    stats.emitted_defs["Continuous.ExperiencePowerFollowing.ApproxNoop"] += 1
                    continue

                cannot_stand_without_named = re.match(
                    r'^If you do not have another character with "[^"]+" in (?:its )?card name, this card cannot 【STAND】 during your stand phase\.?$',
                    conditional_body,
                    re.I,
                )
                if cannot_stand_without_named:
                    ability_defs.append(
                        _ability_def(
                            "Continuous",
                            None,
                            effects=[_template("Draw", count=0)],
                            targets=[],
                            conditions=with_approx_condition(),
                        )
                    )
                    stats.parsed_lines += 1
                    stats.emitted_defs["Continuous.CannotStandWithoutNamed.ApproxNoop"] += 1
                    continue

                hand_level_if_trait_count = re.match(
                    rf"^If you have ({COUNT_TOKEN_RE}) or more 《[^》]+》 characters, this card gets -({COUNT_TOKEN_RE}) level while in your hand\.?$",
                    conditional_body,
                    re.I,
                )
                if hand_level_if_trait_count:
                    ability_defs.append(
                        _ability_def(
                            "Continuous",
                            None,
                            effects=[_template("Draw", count=0)],
                            targets=[],
                            conditions=with_approx_condition(),
                        )
                    )
                    stats.parsed_lines += 1
                    stats.emitted_defs["Continuous.HandLevelTraitCount.ApproxNoop"] += 1
                    continue

                hand_level_if_trait_count_alt = re.match(
                    rf"^If the number of 《[^》]+》 characters you have is ({COUNT_TOKEN_RE}) or more, this card gets -({COUNT_TOKEN_RE}) level while in your hand\.?$",
                    conditional_body,
                    re.I,
                )
                if hand_level_if_trait_count_alt:
                    ability_defs.append(
                        _ability_def(
                            "Continuous",
                            None,
                            effects=[_template("Draw", count=0)],
                            targets=[],
                            conditions=with_approx_condition(),
                        )
                    )
                    stats.parsed_lines += 1
                    stats.emitted_defs["Continuous.HandLevelTraitCountAlt.ApproxNoop"] += 1
                    continue

                hand_level_if_named_count_in_wr = re.match(
                    rf'^If you have ({COUNT_TOKEN_RE}) or more "[^"]+" in your waiting room, this card gets -({COUNT_TOKEN_RE}) level while in your hand\.?$',
                    conditional_body,
                    re.I,
                )
                if hand_level_if_named_count_in_wr:
                    ability_defs.append(
                        _ability_def(
                            "Continuous",
                            None,
                            effects=[_template("Draw", count=0)],
                            targets=[],
                            conditions=with_approx_condition(),
                        )
                    )
                    stats.parsed_lines += 1
                    stats.emitted_defs["Continuous.HandLevelNamedCountWr.ApproxNoop"] += 1
                    continue

                hand_level_if_wr_climax_more = re.match(
                    rf"^If your waiting room has ({COUNT_TOKEN_RE}) or more climax, this card gets -({COUNT_TOKEN_RE}) level while in your hand\.?$",
                    conditional_body,
                    re.I,
                )
                if hand_level_if_wr_climax_more:
                    ability_defs.append(
                        _ability_def(
                            "Continuous",
                            None,
                            effects=[_template("Draw", count=0)],
                            targets=[],
                            conditions=with_approx_condition(),
                        )
                    )
                    stats.parsed_lines += 1
                    stats.emitted_defs["Continuous.HandLevelWrClimaxMore.ApproxNoop"] += 1
                    continue

                hand_level_if_named_in_clock = re.match(
                    rf'^If a card named "[^"]+" is in your clock, this card gets -({COUNT_TOKEN_RE}) level while in your hand\.?$',
                    conditional_body,
                    re.I,
                )
                if hand_level_if_named_in_clock:
                    ability_defs.append(
                        _ability_def(
                            "Continuous",
                            None,
                            effects=[_template("Draw", count=0)],
                            targets=[],
                            conditions=with_approx_condition(),
                        )
                    )
                    stats.parsed_lines += 1
                    stats.emitted_defs["Continuous.HandLevelNamedClock.ApproxNoop"] += 1
                    continue

                per_other_level_cap_power = re.match(
                    rf"^This card gets \+([+-]?\d+) power for each of your other level ({COUNT_TOKEN_RE}) or lower characters\.?$",
                    conditional_body,
                    re.I,
                )
                if per_other_level_cap_power:
                    ability_defs.append(
                        _ability_def(
                            "Continuous",
                            None,
                            effects=[_template("Draw", count=0)],
                            targets=[],
                            conditions=with_approx_condition(),
                        )
                    )
                    stats.parsed_lines += 1
                    stats.emitted_defs["Continuous.PerOtherLevelCapPower.ApproxNoop"] += 1
                    continue

                marker_power_following = re.match(
                    r'^If there is a marker underneath this card, this card gets \+([+-]?\d+) power and the following ability\.\s*"[^"]+"\.?$',
                    conditional_body,
                    re.I,
                )
                if marker_power_following:
                    ability_defs.append(
                        _ability_def(
                            "Continuous",
                            None,
                            effects=[
                                _template(
                                    "ConditionalAddPower",
                                    amount=int(marker_power_following.group(1)),
                                    turn=turn_condition,
                                    zone_count=None,
                                    require_source_marker=True,
                                    per_source_marker=False,
                                    per_zone_count=False,
                                    exclude_source=False,
                                    target_ids=[],
                                )
                            ],
                            targets=["This"],
                            conditions=with_approx_condition(),
                        )
                    )
                    stats.parsed_lines += 1
                    stats.emitted_defs["Continuous.MarkerPowerFollowing.Approx"] += 1
                    continue

                all_chars_trait_power = re.match(
                    r"^If all of your characters are ((?:《[^》]+》(?: or )?)+), this card gets \+([+-]?\d+) power\.?$",
                    conditional_body,
                    re.I,
                )
                if all_chars_trait_power:
                    trait_names = [
                        trait.strip()
                        for trait in re.findall(r"《([^》]+)》", all_chars_trait_power.group(1))
                        if trait.strip()
                    ]
                    zone_count = build_all_characters_trait_zone_count(trait_names)
                    if zone_count is None:
                        mark_unsupported(line)
                        continue
                    ability_defs.append(
                        _ability_def(
                            "Continuous",
                            None,
                            effects=[
                                _template(
                                    "ConditionalAddPower",
                                    amount=int(all_chars_trait_power.group(2)),
                                    turn=turn_condition,
                                    zone_count=zone_count,
                                    require_source_marker=False,
                                    per_source_marker=False,
                                    per_zone_count=False,
                                    exclude_source=False,
                                    target_ids=[],
                                )
                            ],
                            targets=["This"],
                        )
                    )
                    stats.parsed_lines += 1
                    stats.emitted_defs["Continuous.AllCharactersTraitPower.Exact"] += 1
                    continue

                if "following ability" in conditional_body.lower():
                    ability_defs.append(
                        _ability_def(
                            "Continuous",
                            None,
                            effects=[_template("Draw", count=0)],
                            targets=[],
                            conditions=with_approx_condition(),
                        )
                    )
                    stats.parsed_lines += 1
                    stats.emitted_defs["Continuous.FollowingAbility.Generic.ApproxNoop"] += 1
                    continue

        if line_clean.startswith("【AUTO】"):
            remainder = line_clean[len("【AUTO】") :].strip()
            remainder = strip_activation_cap_prefix(remainder, True)
            remainder = re.sub(r"^【CXCOMBO】\s*", "", remainder, flags=re.I).strip()

            handled_auto_rule = False
            for rule in AUTO_RULES:
                if rule.id not in (
                    "Auto.TeamPowerOnClimaxPlaced",
                    "Auto.TeamPowerSoulOnClimaxPlaced",
                    "Auto.TeamPowerOnClimaxPlaced.OpponentNextTurn",
                ):
                    continue
                match = rule.pattern.match(remainder)
                if not match:
                    continue
                if not rule_enabled(rule):
                    continue
                count = parse_count_token(match.group(1))
                if count is None:
                    mark_unsupported(line)
                    handled_auto_rule = True
                    break
                if rule.id == "Auto.TeamPowerOnClimaxPlaced":
                    ability_defs.append(
                        _ability_def(
                            "Auto",
                            "AfterClimaxPhase",
                            effects=[
                                _template(
                                    "AddPower",
                                    amount=int(match.group(2)),
                                    duration_turn=True,
                                )
                            ],
                            targets=["SelfStage"],
                            target_limit=count,
                            conditions={"climax_area": {"side": "SelfSide", "card_ids": []}},
                        )
                    )
                elif rule.id == "Auto.TeamPowerOnClimaxPlaced.OpponentNextTurn":
                    granted = _ability_def(
                        "Continuous",
                        None,
                        effects=[
                            _template(
                                "AddPower",
                                amount=int(match.group(2)),
                                duration_turn=False,
                            )
                        ],
                        targets=["This"],
                    )
                    ability_defs.append(
                        _ability_def(
                            "Auto",
                            "AfterClimaxPhase",
                            effects=[
                                _template(
                                    "GrantAbilityDef",
                                    ability=granted,
                                    duration="UntilEndOfOpponentsNextTurn",
                                )
                            ],
                            targets=["SelfStage"],
                            target_limit=count,
                            conditions={"climax_area": {"side": "SelfSide", "card_ids": []}},
                        )
                    )
                else:
                    ability_defs.append(
                        _ability_def(
                            "Auto",
                            "AfterClimaxPhase",
                            effects=[
                                _template(
                                    "AddPower",
                                    amount=int(match.group(2)),
                                    duration_turn=True,
                                ),
                                _template(
                                    "AddSoul",
                                    amount=int(match.group(3)),
                                    duration_turn=True,
                                ),
                            ],
                            targets=["SelfStage", "SelfStage"],
                            target_limit=count,
                            conditions={"climax_area": {"side": "SelfSide", "card_ids": []}},
                        )
                    )
                stats.parsed_lines += 1
                stats.emitted_defs[rule.id] += 1
                handled_auto_rule = True
                break
            if handled_auto_rule:
                continue

            if allow_approx_effects:
                cxcombo_climax_named_following = re.match(
                    rf'^When "[^"]+" is placed on your climax area, choose (up to )?({COUNT_TOKEN_RE}) of your other characters, and that character gets the following ability until end of turn\.\s*"[^"]+"\.?$',
                    remainder,
                    re.I,
                )
                if cxcombo_climax_named_following:
                    ability_defs.append(
                        _ability_def(
                            "Auto",
                            "AfterClimaxPhase",
                            effects=[_template("Draw", count=0)],
                            targets=[],
                            conditions=with_approx_condition(
                                {"climax_area": {"side": "SelfSide", "card_ids": []}}
                            ),
                        )
                    )
                    stats.parsed_lines += 1
                    stats.emitted_defs["Auto.CxcomboFollowingAbility.ApproxNoop"] += 1
                    continue

            if allow_approx_effects:
                on_play_cancel_burnx = re.match(
                    rf"^During the turn that this card is placed on the stage from your hand, when damage dealt by this card is canceled, put the top card of your deck into your waiting room, and deal X damage to your opponent\. X is equal to the level of that card \+({COUNT_TOKEN_RE})\. \(Climax are regarded as level ({COUNT_TOKEN_RE})\. Damage may be canceled\)\.?$",
                    remainder,
                    re.I,
                )
                if on_play_cancel_burnx:
                    ability_defs.append(
                        _ability_def(
                            "Auto",
                            None,
                            effects=[_template("Draw", count=0)],
                            targets=[],
                            conditions=with_approx_condition(),
                        )
                    )
                    stats.parsed_lines += 1
                    stats.emitted_defs["Auto.CancelBurnX.OnPlayTurn.ApproxNoop"] += 1
                    continue

            begin_climax_team_power = re.match(
                rf"^At the beginning of (?:your )?climax phase, choose (up to )?({COUNT_TOKEN_RE}) of your characters, and that character gets \+(\d+) power until end of turn\.?$",
                remainder,
                re.I,
            )
            if begin_climax_team_power:
                choose_count = parse_count_token(begin_climax_team_power.group(2))
                if choose_count is None:
                    mark_unsupported(line)
                    continue
                optional = bool(begin_climax_team_power.group(1))
                ability_defs.append(
                    _ability_def(
                        "Auto",
                        "BeginClimaxPhase",
                        effects=[
                            _template(
                                "AddPower",
                                amount=int(begin_climax_team_power.group(3)),
                                duration_turn=True,
                            )
                        ],
                        targets=["SelfStage"],
                        cost=cost,
                        target_limit=choose_count,
                        effect_optional=[optional] if optional else [],
                    )
                )
                stats.parsed_lines += 1
                stats.emitted_defs["Auto.AddPower.BeginClimaxPhase"] += 1
                continue

            resonate_begin_climax_self_power = re.match(
                r"^(?:(?:Resonate|Accelerate)\s+)?At the beginning of (?:your )?climax phase, you may pay the cost\. If you do, this card gets \+(\d+) power until end of turn\.?$",
                remainder,
                re.I,
            )
            if resonate_begin_climax_self_power:
                if cost_is_empty(cost):
                    mark_unsupported(line)
                    continue
                ability_defs.append(
                    _ability_def(
                        "Auto",
                        "BeginClimaxPhase",
                        effects=[
                            _template(
                                "AddPower",
                                amount=int(resonate_begin_climax_self_power.group(1)),
                                duration_turn=True,
                            )
                        ],
                        targets=["This"],
                        cost=cost,
                        effect_optional=[True],
                    )
                )
                stats.parsed_lines += 1
                stats.emitted_defs["Auto.AddPowerSelf.BeginClimaxPhase.Paid"] += 1
                continue

            trigger_check_climax_team_power = re.match(
                rf"^When your character's trigger check reveals a climax, choose (up to )?({COUNT_TOKEN_RE}) of your characters, and that character gets \+(\d+) power until end of turn\.?$",
                remainder,
                re.I,
            )
            if trigger_check_climax_team_power:
                choose_count = parse_count_token(trigger_check_climax_team_power.group(2))
                if choose_count is None:
                    mark_unsupported(line)
                    continue
                optional = bool(trigger_check_climax_team_power.group(1))
                ability_defs.append(
                    _ability_def(
                        "Auto",
                        "TriggerResolution",
                        effects=[
                            _template(
                                "AddPower",
                                amount=int(trigger_check_climax_team_power.group(3)),
                                duration_turn=True,
                            )
                        ],
                        targets=["SelfStage"],
                        cost=cost,
                        target_limit=choose_count,
                        effect_optional=[optional] if optional else [],
                        conditions={"trigger_check_revealed_climax": True},
                    )
                )
                stats.parsed_lines += 1
                stats.emitted_defs["Auto.AddPower.OnTriggerCheckClimax"] += 1
                continue

            use_act_team_power = re.match(
                rf"^When you use an 【ACT】, choose (up to )?({COUNT_TOKEN_RE}) of your characters(?: in battle)?, and that character gets \+(\d+) power until end of turn\.?$",
                remainder,
                re.I,
            )
            if use_act_team_power:
                choose_count = parse_count_token(use_act_team_power.group(2))
                if choose_count is None:
                    mark_unsupported(line)
                    continue
                optional = bool(use_act_team_power.group(1))
                ability_defs.append(
                    _ability_def(
                        "Auto",
                        "UseAct",
                        effects=[
                            _template(
                                "AddPower",
                                amount=int(use_act_team_power.group(3)),
                                duration_turn=True,
                            )
                        ],
                        targets=["SelfStage"],
                        cost=cost,
                        target_limit=choose_count,
                        effect_optional=[optional],
                    )
                )
                stats.parsed_lines += 1
                stats.emitted_defs["Auto.AddPower.OnUseAct"] += 1
                continue

            use_this_card_if_has_trait_buff = re.match(
                rf'^When you use this card\'s "[^"]+", if you have a 《([^》]+)》 character, choose (up to )?({COUNT_TOKEN_RE}) of your characters in battle, and that character gets \+([+-]?\d+) power until end of turn\.?$',
                remainder,
                re.I,
            )
            if use_this_card_if_has_trait_buff:
                choose_count = parse_count_token(use_this_card_if_has_trait_buff.group(3))
                trait_name = use_this_card_if_has_trait_buff.group(1).strip()
                trait_ids = sorted(set((trait_to_ids or {}).get(trait_name, [])))
                if choose_count is None or not trait_ids:
                    mark_unsupported(line)
                    continue
                optional = bool(use_this_card_if_has_trait_buff.group(2))
                ability_defs.append(
                    _ability_def(
                        "Auto",
                        "UseAct",
                        effects=[
                            _template(
                                "TimedConditionalAddPower",
                                amount=int(use_this_card_if_has_trait_buff.group(4)),
                                duration_turn=True,
                                turn=None,
                                zone_count={
                                    "side": "SelfSide",
                                    "zone": "Stage",
                                    "cmp": "AtLeast",
                                    "value": 1,
                                    "card_ids": trait_ids,
                                },
                                require_source_marker=False,
                                per_source_marker=False,
                                per_zone_count=False,
                                exclude_source=False,
                                target_ids=[],
                            )
                        ],
                        targets=["SelfStage"],
                        cost=cost,
                        effect_optional=[optional] if optional else [],
                        target_limit=choose_count,
                    )
                )
                stats.parsed_lines += 1
                stats.emitted_defs["Auto.AddPower.IfHasTrait.OnUseThisCard"] += 1
                continue

            use_this_card_if_has_traits_buff = (
                re.match(
                    rf'^When you use this card\'s "[^"]+", if you have ({COUNT_TOKEN_RE}) or more ((?:《[^》]+》(?: or )?)+) characters?, choose (up to )?({COUNT_TOKEN_RE}) of your characters in battle, and that character gets \+([+-]?\d+) power until end of turn\.?$',
                    remainder,
                    re.I,
                )
                or re.match(
                    rf'^When you use this card\'s "[^"]+", if you have an? ((?:《[^》]+》(?: or )?)+) character, choose (up to )?({COUNT_TOKEN_RE}) of your characters in battle, and that character gets \+([+-]?\d+) power until end of turn\.?$',
                    remainder,
                    re.I,
                )
                or re.match(
                    rf'^When you use this card\'s "[^"]+", if the number of ((?:《[^》]+》(?: or )?)+) characters you have is ({COUNT_TOKEN_RE}) or more, choose (up to )?({COUNT_TOKEN_RE}) of your characters in battle, and that character gets \+([+-]?\d+) power until end of turn\.?$',
                    remainder,
                    re.I,
                )
            )
            if use_this_card_if_has_traits_buff:
                groups = use_this_card_if_has_traits_buff.groups()
                min_count = 1
                trait_blob = ""
                optional_flag = False
                choose_count_token: Optional[str] = None
                amount_token: Optional[str] = None
                # Pattern A: (min_count, trait_blob, optional, choose_count, amount)
                if len(groups) == 5 and groups[1] is not None:
                    parsed_min = parse_count_token(groups[0])
                    min_count = parsed_min if parsed_min is not None else 1
                    trait_blob = groups[1]
                    optional_flag = bool(groups[2])
                    choose_count_token = groups[3]
                    amount_token = groups[4]
                # Pattern B: (trait_blob, optional, choose_count, amount)
                elif len(groups) == 4 and groups[1] is not None:
                    trait_blob = groups[0]
                    optional_flag = bool(groups[1])
                    choose_count_token = groups[2]
                    amount_token = groups[3]
                # Pattern C: (trait_blob, min_count, optional, choose_count, amount)
                elif len(groups) == 5:
                    trait_blob = groups[0]
                    parsed_min = parse_count_token(groups[1])
                    min_count = parsed_min if parsed_min is not None else 1
                    optional_flag = bool(groups[2])
                    choose_count_token = groups[3]
                    amount_token = groups[4]
                choose_count = parse_count_token(choose_count_token or "")
                if choose_count is None or amount_token is None:
                    mark_unsupported(line)
                    continue
                trait_names = [
                    trait.strip()
                    for trait in re.findall(r"《([^》]+)》", trait_blob)
                    if trait.strip()
                ]
                trait_ids: set[int] = set()
                for trait_name in trait_names:
                    for card_id in (trait_to_ids or {}).get(trait_name, []):
                        trait_ids.add(card_id)
                if not trait_ids:
                    mark_unsupported(line)
                    continue
                ability_defs.append(
                    _ability_def(
                        "Auto",
                        "UseAct",
                        effects=[
                            _template(
                                "TimedConditionalAddPower",
                                amount=int(amount_token),
                                duration_turn=True,
                                turn=None,
                                zone_count={
                                    "side": "SelfSide",
                                    "zone": "Stage",
                                    "cmp": "AtLeast",
                                    "value": max(1, int(min_count)),
                                    "card_ids": sorted(trait_ids),
                                },
                                require_source_marker=False,
                                per_source_marker=False,
                                per_zone_count=False,
                                exclude_source=False,
                                target_ids=[],
                            )
                        ],
                        targets=["SelfStage"],
                        cost=cost,
                        effect_optional=[optional_flag] if optional_flag else [],
                        target_limit=choose_count,
                    )
                )
                stats.parsed_lines += 1
                stats.emitted_defs["Auto.AddPower.IfHasTraits.OnUseThisCard"] += 1
                continue

            use_act_self_power = re.match(
                r"^When you use an 【ACT】, this card gets \+(\d+) power until end of turn\.?$",
                remainder,
                re.I,
            )
            if use_act_self_power:
                ability_defs.append(
                    _ability_def(
                        "Auto",
                        "UseAct",
                        effects=[
                            _template(
                                "AddPower",
                                amount=int(use_act_self_power.group(1)),
                                duration_turn=True,
                            )
                        ],
                        targets=["This"],
                        cost=cost,
                    )
                )
                stats.parsed_lines += 1
                stats.emitted_defs["Auto.AddPowerSelf.OnUseAct"] += 1
                continue

            use_act_following = re.match(
                r'^When you use an 【ACT】, this card gets the following ability until end of turn\.\s*"(.+)"\.?$',
                remainder,
                re.I,
            )
            if use_act_following:
                nested_effect = parse_following_effect_or_grant(
                    use_act_following.group(1),
                    duration_turn=True,
                    grant_duration="UntilEndOfTurn",
                )
                if nested_effect is not None:
                    ability_defs.append(
                        _ability_def(
                            "Auto",
                            "UseAct",
                            effects=[nested_effect],
                            targets=["This"],
                            cost=cost,
                        )
                    )
                    stats.parsed_lines += 1
                    stats.emitted_defs["Auto.FollowingAbilityFlattened.OnUseAct"] += 1
                    continue
                if allow_approx_effects:
                    ability_defs.append(
                        _ability_def(
                            "Auto",
                            "UseAct",
                            effects=[_template("Draw", count=0)],
                            targets=[],
                            conditions=with_approx_condition(),
                        )
                    )
                    stats.parsed_lines += 1
                    stats.emitted_defs["Auto.FollowingAbility.OnUseAct.ApproxNoop"] += 1
                    continue

            use_this_card_following = re.match(
                rf'^(?:This ability activates up to (?:{COUNT_TOKEN_RE}) time(?:s)? per turn\.\s*)?When you use this card\'s "[^"]+", choose (up to )?({COUNT_TOKEN_RE}) of your characters(?: in battle)?, and that character gets the following ability until end of turn\.\s*"(.+)"\.?$',
                remainder,
                re.I,
            )
            if use_this_card_following:
                choose_count = parse_count_token(use_this_card_following.group(2))
                if choose_count is None:
                    if try_parser_v2_fallback(line):
                        continue
                    mark_unsupported(line)
                    continue
                optional = bool(use_this_card_following.group(1))
                nested_effect = parse_following_effect_or_grant(
                    use_this_card_following.group(3),
                    duration_turn=True,
                    grant_duration="UntilEndOfTurn",
                )
                if nested_effect is None:
                    if allow_approx_effects:
                        ability_defs.append(
                            _ability_def(
                                "Auto",
                                None,
                                effects=[_template("Draw", count=0)],
                                targets=[],
                                cost=cost,
                                conditions=with_approx_condition(),
                            )
                        )
                        stats.parsed_lines += 1
                        stats.emitted_defs["Auto.FollowingAbility.OnUseThisCard.ApproxNoop"] += 1
                    else:
                        if try_parser_v2_fallback(line):
                            continue
                        mark_unsupported(line)
                    continue
                ability_defs.append(
                    _ability_def(
                        "Auto",
                        "UseAct",
                        effects=[nested_effect],
                        targets=["SelfStage"],
                        cost=cost,
                        effect_optional=[optional] if optional else [],
                        target_limit=choose_count,
                    )
                )
                stats.parsed_lines += 1
                stats.emitted_defs["Auto.FollowingAbilityFlattened.OnUseThisCard"] += 1
                continue

            use_this_card_mill_top = re.match(
                rf'^(?:This ability activates up to (?:{COUNT_TOKEN_RE}) time(?:s)? per turn\.\s*)?When you use this card\'s "[^"]+", put the top ({COUNT_TOKEN_RE}) card(?:s)? of your deck into your waiting room\.?$',
                remainder,
                re.I,
            )
            if use_this_card_mill_top:
                mill_count = parse_count_token(use_this_card_mill_top.group(1))
                if mill_count is None:
                    mark_unsupported(line)
                    continue
                ability_defs.append(
                    _ability_def(
                        "Auto",
                        "UseAct",
                        effects=[_template("MoveToWaitingRoom")],
                        targets=["SelfDeckTop"],
                        cost=cost,
                        target_limit=mill_count,
                    )
                )
                stats.parsed_lines += 1
                stats.emitted_defs["Auto.MillTop.OnUseThisCard"] += 1
                continue

            use_this_card_look_top_or_bottom = re.match(
                r'^When you use this card\'s "[^"]+", look at the top card of your deck, and put it on the top or (?:at )?the bottom of your deck\.?$',
                remainder,
                re.I,
            )
            if use_this_card_look_top_or_bottom:
                ability_defs.append(
                    _ability_def(
                        "Auto",
                        "UseAct",
                        effects=[_template("LookTopCardTopOrBottom")],
                        targets=["SelfDeckTop"],
                        cost=cost,
                        target_limit=1,
                    )
                )
                stats.parsed_lines += 1
                stats.emitted_defs["Auto.LookTopTopOrBottom.OnUseThisCard"] += 1
                continue

            use_this_card_look_top_or_waiting = re.match(
                r'^When you use this card\'s "[^"]+", look at the top card of your deck, and put it on the top of your deck or into your waiting room\.?$',
                remainder,
                re.I,
            )
            if use_this_card_look_top_or_waiting:
                ability_defs.append(
                    _ability_def(
                        "Auto",
                        "UseAct",
                        effects=[_template("LookTopCardTopOrWaitingRoom")],
                        targets=["SelfDeckTop"],
                        cost=cost,
                        target_limit=1,
                    )
                )
                stats.parsed_lines += 1
                stats.emitted_defs["Auto.LookTopTopOrWaitingRoom.OnUseThisCard"] += 1
                continue

            use_this_card_if_all_trait_stock = re.match(
                r'^When you use this card\'s "[^"]+", if all of your characters are ((?:《[^》]+》(?: or )?)+), you may put the top card of your deck into your stock\.?$',
                remainder,
                re.I,
            )
            if use_this_card_if_all_trait_stock:
                trait_names = [
                    trait.strip()
                    for trait in re.findall(
                        r"《([^》]+)》", use_this_card_if_all_trait_stock.group(1)
                    )
                    if trait.strip()
                ]
                zone_count = build_all_characters_trait_zone_count(trait_names)
                if zone_count is None:
                    mark_unsupported(line)
                    continue
                ability_defs.append(
                    _ability_def(
                        "Auto",
                        "UseAct",
                        effects=[_template("MoveToStock")],
                        targets=["SelfDeckTop"],
                        cost=cost,
                        effect_optional=[True],
                        target_limit=1,
                        conditions={"zone_count": zone_count},
                    )
                )
                stats.parsed_lines += 1
                stats.emitted_defs["Auto.StockIfAllCharactersTrait.OnUseThisCard"] += 1
                continue

            use_this_card_recycle = re.match(
                rf'^(?:This ability activates up to (?:{COUNT_TOKEN_RE}) time(?:s)? per turn\.\s*)?When you use this card\'s "[^"]+", you may pay the cost\. If you do, return all cards from your waiting room to your deck, and shuffle your deck\.?$',
                remainder,
                re.I,
            )
            if use_this_card_recycle:
                ability_defs.append(
                    _ability_def(
                        "Auto",
                        "UseAct",
                        effects=[_template("RecycleWaitingRoomToDeckShuffle")],
                        targets=[],
                        cost=cost,
                    )
                )
                stats.parsed_lines += 1
                stats.emitted_defs["Auto.RecycleWaitingRoom.OnUseThisCard"] += 1
                continue

            use_this_card_paid_salvage = re.match(
                rf'^(?:This ability activates up to (?:{COUNT_TOKEN_RE}) time(?:s)? per turn\.\s*)?When you use this card\'s "[^"]+", you may pay the cost\. If you do, choose (up to )?({COUNT_TOKEN_RE}) (.+?) in your waiting room, and return (?:it|them) to your hand\.?$',
                remainder,
                re.I,
            )
            if use_this_card_paid_salvage:
                choose_count = parse_count_token(use_this_card_paid_salvage.group(2))
                if choose_count is None:
                    mark_unsupported(line)
                    continue
                selector_constraints = parse_generic_selector_constraints(
                    use_this_card_paid_salvage.group(3)
                )
                if selector_constraints is not None:
                    ability_defs.append(
                        _ability_def(
                            "Auto",
                            "UseAct",
                            effects=[_template("MoveToHand")],
                            targets=["SelfWaitingRoom"],
                            cost=cost,
                            effect_optional=[True],
                            target_card_type=selector_constraints["target_card_type"],
                            target_trait=selector_constraints["target_trait"],
                            target_level_max=selector_constraints["target_level_max"],
                            target_cost_max=selector_constraints["target_cost_max"],
                            target_card_ids=selector_constraints["target_card_ids"],
                            target_limit=choose_count,
                        )
                    )
                    stats.parsed_lines += 1
                    stats.emitted_defs["Auto.SalvageWaitingRoom.OnUseThisCard.Paid"] += 1
                    continue

            use_this_card_paid_damage = re.match(
                rf'^(?:This ability activates up to (?:{COUNT_TOKEN_RE}) time(?:s)? per turn\.\s*)?When you use this card\'s "[^"]+", you may pay the cost\. If you do, deal ({COUNT_TOKEN_RE}) damage to your opponent\.?(?:\s*\([^)]*\))?$',
                remainder,
                re.I,
            )
            if use_this_card_paid_damage:
                amount = parse_count_token(use_this_card_paid_damage.group(1))
                if amount is None:
                    mark_unsupported(line)
                    continue
                cancelable = "cannot be canceled" not in remainder.lower()
                ability_defs.append(
                    _ability_def(
                        "Auto",
                        "UseAct",
                        effects=[_template("DealDamage", amount=amount, cancelable=cancelable)],
                        targets=[],
                        cost=cost,
                        effect_optional=[True],
                    )
                )
                stats.parsed_lines += 1
                stats.emitted_defs["Auto.DealDamage.OnUseThisCard.Paid"] += 1
                continue

            use_this_card_team_power = re.match(
                rf'^(?:This ability activates up to (?:{COUNT_TOKEN_RE}) time(?:s)? per turn\.\s*)?When you use this card\'s "[^"]+", (?:if you have another ((?:《[^》]+》(?: or )?)+) character, )?choose (up to )?({COUNT_TOKEN_RE}) of your(?: (.+?))? characters(?: in battle)?, and that character gets \+([+-]?\d+) power until end of turn\.?$',
                remainder,
                re.I,
            )
            if use_this_card_team_power:
                gate_trait_blob = (use_this_card_team_power.group(1) or "").strip()
                choose_count = parse_count_token(use_this_card_team_power.group(3))
                if choose_count is None:
                    mark_unsupported(line)
                    continue
                selector = (use_this_card_team_power.group(4) or "").strip()
                optional = bool(use_this_card_team_power.group(2))

                target_trait: Optional[int] = None
                target_card_ids: List[int] = []
                exclude_source = False
                if selector:
                    selector_for_resolution = selector
                    selector_lower = selector.lower()
                    if selector_lower.startswith("other "):
                        exclude_source = True
                        selector_for_resolution = selector[6:].strip()
                    target_trait = resolve_single_trait_selector(selector_for_resolution)
                    if target_trait is None:
                        target_card_ids = (
                            resolve_stage_selector_card_ids(selector_for_resolution) or []
                        )
                        if not target_card_ids:
                            if allow_approx_effects:
                                ability_defs.append(
                                    _ability_def(
                                        "Auto",
                                        "UseAct",
                                        effects=[_template("Draw", count=0)],
                                        targets=[],
                                        cost=cost,
                                        conditions=with_approx_condition(),
                                    )
                                )
                                stats.parsed_lines += 1
                                stats.emitted_defs["Auto.UseThisCardAbility.ApproxNoop"] += 1
                                continue
                            mark_unsupported(line)
                            continue

                conditions: Dict[str, Any] = {}
                if gate_trait_blob:
                    gate_traits = [
                        trait.strip()
                        for trait in re.findall(r"《([^》]+)》", gate_trait_blob)
                        if trait.strip()
                    ]
                    gate_ids: set[int] = set()
                    for trait_name in gate_traits:
                        gate_ids.update((trait_to_ids or {}).get(trait_name, []))
                    if not gate_ids:
                        mark_unsupported(line)
                        continue
                    threshold = 1 + (
                        1 if source_card_id is not None and source_card_id in gate_ids else 0
                    )
                    conditions["zone_count"] = {
                        "side": "SelfSide",
                        "zone": "Stage",
                        "cmp": "AtLeast",
                        "value": threshold,
                        "card_ids": sorted(gate_ids),
                    }

                if exclude_source:
                    effect = _template(
                        "TimedConditionalAddPower",
                        amount=int(use_this_card_team_power.group(5)),
                        duration_turn=True,
                        turn=None,
                        zone_count=None,
                        require_source_marker=False,
                        per_source_marker=False,
                        per_zone_count=False,
                        exclude_source=True,
                        target_ids=[],
                    )
                else:
                    effect = _template(
                        "AddPower",
                        amount=int(use_this_card_team_power.group(5)),
                        duration_turn=True,
                    )
                ability_defs.append(
                    _ability_def(
                        "Auto",
                        "UseAct",
                        effects=[effect],
                        targets=["SelfStage"],
                        cost=cost,
                        conditions=conditions or None,
                        effect_optional=[optional] if optional else [],
                        target_trait=target_trait,
                        target_card_ids=target_card_ids,
                        target_limit=choose_count,
                    )
                )
                stats.parsed_lines += 1
                stats.emitted_defs["Auto.AddPower.OnUseThisCard"] += 1
                continue

            use_this_card_team_soul = re.match(
                rf'^(?:This ability activates up to (?:{COUNT_TOKEN_RE}) time(?:s)? per turn\.\s*)?When you use this card\'s "[^"]+", choose (up to )?({COUNT_TOKEN_RE}) of your characters(?: in battle)?, and that character gets \+([+-]?\d+) soul until end of turn\.?$',
                remainder,
                re.I,
            )
            if use_this_card_team_soul:
                choose_count = parse_count_token(use_this_card_team_soul.group(2))
                if choose_count is None:
                    mark_unsupported(line)
                    continue
                optional = bool(use_this_card_team_soul.group(1))
                ability_defs.append(
                    _ability_def(
                        "Auto",
                        "UseAct",
                        effects=[
                            _template(
                                "AddSoul",
                                amount=int(use_this_card_team_soul.group(3)),
                                duration_turn=True,
                            )
                        ],
                        targets=["SelfStage"],
                        cost=cost,
                        effect_optional=[optional] if optional else [],
                        target_limit=choose_count,
                    )
                )
                stats.parsed_lines += 1
                stats.emitted_defs["Auto.AddSoul.OnUseThisCard"] += 1
                continue

            use_this_card_reduce_opponent_soul_paid = re.match(
                rf'^(?:This ability activates up to (?:{COUNT_TOKEN_RE}) time(?:s)? per turn\.\s*)?When you use this card\'s "[^"]+", you may pay the cost\. If you do, choose (up to )?({COUNT_TOKEN_RE}) of your opponent\'s characters(?: in battle)?, and that character gets -([+-]?\d+) soul until end of turn\.?$',
                remainder,
                re.I,
            )
            if use_this_card_reduce_opponent_soul_paid:
                choose_count = parse_count_token(use_this_card_reduce_opponent_soul_paid.group(2))
                if choose_count is None:
                    mark_unsupported(line)
                    continue
                ability_defs.append(
                    _ability_def(
                        "Auto",
                        "UseAct",
                        effects=[
                            _template(
                                "AddSoul",
                                amount=-int(use_this_card_reduce_opponent_soul_paid.group(3)),
                                duration_turn=True,
                            )
                        ],
                        targets=["OppStage"],
                        cost=cost,
                        effect_optional=[True],
                        target_limit=choose_count,
                    )
                )
                stats.parsed_lines += 1
                stats.emitted_defs["Auto.ReduceSoul.OnUseThisCard.Paid"] += 1
                continue

            on_play_following_next_turn = re.match(
                rf'^When this card is placed on (?:the )?stage from your hand, choose (up to )?({COUNT_TOKEN_RE}) of your opponent\'s characters, and that character gets the following ability until the end of your opponent\'s next turn\.\s*"(.+)"\.?$',
                remainder,
                re.I,
            )
            if on_play_following_next_turn:
                choose_count = parse_count_token(on_play_following_next_turn.group(2))
                if choose_count is None:
                    if try_parser_v2_fallback(line):
                        continue
                    mark_unsupported(line)
                    continue
                nested_effect = parse_following_effect_or_grant(
                    on_play_following_next_turn.group(3),
                    duration_turn=True,
                    grant_duration="UntilEndOfOpponentsNextTurn",
                )
                if nested_effect is not None:
                    ability_defs.append(
                        _ability_def(
                            "Auto",
                            "OnPlay",
                            effects=[nested_effect],
                            targets=["OppStage"],
                            cost=cost,
                            effect_optional=(
                                [True] if on_play_following_next_turn.group(1) else []
                            ),
                            target_limit=choose_count,
                        )
                    )
                    stats.parsed_lines += 1
                    stats.emitted_defs["Auto.OnPlayFollowingAbilityOpponentNextTurn.Flattened"] += 1
                elif allow_approx_effects:
                    ability_defs.append(
                        _ability_def(
                            "Auto",
                            "OnPlay",
                            effects=[_template("Draw", count=0)],
                            targets=[],
                            cost=cost,
                            conditions=with_approx_condition(),
                        )
                    )
                    stats.parsed_lines += 1
                    stats.emitted_defs[
                        "Auto.OnPlayFollowingAbilityOpponentNextTurn.ApproxNoop"
                    ] += 1
                else:
                    if try_parser_v2_fallback(line):
                        continue
                    mark_unsupported(line)
                continue

            if allow_approx_effects:
                approx_use_this_card = re.match(
                    rf'^(?:This ability activates up to (?:{COUNT_TOKEN_RE}) time(?:s)? per turn\.\s*)?When you use this card\'s "([^"]+)", .+$',
                    remainder,
                    re.I,
                )
                if approx_use_this_card:
                    ability_defs.append(
                        _ability_def(
                            "Auto",
                            None,
                            effects=[_template("Draw", count=0)],
                            targets=[],
                            cost=cost,
                            conditions=with_approx_condition(),
                        )
                    )
                    stats.parsed_lines += 1
                    stats.emitted_defs["Auto.UseThisCardAbility.ApproxNoop"] += 1
                    continue

                if re.search(r'When you use this card\'s "([^"]+)"', remainder, re.I):
                    ability_defs.append(
                        _ability_def(
                            "Auto",
                            None,
                            effects=[_template("Draw", count=0)],
                            targets=[],
                            cost=cost,
                            conditions=with_approx_condition(),
                        )
                    )
                    stats.parsed_lines += 1
                    stats.emitted_defs["Auto.UseThisCardAbility.GenericApproxNoop"] += 1
                    continue

                approx_on_play_all_players_action = re.match(
                    r'^When this card is placed on (?:the )?stage from your hand, all players perform the following action\.\s*"[^"]+"\.?$',
                    remainder,
                    re.I,
                )
                if approx_on_play_all_players_action:
                    ability_defs.append(
                        _ability_def(
                            "Auto",
                            "OnPlay",
                            effects=[_template("Draw", count=0)],
                            targets=[],
                            cost=cost,
                            conditions=with_approx_condition(),
                        )
                    )
                    stats.parsed_lines += 1
                    stats.emitted_defs["Auto.OnPlayAllPlayersAction.ApproxNoop"] += 1
                    continue

                approx_on_play_memory_stage_reset = re.match(
                    rf"^When this card is placed on (?:the )?stage from your hand, choose up to ({COUNT_TOKEN_RE}) of your opponent's characters, put it into their memory, and your opponent puts that character from their memory (?:on|in) any position of their stage\.?$",
                    remainder,
                    re.I,
                )
                if approx_on_play_memory_stage_reset:
                    ability_defs.append(
                        _ability_def(
                            "Auto",
                            "OnPlay",
                            effects=[_template("Draw", count=0)],
                            targets=[],
                            cost=cost,
                            conditions=with_approx_condition(),
                        )
                    )
                    stats.parsed_lines += 1
                    stats.emitted_defs["Auto.OnPlay.MemoryStageReset.ApproxNoop"] += 1
                    continue

                approx_frontal_attacked_look_top = re.match(
                    r"^When this card is frontal attacked, look at the top card of your deck, and put it on the top of your deck or into your waiting room\.?$",
                    remainder,
                    re.I,
                )
                if approx_frontal_attacked_look_top:
                    ability_defs.append(
                        _ability_def(
                            "Auto",
                            None,
                            effects=[_template("Draw", count=0)],
                            targets=[],
                            cost=cost,
                            conditions=with_approx_condition(),
                        )
                    )
                    stats.parsed_lines += 1
                    stats.emitted_defs["Auto.FrontalAttacked.LookTopDecision.ApproxNoop"] += 1
                    continue

            damage_not_canceled = re.match(
                r"^(?:During this card's battle, )?when damage dealt by this card is not canceled, this card gets \+(\d+) power until end of turn\.?$",
                remainder,
                re.I,
            )
            if damage_not_canceled:
                if not cost_is_empty(cost):
                    mark_unsupported(line)
                    continue
                ability_defs.append(
                    _ability_def(
                        "Auto",
                        "DamageDealtNotCanceled",
                        effects=[
                            _template(
                                "AddPower",
                                amount=int(damage_not_canceled.group(1)),
                                duration_turn=True,
                            )
                        ],
                        targets=["This"],
                    )
                )
                stats.parsed_lines += 1
                stats.emitted_defs["Auto.AddPower.OnDamageDealtNotCanceled"] += 1
                continue

            damage_received_not_canceled = re.match(
                r"^During this card's battle, when the damage you received is not canceled, this card gets \+(\d+) power until end of turn\.?$",
                remainder,
                re.I,
            )
            if damage_received_not_canceled:
                if not cost_is_empty(cost):
                    mark_unsupported(line)
                    continue
                ability_defs.append(
                    _ability_def(
                        "Auto",
                        "DamageReceivedNotCanceled",
                        effects=[
                            _template(
                                "AddPower",
                                amount=int(damage_received_not_canceled.group(1)),
                                duration_turn=True,
                            )
                        ],
                        targets=["This"],
                    )
                )
                stats.parsed_lines += 1
                stats.emitted_defs["Auto.AddPower.OnDamageReceivedNotCanceled"] += 1
                continue

            damage_cancel_dealt = re.match(
                r"^(?:During this card's battle, )?when damage dealt by this card is canceled, you may (return this card to your hand|put this card into your stock)\.?$",
                remainder,
                re.I,
            )
            if damage_cancel_dealt:
                if not cost_is_empty(cost):
                    mark_unsupported(line)
                    continue
                move_to_stock = "stock" in damage_cancel_dealt.group(1).lower()
                ability_defs.append(
                    _ability_def(
                        "Auto",
                        "DamageDealtCanceled",
                        effects=[_template("MoveToStock" if move_to_stock else "MoveToHand")],
                        targets=["This"],
                        effect_optional=[True],
                    )
                )
                stats.parsed_lines += 1
                if move_to_stock:
                    stats.emitted_defs["Auto.MoveSelfToStock.OnDamageDealtCanceled"] += 1
                else:
                    stats.emitted_defs["Auto.MoveSelfToHand.OnDamageDealtCanceled"] += 1
                continue

            damage_cancel_received = re.match(
                r"^During this card's battle, when the damage you received is canceled, you may (return this card to your hand|put this card into your stock)\.?$",
                remainder,
                re.I,
            )
            if damage_cancel_received:
                if not cost_is_empty(cost):
                    mark_unsupported(line)
                    continue
                move_to_stock = "stock" in damage_cancel_received.group(1).lower()
                ability_defs.append(
                    _ability_def(
                        "Auto",
                        "DamageReceivedCanceled",
                        effects=[_template("MoveToStock" if move_to_stock else "MoveToHand")],
                        targets=["This"],
                        effect_optional=[True],
                    )
                )
                stats.parsed_lines += 1
                if move_to_stock:
                    stats.emitted_defs["Auto.MoveSelfToStock.OnDamageReceivedCanceled"] += 1
                else:
                    stats.emitted_defs["Auto.MoveSelfToHand.OnDamageReceivedCanceled"] += 1
                continue

            if re.match(r"^Bond\s*/", remainder, re.I):
                lower = remainder.lower()
                if (
                    "when this card is played and placed" in lower
                    and "you may pay the cost" in lower
                    and "in your waiting room" in lower
                    and "return" in lower
                    and "to your hand" in lower
                ):
                    count = 1
                    choose_match = re.search(
                        rf"choose (?:up to )?({COUNT_TOKEN_RE})\s+",
                        remainder,
                        re.I,
                    )
                    if choose_match:
                        parsed_count = parse_count_token(choose_match.group(1))
                        if parsed_count is not None:
                            count = parsed_count
                    target_ids: List[int] = []
                    if name_to_ids:
                        seen_ids = set()
                        for quoted in re.findall(r'"([^"]+)"', remainder):
                            name = quoted.strip()
                            if not name:
                                continue
                            for card_id in name_to_ids.get(name, []):
                                if card_id in seen_ids:
                                    continue
                                seen_ids.add(card_id)
                                target_ids.append(card_id)
                    if not target_ids:
                        # Keep unresolved Bond targets as a strict unsupported signal.
                        mark_unsupported(line, allow_parser_v2=False)
                        continue
                    abilities.append(
                        _template("Bond", cost=cost, count=count, target_ids=target_ids)
                    )
                    stats.parsed_lines += 1
                    stats.emitted_templates["Bond"] += 1
                    continue

            if (
                remainder.lower().startswith("encore")
                and "when this card is put into your waiting room from the stage"
                in remainder.lower()
                and re.search(
                    r"return this card to its previous stage position as\s*【rest】",
                    remainder,
                    re.I,
                )
            ):
                if cost_is_empty(cost):
                    mark_unsupported(line)
                else:
                    abilities.append(_template("EncoreVariant", cost=cost))
                    stats.parsed_lines += 1
                    stats.emitted_templates["EncoreVariant"] += 1
                continue

            paid_attack_trigger = re.match(
                rf"^When this card attacks, you may pay the cost\. If you do, during that attack, perform a trigger check ({COUNT_TOKEN_RE}) times? on the trigger step\.?$",
                remainder,
                re.I,
            )
            if paid_attack_trigger:
                trigger_count = parse_count_token(paid_attack_trigger.group(1))
                if trigger_count is None:
                    mark_unsupported(line)
                    continue
                if cost_is_empty(cost):
                    mark_unsupported(line)
                    continue
                ability_defs.append(
                    _ability_def(
                        "Auto",
                        "AttackDeclaration",
                        effects=[_template("SetTriggerCheckCount", count=trigger_count)],
                        targets=[],
                        cost=cost,
                    )
                )
                stats.parsed_lines += 1
                stats.emitted_defs["Auto.SetTriggerChecks.AttackDeclaration"] += 1
                continue

            encore_step_heal_on_play_turn = re.match(
                r"^During the turn that this card is placed on the stage from your hand, at the beginning of the encore step, you may put the top card of your clock into your waiting room\.?$",
                remainder,
                re.I,
            )
            if encore_step_heal_on_play_turn:
                if not cost_is_empty(cost):
                    mark_unsupported(line)
                    continue
                ability_defs.append(
                    _ability_def(
                        "Auto",
                        "BeginEncoreStep",
                        effects=[_template("HealIfSourcePlayedFromHandThisTurn")],
                        targets=[],
                        effect_optional=[True],
                    )
                )
                stats.parsed_lines += 1
                stats.emitted_defs["Auto.Heal.IfPlayedFromHandThisTurn.BeginEncoreStep"] += 1
                continue

            paid_encore_rest = re.match(
                r"^At the beginning of the encore step, if you do not have another 【REST】 character (?:in|on) your center stage, you may pay the cost\. If you do, 【REST】 this card\.?$",
                remainder,
                re.I,
            )
            if paid_encore_rest:
                if cost_is_empty(cost):
                    mark_unsupported(line)
                    continue
                ability_defs.append(
                    _ability_def(
                        "Auto",
                        "BeginEncoreStep",
                        effects=[_template("RestThisIfNoOtherRestCenter")],
                        targets=["This"],
                        cost=cost,
                    )
                )
                stats.parsed_lines += 1
                stats.emitted_defs["Auto.RestSelf.IfNoOtherRestCenter.BeginEncoreStep"] += 1
                continue

            paid_on_play = re.match(
                r'^When this card is placed on (?:the )?stage from your hand(?: or by the 【AUTO】 effect of "[^"]+"| or by [^,]+)?, you may pay the cost\. If you do, (.+)$',
                remainder,
                re.I,
            )
            if paid_on_play:
                if cost_is_empty(cost):
                    mark_unsupported(line)
                    continue
                paid_effect = paid_on_play.group(1).strip()
                paid_effect, cxcombo_conditions = strip_cxcombo_condition_prefix(
                    paid_effect, has_cxcombo_tag
                )

                match = re.match(
                    r"^put the top card of your clock into your stock\.?$",
                    paid_effect,
                    re.I,
                )
                if match:
                    ability_defs.append(
                        _ability_def(
                            "Auto",
                            "OnPlay",
                            effects=[_template("MoveToStock")],
                            targets=["SelfClock"],
                            cost=cost,
                            conditions=cxcombo_conditions,
                            target_limit=1,
                        )
                    )
                    stats.parsed_lines += 1
                    stats.emitted_defs["Auto.MoveClockTopToStock.OnPlay.Paid"] += 1
                    continue

                stock_reset = re.match(
                    r"^your opponent puts all of their stock into their waiting room, and puts the same number of cards from the top of their deck into their stock\.?$",
                    paid_effect,
                    re.I,
                )
                if stock_reset:
                    ability_defs.append(
                        _ability_def(
                            "Auto",
                            "OnPlay",
                            effects=[_template("ResetStockFromDeckTop", target="Opponent")],
                            targets=[],
                            cost=cost,
                            conditions=cxcombo_conditions,
                        )
                    )
                    stats.parsed_lines += 1
                    stats.emitted_defs["Auto.ResetOpponentStockFromDeckTop.OnPlay.Paid"] += 1
                    continue

                match = re.match(
                    rf"^choose (up to )?({COUNT_TOKEN_RE}) (.+?) in your waiting room, and return (?:it|them) to your hand\.?$",
                    paid_effect,
                    re.I,
                )
                if match:
                    optional = bool(match.group(1))
                    count = parse_count_token(match.group(2))
                    if count is None:
                        mark_unsupported(line)
                        continue
                    selector = match.group(3).strip()
                    selector_lower = selector.lower()
                    card_type_hint = None
                    if "character" in selector_lower:
                        card_type_hint = "Character"
                    elif "climax" in selector_lower:
                        card_type_hint = "Climax"
                    elif "event" in selector_lower:
                        card_type_hint = "Event"
                    ability_defs.append(
                        _ability_def(
                            "Auto",
                            "OnPlay",
                            effects=[_template("MoveToHand")],
                            targets=["SelfWaitingRoom"],
                            cost=cost,
                            conditions=cxcombo_conditions,
                            effect_optional=[optional],
                            target_card_type=card_type_hint,
                            target_limit=count,
                        )
                    )
                    stats.parsed_lines += 1
                    stats.emitted_defs["Auto.SalvageWaitingRoom.OnPlay.Paid"] += 1
                    continue

                recycle_waiting_room = re.match(
                    r"^return all cards from your waiting room to your deck, and shuffle your deck\.?$",
                    paid_effect,
                    re.I,
                )
                if recycle_waiting_room:
                    ability_defs.append(
                        _ability_def(
                            "Auto",
                            "OnPlay",
                            effects=[_template("RecycleWaitingRoomToDeckShuffle")],
                            targets=[],
                            cost=cost,
                            conditions=cxcombo_conditions,
                        )
                    )
                    stats.parsed_lines += 1
                    stats.emitted_defs["Auto.RecycleWaitingRoom.OnPlay.Paid"] += 1
                    continue

                reveal_top_salvage_by_level = re.match(
                    rf"^reveal the top card of your deck, choose ({COUNT_TOKEN_RE}) level X or lower character in your waiting room, and return it to your hand\. X is equal to the level of the revealed card\. \(Climax are regarded as level (\d+)\. Return the revealed card to its original place\)\.?$",
                    paid_effect,
                    re.I,
                )
                if reveal_top_salvage_by_level:
                    choose_count = parse_count_token(reveal_top_salvage_by_level.group(1))
                    if choose_count is None:
                        mark_unsupported(line)
                        continue
                    ability_defs.append(
                        _ability_def(
                            "Auto",
                            "OnPlay",
                            effects=[
                                _template(
                                    "RevealTopAndSalvageByRevealedLevel",
                                    count=choose_count,
                                    climax_level=int(reveal_top_salvage_by_level.group(2)),
                                )
                            ],
                            targets=[],
                            cost=cost,
                            conditions=cxcombo_conditions,
                        )
                    )
                    stats.parsed_lines += 1
                    stats.emitted_defs["Auto.RevealTopSalvageByLevel.OnPlay.Paid"] += 1
                    continue

                topdeck_search_paid = parse_on_play_topdeck_search(paid_effect)
                if topdeck_search_paid is not None:
                    if not topdeck_search_paid:
                        mark_unsupported(line)
                        continue
                    ability_defs.append(
                        _ability_def(
                            "Auto",
                            "OnPlay",
                            effects=[_template("MoveToHand")],
                            targets=["SelfDeckTop"],
                            cost=cost,
                            conditions=cxcombo_conditions,
                            effect_optional=[True],
                            target_card_type=topdeck_search_paid["card_type_hint"],
                            target_trait=topdeck_search_paid["trait_id"],
                            target_level_max=topdeck_search_paid["target_level_max"],
                            target_cost_max=topdeck_search_paid["target_cost_max"],
                            target_card_ids=topdeck_search_paid["target_card_ids"],
                            target_limit=topdeck_search_paid["look_count"],
                        )
                    )
                    stats.parsed_lines += 1
                    stats.emitted_defs["Auto.SearchDeckTopToHand.OnPlay.Paid"] += 1
                    continue

                paid_bounce_opp_stage = re.match(
                    rf"^choose (up to )?({COUNT_TOKEN_RE}) of your opponent's characters, and return it to their hand\.?$",
                    paid_effect,
                    re.I,
                )
                if paid_bounce_opp_stage:
                    choose_count = parse_count_token(paid_bounce_opp_stage.group(2))
                    if choose_count is None:
                        mark_unsupported(line)
                        continue
                    ability_defs.append(
                        _ability_def(
                            "Auto",
                            "OnPlay",
                            effects=[_template("MoveToHand")],
                            targets=["OppStage"],
                            cost=cost,
                            conditions=cxcombo_conditions,
                            effect_optional=[bool(paid_bounce_opp_stage.group(1))],
                            target_limit=choose_count,
                        )
                    )
                    stats.parsed_lines += 1
                    stats.emitted_defs["Auto.BounceOpponentStage.OnPlay.Paid"] += 1
                    continue

                paid_search_deck_to_hand_generic = re.match(
                    rf"^search your deck for up to ({COUNT_TOKEN_RE}) (.+?), reveal (?:it|them) to your opponent, (?:and )?put (?:it|them) into your hand(?:, and shuffle your deck(?: afterwards)?|\. shuffle your deck(?: afterwards)?|\. shuffle your deck|, shuffle your deck)\.?$",
                    paid_effect,
                    re.I,
                )
                if paid_search_deck_to_hand_generic:
                    choose_count = parse_count_token(paid_search_deck_to_hand_generic.group(1))
                    if choose_count is None:
                        mark_unsupported(line)
                        continue
                    selector_constraints = parse_generic_selector_constraints(
                        paid_search_deck_to_hand_generic.group(2)
                    )
                    if selector_constraints is not None:
                        ability_defs.append(
                            _ability_def(
                                "Auto",
                                "OnPlay",
                                effects=[_template("MoveToHand")],
                                targets=["SelfDeckTop"],
                                cost=cost,
                                conditions=cxcombo_conditions,
                                effect_optional=[True],
                                target_card_type=selector_constraints["target_card_type"],
                                target_trait=selector_constraints["target_trait"],
                                target_level_max=selector_constraints["target_level_max"],
                                target_cost_max=selector_constraints["target_cost_max"],
                                target_card_ids=selector_constraints["target_card_ids"],
                                target_limit=choose_count,
                            )
                        )
                        stats.parsed_lines += 1
                        stats.emitted_defs["Auto.SearchDeckToHand.OnPlay.Paid.Generic"] += 1
                        continue

                paid_search_deck_level = re.match(
                    rf"^search your deck for up to ({COUNT_TOKEN_RE}) level (\d+) or lower character, reveal (?:it|them) to your opponent, put (?:it|them) into your hand, and shuffle your deck(?: afterwards)?\.?$",
                    paid_effect,
                    re.I,
                )
                paid_search_deck_trait = re.match(
                    rf"^search your deck for up to ({COUNT_TOKEN_RE}) 《([^》]+)》 character, reveal (?:it|them) to your opponent, put (?:it|them) into your hand, and shuffle your deck(?: afterwards)?\.?$",
                    paid_effect,
                    re.I,
                )
                if paid_search_deck_level or paid_search_deck_trait:
                    match = paid_search_deck_level or paid_search_deck_trait
                    assert match is not None
                    choose_count = parse_count_token(match.group(1))
                    if choose_count is None or choose_count != 1:
                        mark_unsupported(line)
                        continue
                    target_level_max = None
                    target_trait = None
                    if paid_search_deck_level:
                        target_level_max = int(paid_search_deck_level.group(2))
                    else:
                        trait_name = paid_search_deck_trait.group(2).strip()
                        target_trait = (trait_map or {}).get(trait_name)
                        if target_trait is None:
                            mark_unsupported(line)
                            continue
                    ability_defs.append(
                        _ability_def(
                            "Auto",
                            "OnPlay",
                            effects=[_template("MoveToHand")],
                            targets=["SelfDeckTop"],
                            cost=cost,
                            conditions=cxcombo_conditions,
                            effect_optional=[True],
                            target_card_type="Character",
                            target_trait=target_trait,
                            target_level_max=target_level_max,
                        )
                    )
                    stats.parsed_lines += 1
                    stats.emitted_defs["Auto.SearchDeckToHand.OnPlay.Paid"] += 1
                    continue

                paid_salvage_then_buff_trait = re.match(
                    rf"^choose ({COUNT_TOKEN_RE}) 《([^》]+)》 character in your waiting room, return (?:it|them) to your hand, choose ({COUNT_TOKEN_RE}) of your other 《([^》]+)》 characters, and that character gets \+([+-]?\d+) power until end of turn\.?$",
                    paid_effect,
                    re.I,
                )
                if paid_salvage_then_buff_trait and allow_approx_effects:
                    ability_defs.append(
                        _ability_def(
                            "Auto",
                            "OnPlay",
                            effects=[_template("Draw", count=0)],
                            targets=[],
                            cost=cost,
                            conditions=with_approx_condition(cxcombo_conditions),
                            effect_optional=[True],
                        )
                    )
                    stats.parsed_lines += 1
                    stats.emitted_defs["Auto.SalvageThenBuffTrait.OnPlay.Paid.Approx"] += 1
                    continue

                paid_search_named_to_hand = re.match(
                    rf'^search your deck for up to ({COUNT_TOKEN_RE}) "[^"]+", reveal (?:it|them) to your opponent, put (?:it|them) into your hand, and shuffle your deck(?: afterwards)?\.?$',
                    paid_effect,
                    re.I,
                )
                if paid_search_named_to_hand:
                    choose_count = parse_count_token(paid_search_named_to_hand.group(1))
                    if choose_count is None:
                        mark_unsupported(line)
                        continue
                    target_ids = resolve_exact_quoted_name_ids(paid_effect)
                    if target_ids:
                        ability_defs.append(
                            _ability_def(
                                "Auto",
                                "OnPlay",
                                effects=[_template("MoveToHand")],
                                targets=["SelfDeckTop"],
                                cost=cost,
                                conditions=cxcombo_conditions,
                                effect_optional=[True],
                                target_card_ids=target_ids,
                                target_limit=choose_count,
                            )
                        )
                        stats.parsed_lines += 1
                        stats.emitted_defs["Auto.SearchDeckNamedToHand.OnPlay.Paid"] += 1
                        continue
                    if allow_approx_effects:
                        ability_defs.append(
                            _ability_def(
                                "Auto",
                                "OnPlay",
                                effects=[_template("Draw", count=choose_count)],
                                targets=[],
                                cost=cost,
                                conditions=with_approx_condition(cxcombo_conditions),
                                effect_optional=[True],
                            )
                        )
                        stats.parsed_lines += 1
                        stats.emitted_defs["Auto.SearchDeckNamedToHand.OnPlay.Paid.ApproxDraw"] += 1
                        continue
                    mark_unsupported(line)
                    continue

                if allow_approx_effects:
                    paid_mill_if_climax_then_salvage = re.match(
                        rf"^put the top ({COUNT_TOKEN_RE}) cards? of your deck into your waiting room\. If there is a climax among those cards, you may pay the cost\. If you do, choose ({COUNT_TOKEN_RE}) character in your waiting room, and return (?:it|them) to your hand\.?$",
                        paid_effect,
                        re.I,
                    )
                    if paid_mill_if_climax_then_salvage:
                        mill_count = parse_count_token(paid_mill_if_climax_then_salvage.group(1))
                        salvage_count = parse_count_token(paid_mill_if_climax_then_salvage.group(2))
                        if mill_count is None or salvage_count is None:
                            mark_unsupported(line)
                            continue
                        ability_defs.append(
                            _ability_def(
                                "Auto",
                                "OnPlay",
                                effects=[_template("MoveToWaitingRoom"), _template("MoveToHand")],
                                targets=["SelfDeckTop", "SelfWaitingRoom"],
                                cost=cost,
                                conditions=with_approx_condition(cxcombo_conditions),
                                effect_optional=[False, True],
                                target_limit=mill_count,
                            )
                        )
                        stats.parsed_lines += 1
                        stats.emitted_defs["Auto.MillIfClimaxThenSalvage.OnPlay.Paid.Approx"] += 1
                        continue

                    paid_search_named_to_stage = re.match(
                        rf'^search your deck for up to ({COUNT_TOKEN_RE}) "[^"]+", put it (?:on|in) any position of your stage, and shuffle your deck\.?$',
                        paid_effect,
                        re.I,
                    )
                    if paid_search_named_to_stage:
                        ability_defs.append(
                            _ability_def(
                                "Auto",
                                "OnPlay",
                                effects=[_template("Draw", count=0)],
                                targets=[],
                                cost=cost,
                                conditions=with_approx_condition(cxcombo_conditions),
                                effect_optional=[True],
                            )
                        )
                        stats.parsed_lines += 1
                        stats.emitted_defs[
                            "Auto.SearchDeckNamedToStage.OnPlay.Paid.ApproxNoop"
                        ] += 1
                        continue

                    paid_waiting_room_to_stage = re.match(
                        rf"^choose (up to )?({COUNT_TOKEN_RE}) level (\d+) or lower character in your waiting room, and put it (?:on|in) any position of your stage\.?$",
                        paid_effect,
                        re.I,
                    )
                    if paid_waiting_room_to_stage:
                        ability_defs.append(
                            _ability_def(
                                "Auto",
                                "OnPlay",
                                effects=[_template("Draw", count=0)],
                                targets=[],
                                cost=cost,
                                conditions=with_approx_condition(cxcombo_conditions),
                                effect_optional=[True],
                            )
                        )
                        stats.parsed_lines += 1
                        stats.emitted_defs["Auto.WaitingRoomToAnyStage.OnPlay.Paid.ApproxNoop"] += 1
                        continue

                    paid_look_event_then_discard = re.match(
                        rf"^look at up to ({COUNT_TOKEN_RE}) cards? from the top of your deck, choose up to ({COUNT_TOKEN_RE}) event from among them, reveal it to your opponent, put it into your hand, and put the rest into your waiting room\. If you put ({COUNT_TOKEN_RE}) card into your hand, choose ({COUNT_TOKEN_RE}) card in your hand, and put it into your waiting room\.?$",
                        paid_effect,
                        re.I,
                    )
                    if paid_look_event_then_discard:
                        ability_defs.append(
                            _ability_def(
                                "Auto",
                                "OnPlay",
                                effects=[_template("Draw", count=0)],
                                targets=[],
                                cost=cost,
                                conditions=with_approx_condition(cxcombo_conditions),
                                effect_optional=[True],
                            )
                        )
                        stats.parsed_lines += 1
                        stats.emitted_defs["Auto.LookEventThenDiscard.OnPlay.Paid.ApproxNoop"] += 1
                        continue

                if allow_approx_effects and looks_like_search_or_salvage_text(paid_effect):
                    optional = bool(re.search(r"\byou may\b|\bup to\b", paid_effect, re.I))
                    ability_defs.append(
                        _ability_def(
                            "Auto",
                            "OnPlay",
                            effects=[_template("Draw", count=0)],
                            targets=[],
                            cost=cost,
                            conditions=with_approx_condition(cxcombo_conditions),
                            effect_optional=[optional] if optional else [],
                        )
                    )
                    stats.parsed_lines += 1
                    stats.emitted_defs["Auto.SearchSalvage.Generic.OnPlay.Paid.ApproxNoop"] += 1
                    continue

            paid_on_leave_stage_topdeck = re.match(
                rf"^When this card is put into your waiting room from the stage, you may pay the cost\. If you do, look at up to ({COUNT_TOKEN_RE}) cards? from the top of your deck, choose up to ({COUNT_TOKEN_RE}) level (\d+) or higher card from among them, reveal it to your opponent, put it into your hand, and put the rest into your waiting room\. \(Climax are regarded as level (\d+)\)\.?$",
                remainder,
                re.I,
            )
            if paid_on_leave_stage_topdeck:
                look_count = parse_count_token(paid_on_leave_stage_topdeck.group(1))
                choose_count = parse_count_token(paid_on_leave_stage_topdeck.group(2))
                min_level = int(paid_on_leave_stage_topdeck.group(3))
                climax_level = int(paid_on_leave_stage_topdeck.group(4))
                if (
                    look_count is None
                    or choose_count is None
                    or choose_count == 0
                    or climax_level != 0
                ):
                    mark_unsupported(line)
                    continue
                ability_defs.append(
                    _ability_def(
                        "Auto",
                        "OnReverse",
                        effects=[
                            _template(
                                "SearchTopDeckToHandLevelAtLeastMillRest",
                                look_count=look_count,
                                choose_count=choose_count,
                                min_level=min_level,
                            )
                        ],
                        targets=[],
                        cost=cost,
                    )
                )
                stats.parsed_lines += 1
                stats.emitted_defs["Auto.SearchTopDeckLevelAtLeast.OnStageToWaitingRoom.Paid"] += 1
                continue

            paid_on_leave_stage_named_salvage = re.match(
                rf'^When this card is put into your waiting room from the stage, you may pay the cost\. If you do, choose ({COUNT_TOKEN_RE}) "[^"]+" in your waiting room, and return (?:it|them) to your hand\.?$',
                remainder,
                re.I,
            )
            if paid_on_leave_stage_named_salvage:
                choose_count = parse_count_token(paid_on_leave_stage_named_salvage.group(1))
                target_ids = resolve_exact_quoted_name_ids(remainder)
                if choose_count is not None and target_ids:
                    ability_defs.append(
                        _ability_def(
                            "Auto",
                            "OnReverse",
                            effects=[_template("MoveToHand")],
                            targets=["SelfWaitingRoom"],
                            cost=cost,
                            effect_optional=[True],
                            target_card_ids=target_ids,
                            target_limit=choose_count,
                        )
                    )
                    stats.parsed_lines += 1
                    stats.emitted_defs["Auto.NamedSalvage.OnStageToWaitingRoom.Paid"] += 1
                    continue
                if allow_approx_effects:
                    ability_defs.append(
                        _ability_def(
                            "Auto",
                            "OnReverse",
                            effects=[_template("Draw", count=0)],
                            targets=[],
                            cost=cost,
                            conditions=with_approx_condition(),
                            effect_optional=[True],
                        )
                    )
                    stats.parsed_lines += 1
                    stats.emitted_defs[
                        "Auto.NamedSalvage.OnStageToWaitingRoom.Paid.ApproxNoop"
                    ] += 1
                    continue
                mark_unsupported(line)
                continue

            paid_on_attack = re.match(
                r"^When this card attacks, (.+)$",
                remainder,
                re.I,
            )
            if paid_on_attack:
                paid_effect = paid_on_attack.group(1).strip()
                paid_effect, cxcombo_conditions = strip_cxcombo_condition_prefix(
                    paid_effect, has_cxcombo_tag
                )
                paid_dmg = re.match(
                    rf"^you may pay the cost\. If you do, deal ({COUNT_TOKEN_RE}) damage to your opponent\.?(?:\s*\([^)]*\))?$",
                    paid_effect,
                    re.I,
                )
                if paid_dmg:
                    amount = parse_count_token(paid_dmg.group(1))
                    if amount is None or cost_is_empty(cost):
                        mark_unsupported(line)
                        continue
                    cancelable = "cannot be canceled" not in paid_effect.lower()
                    ability_defs.append(
                        _ability_def(
                            "Auto",
                            "AttackDeclaration",
                            effects=[
                                _template(
                                    "DealDamage",
                                    amount=amount,
                                    cancelable=cancelable,
                                )
                            ],
                            targets=[],
                            cost=cost,
                            conditions=cxcombo_conditions,
                        )
                    )
                    stats.parsed_lines += 1
                    stats.emitted_defs["Auto.DealDamage.AttackDeclaration.Paid"] += 1
                    continue

            paid_begin_opp_attack_back_move = re.match(
                r"^At the beginning of your opponent's attack phase, you may pay the cost\. If you do, move this card to an open position of your back stage\.?$",
                remainder,
                re.I,
            )
            if paid_begin_opp_attack_back_move:
                if cost_is_empty(cost):
                    mark_unsupported(line)
                    continue
                ability_defs.append(
                    _ability_def(
                        "Auto",
                        "BeginAttackPhase",
                        effects=[_template("MoveThisToOpenBack")],
                        targets=[],
                        cost=cost,
                    )
                )
                stats.parsed_lines += 1
                stats.emitted_defs["Auto.MoveThisToOpenBack.BeginAttackPhase.Paid"] += 1
                continue

            if allow_approx_effects:
                paid_on_play_mill_if_climax_then_salvage = re.match(
                    rf"^When this card is placed on (?:the )?stage from your hand, put the top ({COUNT_TOKEN_RE}) cards? of your deck into your waiting room\. If there is a climax among those cards, you may pay the cost\. If you do, choose ({COUNT_TOKEN_RE}) character in your waiting room, and return (?:it|them) to your hand\.?$",
                    remainder,
                    re.I,
                )
                if paid_on_play_mill_if_climax_then_salvage:
                    mill_count = parse_count_token(
                        paid_on_play_mill_if_climax_then_salvage.group(1)
                    )
                    if mill_count is None:
                        mark_unsupported(line)
                        continue
                    ability_defs.append(
                        _ability_def(
                            "Auto",
                            "OnPlay",
                            effects=[_template("MoveToWaitingRoom")],
                            targets=["SelfDeckTop"],
                            cost=cost,
                            conditions=with_approx_condition(),
                            target_limit=mill_count,
                        )
                    )
                    stats.parsed_lines += 1
                    stats.emitted_defs[
                        "Auto.MillIfClimaxThenSalvage.OnPlay.Costed.PartialApprox"
                    ] += 1
                    continue

            paid_on_climax_buff = re.match(
                rf"^When (?:a|your) climax is placed on your climax area, you may pay the cost\. If you do, choose ({COUNT_TOKEN_RE}) of your characters, and that character gets \+([+-]?\d+) power until end of turn\.?$",
                remainder,
                re.I,
            )
            if paid_on_climax_buff:
                choose_count = parse_count_token(paid_on_climax_buff.group(1))
                if choose_count is None:
                    mark_unsupported(line)
                    continue
                ability_defs.append(
                    _ability_def(
                        "Auto",
                        "AfterClimaxPhase",
                        effects=[
                            _template(
                                "AddPower",
                                amount=int(paid_on_climax_buff.group(2)),
                                duration_turn=True,
                            )
                        ],
                        targets=["SelfStage"],
                        cost=cost,
                        conditions={"climax_area": {"side": "SelfSide", "card_ids": []}},
                        effect_optional=[True],
                        target_limit=choose_count,
                    )
                )
                stats.parsed_lines += 1
                stats.emitted_defs["Auto.AddPower.OnClimaxPlaced.Paid"] += 1
                continue

            paid_on_climax_soul = re.match(
                rf"^When (?:a|your) climax is placed on your climax area, you may pay the cost\. If you do, choose ({COUNT_TOKEN_RE}) of your characters, and that character gets \+([+-]?\d+) soul until end of turn\.?$",
                remainder,
                re.I,
            )
            if paid_on_climax_soul:
                choose_count = parse_count_token(paid_on_climax_soul.group(1))
                if choose_count is None:
                    mark_unsupported(line)
                    continue
                ability_defs.append(
                    _ability_def(
                        "Auto",
                        "AfterClimaxPhase",
                        effects=[
                            _template(
                                "AddSoul",
                                amount=int(paid_on_climax_soul.group(2)),
                                duration_turn=True,
                            )
                        ],
                        targets=["SelfStage"],
                        cost=cost,
                        conditions={"climax_area": {"side": "SelfSide", "card_ids": []}},
                        effect_optional=[True],
                        target_limit=choose_count,
                    )
                )
                stats.parsed_lines += 1
                stats.emitted_defs["Auto.AddSoul.OnClimaxPlaced.Paid"] += 1
                continue

            paid_attack_power_following_exact = re.match(
                r'^When this card attacks, you may pay the cost\. If you do, this card gets \+([+-]?\d+) power and the following ability until end of turn\.\s*"(.+)"(?:\s*\(Damage may be canceled\))?\.?$',
                remainder,
                re.I,
            )
            if paid_attack_power_following_exact:
                nested_effect = parse_following_effect_or_grant(
                    paid_attack_power_following_exact.group(2),
                    duration_turn=True,
                    grant_duration="UntilEndOfTurn",
                )
                if nested_effect is not None:
                    ability_defs.append(
                        _ability_def(
                            "Auto",
                            "AttackDeclaration",
                            effects=[
                                _template(
                                    "AddPower",
                                    amount=int(paid_attack_power_following_exact.group(1)),
                                    duration_turn=True,
                                ),
                                nested_effect,
                            ],
                            targets=["This", "This"],
                            cost=cost,
                            effect_optional=[True],
                        )
                    )
                    stats.parsed_lines += 1
                    stats.emitted_defs["Auto.AttackPowerFollowingAbilityFlattened.Paid"] += 1
                    continue

            if allow_approx_effects:
                paid_attack_power_following = re.match(
                    r"^When this card attacks, you may pay the cost\. If you do, this card gets \+([+-]?\d+) power and the following ability until end of turn\.\s*\"[^\"]+\"\s*\(Damage may be canceled\)\.?$",
                    remainder,
                    re.I,
                )
                if paid_attack_power_following:
                    ability_defs.append(
                        _ability_def(
                            "Auto",
                            "AttackDeclaration",
                            effects=[_template("Draw", count=0)],
                            targets=[],
                            cost=cost,
                            conditions=with_approx_condition(),
                            effect_optional=[True],
                        )
                    )
                    stats.parsed_lines += 1
                    stats.emitted_defs["Auto.AttackPowerFollowingAbility.Paid.ApproxNoop"] += 1
                    continue

                paid_frontal_attack_reduce_level = re.match(
                    rf"^When this card frontal attacks, you may pay the cost\. If you do, choose ({COUNT_TOKEN_RE}) of your opponent's characters, and that character gets -({COUNT_TOKEN_RE}) level until end of turn\.?$",
                    remainder,
                    re.I,
                )
                if paid_frontal_attack_reduce_level:
                    ability_defs.append(
                        _ability_def(
                            "Auto",
                            "AttackDeclaration",
                            effects=[_template("Draw", count=0)],
                            targets=[],
                            cost=cost,
                            conditions=with_approx_condition(),
                            effect_optional=[True],
                        )
                    )
                    stats.parsed_lines += 1
                    stats.emitted_defs["Auto.PaidFrontalAttackReduceLevel.ApproxNoop"] += 1
                    continue

                paid_on_play_turn_reverse_clock = re.match(
                    r"^During the turn that this card is placed on the stage from your hand, when this card's battle opponent becomes 【REVERSE】, you may pay the cost\. If you do, put that character into your opponent's clock\.?$",
                    remainder,
                    re.I,
                )
                if paid_on_play_turn_reverse_clock:
                    ability_defs.append(
                        _ability_def(
                            "Auto",
                            "BattleOpponentReverse",
                            effects=[_template("Draw", count=0)],
                            targets=[],
                            cost=cost,
                            conditions=with_approx_condition(),
                            effect_optional=[True],
                        )
                    )
                    stats.parsed_lines += 1
                    stats.emitted_defs["Auto.ReverseClock.OnPlayTurn.Paid.ApproxNoop"] += 1
                    continue

                paid_replace_prev_slot = re.match(
                    r"^When your other character is put into your waiting room from the stage, if this card is in your back stage, you may pay the cost\. If you do, return that character to its previous stage position as 【REST】\.?$",
                    remainder,
                    re.I,
                )
                if paid_replace_prev_slot:
                    ability_defs.append(
                        _ability_def(
                            "Auto",
                            None,
                            effects=[_template("Draw", count=0)],
                            targets=[],
                            cost=cost,
                            conditions=with_approx_condition(),
                            effect_optional=[True],
                        )
                    )
                    stats.parsed_lines += 1
                    stats.emitted_defs["Auto.ReturnToPreviousSlot.ApproxNoop"] += 1
                    continue

                paid_trigger_climax_look_to_hand = re.match(
                    rf"^When your character's trigger check reveals a climax(?: with(?: [^,]+)? in its trigger icon)?, you may pay the cost\. If you do, look at up to ({COUNT_TOKEN_RE}) cards? from the top of your deck, choose up to ({COUNT_TOKEN_RE}) card from among them, put it into your hand, and put the rest into your waiting room\.?$",
                    remainder,
                    re.I,
                )
                if paid_trigger_climax_look_to_hand:
                    ability_defs.append(
                        _ability_def(
                            "Auto",
                            "TriggerResolution",
                            effects=[_template("Draw", count=0)],
                            targets=[],
                            cost=cost,
                            conditions=with_approx_condition(
                                {"trigger_check_revealed_climax": True}
                            ),
                            effect_optional=[True],
                        )
                    )
                    stats.parsed_lines += 1
                    stats.emitted_defs["Auto.TriggerClimaxLookTopToHand.Paid.ApproxNoop"] += 1
                    continue

                paid_attack_power_following_simple = re.match(
                    r"^When this card attacks, you may pay the cost\. If you do, this card gets \+([+-]?\d+) power and the following ability until end of turn\.\s*\"[^\"]+\"\.?$",
                    remainder,
                    re.I,
                )
                if paid_attack_power_following_simple:
                    ability_defs.append(
                        _ability_def(
                            "Auto",
                            "AttackDeclaration",
                            effects=[_template("Draw", count=0)],
                            targets=[],
                            cost=cost,
                            conditions=with_approx_condition(),
                            effect_optional=[True],
                        )
                    )
                    stats.parsed_lines += 1
                    stats.emitted_defs[
                        "Auto.AttackPowerFollowingAbility.Paid.Simple.ApproxNoop"
                    ] += 1
                    continue

                paid_opp_turn_reverse_rest_delayed_wr = re.match(
                    r"^During your opponent's turn, when this card becomes 【REVERSE】 in battle, you may pay the cost\. If you do, 【REST】 this card, and at the beginning of your next encore step, put this card into your waiting room\.?$",
                    remainder,
                    re.I,
                )
                if paid_opp_turn_reverse_rest_delayed_wr:
                    ability_defs.append(
                        _ability_def(
                            "Auto",
                            "OnReverse",
                            effects=[_template("Draw", count=0)],
                            targets=[],
                            cost=cost,
                            conditions=with_approx_condition({"turn": "OpponentTurn"}),
                            effect_optional=[True],
                        )
                    )
                    stats.parsed_lines += 1
                    stats.emitted_defs["Auto.OppTurnReverseRestDelayedWr.Paid.ApproxNoop"] += 1
                    continue

            paid_reverse_burn = re.match(
                rf"^When this card's battle opponent becomes 【REVERSE】, you may pay the cost\. If you do, deal ({COUNT_TOKEN_RE}) damage to your opponent\.?(?:\s*\([^)]*\))?$",
                remainder,
                re.I,
            )
            if paid_reverse_burn:
                amount = parse_count_token(paid_reverse_burn.group(1))
                if amount is None:
                    mark_unsupported(line)
                    continue
                ability_defs.append(
                    _ability_def(
                        "Auto",
                        "BattleOpponentReverse",
                        effects=[
                            _template(
                                "DealDamage",
                                amount=amount,
                                cancelable="cannot be canceled" not in remainder.lower(),
                            )
                        ],
                        targets=[],
                        cost=cost,
                        effect_optional=[True],
                    )
                )
                stats.parsed_lines += 1
                stats.emitted_defs["Auto.DealDamage.BattleOpponentReverse.Paid"] += 1
                continue

            if allow_approx_effects:
                paid_attack_named_climax_search = re.match(
                    r'^When this card attacks, if a card named "[^"]+" is in your climax area, you may pay the cost\. If you do, search your deck for up to one 《[^》]+》 character, reveal it to your opponent, and put it into your hand\. Shuffle your deck.*$',
                    remainder,
                    re.I,
                )
                if paid_attack_named_climax_search:
                    ability_defs.append(
                        _ability_def(
                            "Auto",
                            "AttackDeclaration",
                            effects=[_template("Draw", count=0)],
                            targets=[],
                            cost=cost,
                            conditions=with_approx_condition(),
                            effect_optional=[True],
                        )
                    )
                    stats.parsed_lines += 1
                    stats.emitted_defs["Auto.AttackNamedClimaxSearch.Paid.ApproxNoop"] += 1
                    continue

                paid_on_play_named_wr_to_stage = re.match(
                    rf'^When this card is placed on (?:the )?stage from your hand, you may pay the cost\. If you do, choose ({COUNT_TOKEN_RE}) "[^"]+" in your waiting room, and put it on any position of your stage\.?$',
                    remainder,
                    re.I,
                )
                if paid_on_play_named_wr_to_stage:
                    ability_defs.append(
                        _ability_def(
                            "Auto",
                            "OnPlay",
                            effects=[_template("Draw", count=0)],
                            targets=[],
                            cost=cost,
                            conditions=with_approx_condition(),
                            effect_optional=[True],
                        )
                    )
                    stats.parsed_lines += 1
                    stats.emitted_defs["Auto.OnPlayNamedWrToStage.Paid.ApproxNoop"] += 1
                    continue

                paid_on_play_discard_character_or_move_self = re.match(
                    r"^When this card is placed on the stage from your hand, you may pay the cost\. If you do not, put this card into your waiting room\.?$",
                    remainder,
                    re.I,
                )
                if paid_on_play_discard_character_or_move_self:
                    ability_defs.append(
                        _ability_def(
                            "Auto",
                            "OnPlay",
                            effects=[_template("Draw", count=0)],
                            targets=[],
                            cost=cost,
                            conditions=with_approx_condition(),
                            effect_optional=[True],
                        )
                    )
                    stats.parsed_lines += 1
                    stats.emitted_defs["Auto.OnPlayPayOrMoveSelfToWr.ApproxNoop"] += 1
                    continue

                paid_on_leave_clock_top_search = re.match(
                    rf"^When this card is put into your waiting room from the stage, you may pay the cost\. If you do, look at up to ({COUNT_TOKEN_RE}) cards? from the top of your deck, choose up to ({COUNT_TOKEN_RE}) character from among them, reveal it to your opponent, put it into your hand, and put the rest into your waiting room\.?$",
                    remainder,
                    re.I,
                )
                if paid_on_leave_clock_top_search:
                    ability_defs.append(
                        _ability_def(
                            "Auto",
                            "OnReverse",
                            effects=[_template("Draw", count=0)],
                            targets=[],
                            cost=cost,
                            conditions=with_approx_condition(),
                            effect_optional=[True],
                        )
                    )
                    stats.parsed_lines += 1
                    stats.emitted_defs["Auto.OnLeaveClockTopSearch.Paid.ApproxNoop"] += 1
                    continue

            if not cost_is_empty(cost):
                mark_unsupported(line)
                continue

            begin_opponent_attack_move = re.match(
                r"^At the beginning of your opponent's attack phase, (?:if there is a character facing this card, )?you may move this card to an open position of your center stage(?: with a character facing this card)?\.?$",
                remainder,
                re.I,
            )
            if begin_opponent_attack_move:
                require_facing = bool(
                    re.search(
                        r"with a character facing this card|if there is a character facing this card",
                        remainder,
                        re.I,
                    )
                )
                ability_defs.append(
                    _ability_def(
                        "Auto",
                        "BeginAttackPhase",
                        effects=[
                            _template(
                                "MoveThisToOpenCenter",
                                require_facing=require_facing,
                            )
                        ],
                        targets=[],
                        effect_optional=[True],
                    )
                )
                stats.parsed_lines += 1
                stats.emitted_defs["Auto.MoveThisToOpenCenter.BeginAttackPhase"] += 1
                continue

            begin_opponent_attack_move_middle = re.match(
                r"^At the beginning of your opponent's attack phase, you may move this card to the open middle position of your center stage\.?$",
                remainder,
                re.I,
            )
            if begin_opponent_attack_move_middle:
                ability_defs.append(
                    _ability_def(
                        "Auto",
                        "BeginAttackPhase",
                        effects=[_template("MoveStageSlot", slot=1)],
                        targets=["This"],
                        effect_optional=[True],
                    )
                )
                stats.parsed_lines += 1
                stats.emitted_defs["Auto.MoveThisToMiddleCenter.BeginAttackPhase"] += 1
                continue

            begin_opponent_draw_reveal_gate = re.match(
                r"^At the beginning of your opponent's draw phase, reveal the top card of your deck\. If that card is level (\d+) or higher, you may return this card to your hand\. \((?:Climax are regarded as level 0\. )?Return the revealed card to (?:its original place|the original place)\)\.?$",
                remainder,
                re.I,
            )
            if begin_opponent_draw_reveal_gate:
                ability_defs.append(
                    _ability_def(
                        "Auto",
                        "BeginDrawPhase",
                        effects=[
                            _template(
                                "RevealTopIfLevelAtLeastMoveThisToHand",
                                min_level=int(begin_opponent_draw_reveal_gate.group(1)),
                            )
                        ],
                        targets=[],
                        conditions={"turn": "OpponentTurn"},
                        effect_optional=[True],
                    )
                )
                stats.parsed_lines += 1
                stats.emitted_defs["Auto.RevealTopGate.MoveSelfToHand.BeginOpponentDraw"] += 1
                continue

            climax_area_static_trigger = re.match(
                r"^When (?:a|your) climax is (?:placed|put) on your climax area, this card gets \+(\d+) power until end of turn\.?$",
                remainder,
                re.I,
            )
            if climax_area_static_trigger:
                ability_defs.append(
                    _ability_def(
                        "Auto",
                        "AfterClimaxPhase",
                        effects=[
                            _template(
                                "AddPower",
                                amount=int(climax_area_static_trigger.group(1)),
                                duration_turn=True,
                            )
                        ],
                        targets=["This"],
                        conditions={"climax_area": {"side": "SelfSide", "card_ids": []}},
                    )
                )
                stats.parsed_lines += 1
                stats.emitted_defs["Auto.AddPowerSelf.OnClimaxPlaced"] += 1
                continue

            climax_area_opp_stock_trigger = re.match(
                r"^When (?:a climax is placed on your opponent's climax area|your opponent's climax is placed on their climax area), you may put this card into your stock\.?$",
                remainder,
                re.I,
            )
            if climax_area_opp_stock_trigger:
                ability_defs.append(
                    _ability_def(
                        "Auto",
                        "AfterClimaxPhase",
                        effects=[_template("MoveToStock")],
                        targets=["This"],
                        conditions={"climax_area": {"side": "Opponent", "card_ids": []}},
                        effect_optional=[True],
                    )
                )
                stats.parsed_lines += 1
                stats.emitted_defs["Auto.MoveSelfToStock.OnOpponentClimaxPlaced"] += 1
                continue

            on_climax_play = re.match(
                r"^When this card is placed on your climax area from (?:your )?hand,\s*(.+)$",
                remainder,
                re.I,
            )
            if on_climax_play:
                effect = on_climax_play.group(1).strip()

                match = re.match(
                    r"^perform the(?: \[STANDBY\])? effect\.?$",
                    effect,
                    re.I,
                )
                if match and "[STANDBY]" in line.upper():
                    ability_defs.append(
                        _ability_def(
                            "Auto",
                            "OnPlay",
                            effects=[_template("TriggerIcon", icon="Standby")],
                            targets=[],
                        )
                    )
                    stats.parsed_lines += 1
                    stats.emitted_defs["Auto.PerformStandby.OnClimaxPlay"] += 1
                    continue

                match = re.match(
                    r"^draw a card, choose one of your characters, and that character gets \+(\d+) power and \+(\d+) soul until end of turn\.?$",
                    effect,
                    re.I,
                )
                if match:
                    ability_defs.append(
                        _ability_def(
                            "Auto",
                            "OnPlay",
                            effects=[
                                _template("Draw", count=1),
                                _template(
                                    "AddPower",
                                    amount=int(match.group(1)),
                                    duration_turn=True,
                                ),
                                _template(
                                    "AddSoul",
                                    amount=int(match.group(2)),
                                    duration_turn=True,
                                ),
                            ],
                            targets=["SelfStage", "SelfStage"],
                        )
                    )
                    stats.parsed_lines += 1
                    stats.emitted_defs["Auto.DrawBuffSoul.OnClimaxPlay"] += 1
                    continue

                match = re.match(
                    rf"^choose up to ({COUNT_TOKEN_RE}) of your characters, and those characters get \+([+-]?\d+) power and \+([+-]?\d+) soul until end of turn\.?$",
                    effect,
                    re.I,
                )
                if match:
                    choose_count = parse_count_token(match.group(1))
                    if choose_count is None:
                        mark_unsupported(line)
                        continue
                    ability_defs.append(
                        _ability_def(
                            "Auto",
                            "OnPlay",
                            effects=[
                                _template(
                                    "AddPower",
                                    amount=int(match.group(2)),
                                    duration_turn=True,
                                ),
                                _template(
                                    "AddSoul",
                                    amount=int(match.group(3)),
                                    duration_turn=True,
                                ),
                            ],
                            targets=["SelfStage", "SelfStage"],
                            effect_optional=[True, True],
                            target_limit=choose_count,
                        )
                    )
                    stats.parsed_lines += 1
                    stats.emitted_defs["Auto.TeamPowerSoul.OnClimaxPlay"] += 1
                    continue

                match = re.match(
                    r"^put the top card of your deck into your stock, and all of your characters get \+(\d+) soul until end of turn\.?$",
                    effect,
                    re.I,
                )
                if match:
                    ability_defs.append(
                        _ability_def(
                            "Auto",
                            "OnPlay",
                            effects=[
                                _template("MoveToStock"),
                                _template(
                                    "AddSoul",
                                    amount=int(match.group(1)),
                                    duration_turn=True,
                                ),
                            ],
                            targets=["SelfDeckTop", "SelfStage"],
                            target_limit=1,
                        )
                    )
                    stats.parsed_lines += 1
                    stats.emitted_defs["Auto.DeckTopStockAndTeamSoul.OnClimaxPlay"] += 1
                    continue

                match = re.match(
                    rf"^choose up to ({COUNT_TOKEN_RE}) level (\d+) or lower character in your waiting room, put (?:it|them) into your stock, and all of your characters get \+(\d+) soul until end of turn\.?$",
                    effect,
                    re.I,
                )
                if match:
                    choose_count = parse_count_token(match.group(1))
                    if choose_count is None:
                        mark_unsupported(line)
                        continue
                    ability_defs.append(
                        _ability_def(
                            "Auto",
                            "OnPlay",
                            effects=[
                                _template("MoveToStock"),
                                _template(
                                    "AddSoul",
                                    amount=int(match.group(3)),
                                    duration_turn=True,
                                ),
                            ],
                            targets=["SelfWaitingRoom", "SelfStage"],
                            effect_optional=[True],
                            target_card_type="Character",
                            target_level_max=int(match.group(2)),
                            target_limit=choose_count,
                        )
                    )
                    stats.parsed_lines += 1
                    stats.emitted_defs[
                        "Auto.StockFromWaitingRoomLevelCapAndTeamSoul.OnClimaxPlay"
                    ] += 1
                    continue

                match = re.match(
                    r"^choose up to one (?:[a-z]+\s+)?card in your waiting room, put it into your stock, and all of your characters get \+(\d+) soul until end of turn\.?$",
                    effect,
                    re.I,
                )
                if match:
                    ability_defs.append(
                        _ability_def(
                            "Auto",
                            "OnPlay",
                            effects=[
                                _template("MoveToStock"),
                                _template(
                                    "AddSoul",
                                    amount=int(match.group(1)),
                                    duration_turn=True,
                                ),
                            ],
                            targets=["SelfWaitingRoom", "SelfStage"],
                            effect_optional=[True],
                            target_limit=1,
                        )
                    )
                    stats.parsed_lines += 1
                    stats.emitted_defs["Auto.StockAndTeamSoul.OnClimaxPlay"] += 1
                    continue

                match = re.match(
                    rf"^choose up to ({COUNT_TOKEN_RE}) (?:[a-z]+\s+)?card(?:s)? in your waiting room, put (?:it|them) into your stock, and all of your characters get \+(\d+) soul until end of turn\.?$",
                    effect,
                    re.I,
                )
                if match:
                    choose_count = parse_count_token(match.group(1))
                    if choose_count is None:
                        mark_unsupported(line)
                        continue
                    ability_defs.append(
                        _ability_def(
                            "Auto",
                            "OnPlay",
                            effects=[
                                _template("MoveToStock"),
                                _template(
                                    "AddSoul",
                                    amount=int(match.group(2)),
                                    duration_turn=True,
                                ),
                            ],
                            targets=["SelfWaitingRoom", "SelfStage"],
                            effect_optional=[True],
                            target_limit=choose_count,
                        )
                    )
                    stats.parsed_lines += 1
                    stats.emitted_defs["Auto.StockAndTeamSoul.Counted.OnClimaxPlay"] += 1
                    continue

            on_play = re.match(
                r'^When this card is placed on (?:the )?stage from your hand(?: or by the 【AUTO】 effect of "[^"]+"| or by [^,]+)?,\s*(.+)$',
                remainder,
                re.I,
            )
            if on_play:
                effect = on_play.group(1).strip()
                effect, cxcombo_conditions = strip_cxcombo_condition_prefix(effect, has_cxcombo_tag)

                if allow_approx_effects:
                    on_play_look_event_then_discard = re.match(
                        rf"^look at up to ({COUNT_TOKEN_RE}) cards? from the top of your deck, choose up to ({COUNT_TOKEN_RE}) event from among them, reveal it to your opponent, put it into your hand, and put the rest into your waiting room\. If you put ({COUNT_TOKEN_RE}) card into your hand, choose ({COUNT_TOKEN_RE}) card in your hand, and put it into your waiting room\.?$",
                        effect,
                        re.I,
                    )
                    if on_play_look_event_then_discard:
                        ability_defs.append(
                            _ability_def(
                                "Auto",
                                "OnPlay",
                                effects=[_template("Draw", count=0)],
                                targets=[],
                                conditions=with_approx_condition(cxcombo_conditions),
                                effect_optional=[True],
                            )
                        )
                        stats.parsed_lines += 1
                        stats.emitted_defs["Auto.LookEventThenDiscard.OnPlay.ApproxNoop"] += 1
                        continue

                    on_play_mill_if_climax_then_salvage = re.match(
                        rf"^put the top ({COUNT_TOKEN_RE}) cards? of your deck into your waiting room\. If there is a climax among those cards, you may pay the cost\. If you do, choose ({COUNT_TOKEN_RE}) character in your waiting room, and return (?:it|them) to your hand\.?$",
                        effect,
                        re.I,
                    )
                    if on_play_mill_if_climax_then_salvage:
                        mill_count = parse_count_token(on_play_mill_if_climax_then_salvage.group(1))
                        if mill_count is None:
                            mark_unsupported(line)
                            continue
                        ability_defs.append(
                            _ability_def(
                                "Auto",
                                "OnPlay",
                                effects=[_template("MoveToWaitingRoom")],
                                targets=["SelfDeckTop"],
                                cost=cost,
                                conditions=with_approx_condition(cxcombo_conditions),
                                target_limit=mill_count,
                            )
                        )
                        stats.parsed_lines += 1
                        stats.emitted_defs["Auto.MillIfClimaxThenSalvage.OnPlay.PartialApprox"] += 1
                        continue

                    on_play_reorder_then_bounce = re.match(
                        rf"^look at up to ({COUNT_TOKEN_RE}) cards? from the top of your deck, put them on the top of your deck in any order, choose up to ({COUNT_TOKEN_RE}) of your opponent's characters, and return it to their hand\.?$",
                        effect,
                        re.I,
                    )
                    if on_play_reorder_then_bounce:
                        choose_count = parse_count_token(on_play_reorder_then_bounce.group(2))
                        if choose_count is None:
                            mark_unsupported(line)
                            continue
                        ability_defs.append(
                            _ability_def(
                                "Auto",
                                "OnPlay",
                                effects=[_template("MoveToHand")],
                                targets=["OppStage"],
                                cost=cost,
                                conditions=with_approx_condition(cxcombo_conditions),
                                effect_optional=[True],
                                target_limit=choose_count,
                            )
                        )
                        stats.parsed_lines += 1
                        stats.emitted_defs[
                            "Auto.LookTopReorderThenBounce.OnPlay.PartialApprox"
                        ] += 1
                        continue

                on_play_clock_swap = re.match(
                    rf"^you may choose ({COUNT_TOKEN_RE}) card in your clock, and return it to your hand\. If you do, choose ({COUNT_TOKEN_RE}) card in your hand, and put it into your clock\.?$",
                    effect,
                    re.I,
                )
                if on_play_clock_swap:
                    clock_to_hand = parse_count_token(on_play_clock_swap.group(1))
                    hand_to_clock = parse_count_token(on_play_clock_swap.group(2))
                    if (
                        clock_to_hand is None
                        or hand_to_clock is None
                        or clock_to_hand != 1
                        or hand_to_clock != 1
                    ):
                        mark_unsupported(line)
                        continue
                    ability_defs.append(
                        _ability_def(
                            "Auto",
                            "OnPlay",
                            effects=[_template("MoveToHand"), _template("MoveToClock")],
                            targets=["SelfClock", "SelfHand"],
                            effect_optional=[True, True],
                            target_limit=1,
                            conditions=cxcombo_conditions,
                        )
                    )
                    stats.parsed_lines += 1
                    stats.emitted_defs["Auto.ClockSwap.OnPlay"] += 1
                    continue

                if allow_approx_effects:
                    on_play_level_waiting_exchange = re.match(
                        rf"^you may choose ({COUNT_TOKEN_RE}) card in your level and ({COUNT_TOKEN_RE}) card in your waiting room, and exchange them\.?$",
                        effect,
                        re.I,
                    )
                    if on_play_level_waiting_exchange:
                        ability_defs.append(
                            _ability_def(
                                "Auto",
                                "OnPlay",
                                effects=[_template("Draw", count=0)],
                                targets=[],
                                conditions=with_approx_condition(cxcombo_conditions),
                            )
                        )
                        stats.parsed_lines += 1
                        stats.emitted_defs["Auto.LevelWaitingExchange.OnPlay.ApproxNoop"] += 1
                        continue

                match = re.match(
                    r"^this card gets \+(\d+) power until end of turn\.?$",
                    effect,
                    re.I,
                )
                if match:
                    ability_defs.append(
                        _ability_def(
                            "Auto",
                            "OnPlay",
                            effects=[
                                _template(
                                    "AddPower", amount=int(match.group(1)), duration_turn=True
                                )
                            ],
                            targets=["This"],
                            conditions=cxcombo_conditions,
                        )
                    )
                    stats.parsed_lines += 1
                    stats.emitted_defs["Auto.AddPowerSelf.OnPlay"] += 1
                    continue

                match = re.match(
                    r"^all characters in your opponent's center stage get -(\d+) power until end of turn\.?$",
                    effect,
                    re.I,
                )
                if match:
                    ability_defs.append(
                        _ability_def(
                            "Auto",
                            "OnPlay",
                            effects=[
                                _template(
                                    "AddPower",
                                    amount=-int(match.group(1)),
                                    duration_turn=True,
                                )
                            ],
                            targets=["OppFrontRow"],
                            conditions=cxcombo_conditions,
                        )
                    )
                    stats.parsed_lines += 1
                    stats.emitted_defs["Auto.ReducePower.AllOppCenter.OnPlay"] += 1
                    continue

                match = re.match(
                    rf"^choose ({COUNT_TOKEN_RE}) character in your opponent's center stage, and that character gets -(\d+) power until end of turn\.?$",
                    effect,
                    re.I,
                )
                if match:
                    choose_count = parse_count_token(match.group(1))
                    if choose_count is None:
                        mark_unsupported(line)
                        continue
                    ability_defs.append(
                        _ability_def(
                            "Auto",
                            "OnPlay",
                            effects=[
                                _template(
                                    "AddPower",
                                    amount=-int(match.group(2)),
                                    duration_turn=True,
                                )
                            ],
                            targets=["OppFrontRow"],
                            conditions=cxcombo_conditions,
                            target_limit=choose_count,
                        )
                    )
                    stats.parsed_lines += 1
                    stats.emitted_defs["Auto.ReducePower.OppCenter.OnPlay"] += 1
                    continue

                match = re.match(
                    r"^(you may )?put the top card of your clock into your waiting room\.?$",
                    effect,
                    re.I,
                )
                if match:
                    optional = bool(match.group(1))
                    ability_defs.append(
                        _ability_def(
                            "Auto",
                            "OnPlay",
                            effects=[_template("Heal")],
                            targets=["SelfClock"],
                            conditions=cxcombo_conditions,
                            effect_optional=[optional],
                            target_limit=1,
                        )
                    )
                    stats.parsed_lines += 1
                    stats.emitted_defs["Auto.HealTopClock.OnPlay"] += 1
                    continue

                match = re.match(
                    r"^look at the top card of your deck, and put it on the top of your deck or into your waiting room\.?$",
                    effect,
                    re.I,
                )
                if match:
                    ability_defs.append(
                        _ability_def(
                            "Auto",
                            "OnPlay",
                            effects=[_template("MoveToWaitingRoom")],
                            targets=["SelfDeckTop"],
                            conditions=cxcombo_conditions,
                            effect_optional=[True],
                            target_limit=1,
                        )
                    )
                    stats.parsed_lines += 1
                    stats.emitted_defs["Auto.LookTopDeck.MayMill.OnPlay"] += 1
                    continue

                match = re.match(
                    r"^look at the top card of your deck, and put it on the top or (?:at )?the bottom of your deck\.?$",
                    effect,
                    re.I,
                )
                if match:
                    ability_defs.append(
                        _ability_def(
                            "Auto",
                            "OnPlay",
                            effects=[_template("MoveToDeckBottom")],
                            targets=["SelfDeckTop"],
                            conditions=cxcombo_conditions,
                            effect_optional=[True],
                            target_limit=1,
                        )
                    )
                    stats.parsed_lines += 1
                    stats.emitted_defs["Auto.LookTopDeck.MayBottom.OnPlay"] += 1
                    continue

                match = re.match(
                    rf"^(you may )?put the top ({COUNT_TOKEN_RE}) card(?:s)? of your deck into your waiting room\.?$",
                    effect,
                    re.I,
                )
                if match:
                    count = parse_count_token(match.group(2))
                    if count is None:
                        mark_unsupported(line)
                        continue
                    if match.group(1):
                        ability_defs.append(
                            _ability_def(
                                "Auto",
                                "OnPlay",
                                effects=[_template("MoveToWaitingRoom")],
                                targets=["SelfDeckTop"],
                                conditions=cxcombo_conditions,
                                effect_optional=[True],
                                target_limit=count,
                            )
                        )
                        stats.parsed_lines += 1
                        stats.emitted_defs["Auto.MayMillTop.OnPlay"] += 1
                    else:
                        abilities.append(_template("AutoOnPlayMillTop", count=count))
                        stats.parsed_lines += 1
                        stats.emitted_templates["AutoOnPlayMillTop"] += 1
                    continue

                match = re.match(
                    rf"^look at up to ({COUNT_TOKEN_RE}) cards? from the top of your deck, and put them on the top of your deck in any order\.?$",
                    effect,
                    re.I,
                )
                if match:
                    count = parse_count_token(match.group(1))
                    if count is None:
                        mark_unsupported(line)
                        continue
                    abilities.append(_template("AutoOnPlayRevealDeckTop", count=count))
                    stats.parsed_lines += 1
                    stats.emitted_templates["AutoOnPlayRevealDeckTop"] += 1
                    continue

                topdeck_search = parse_on_play_topdeck_search(effect)
                if topdeck_search is not None:
                    if not topdeck_search:
                        mark_unsupported(line)
                        continue
                    ability_defs.append(
                        _ability_def(
                            "Auto",
                            "OnPlay",
                            effects=[_template("MoveToHand")],
                            targets=["SelfDeckTop"],
                            conditions=cxcombo_conditions,
                            effect_optional=[True],
                            target_card_type=topdeck_search["card_type_hint"],
                            target_trait=topdeck_search["trait_id"],
                            target_level_max=topdeck_search["target_level_max"],
                            target_cost_max=topdeck_search["target_cost_max"],
                            target_card_ids=topdeck_search["target_card_ids"],
                            target_limit=topdeck_search["look_count"],
                        )
                    )
                    stats.parsed_lines += 1
                    stats.emitted_defs["Auto.SearchDeckTopToHand.OnPlay"] += 1
                    continue

                match = re.match(
                    rf"^(you may )?choose (up to )?({COUNT_TOKEN_RE}) card(?:s)? in your hand, and put (?:it|them) into your stock\.?$",
                    effect,
                    re.I,
                )
                if match:
                    count = parse_count_token(match.group(3))
                    if count is None:
                        mark_unsupported(line)
                        continue
                    optional = bool(match.group(1)) or bool(match.group(2))
                    ability_defs.append(
                        _ability_def(
                            "Auto",
                            "OnPlay",
                            effects=[_template("MoveToStock")],
                            targets=["SelfHand"],
                            conditions=cxcombo_conditions,
                            effect_optional=[optional],
                            target_limit=count,
                        )
                    )
                    stats.parsed_lines += 1
                    stats.emitted_defs["Auto.MoveHandToStock.OnPlay"] += 1
                    continue

                match = re.match(
                    rf"^draw up to ({COUNT_TOKEN_RE}) cards?, choose ({COUNT_TOKEN_RE}) card(?:s)? in your hand, and put (?:it|them) into your waiting room\.?$",
                    effect,
                    re.I,
                )
                if match:
                    draw_count = parse_count_token(match.group(1))
                    discard_count = parse_count_token(match.group(2))
                    if draw_count is None or discard_count is None:
                        mark_unsupported(line)
                        continue
                    ability_defs.append(
                        _ability_def(
                            "Auto",
                            "OnPlay",
                            effects=[
                                _template("Draw", count=draw_count),
                                _template("MoveToWaitingRoom"),
                            ],
                            targets=["SelfHand"],
                            conditions=cxcombo_conditions,
                            effect_optional=[True],
                            target_limit=discard_count,
                        )
                    )
                    stats.parsed_lines += 1
                    stats.emitted_defs["Auto.DrawUpToDiscard.OnPlay"] += 1
                    continue

                match = re.match(
                    rf"^draw ({COUNT_TOKEN_RE}) card(?:s)?, choose ({COUNT_TOKEN_RE}) card(?:s)? in your hand, and put (?:it|them) into your waiting room\.?$",
                    effect,
                    re.I,
                )
                if match:
                    draw_count = parse_count_token(match.group(1))
                    discard_count = parse_count_token(match.group(2))
                    if draw_count is None or discard_count is None:
                        mark_unsupported(line)
                        continue
                    ability_defs.append(
                        _ability_def(
                            "Auto",
                            "OnPlay",
                            effects=[
                                _template("Draw", count=draw_count),
                                _template("MoveToWaitingRoom"),
                            ],
                            targets=["SelfHand"],
                            conditions=cxcombo_conditions,
                            target_limit=discard_count,
                        )
                    )
                    stats.parsed_lines += 1
                    stats.emitted_defs["Auto.DrawDiscard.OnPlay"] += 1
                    continue

                match = re.match(
                    rf"^draw up to ({COUNT_TOKEN_RE}) cards?, choose ({COUNT_TOKEN_RE}) cards? in your hand, put (?:it|them) into your waiting room, and put up to ({COUNT_TOKEN_RE}) card(?:s)? from the top of your deck into your stock\.?$",
                    effect,
                    re.I,
                )
                if match:
                    draw_count = parse_count_token(match.group(1))
                    discard_count = parse_count_token(match.group(2))
                    stock_count = parse_count_token(match.group(3))
                    if draw_count is None or discard_count is None or stock_count is None:
                        mark_unsupported(line)
                        continue
                    ability_defs.append(
                        _ability_def(
                            "Auto",
                            "OnPlay",
                            effects=[
                                _template("Draw", count=draw_count),
                                _template("MoveToWaitingRoom"),
                            ],
                            targets=["SelfHand"],
                            conditions=cxcombo_conditions,
                            effect_optional=[True],
                            target_limit=discard_count,
                        )
                    )
                    ability_defs.append(
                        _ability_def(
                            "Auto",
                            "OnPlay",
                            effects=[_template("MoveToStock")],
                            targets=["SelfDeckTop"],
                            conditions=cxcombo_conditions,
                            effect_optional=[True],
                            target_limit=stock_count,
                        )
                    )
                    stats.parsed_lines += 1
                    stats.emitted_defs["Auto.DrawDiscardStockTopSplit.OnPlay"] += 1
                    continue

                match = re.match(
                    rf"^draw ({COUNT_TOKEN_RE}) cards?, choose ({COUNT_TOKEN_RE}) cards? in your hand, put (?:it|them) into your waiting room, and put up to ({COUNT_TOKEN_RE}) card(?:s)? from the top of your deck into your stock\.?$",
                    effect,
                    re.I,
                )
                if match:
                    draw_count = parse_count_token(match.group(1))
                    discard_count = parse_count_token(match.group(2))
                    stock_count = parse_count_token(match.group(3))
                    if draw_count is None or discard_count is None or stock_count is None:
                        mark_unsupported(line)
                        continue
                    ability_defs.append(
                        _ability_def(
                            "Auto",
                            "OnPlay",
                            effects=[
                                _template("Draw", count=draw_count),
                                _template("MoveToWaitingRoom"),
                            ],
                            targets=["SelfHand"],
                            conditions=cxcombo_conditions,
                            target_limit=discard_count,
                        )
                    )
                    ability_defs.append(
                        _ability_def(
                            "Auto",
                            "OnPlay",
                            effects=[_template("MoveToStock")],
                            targets=["SelfDeckTop"],
                            conditions=cxcombo_conditions,
                            effect_optional=[True],
                            target_limit=stock_count,
                        )
                    )
                    stats.parsed_lines += 1
                    stats.emitted_defs["Auto.DrawDiscardStockTopSplit.Exact.OnPlay"] += 1
                    continue

                match = re.match(
                    rf"^draw ({COUNT_TOKEN_RE}) card(?:s)?, choose ({COUNT_TOKEN_RE}) card(?:s)? in your hand, put (?:it|them) into your waiting room, choose up to ({COUNT_TOKEN_RE}) of your opponent's characters, and return (?:it|them) to their hand\.?$",
                    effect,
                    re.I,
                )
                if match:
                    draw_count = parse_count_token(match.group(1))
                    discard_count = parse_count_token(match.group(2))
                    bounce_count = parse_count_token(match.group(3))
                    if draw_count is None or discard_count is None or bounce_count is None:
                        mark_unsupported(line)
                        continue
                    ability_defs.append(
                        _ability_def(
                            "Auto",
                            "OnPlay",
                            effects=[
                                _template("Draw", count=draw_count),
                                _template("MoveToWaitingRoom"),
                            ],
                            targets=["SelfHand"],
                            conditions=cxcombo_conditions,
                            target_limit=discard_count,
                        )
                    )
                    ability_defs.append(
                        _ability_def(
                            "Auto",
                            "OnPlay",
                            effects=[_template("MoveToHand")],
                            targets=["OppStage"],
                            conditions=cxcombo_conditions,
                            effect_optional=[True],
                            target_limit=bounce_count,
                        )
                    )
                    stats.parsed_lines += 1
                    stats.emitted_defs["Auto.DrawDiscardThenBounce.OnPlay"] += 1
                    continue

                match = re.match(
                    rf"^draw up to ({COUNT_TOKEN_RE}) card(?:s)?, and this card gets \+([+-]?\d+) power until end of turn\.?$",
                    effect,
                    re.I,
                )
                if match:
                    draw_count = parse_count_token(match.group(1))
                    if draw_count is None:
                        mark_unsupported(line)
                        continue
                    ability_defs.append(
                        _ability_def(
                            "Auto",
                            "OnPlay",
                            effects=[
                                _template("Draw", count=draw_count),
                                _template(
                                    "AddPower",
                                    amount=int(match.group(2)),
                                    duration_turn=True,
                                ),
                            ],
                            targets=["This"],
                            conditions=cxcombo_conditions,
                            effect_optional=[True],
                        )
                    )
                    stats.parsed_lines += 1
                    stats.emitted_defs["Auto.DrawUpToThenSelfPower.OnPlay"] += 1
                    continue

                match = re.match(
                    rf"^draw ({COUNT_TOKEN_RE}) card(?:s)?, and this card gets \+([+-]?\d+) power until end of turn\.?$",
                    effect,
                    re.I,
                )
                if match:
                    draw_count = parse_count_token(match.group(1))
                    if draw_count is None:
                        mark_unsupported(line)
                        continue
                    ability_defs.append(
                        _ability_def(
                            "Auto",
                            "OnPlay",
                            effects=[
                                _template("Draw", count=draw_count),
                                _template(
                                    "AddPower",
                                    amount=int(match.group(2)),
                                    duration_turn=True,
                                ),
                            ],
                            targets=["This"],
                            conditions=cxcombo_conditions,
                        )
                    )
                    stats.parsed_lines += 1
                    stats.emitted_defs["Auto.DrawThenSelfPower.OnPlay"] += 1
                    continue

                reveal_top_gate_move = re.match(
                    r"^reveal the top card of your deck\. If that card is level (\d+) or higher, you may return this card to your hand\. \((?:Otherwise, )?return the revealed card to (?:its original place|the original place)\. Climax are regarded as level 0\)\.?$",
                    effect,
                    re.I,
                )
                if reveal_top_gate_move:
                    ability_defs.append(
                        _ability_def(
                            "Auto",
                            "OnPlay",
                            effects=[
                                _template(
                                    "RevealTopIfLevelAtLeastMoveThisToHand",
                                    min_level=int(reveal_top_gate_move.group(1)),
                                )
                            ],
                            targets=[],
                            conditions=cxcombo_conditions,
                            effect_optional=[True],
                        )
                    )
                    stats.parsed_lines += 1
                    stats.emitted_defs["Auto.RevealTopGate.MoveSelfToHand.OnPlay"] += 1
                    continue

                reveal_top_gate_stock = re.match(
                    r"^reveal the top card of your deck\. If that card is level (\d+) or higher, put it into your stock\. \((?:Otherwise, )?return (?:it|the revealed card) to (?:its original place|the original place)\. Climax are regarded as level 0\)\.?$",
                    effect,
                    re.I,
                )
                if reveal_top_gate_stock:
                    ability_defs.append(
                        _ability_def(
                            "Auto",
                            "OnPlay",
                            effects=[
                                _template(
                                    "RevealTopIfLevelAtLeastMoveTopToStock",
                                    min_level=int(reveal_top_gate_stock.group(1)),
                                )
                            ],
                            targets=[],
                            conditions=cxcombo_conditions,
                        )
                    )
                    stats.parsed_lines += 1
                    stats.emitted_defs["Auto.RevealTopGate.MoveTopToStock.OnPlay"] += 1
                    continue

                mill_then_climax_branch = re.match(
                    rf"^put the top ({COUNT_TOKEN_RE}) cards? of your deck into your waiting room\. If there is a climax(?: card)?(?: revealed)? among those cards, (.+)$",
                    effect,
                    re.I,
                )
                if mill_then_climax_branch:
                    mill_count = parse_count_token(mill_then_climax_branch.group(1))
                    branch_effect = mill_then_climax_branch.group(2).strip()
                    if mill_count is None:
                        mark_unsupported(line)
                        continue
                    # Base mill effect always applies.
                    ability_defs.append(
                        _ability_def(
                            "Auto",
                            "OnPlay",
                            effects=[_template("MoveToWaitingRoom")],
                            targets=["SelfDeckTop"],
                            conditions=cxcombo_conditions,
                            target_limit=mill_count,
                        )
                    )

                    branch_match = re.match(
                        rf"^choose ({COUNT_TOKEN_RE}) of your characters, and that character gets \+(\d+) power until end of turn\.?$",
                        branch_effect,
                        re.I,
                    )
                    if branch_match:
                        choose_count = parse_count_token(branch_match.group(1))
                        if choose_count is None:
                            mark_unsupported(line)
                            continue
                        ability_defs.append(
                            _ability_def(
                                "Auto",
                                "OnPlay",
                                effects=[
                                    _template(
                                        "AddPower",
                                        amount=int(branch_match.group(2)),
                                        duration_turn=True,
                                    )
                                ],
                                targets=["SelfStage"],
                                conditions=cxcombo_conditions,
                                target_limit=choose_count,
                            )
                        )
                        stats.parsed_lines += 1
                        stats.emitted_defs["Auto.MillIfClimax.ThenTeamBuff.OnPlay"] += 1
                        continue

                    branch_match = re.match(
                        rf"^choose ({COUNT_TOKEN_RE}) character in your opponent's center stage, and that character gets -(\d+) power until end of turn\.?$",
                        branch_effect,
                        re.I,
                    )
                    if branch_match:
                        choose_count = parse_count_token(branch_match.group(1))
                        if choose_count is None:
                            mark_unsupported(line)
                            continue
                        ability_defs.append(
                            _ability_def(
                                "Auto",
                                "OnPlay",
                                effects=[
                                    _template(
                                        "AddPower",
                                        amount=-int(branch_match.group(2)),
                                        duration_turn=True,
                                    )
                                ],
                                targets=["OppFrontRow"],
                                conditions=cxcombo_conditions,
                                target_limit=choose_count,
                            )
                        )
                        stats.parsed_lines += 1
                        stats.emitted_defs["Auto.MillIfClimax.ThenOppCenterDebuff.OnPlay"] += 1
                        continue

                    branch_match = re.match(
                        r"^this card gets \+(\d+) power until end of turn\.?$",
                        branch_effect,
                        re.I,
                    )
                    if branch_match:
                        ability_defs.append(
                            _ability_def(
                                "Auto",
                                "OnPlay",
                                effects=[
                                    _template(
                                        "AddPower",
                                        amount=int(branch_match.group(1)),
                                        duration_turn=True,
                                    )
                                ],
                                targets=["This"],
                                conditions=cxcombo_conditions,
                            )
                        )
                        stats.parsed_lines += 1
                        stats.emitted_defs["Auto.MillIfClimax.ThenSelfPower.OnPlay"] += 1
                        continue

                    branch_match = re.match(
                        r"^this card gets \+(\d+) soul until end of turn\.?$",
                        branch_effect,
                        re.I,
                    )
                    if branch_match:
                        ability_defs.append(
                            _ability_def(
                                "Auto",
                                "OnPlay",
                                effects=[
                                    _template(
                                        "AddSoul",
                                        amount=int(branch_match.group(1)),
                                        duration_turn=True,
                                    )
                                ],
                                targets=["This"],
                                conditions=cxcombo_conditions,
                            )
                        )
                        stats.parsed_lines += 1
                        stats.emitted_defs["Auto.MillIfClimax.ThenSelfSoul.OnPlay"] += 1
                        continue

                    # Revert the base mill-only fallback if branch parse is unknown.
                    ability_defs.pop()

                following_grant = parse_following_ability_grant(effect)
                if following_grant is not None:
                    optional = [following_grant["optional"]] if following_grant["optional"] else []
                    grant_conditions = cxcombo_conditions
                    if following_grant.get("approx"):
                        grant_conditions = with_approx_condition(cxcombo_conditions)
                    ability_defs.append(
                        _ability_def(
                            "Auto",
                            "OnPlay",
                            effects=following_grant.get("effects", [following_grant["effect"]]),
                            targets=following_grant["targets"],
                            conditions=grant_conditions,
                            effect_optional=optional,
                            target_limit=following_grant.get("target_limit"),
                        )
                    )
                    stats.parsed_lines += 1
                    if following_grant.get("approx"):
                        stats.emitted_defs["Auto.FollowingAbilityApproxNoop.OnPlay"] += 1
                    else:
                        stats.emitted_defs["Auto.FollowingAbilityFlattened.OnPlay"] += 1
                    continue

                on_play_power_following_opp_next_exact = re.match(
                    r'^this card gets \+([+-]?\d+) power and the following ability until the end of your opponent\'s next turn\.\s*"(.+)"\.?$',
                    effect,
                    re.I,
                )
                if on_play_power_following_opp_next_exact:
                    nested_effect = parse_following_effect_or_grant(
                        on_play_power_following_opp_next_exact.group(2),
                        duration_turn=True,
                        grant_duration="UntilEndOfOpponentsNextTurn",
                    )
                    if nested_effect is not None:
                        ability_defs.append(
                            _ability_def(
                                "Auto",
                                "OnPlay",
                                effects=[
                                    _template(
                                        "AddPower",
                                        amount=int(on_play_power_following_opp_next_exact.group(1)),
                                        duration_turn=True,
                                    ),
                                    nested_effect,
                                ],
                                targets=["This", "This"],
                                conditions=cxcombo_conditions,
                            )
                        )
                        stats.parsed_lines += 1
                        stats.emitted_defs[
                            "Auto.AddPowerWithFollowingAbilityFlattened.OpponentNextTurn.OnPlay"
                        ] += 1
                        continue

                if allow_approx_effects:
                    on_play_power_following_opp_next = re.match(
                        r'^this card gets \+(\d+) power and the following ability until the end of your opponent\'s next turn\.\s*"[^"]+".*$',
                        effect,
                        re.I,
                    )
                    if on_play_power_following_opp_next:
                        ability_defs.append(
                            _ability_def(
                                "Auto",
                                "OnPlay",
                                effects=[
                                    _template(
                                        "AddPower",
                                        amount=int(on_play_power_following_opp_next.group(1)),
                                        duration_turn=True,
                                    )
                                ],
                                targets=["This"],
                                conditions=with_approx_condition(cxcombo_conditions),
                            )
                        )
                        stats.parsed_lines += 1
                        stats.emitted_defs[
                            "Auto.AddPowerWithFollowingAbility.OpponentNextTurn.Approx.OnPlay"
                        ] += 1
                        continue

                per_self_selector_count_buff = re.match(
                    r"^this card gets \+X power until end of turn\. X is equal to the number of (your )?(other )?(.+?) characters(?: you have)? ×(\d+)\.?$",
                    effect,
                    re.I,
                )
                if per_self_selector_count_buff:
                    selector = per_self_selector_count_buff.group(3).strip()
                    amount = int(per_self_selector_count_buff.group(4))
                    other_only = bool(per_self_selector_count_buff.group(2))
                    selector_card_ids = resolve_stage_selector_card_ids(selector)
                    if not selector_card_ids:
                        mark_unsupported(line)
                        continue
                    effects: List[Any] = [
                        _template(
                            "TimedConditionalAddPower",
                            amount=amount,
                            duration_turn=True,
                            turn=None,
                            zone_count={
                                "side": "SelfSide",
                                "zone": "Stage",
                                "cmp": "AtLeast",
                                "value": 0,
                                "card_ids": selector_card_ids,
                            },
                            require_source_marker=False,
                            per_source_marker=False,
                            per_zone_count=True,
                            exclude_source=False,
                            target_ids=[],
                        )
                    ]
                    if (
                        other_only
                        and source_card_id is not None
                        and source_card_id in selector_card_ids
                    ):
                        effects.append(
                            _template(
                                "AddPower",
                                amount=-amount,
                                duration_turn=True,
                            )
                        )
                    ability_defs.append(
                        _ability_def(
                            "Auto",
                            "OnPlay",
                            effects=effects,
                            targets=["This"],
                            conditions=cxcombo_conditions,
                        )
                    )
                    stats.parsed_lines += 1
                    stats.emitted_defs["Auto.ConditionalPower.PerSelfSelectorCount.OnPlay"] += 1
                    continue

                per_self_selector_count_multiplied_buff = re.match(
                    r"^this card gets \+X power until end of turn\. X is equal to (\d+) multiplied by the number of (your )?(other )?(.+?) characters(?: you have)?\.?$",
                    effect,
                    re.I,
                )
                if per_self_selector_count_multiplied_buff:
                    amount = int(per_self_selector_count_multiplied_buff.group(1))
                    selector = per_self_selector_count_multiplied_buff.group(4).strip()
                    other_only = bool(per_self_selector_count_multiplied_buff.group(3))
                    selector_card_ids = resolve_stage_selector_card_ids(selector)
                    if not selector_card_ids:
                        mark_unsupported(line)
                        continue
                    effects: List[Any] = [
                        _template(
                            "TimedConditionalAddPower",
                            amount=amount,
                            duration_turn=True,
                            turn=None,
                            zone_count={
                                "side": "SelfSide",
                                "zone": "Stage",
                                "cmp": "AtLeast",
                                "value": 0,
                                "card_ids": selector_card_ids,
                            },
                            require_source_marker=False,
                            per_source_marker=False,
                            per_zone_count=True,
                            exclude_source=False,
                            target_ids=[],
                        )
                    ]
                    if (
                        other_only
                        and source_card_id is not None
                        and source_card_id in selector_card_ids
                    ):
                        effects.append(
                            _template(
                                "AddPower",
                                amount=-amount,
                                duration_turn=True,
                            )
                        )
                    ability_defs.append(
                        _ability_def(
                            "Auto",
                            "OnPlay",
                            effects=effects,
                            targets=["This"],
                            conditions=cxcombo_conditions,
                        )
                    )
                    stats.parsed_lines += 1
                    stats.emitted_defs[
                        "Auto.ConditionalPower.PerSelfSelectorCount.Multiplied.OnPlay"
                    ] += 1
                    continue

                per_opp_stage_count_buff = re.match(
                    r"^this card gets \+X power until end of turn\. X is equal to the number of characters your opponent has ×(\d+)\.?$",
                    effect,
                    re.I,
                )
                if per_opp_stage_count_buff:
                    ability_defs.append(
                        _ability_def(
                            "Auto",
                            "OnPlay",
                            effects=[
                                _template(
                                    "TimedConditionalAddPower",
                                    amount=int(per_opp_stage_count_buff.group(1)),
                                    duration_turn=True,
                                    turn=None,
                                    zone_count={
                                        "side": "Opponent",
                                        "zone": "Stage",
                                        "cmp": "AtLeast",
                                        "value": 0,
                                    },
                                    require_source_marker=False,
                                    per_source_marker=False,
                                    per_zone_count=True,
                                    exclude_source=False,
                                    target_ids=[],
                                )
                            ],
                            targets=["This"],
                            conditions=cxcombo_conditions,
                        )
                    )
                    stats.parsed_lines += 1
                    stats.emitted_defs["Auto.ConditionalPower.PerOpponentStageCount.OnPlay"] += 1
                    continue

                if allow_approx_effects:
                    approx_all_players_action = re.match(
                        r'^all players perform the following action\.\s*"[^"]+"\.?$',
                        effect,
                        re.I,
                    )
                    if approx_all_players_action:
                        ability_defs.append(
                            _ability_def(
                                "Auto",
                                "OnPlay",
                                effects=[_template("Draw", count=0)],
                                targets=[],
                                conditions=with_approx_condition(cxcombo_conditions),
                            )
                        )
                        stats.parsed_lines += 1
                        stats.emitted_defs["Auto.AllPlayersAction.ApproxNoop.OnPlay"] += 1
                        continue

                if "may" in effect.lower() and "choose" not in effect.lower():
                    mark_unsupported(line)
                    continue

                match = re.match(r"^draw (\d+) card(?:s)?\.?$", effect, re.I)
                if match and "may" not in effect.lower() and "up to" not in effect.lower():
                    abilities.append(_template("AutoOnPlayDraw", count=int(match.group(1))))
                    stats.parsed_lines += 1
                    stats.emitted_templates["AutoOnPlayDraw"] += 1
                    continue

                match = re.match(
                    r"^put the top (\d+) card(?:s)? of your deck into your waiting room\.?$",
                    effect,
                    re.I,
                )
                if match and "may" not in effect.lower():
                    abilities.append(_template("AutoOnPlayMillTop", count=int(match.group(1))))
                    stats.parsed_lines += 1
                    stats.emitted_templates["AutoOnPlayMillTop"] += 1
                    continue

                match = re.match(
                    r"^put the top (\d+) card(?:s)? of your deck into your stock\.?$",
                    effect,
                    re.I,
                )
                if match and "may" not in effect.lower():
                    abilities.append(_template("AutoOnPlayStockCharge", count=int(match.group(1))))
                    stats.parsed_lines += 1
                    stats.emitted_templates["AutoOnPlayStockCharge"] += 1
                    continue

                match = re.match(
                    r"^put the top card of your clock into your waiting room\.?$",
                    effect,
                    re.I,
                )
                if match and "may" not in effect.lower():
                    abilities.append(_template("AutoOnPlayHeal", count=1))
                    stats.parsed_lines += 1
                    stats.emitted_templates["AutoOnPlayHeal"] += 1
                    continue

                match = re.match(
                    rf"^(you may )?choose (up to )?({COUNT_TOKEN_RE}) (.+?) in your waiting room, and return (?:it|them) to your hand\.?$",
                    effect,
                    re.I,
                )
                if match:
                    count = parse_count_token(match.group(3))
                    if count is None:
                        mark_unsupported(line)
                        continue
                    optional = bool(match.group(1)) or bool(match.group(2))
                    selector = match.group(4).strip()
                    selector_constraints = parse_generic_selector_constraints(selector)
                    if selector_constraints is not None:
                        ability_defs.append(
                            _ability_def(
                                "Auto",
                                "OnPlay",
                                effects=[_template("MoveToHand")],
                                targets=["SelfWaitingRoom"],
                                conditions=cxcombo_conditions,
                                effect_optional=[optional] if optional else [],
                                target_card_type=selector_constraints["target_card_type"],
                                target_trait=selector_constraints["target_trait"],
                                target_level_max=selector_constraints["target_level_max"],
                                target_cost_max=selector_constraints["target_cost_max"],
                                target_card_ids=selector_constraints["target_card_ids"],
                                target_limit=count,
                            )
                        )
                        stats.parsed_lines += 1
                        stats.emitted_defs["Auto.SalvageWaitingRoom.OnPlay"] += 1
                        continue

                    type_text = selector.lower()
                    if re.fullmatch(r"[a-z ]+", type_text):
                        card_type_hint = None
                        if "character" in type_text:
                            card_type_hint = "Character"
                        elif "climax" in type_text:
                            card_type_hint = "Climax"
                        elif "event" in type_text:
                            card_type_hint = "Event"
                        abilities.append(
                            _template(
                                "AutoOnPlaySalvage",
                                count=count,
                                optional=optional,
                                card_type=card_type_hint,
                            )
                        )
                        stats.parsed_lines += 1
                        stats.emitted_templates["AutoOnPlaySalvage"] += 1
                        continue

                marker_match = re.match(
                    r"^(you may )?choose (up to )?(\d+) (.+?) in your waiting room, and put (?:it|them) face up underneath this card as a marker\.?$",
                    effect,
                    re.I,
                )
                if marker_match:
                    optional = bool(marker_match.group(1)) or bool(marker_match.group(2))
                    count = int(marker_match.group(3))
                    if count != 1:
                        mark_unsupported(line)
                        continue
                    selector = marker_match.group(4).strip()
                    card_type_hint = None
                    selector_lower = selector.lower()
                    if "character" in selector_lower:
                        card_type_hint = "Character"
                    elif "climax" in selector_lower:
                        card_type_hint = "Climax"
                    elif "event" in selector_lower:
                        card_type_hint = "Event"
                    target_ids: List[int] = []
                    if name_to_ids:
                        seen_ids = set()
                        for quoted in re.findall(r'"([^"]+)"', selector):
                            name = quoted.strip()
                            if not name:
                                continue
                            for card_id in name_to_ids.get(name, []):
                                if card_id in seen_ids:
                                    continue
                                seen_ids.add(card_id)
                                target_ids.append(card_id)
                    ability_defs.append(
                        _ability_def(
                            "Auto",
                            "OnPlay",
                            effects=[_template("MoveToMarker", target_ids=target_ids)],
                            targets=["SelfWaitingRoom"],
                            conditions=cxcombo_conditions,
                            effect_optional=[optional],
                            target_card_type=card_type_hint,
                        )
                    )
                    stats.parsed_lines += 1
                    stats.emitted_defs["Auto.MoveToMarker.OnPlay"] += 1
                    continue

                marker_named_match = re.match(
                    r'^(you may )?choose a card named "([^"]+)" in your waiting room, and put it face down underneath this card as a marker\.?$',
                    effect,
                    re.I,
                )
                if marker_named_match:
                    optional = bool(marker_named_match.group(1))
                    named = marker_named_match.group(2).strip()
                    target_ids = sorted(set((name_to_ids or {}).get(named, [])))
                    if not target_ids:
                        mark_unsupported(line)
                        continue
                    ability_defs.append(
                        _ability_def(
                            "Auto",
                            "OnPlay",
                            effects=[_template("MoveToMarker", target_ids=target_ids)],
                            targets=["SelfWaitingRoom"],
                            conditions=cxcombo_conditions,
                            effect_optional=[optional],
                            target_limit=1,
                        )
                    )
                    stats.parsed_lines += 1
                    stats.emitted_defs["Auto.MoveNamedToMarker.OnPlay"] += 1
                    continue

                on_play_other_trait_buff = re.match(
                    rf"^choose (up to )?({COUNT_TOKEN_RE}) of your other 《([^》]+)》 characters, and that character gets \+([+-]?\d+) power until end of turn\.?$",
                    effect,
                    re.I,
                )
                if on_play_other_trait_buff:
                    choose_count = parse_count_token(on_play_other_trait_buff.group(2))
                    trait_name = on_play_other_trait_buff.group(3).strip()
                    trait_id = (trait_map or {}).get(trait_name)
                    if choose_count is None or trait_id is None:
                        mark_unsupported(line)
                        continue
                    optional = bool(on_play_other_trait_buff.group(1))
                    ability_defs.append(
                        _ability_def(
                            "Auto",
                            "OnPlay",
                            effects=[
                                _template(
                                    "TimedConditionalAddPower",
                                    amount=int(on_play_other_trait_buff.group(4)),
                                    duration_turn=True,
                                    turn=None,
                                    zone_count=None,
                                    require_source_marker=False,
                                    per_source_marker=False,
                                    per_zone_count=False,
                                    exclude_source=True,
                                    target_ids=[],
                                )
                            ],
                            targets=["SelfStage"],
                            conditions=cxcombo_conditions,
                            effect_optional=[optional] if optional else [],
                            target_trait=trait_id,
                            target_limit=choose_count,
                        )
                    )
                    stats.parsed_lines += 1
                    stats.emitted_defs["Auto.AddPower.OtherTrait.OnPlay"] += 1
                    continue

                on_play_team_power = re.match(
                    rf"^choose (up to )?({COUNT_TOKEN_RE}) of your (.+?) characters, and that character gets \+([+-]?\d+) power until end of turn\.?$",
                    effect,
                    re.I,
                )
                if on_play_team_power:
                    choose_count = parse_count_token(on_play_team_power.group(2))
                    if choose_count is None:
                        mark_unsupported(line)
                        continue
                    selector = on_play_team_power.group(3).strip()
                    optional = bool(on_play_team_power.group(1))
                    selector_for_resolution = selector
                    selector_lower = selector.lower()
                    exclude_source = False
                    if selector_lower.startswith("other "):
                        exclude_source = True
                        selector_for_resolution = selector[6:].strip()
                    target_trait = resolve_single_trait_selector(selector_for_resolution)
                    target_card_ids: List[int] = []
                    selector_compact = re.sub(r"\s+", " ", selector_lower).strip()
                    generic_selector = selector_compact in {"", "other"}
                    if not generic_selector and target_trait is None:
                        target_card_ids = (
                            resolve_stage_selector_card_ids(selector_for_resolution) or []
                        )
                        if not target_card_ids:
                            if allow_approx_effects:
                                ability_defs.append(
                                    _ability_def(
                                        "Auto",
                                        "OnPlay",
                                        effects=[_template("Draw", count=0)],
                                        targets=[],
                                        conditions=with_approx_condition(cxcombo_conditions),
                                        effect_optional=[optional] if optional else [],
                                    )
                                )
                                stats.parsed_lines += 1
                                stats.emitted_defs[
                                    "Auto.AddPower.TeamSelector.OnPlay.ApproxNoop"
                                ] += 1
                                continue
                            mark_unsupported(line)
                            continue
                    if exclude_source:
                        ability_defs.append(
                            _ability_def(
                                "Auto",
                                "OnPlay",
                                effects=[
                                    _template(
                                        "TimedConditionalAddPower",
                                        amount=int(on_play_team_power.group(4)),
                                        duration_turn=True,
                                        turn=None,
                                        zone_count=None,
                                        require_source_marker=False,
                                        per_source_marker=False,
                                        per_zone_count=False,
                                        exclude_source=True,
                                        target_ids=[],
                                    )
                                ],
                                targets=["SelfStage"],
                                conditions=cxcombo_conditions,
                                effect_optional=[optional] if optional else [],
                                target_trait=target_trait,
                                target_card_ids=target_card_ids,
                                target_limit=choose_count,
                            )
                        )
                    else:
                        ability_defs.append(
                            _ability_def(
                                "Auto",
                                "OnPlay",
                                effects=[
                                    _template(
                                        "AddPower",
                                        amount=int(on_play_team_power.group(4)),
                                        duration_turn=True,
                                    )
                                ],
                                targets=["SelfStage"],
                                conditions=cxcombo_conditions,
                                effect_optional=[optional] if optional else [],
                                target_trait=target_trait,
                                target_card_ids=target_card_ids,
                                target_limit=choose_count,
                            )
                        )
                    stats.parsed_lines += 1
                    stats.emitted_defs["Auto.AddPower.TeamSelector.OnPlay"] += 1
                    continue

                on_play_if_has_other_trait = re.match(
                    r"^if you have another 《([^》]+)》 character, this card gets \+([+-]?\d+) power until end of turn\.?$",
                    effect,
                    re.I,
                ) or re.match(
                    rf"^if you have ({COUNT_TOKEN_RE}) or more other 《([^》]+)》 characters, this card gets \+([+-]?\d+) power until end of turn\.?$",
                    effect,
                    re.I,
                )
                if on_play_if_has_other_trait:
                    if re.match(
                        r"^if you have another 《([^》]+)》 character, this card gets \+([+-]?\d+) power until end of turn\.?$",
                        effect,
                        re.I,
                    ):
                        min_count = 1
                        trait_name = on_play_if_has_other_trait.group(1).strip()
                        amount = int(on_play_if_has_other_trait.group(2))
                    else:
                        min_count = parse_count_token(on_play_if_has_other_trait.group(1))
                        trait_name = on_play_if_has_other_trait.group(2).strip()
                        amount = int(on_play_if_has_other_trait.group(3))
                        if min_count is None:
                            mark_unsupported(line)
                            continue
                    trait_ids = sorted(set((trait_to_ids or {}).get(trait_name, [])))
                    if not trait_ids:
                        mark_unsupported(line)
                        continue
                    threshold = min_count + (
                        1 if source_card_id is not None and source_card_id in trait_ids else 0
                    )
                    ability_defs.append(
                        _ability_def(
                            "Auto",
                            "OnPlay",
                            effects=[
                                _template(
                                    "TimedConditionalAddPower",
                                    amount=amount,
                                    duration_turn=True,
                                    turn=None,
                                    zone_count={
                                        "side": "SelfSide",
                                        "zone": "Stage",
                                        "cmp": "AtLeast",
                                        "value": threshold,
                                        "card_ids": trait_ids,
                                    },
                                    require_source_marker=False,
                                    per_source_marker=False,
                                    per_zone_count=False,
                                    exclude_source=False,
                                    target_ids=[],
                                )
                            ],
                            targets=["This"],
                            conditions=cxcombo_conditions,
                        )
                    )
                    stats.parsed_lines += 1
                    stats.emitted_defs["Auto.ConditionalPower.IfHasOtherTrait.OnPlay"] += 1
                    continue

                on_play_search_deck_to_hand = re.match(
                    rf"^(you may )?search your deck for up to ({COUNT_TOKEN_RE}) (.+?), reveal (?:it|them) to your opponent, (?:and )?put (?:it|them) into your hand(?:, and shuffle your deck(?: afterwards)?|\. shuffle your deck(?: afterwards)?|\. shuffle your deck|, shuffle your deck)\.?$",
                    effect,
                    re.I,
                )
                if on_play_search_deck_to_hand:
                    choose_count = parse_count_token(on_play_search_deck_to_hand.group(2))
                    if choose_count is None:
                        mark_unsupported(line)
                        continue
                    selector_constraints = parse_generic_selector_constraints(
                        on_play_search_deck_to_hand.group(3)
                    )
                    if selector_constraints is not None:
                        optional = bool(on_play_search_deck_to_hand.group(1))
                        ability_defs.append(
                            _ability_def(
                                "Auto",
                                "OnPlay",
                                effects=[_template("MoveToHand")],
                                targets=["SelfDeckTop"],
                                conditions=cxcombo_conditions,
                                effect_optional=[optional] if optional else [],
                                target_card_type=selector_constraints["target_card_type"],
                                target_trait=selector_constraints["target_trait"],
                                target_level_max=selector_constraints["target_level_max"],
                                target_cost_max=selector_constraints["target_cost_max"],
                                target_card_ids=selector_constraints["target_card_ids"],
                                target_limit=choose_count,
                            )
                        )
                        stats.parsed_lines += 1
                        stats.emitted_defs["Auto.SearchDeckToHand.OnPlay"] += 1
                        continue

                match = re.match(
                    r"^choose 1 of your characters, and that character gets \+(\d+) power until end of turn\.?$",
                    effect,
                    re.I,
                )
                if match:
                    ability_defs.append(
                        _ability_def(
                            "Auto",
                            "OnPlay",
                            effects=[
                                _template(
                                    "AddPower", amount=int(match.group(1)), duration_turn=True
                                )
                            ],
                            targets=["SelfStage"],
                            conditions=cxcombo_conditions,
                        )
                    )
                    stats.parsed_lines += 1
                    stats.emitted_defs["Auto.AddPower.OnPlay"] += 1
                    continue

                if allow_approx_effects and looks_like_search_or_salvage_text(effect):
                    optional = bool(re.search(r"\byou may\b|\bup to\b", effect, re.I))
                    ability_defs.append(
                        _ability_def(
                            "Auto",
                            "OnPlay",
                            effects=[_template("Draw", count=0)],
                            targets=[],
                            conditions=with_approx_condition(cxcombo_conditions),
                            effect_optional=[optional] if optional else [],
                        )
                    )
                    stats.parsed_lines += 1
                    stats.emitted_defs["Auto.SearchSalvage.Generic.OnPlay.ApproxNoop"] += 1
                    continue

            match = re.match(
                r"^When (?:your other|another of your) (.+?) character(?:s)? attacks, this card gets \+(\d+) power until end of turn\.?$",
                remainder,
                re.I,
            )
            if match:
                if not cost_is_empty(cost):
                    mark_unsupported(line)
                    continue
                selector = match.group(1).strip()
                selector_card_ids = resolve_stage_selector_card_ids(selector)
                if not selector_card_ids:
                    mark_unsupported(line)
                    continue
                ability_defs.append(
                    _ability_def(
                        "Auto",
                        "OtherAttackDeclaration",
                        effects=[
                            _template(
                                "AddPowerIfOtherAttackerMatches",
                                amount=int(match.group(2)),
                                duration_turn=True,
                                attacker_card_ids=selector_card_ids,
                            )
                        ],
                        targets=[],
                    )
                )
                stats.parsed_lines += 1
                stats.emitted_defs["Auto.AddPower.IfOtherAttackerMatches"] += 1
                continue

            direct_attack_other_char_buff = re.match(
                rf"^When this card direct attacks, choose (up to )?({COUNT_TOKEN_RE}) of your other characters, and that character gets \+([+-]?\d+) power until end of turn\.?$",
                remainder,
                re.I,
            )
            if direct_attack_other_char_buff:
                choose_count = parse_count_token(direct_attack_other_char_buff.group(2))
                if choose_count is None:
                    mark_unsupported(line)
                    continue
                optional = bool(direct_attack_other_char_buff.group(1))
                ability_defs.append(
                    _ability_def(
                        "Auto",
                        "AttackDeclaration",
                        effects=[
                            _template(
                                "TimedConditionalAddPower",
                                amount=int(direct_attack_other_char_buff.group(3)),
                                duration_turn=True,
                                turn=None,
                                zone_count=None,
                                require_source_marker=False,
                                per_source_marker=False,
                                per_zone_count=False,
                                exclude_source=True,
                                target_ids=[],
                            )
                        ],
                        targets=["SelfStage"],
                        effect_optional=[optional] if optional else [],
                        target_limit=choose_count,
                    )
                )
                stats.parsed_lines += 1
                stats.emitted_defs["Auto.AddPower.OtherCharacter.DirectAttack"] += 1
                continue

            match = re.match(
                r"^When this card attacks, (.+)$",
                remainder,
                re.I,
            )
            if match:
                effect = match.group(1).strip()
                effect, cxcombo_conditions = strip_cxcombo_condition_prefix(effect, has_cxcombo_tag)

                buff_level_at_least = re.match(
                    r"^if (?:the character facing this card|this card's battle opponent) is level (\d+) or higher, this card gets \+(\d+) power until end of turn\.?$",
                    effect,
                    re.I,
                )
                if buff_level_at_least:
                    ability_defs.append(
                        _ability_def(
                            "Auto",
                            "AttackDeclaration",
                            effects=[
                                _template(
                                    "AddPowerIfBattleOpponentLevelAtLeast",
                                    amount=int(buff_level_at_least.group(2)),
                                    min_level=int(buff_level_at_least.group(1)),
                                    duration_turn=True,
                                )
                            ],
                            targets=[],
                            conditions=cxcombo_conditions,
                        )
                    )
                    stats.parsed_lines += 1
                    stats.emitted_defs[
                        "Auto.AddPowerIfBattleOpponentLevelAtLeast.AttackDeclaration"
                    ] += 1
                    continue

                buff_level_at_least_power_soul = re.match(
                    r"^if (?:the character facing this card|this card's battle opponent) is level (\d+) or higher, this card gets \+(\d+) power and \+(\d+) soul until end of turn\.?$",
                    effect,
                    re.I,
                )
                if buff_level_at_least_power_soul:
                    min_level = int(buff_level_at_least_power_soul.group(1))
                    ability_defs.append(
                        _ability_def(
                            "Auto",
                            "AttackDeclaration",
                            effects=[
                                _template(
                                    "AddPowerIfBattleOpponentLevelAtLeast",
                                    amount=int(buff_level_at_least_power_soul.group(2)),
                                    min_level=min_level,
                                    duration_turn=True,
                                ),
                                _template(
                                    "AddSoulIfBattleOpponentLevelAtLeast",
                                    amount=int(buff_level_at_least_power_soul.group(3)),
                                    min_level=min_level,
                                    duration_turn=True,
                                ),
                            ],
                            targets=[],
                            conditions=cxcombo_conditions,
                        )
                    )
                    stats.parsed_lines += 1
                    stats.emitted_defs[
                        "Auto.AddPowerSoulIfBattleOpponentLevelAtLeast.AttackDeclaration"
                    ] += 1
                    continue

                buff_level_exact = re.match(
                    r"^if (?:the character facing this card|this card's battle opponent) is level (\d+), this card gets \+(\d+) power until end of turn\.?$",
                    effect,
                    re.I,
                )
                if buff_level_exact:
                    ability_defs.append(
                        _ability_def(
                            "Auto",
                            "AttackDeclaration",
                            effects=[
                                _template(
                                    "AddPowerIfBattleOpponentLevelExact",
                                    amount=int(buff_level_exact.group(2)),
                                    level=int(buff_level_exact.group(1)),
                                    duration_turn=True,
                                )
                            ],
                            targets=[],
                            conditions=cxcombo_conditions,
                        )
                    )
                    stats.parsed_lines += 1
                    stats.emitted_defs[
                        "Auto.AddPowerIfBattleOpponentLevelExact.AttackDeclaration"
                    ] += 1
                    continue

                attack_named_climax_team_power = re.match(
                    r'^if a card named "([^"]+)" is in your climax area, all of your characters get \+([+-]?\d+) power until end of turn\.?$',
                    effect,
                    re.I,
                )
                if attack_named_climax_team_power:
                    named = attack_named_climax_team_power.group(1).strip()
                    named_ids = sorted(set((name_to_ids or {}).get(named, [])))
                    if not named_ids:
                        mark_unsupported(line)
                        continue
                    ability_defs.append(
                        _ability_def(
                            "Auto",
                            "AttackDeclaration",
                            effects=[
                                _template(
                                    "AddPower",
                                    amount=int(attack_named_climax_team_power.group(2)),
                                    duration_turn=True,
                                )
                            ],
                            targets=["SelfStage"],
                            conditions={"climax_area": {"side": "SelfSide", "card_ids": named_ids}},
                        )
                    )
                    stats.parsed_lines += 1
                    stats.emitted_defs["Auto.NamedClimaxTeamPower.AttackDeclaration"] += 1
                    continue

                attack_trait_team_power = re.match(
                    rf"^choose (up to )?({COUNT_TOKEN_RE}) of your 《([^》]+)》 characters, and that character gets \+([+-]?\d+) power until end of turn\.?$",
                    effect,
                    re.I,
                ) or re.match(
                    rf"^choose (up to )?({COUNT_TOKEN_RE}) of your 《([^》]+)》 characters, and those characters get \+([+-]?\d+) power until end of turn\.?$",
                    effect,
                    re.I,
                )
                if attack_trait_team_power:
                    choose_count = parse_count_token(attack_trait_team_power.group(2))
                    trait_name = attack_trait_team_power.group(3).strip()
                    trait_id = (trait_map or {}).get(trait_name)
                    if choose_count is None or trait_id is None:
                        mark_unsupported(line)
                        continue
                    optional = bool(attack_trait_team_power.group(1))
                    ability_defs.append(
                        _ability_def(
                            "Auto",
                            "AttackDeclaration",
                            effects=[
                                _template(
                                    "AddPower",
                                    amount=int(attack_trait_team_power.group(4)),
                                    duration_turn=True,
                                )
                            ],
                            targets=["SelfStage"],
                            conditions=cxcombo_conditions,
                            effect_optional=([True] if optional else []),
                            target_trait=trait_id,
                            target_limit=choose_count,
                        )
                    )
                    stats.parsed_lines += 1
                    stats.emitted_defs["Auto.TraitTeamPower.AttackDeclaration"] += 1
                    continue

                attack_look_top_decide = re.match(
                    r"^look at the top card of your deck, and put it on the top of your deck or into your waiting room\.?$",
                    effect,
                    re.I,
                )
                if attack_look_top_decide:
                    ability_defs.append(
                        _ability_def(
                            "Auto",
                            "AttackDeclaration",
                            effects=[_template("MoveToWaitingRoom")],
                            targets=["SelfDeckTop"],
                            conditions=cxcombo_conditions,
                            effect_optional=[True],
                            target_limit=1,
                        )
                    )
                    stats.parsed_lines += 1
                    stats.emitted_defs["Auto.LookTopDeck.MayMill.AttackDeclaration"] += 1
                    continue

                if allow_approx_effects:
                    attack_mill_backstage = re.match(
                        rf"^if you have ({COUNT_TOKEN_RE}) or less other characters, you may put the top card of your deck into your waiting room\. If that card is a level ({COUNT_TOKEN_RE}) or lower character, put it on any position of your back stage\.?$",
                        effect,
                        re.I,
                    )
                    if attack_mill_backstage:
                        ability_defs.append(
                            _ability_def(
                                "Auto",
                                "AttackDeclaration",
                                effects=[_template("MoveToWaitingRoom")],
                                targets=["SelfDeckTop"],
                                effect_optional=[True],
                                target_limit=1,
                                conditions=with_approx_condition(cxcombo_conditions),
                            )
                        )
                        stats.parsed_lines += 1
                        stats.emitted_defs[
                            "Auto.MillTopThenBackStageSummon.AttackDeclaration.PartialApprox"
                        ] += 1
                        continue

                    attack_soul_scaled_power = re.match(
                        rf"^choose ({COUNT_TOKEN_RE}) of your other characters, and that character gets \+X power until end of turn\. X is equal to that character's soul ×({COUNT_TOKEN_RE})\.?$",
                        effect,
                        re.I,
                    )
                    if attack_soul_scaled_power:
                        ability_defs.append(
                            _ability_def(
                                "Auto",
                                "AttackDeclaration",
                                effects=[_template("Draw", count=0)],
                                targets=[],
                                conditions=with_approx_condition(cxcombo_conditions),
                            )
                        )
                        stats.parsed_lines += 1
                        stats.emitted_defs["Auto.SoulScaledPower.AttackDeclaration.ApproxNoop"] += 1
                        continue

                    attack_rest_other_stand = re.match(
                        r"^【REST】 all of your other 【STAND】 characters\.?$",
                        effect,
                        re.I,
                    )
                    if attack_rest_other_stand:
                        ability_defs.append(
                            _ability_def(
                                "Auto",
                                "AttackDeclaration",
                                effects=[_template("Draw", count=0)],
                                targets=[],
                                conditions=with_approx_condition(cxcombo_conditions),
                            )
                        )
                        stats.parsed_lines += 1
                        stats.emitted_defs[
                            "Auto.RestAllOtherStand.AttackDeclaration.ApproxNoop"
                        ] += 1
                        continue

                attack_look_top_keep_one = re.match(
                    rf"^look at up to ({COUNT_TOKEN_RE}) cards? from the top of your deck, choose ({COUNT_TOKEN_RE}) card from among them, put it on the top of your deck, and put the rest into your waiting room\.?$",
                    effect,
                    re.I,
                )
                if attack_look_top_keep_one:
                    look_count = parse_count_token(attack_look_top_keep_one.group(1))
                    choose_count = parse_count_token(attack_look_top_keep_one.group(2))
                    if look_count is None or choose_count is None or choose_count != 1:
                        mark_unsupported(line)
                        continue
                    ability_defs.append(
                        _ability_def(
                            "Auto",
                            "AttackDeclaration",
                            effects=[_template("MoveToWaitingRoom")],
                            targets=["SelfDeckTop"],
                            conditions=cxcombo_conditions,
                            effect_optional=[True],
                            target_limit=look_count,
                        )
                    )
                    stats.parsed_lines += 1
                    stats.emitted_defs["Auto.LookTopChooseOneKeep.AttackDeclaration"] += 1
                    continue

                may_dmg = re.match(
                    rf"^you may deal ({COUNT_TOKEN_RE}) damage to your opponent\.?(?:\s*\([^)]*\))?$",
                    effect,
                    re.I,
                )
                if may_dmg:
                    amount = parse_count_token(may_dmg.group(1))
                    if amount is None:
                        mark_unsupported(line)
                        continue
                    cancelable = "cannot be canceled" not in effect.lower()
                    ability_defs.append(
                        _ability_def(
                            "Auto",
                            "AttackDeclaration",
                            effects=[
                                _template(
                                    "DealDamage",
                                    amount=amount,
                                    cancelable=cancelable,
                                )
                            ],
                            targets=[],
                            cost=cost,
                            conditions=cxcombo_conditions,
                            effect_optional=[True],
                        )
                    )
                    stats.parsed_lines += 1
                    stats.emitted_defs["Auto.DealDamage.AttackDeclaration.Optional"] += 1
                    continue

                following_grant = parse_following_ability_grant(effect)
                if following_grant is not None:
                    optional = [following_grant["optional"]] if following_grant["optional"] else []
                    grant_conditions = cxcombo_conditions
                    if following_grant.get("approx"):
                        grant_conditions = with_approx_condition(cxcombo_conditions)
                    ability_defs.append(
                        _ability_def(
                            "Auto",
                            "AttackDeclaration",
                            effects=following_grant.get("effects", [following_grant["effect"]]),
                            targets=following_grant["targets"],
                            cost=cost,
                            conditions=grant_conditions,
                            effect_optional=optional,
                            target_limit=following_grant.get("target_limit"),
                        )
                    )
                    stats.parsed_lines += 1
                    if following_grant.get("approx"):
                        stats.emitted_defs["Auto.FollowingAbilityApproxNoop.AttackDeclaration"] += 1
                    else:
                        stats.emitted_defs["Auto.FollowingAbilityFlattened.AttackDeclaration"] += 1
                    continue

                per_opp_stage_count_buff = re.match(
                    r"^this card gets \+X power until end of turn\. X is equal to the number of characters your opponent has ×(\d+)\.?$",
                    effect,
                    re.I,
                )
                if per_opp_stage_count_buff:
                    ability_defs.append(
                        _ability_def(
                            "Auto",
                            "AttackDeclaration",
                            effects=[
                                _template(
                                    "TimedConditionalAddPower",
                                    amount=int(per_opp_stage_count_buff.group(1)),
                                    duration_turn=True,
                                    turn=None,
                                    zone_count={
                                        "side": "Opponent",
                                        "zone": "Stage",
                                        "cmp": "AtLeast",
                                        "value": 0,
                                    },
                                    require_source_marker=False,
                                    per_source_marker=False,
                                    per_zone_count=True,
                                    exclude_source=False,
                                    target_ids=[],
                                )
                            ],
                            targets=["This"],
                            conditions=cxcombo_conditions,
                        )
                    )
                    stats.parsed_lines += 1
                    stats.emitted_defs["Auto.ConditionalPower.PerOpponentStageCount.Attack"] += 1
                    continue

                per_self_selector_count_buff = re.match(
                    r"^this card gets \+X power until end of turn\. X is equal to the number of (your )?(other )?(.+?) characters(?: you have)? ×(\d+)\.?$",
                    effect,
                    re.I,
                )
                if per_self_selector_count_buff:
                    selector = per_self_selector_count_buff.group(3).strip()
                    amount = int(per_self_selector_count_buff.group(4))
                    other_only = bool(per_self_selector_count_buff.group(2))
                    selector_card_ids = resolve_stage_selector_card_ids(selector)
                    if not selector_card_ids:
                        mark_unsupported(line)
                        continue
                    effects: List[Any] = [
                        _template(
                            "TimedConditionalAddPower",
                            amount=amount,
                            duration_turn=True,
                            turn=None,
                            zone_count={
                                "side": "SelfSide",
                                "zone": "Stage",
                                "cmp": "AtLeast",
                                "value": 0,
                                "card_ids": selector_card_ids,
                            },
                            require_source_marker=False,
                            per_source_marker=False,
                            per_zone_count=True,
                            exclude_source=False,
                            target_ids=[],
                        )
                    ]
                    if (
                        other_only
                        and source_card_id is not None
                        and source_card_id in selector_card_ids
                    ):
                        effects.append(
                            _template(
                                "AddPower",
                                amount=-amount,
                                duration_turn=True,
                            )
                        )
                    ability_defs.append(
                        _ability_def(
                            "Auto",
                            "AttackDeclaration",
                            effects=effects,
                            targets=["This"],
                            conditions=cxcombo_conditions,
                        )
                    )
                    stats.parsed_lines += 1
                    stats.emitted_defs["Auto.ConditionalPower.PerSelfSelectorCount.Attack"] += 1
                    continue

                per_self_selector_count_multiplied_buff = re.match(
                    r"^this card gets \+X power until end of turn\. X is equal to (\d+) multiplied by the number of (your )?(other )?(.+?) characters(?: you have)?\.?$",
                    effect,
                    re.I,
                )
                if per_self_selector_count_multiplied_buff:
                    amount = int(per_self_selector_count_multiplied_buff.group(1))
                    selector = per_self_selector_count_multiplied_buff.group(4).strip()
                    other_only = bool(per_self_selector_count_multiplied_buff.group(3))
                    selector_card_ids = resolve_stage_selector_card_ids(selector)
                    if not selector_card_ids:
                        mark_unsupported(line)
                        continue
                    effects: List[Any] = [
                        _template(
                            "TimedConditionalAddPower",
                            amount=amount,
                            duration_turn=True,
                            turn=None,
                            zone_count={
                                "side": "SelfSide",
                                "zone": "Stage",
                                "cmp": "AtLeast",
                                "value": 0,
                                "card_ids": selector_card_ids,
                            },
                            require_source_marker=False,
                            per_source_marker=False,
                            per_zone_count=True,
                            exclude_source=False,
                            target_ids=[],
                        )
                    ]
                    if (
                        other_only
                        and source_card_id is not None
                        and source_card_id in selector_card_ids
                    ):
                        effects.append(
                            _template(
                                "AddPower",
                                amount=-amount,
                                duration_turn=True,
                            )
                        )
                    ability_defs.append(
                        _ability_def(
                            "Auto",
                            "AttackDeclaration",
                            effects=effects,
                            targets=["This"],
                            conditions=cxcombo_conditions,
                        )
                    )
                    stats.parsed_lines += 1
                    stats.emitted_defs[
                        "Auto.ConditionalPower.PerSelfSelectorCount.Multiplied.Attack"
                    ] += 1
                    continue

                attack_other_char_buff = re.match(
                    rf"^choose (up to )?({COUNT_TOKEN_RE}) of your other characters, and that character gets \+([+-]?\d+) power until end of turn\.?$",
                    effect,
                    re.I,
                )
                if attack_other_char_buff:
                    choose_count = parse_count_token(attack_other_char_buff.group(2))
                    if choose_count is None:
                        mark_unsupported(line)
                        continue
                    optional = bool(attack_other_char_buff.group(1))
                    ability_defs.append(
                        _ability_def(
                            "Auto",
                            "AttackDeclaration",
                            effects=[
                                _template(
                                    "TimedConditionalAddPower",
                                    amount=int(attack_other_char_buff.group(3)),
                                    duration_turn=True,
                                    turn=None,
                                    zone_count=None,
                                    require_source_marker=False,
                                    per_source_marker=False,
                                    per_zone_count=False,
                                    exclude_source=True,
                                    target_ids=[],
                                )
                            ],
                            targets=["SelfStage"],
                            conditions=cxcombo_conditions,
                            effect_optional=[optional] if optional else [],
                            target_limit=choose_count,
                        )
                    )
                    stats.parsed_lines += 1
                    stats.emitted_defs["Auto.AddPower.OtherCharacter.AttackDeclaration"] += 1
                    continue

                attack_other_trait_buff = re.match(
                    rf"^choose (up to )?({COUNT_TOKEN_RE}) of your other 《([^》]+)》 characters, and that character gets \+([+-]?\d+) power until end of turn\.?$",
                    effect,
                    re.I,
                )
                if attack_other_trait_buff:
                    choose_count = parse_count_token(attack_other_trait_buff.group(2))
                    trait_name = attack_other_trait_buff.group(3).strip()
                    trait_id = (trait_map or {}).get(trait_name)
                    if choose_count is None or trait_id is None:
                        mark_unsupported(line)
                        continue
                    optional = bool(attack_other_trait_buff.group(1))
                    ability_defs.append(
                        _ability_def(
                            "Auto",
                            "AttackDeclaration",
                            effects=[
                                _template(
                                    "TimedConditionalAddPower",
                                    amount=int(attack_other_trait_buff.group(4)),
                                    duration_turn=True,
                                    turn=None,
                                    zone_count=None,
                                    require_source_marker=False,
                                    per_source_marker=False,
                                    per_zone_count=False,
                                    exclude_source=True,
                                    target_ids=[],
                                )
                            ],
                            targets=["SelfStage"],
                            conditions=cxcombo_conditions,
                            effect_optional=[optional] if optional else [],
                            target_trait=trait_id,
                            target_limit=choose_count,
                        )
                    )
                    stats.parsed_lines += 1
                    stats.emitted_defs["Auto.AddPower.OtherTrait.AttackDeclaration"] += 1
                    continue

                attack_other_trait_scaling = re.match(
                    rf"^choose (up to )?({COUNT_TOKEN_RE}) of your other 《([^》]+)》 characters, and that character gets \+X power until end of turn\. X is equal to the number of other 《([^》]+)》 characters you have ×(\d+)\.?$",
                    effect,
                    re.I,
                )
                if attack_other_trait_scaling:
                    choose_count = parse_count_token(attack_other_trait_scaling.group(2))
                    trait_name = attack_other_trait_scaling.group(3).strip()
                    trait_name_count = attack_other_trait_scaling.group(4).strip()
                    trait_id = (trait_map or {}).get(trait_name)
                    trait_ids = sorted(set((trait_to_ids or {}).get(trait_name, [])))
                    if (
                        choose_count is None
                        or trait_id is None
                        or not trait_ids
                        or trait_name.casefold() != trait_name_count.casefold()
                    ):
                        mark_unsupported(line)
                        continue
                    amount = int(attack_other_trait_scaling.group(5))
                    optional = bool(attack_other_trait_scaling.group(1))
                    effects: List[Any] = [
                        _template(
                            "TimedConditionalAddPower",
                            amount=amount,
                            duration_turn=True,
                            turn=None,
                            zone_count={
                                "side": "SelfSide",
                                "zone": "Stage",
                                "cmp": "AtLeast",
                                "value": 0,
                                "card_ids": trait_ids,
                            },
                            require_source_marker=False,
                            per_source_marker=False,
                            per_zone_count=True,
                            exclude_source=True,
                            target_ids=[],
                        )
                    ]
                    if source_card_id is not None and source_card_id in trait_ids:
                        effects.append(
                            _template(
                                "AddPower",
                                amount=-amount,
                                duration_turn=True,
                            )
                        )
                    ability_defs.append(
                        _ability_def(
                            "Auto",
                            "AttackDeclaration",
                            effects=effects,
                            targets=["SelfStage", "SelfStage"],
                            conditions=cxcombo_conditions,
                            effect_optional=[optional] if optional else [],
                            target_trait=trait_id,
                            target_limit=choose_count,
                        )
                    )
                    stats.parsed_lines += 1
                    stats.emitted_defs["Auto.ConditionalPower.OtherTraitCount.AttackTarget"] += 1
                    continue

                attack_if_has_other_trait_power = re.match(
                    rf"^if you have ({COUNT_TOKEN_RE}) or more other 《([^》]+)》 characters, this card gets \+([+-]?\d+) power until end of turn\.?$",
                    effect,
                    re.I,
                )
                if attack_if_has_other_trait_power:
                    min_count = parse_count_token(attack_if_has_other_trait_power.group(1))
                    trait_name = attack_if_has_other_trait_power.group(2).strip()
                    trait_ids = sorted(set((trait_to_ids or {}).get(trait_name, [])))
                    if min_count is None or not trait_ids:
                        mark_unsupported(line)
                        continue
                    threshold = min_count + (
                        1 if source_card_id is not None and source_card_id in trait_ids else 0
                    )
                    ability_defs.append(
                        _ability_def(
                            "Auto",
                            "AttackDeclaration",
                            effects=[
                                _template(
                                    "TimedConditionalAddPower",
                                    amount=int(attack_if_has_other_trait_power.group(3)),
                                    duration_turn=True,
                                    turn=None,
                                    zone_count={
                                        "side": "SelfSide",
                                        "zone": "Stage",
                                        "cmp": "AtLeast",
                                        "value": threshold,
                                        "card_ids": trait_ids,
                                    },
                                    require_source_marker=False,
                                    per_source_marker=False,
                                    per_zone_count=False,
                                    exclude_source=False,
                                    target_ids=[],
                                )
                            ],
                            targets=["This"],
                            conditions=cxcombo_conditions,
                        )
                    )
                    stats.parsed_lines += 1
                    stats.emitted_defs["Auto.ConditionalPower.IfHasOtherTrait.Attack"] += 1
                    continue

                if "may" in effect.lower():
                    mark_unsupported(line)
                    continue
                dmg = re.match(
                    r"^(?:you )?deal (\d+) damage to your opponent\.?$",
                    effect,
                    re.I,
                )
                if dmg:
                    cancelable = "cannot be canceled" not in effect.lower()
                    abilities.append(
                        _template(
                            "AutoOnAttackDealDamage",
                            amount=int(dmg.group(1)),
                            cancelable=cancelable,
                        )
                    )
                    stats.parsed_lines += 1
                    stats.emitted_templates["AutoOnAttackDealDamage"] += 1
                    continue

                buff = re.match(
                    r"^choose 1 of your characters, and that character gets \+(\d+) power until end of turn\.?$",
                    effect,
                    re.I,
                )
                if buff:
                    ability_defs.append(
                        _ability_def(
                            "Auto",
                            "AttackDeclaration",
                            effects=[
                                _template("AddPower", amount=int(buff.group(1)), duration_turn=True)
                            ],
                            targets=["SelfStage"],
                            conditions=cxcombo_conditions,
                        )
                    )
                    stats.parsed_lines += 1
                    stats.emitted_defs["Auto.AddPower.AttackDeclaration"] += 1
                    continue

            match = re.match(
                r"^When this card's level (\d+) or higher battle opponent becomes 【REVERSE】, you may put the top card of your deck into your stock\.?$",
                remainder,
                re.I,
            )
            if match:
                ability_defs.append(
                    _ability_def(
                        "Auto",
                        "BattleOpponentReverse",
                        effects=[
                            _template(
                                "BattleOpponentTopDeckToStockIf",
                                min_level=int(match.group(1)),
                            )
                        ],
                        targets=[],
                        effect_optional=[True],
                    )
                )
                stats.parsed_lines += 1
                stats.emitted_defs[
                    "Auto.StockTop.IfBattleOpponentMinLevel.BattleOpponentReverse"
                ] += 1
                continue

            match = re.match(
                r"^When this card's battle opponent becomes 【REVERSE】, (.+)$",
                remainder,
                re.I,
            )
            if match:
                effect = match.group(1).strip()
                effect, cxcombo_conditions = strip_cxcombo_condition_prefix(effect, has_cxcombo_tag)

                stock_if_climax = re.match(
                    r"^if there is a climax in your climax area, you may put the top card of your deck into your stock\.?$",
                    effect,
                    re.I,
                )
                if stock_if_climax:
                    ability_defs.append(
                        _ability_def(
                            "Auto",
                            "BattleOpponentReverse",
                            effects=[_template("StockCharge", count=1)],
                            targets=[],
                            conditions={
                                "climax_area": {"side": "SelfSide", "card_ids": []},
                                **(cxcombo_conditions or {}),
                            },
                            effect_optional=[True],
                        )
                    )
                    stats.parsed_lines += 1
                    stats.emitted_defs["Auto.StockCharge.IfClimaxArea.BattleOpponentReverse"] += 1
                    continue

                move_clock_any = re.match(
                    r"^if \"[^\"]+\" is in your climax area, you may put that character into your opponent's clock\.?$",
                    effect,
                    re.I,
                ) or re.match(
                    r"^you may put that character into your opponent's clock\.?$",
                    effect,
                    re.I,
                )
                if move_clock_any:
                    ability_defs.append(
                        _ability_def(
                            "Auto",
                            "BattleOpponentReverse",
                            effects=[
                                _template(
                                    "BattleOpponentMoveToClockIf",
                                    max_level=None,
                                    max_cost=None,
                                    level_gt_opponent_level=False,
                                )
                            ],
                            targets=[],
                            conditions=cxcombo_conditions,
                        )
                    )
                    stats.parsed_lines += 1
                    stats.emitted_defs["Auto.ClockBattleOpponent.Any.BattleOpponentReverse"] += 1
                    continue

                salvage_if_named_climax = re.match(
                    rf'^if "([^"]+)" is in your climax area, you may choose (?:up to )?({COUNT_TOKEN_RE}) ([a-z ]+?) in your waiting room, and return (?:it|them) to your hand\.?$',
                    effect,
                    re.I,
                )
                if salvage_if_named_climax:
                    choose_count = parse_count_token(salvage_if_named_climax.group(2))
                    if choose_count is None:
                        mark_unsupported(line)
                        continue
                    type_text = salvage_if_named_climax.group(3).strip().lower()
                    card_type_hint = None
                    if "character" in type_text:
                        card_type_hint = "Character"
                    elif "event" in type_text:
                        card_type_hint = "Event"
                    elif "climax" in type_text:
                        card_type_hint = "Climax"
                    named = salvage_if_named_climax.group(1).strip()
                    named_ids = sorted(name_to_ids.get(named, [])) if name_to_ids else []
                    ability_defs.append(
                        _ability_def(
                            "Auto",
                            "BattleOpponentReverse",
                            effects=[_template("MoveToHand")],
                            targets=["SelfWaitingRoom"],
                            target_limit=choose_count,
                            target_card_type=card_type_hint,
                            conditions={
                                "climax_area": {"side": "SelfSide", "card_ids": named_ids},
                                **(cxcombo_conditions or {}),
                            },
                            effect_optional=[True],
                        )
                    )
                    stats.parsed_lines += 1
                    stats.emitted_defs["Auto.Salvage.IfNamedClimax.BattleOpponentReverse"] += 1
                    continue

                salvage_if_existing_conditions = re.match(
                    rf"^you may choose (?:up to )?({COUNT_TOKEN_RE}) ([a-z ]+?) in your waiting room, and return (?:it|them) to your hand\.?$",
                    effect,
                    re.I,
                )
                if salvage_if_existing_conditions and cxcombo_conditions is not None:
                    choose_count = parse_count_token(salvage_if_existing_conditions.group(1))
                    if choose_count is None:
                        mark_unsupported(line)
                        continue
                    type_text = salvage_if_existing_conditions.group(2).strip().lower()
                    card_type_hint = None
                    if "character" in type_text:
                        card_type_hint = "Character"
                    elif "event" in type_text:
                        card_type_hint = "Event"
                    elif "climax" in type_text:
                        card_type_hint = "Climax"
                    ability_defs.append(
                        _ability_def(
                            "Auto",
                            "BattleOpponentReverse",
                            effects=[_template("MoveToHand")],
                            targets=["SelfWaitingRoom"],
                            target_limit=choose_count,
                            target_card_type=card_type_hint,
                            conditions=cxcombo_conditions,
                            effect_optional=[True],
                        )
                    )
                    stats.parsed_lines += 1
                    stats.emitted_defs["Auto.Salvage.IfClimaxCondition.BattleOpponentReverse"] += 1
                    continue

                clock_if_named_climax = re.match(
                    r'^if "([^"]+)" is in your climax area, you may put that character into your opponent\'s clock\.?$',
                    effect,
                    re.I,
                )
                if clock_if_named_climax:
                    named = clock_if_named_climax.group(1).strip()
                    named_ids = sorted(name_to_ids.get(named, [])) if name_to_ids else []
                    ability_defs.append(
                        _ability_def(
                            "Auto",
                            "BattleOpponentReverse",
                            effects=[
                                _template(
                                    "BattleOpponentMoveToClockIf",
                                    max_level=None,
                                    max_cost=None,
                                    level_gt_opponent_level=False,
                                )
                            ],
                            targets=[],
                            conditions={
                                "climax_area": {"side": "SelfSide", "card_ids": named_ids},
                                **(cxcombo_conditions or {}),
                            },
                            effect_optional=[True],
                        )
                    )
                    stats.parsed_lines += 1
                    stats.emitted_defs[
                        "Auto.BattleOpponentMoveToClock.IfNamedClimax.BattleOpponentReverse"
                    ] += 1
                    continue

                move_to_memory = re.match(
                    r"^you may put that character into (?:your opponent's|their) memory\.?$",
                    effect,
                    re.I,
                )
                if move_to_memory:
                    ability_defs.append(
                        _ability_def(
                            "Auto",
                            "BattleOpponentReverse",
                            effects=[
                                _template(
                                    "BattleOpponentMoveToMemoryIf",
                                    max_level=None,
                                    max_cost=None,
                                    level_gt_opponent_level=False,
                                )
                            ],
                            targets=[],
                            conditions=cxcombo_conditions,
                            effect_optional=[True],
                        )
                    )
                    stats.parsed_lines += 1
                    stats.emitted_defs["Auto.BattleOpponentMoveToMemory.BattleOpponentReverse"] += 1
                    continue

                stock_swap = re.match(
                    r"^you may put that character into your opponent's stock\. If you do, put (?:the bottom card|a card from the bottom) of your opponent's stock into (?:their|his or her) waiting room\.?$",
                    effect,
                    re.I,
                )
                if stock_swap:
                    ability_defs.append(
                        _ability_def(
                            "Auto",
                            "BattleOpponentReverse",
                            effects=[
                                _template(
                                    "BattleOpponentMoveToStockThenBottomStockToWaitingRoomIf",
                                    max_level=None,
                                    max_cost=None,
                                    level_gt_opponent_level=False,
                                )
                            ],
                            targets=[],
                            conditions=cxcombo_conditions,
                            effect_optional=[True],
                        )
                    )
                    stats.parsed_lines += 1
                    stats.emitted_defs["Auto.StockSwap.BattleOpponentReverse"] += 1
                    continue

                marker_from_top = re.match(
                    r"^you may put the top card of your deck underneath this card as a marker\.?$",
                    effect,
                    re.I,
                )
                if marker_from_top:
                    ability_defs.append(
                        _ability_def(
                            "Auto",
                            "BattleOpponentReverse",
                            effects=[_template("MoveTopDeckToMarker")],
                            targets=[],
                            conditions=cxcombo_conditions,
                            effect_optional=[True],
                        )
                    )
                    stats.parsed_lines += 1
                    stats.emitted_defs["Auto.MoveTopDeckToMarker.BattleOpponentReverse"] += 1
                    continue

                marker_after_look = re.match(
                    r"^you may look at the top card of your deck\. If you do, put that card face down underneath this card as a marker\.?$",
                    effect,
                    re.I,
                )
                if marker_after_look:
                    ability_defs.append(
                        _ability_def(
                            "Auto",
                            "BattleOpponentReverse",
                            effects=[_template("MoveTopDeckToMarker")],
                            targets=[],
                            conditions=cxcombo_conditions,
                            effect_optional=[True],
                        )
                    )
                    stats.parsed_lines += 1
                    stats.emitted_defs["Auto.LookTopThenMoveToMarker.BattleOpponentReverse"] += 1
                    continue

            paid_self_memory_on_reverse_inline = re.match(
                r"^When this card becomes 【REVERSE】 in battle, you may pay the cost\. If you do, put this card into your memory\.?$",
                remainder,
                re.I,
            )
            if paid_self_memory_on_reverse_inline:
                if cost_is_empty(cost):
                    mark_unsupported(line)
                    continue
                ability_defs.append(
                    _ability_def(
                        "Auto",
                        "OnReverse",
                        effects=[_template("MoveToMemory")],
                        targets=["This"],
                        cost=cost,
                        effect_optional=[True],
                    )
                )
                stats.parsed_lines += 1
                stats.emitted_defs["Auto.MoveSelfToMemory.OnReverse.Paid"] += 1
                continue

            match = re.match(
                r"^When this card becomes 【REVERSE】(?: in battle)?, (.+)$",
                remainder,
                re.I,
            )
            if match:
                effect = match.group(1).strip()
                effect, cxcombo_conditions = strip_cxcombo_condition_prefix(effect, has_cxcombo_tag)

                handled_reverse_rule = False
                for rule in AUTO_RULES:
                    if rule.id != "Auto.SelfBottomDeck.OnReverse":
                        continue
                    auto_match = rule.pattern.match(effect)
                    if not auto_match:
                        continue
                    if not rule_enabled(rule):
                        continue
                    ability_defs.append(
                        _ability_def(
                            "Auto",
                            "OnReverse",
                            effects=[_template("MoveToDeckBottom")],
                            targets=["This"],
                            conditions=(
                                with_approx_condition(cxcombo_conditions)
                                if rule.mode == RULE_MODE_APPROX
                                else cxcombo_conditions
                            ),
                        )
                    )
                    stats.parsed_lines += 1
                    stats.emitted_defs[rule.id] += 1
                    handled_reverse_rule = True
                    break
                if handled_reverse_rule:
                    continue

                reverse_clock_and_rest = re.match(
                    r"^put the top card of your deck into your clock, and 【REST】 this card\.?$",
                    effect,
                    re.I,
                )
                if reverse_clock_and_rest:
                    ability_defs.append(
                        _ability_def(
                            "Auto",
                            "OnReverse",
                            effects=[_template("MoveToClock"), _template("RestTarget")],
                            targets=["SelfDeckTop", "This"],
                            conditions=cxcombo_conditions,
                            target_limit=1,
                        )
                    )
                    stats.parsed_lines += 1
                    stats.emitted_defs["Auto.MoveTopDeckToClockAndRestSelf.OnReverse"] += 1
                    continue

                lock_auto_encore_self = re.match(
                    r'^you cannot use "【AUTO】 Encore" until end of turn\. \(.*rule either\)\.?$',
                    effect,
                    re.I,
                )
                if lock_auto_encore_self:
                    ability_defs.append(
                        _ability_def(
                            "Auto",
                            "OnReverse",
                            effects=[_template("CannotUseAutoEncoreForPlayer", target="SelfSide")],
                            targets=[],
                            conditions=cxcombo_conditions,
                        )
                    )
                    stats.parsed_lines += 1
                    stats.emitted_defs["Auto.CannotUseAutoEncore.Self.OnReverse"] += 1
                    continue

                reveal_gate_return = re.match(
                    r"^reveal the top card of your deck\. If that card is level (\d+) or higher, you may return this card to your hand\. \((?:Climax are regarded as level 0\. )?Return the revealed card to (?:its original place|the original place)\)\.?$",
                    effect,
                    re.I,
                )
                if reveal_gate_return:
                    ability_defs.append(
                        _ability_def(
                            "Auto",
                            "OnReverse",
                            effects=[
                                _template(
                                    "RevealTopIfLevelAtLeastMoveThisToHand",
                                    min_level=int(reveal_gate_return.group(1)),
                                )
                            ],
                            targets=[],
                            conditions=cxcombo_conditions,
                            effect_optional=[True],
                        )
                    )
                    stats.parsed_lines += 1
                    stats.emitted_defs["Auto.RevealTopGate.MoveSelfToHand.OnReverse"] += 1
                    continue

                reveal_gate_rest = re.match(
                    r"^reveal the top card of your deck\. If that card is level (\d+) or higher, you may 【REST】 this card\. \((?:Climax are regarded as level 0\. )?Return the revealed card to (?:its original place|the original place)\)\.?$",
                    effect,
                    re.I,
                )
                if reveal_gate_rest:
                    ability_defs.append(
                        _ability_def(
                            "Auto",
                            "OnReverse",
                            effects=[
                                _template(
                                    "RevealTopIfLevelAtLeastRestThis",
                                    min_level=int(reveal_gate_rest.group(1)),
                                )
                            ],
                            targets=[],
                            conditions=cxcombo_conditions,
                            effect_optional=[True],
                        )
                    )
                    stats.parsed_lines += 1
                    stats.emitted_defs["Auto.RevealTopGate.RestSelf.OnReverse"] += 1
                    continue

                self_memory_with_gate = re.match(
                    r"^if your memory has (\d+) or less cards, you may put this card into your memory\.?$",
                    effect,
                    re.I,
                )
                if self_memory_with_gate:
                    merged_conditions = dict(cxcombo_conditions or {})
                    merged_conditions["self_memory_at_most"] = int(self_memory_with_gate.group(1))
                    ability_defs.append(
                        _ability_def(
                            "Auto",
                            "OnReverse",
                            effects=[_template("MoveToMemory")],
                            targets=["This"],
                            conditions=merged_conditions,
                            effect_optional=[True],
                        )
                    )
                    stats.parsed_lines += 1
                    stats.emitted_defs["Auto.MoveSelfToMemory.IfMemoryAtMost.OnReverse"] += 1
                    continue

                self_memory = re.match(
                    r"^(you may )?put this card into your memory\.?$",
                    effect,
                    re.I,
                )
                if self_memory:
                    ability_defs.append(
                        _ability_def(
                            "Auto",
                            "OnReverse",
                            effects=[_template("MoveToMemory")],
                            targets=["This"],
                            conditions=cxcombo_conditions,
                            effect_optional=[bool(self_memory.group(1))],
                        )
                    )
                    stats.parsed_lines += 1
                    stats.emitted_defs["Auto.MoveSelfToMemory.OnReverse"] += 1
                    continue

                paid_self_memory = re.match(
                    r"^you may pay the cost\. If you do, put this card into your memory\.?$",
                    effect,
                    re.I,
                )
                if paid_self_memory:
                    if cost_is_empty(cost):
                        mark_unsupported(line)
                        continue
                    ability_defs.append(
                        _ability_def(
                            "Auto",
                            "OnReverse",
                            effects=[_template("MoveToMemory")],
                            targets=["This"],
                            cost=cost,
                            conditions=cxcombo_conditions,
                            effect_optional=[True],
                        )
                    )
                    stats.parsed_lines += 1
                    stats.emitted_defs["Auto.MoveSelfToMemory.OnReverse.Paid"] += 1
                    continue

                paid_reverse_burn = re.match(
                    rf"^you may pay the cost\. If you do, deal ({COUNT_TOKEN_RE}) damage to your opponent\.?(?:\s*\([^)]*\))?$",
                    effect,
                    re.I,
                )
                if paid_reverse_burn:
                    amount = parse_count_token(paid_reverse_burn.group(1))
                    if amount is None or cost_is_empty(cost):
                        mark_unsupported(line)
                        continue
                    ability_defs.append(
                        _ability_def(
                            "Auto",
                            "OnReverse",
                            effects=[
                                _template(
                                    "DealDamage",
                                    amount=amount,
                                    cancelable="cannot be canceled" not in effect.lower(),
                                )
                            ],
                            targets=[],
                            cost=cost,
                            conditions=cxcombo_conditions,
                            effect_optional=[True],
                        )
                    )
                    stats.parsed_lines += 1
                    stats.emitted_defs["Auto.DealDamage.OnReverse.Paid"] += 1
                    continue

                match = re.match(
                    r"^if this card's battle opponent is level (\d+) or lower, you may 【REVERSE】 that character\.?$",
                    effect,
                    re.I,
                )
                if match:
                    ability_defs.append(
                        _ability_def(
                            "Auto",
                            "OnReverse",
                            effects=[
                                _template(
                                    "BattleOpponentReverseIf",
                                    max_level=int(match.group(1)),
                                    max_cost=None,
                                    level_gt_opponent_level=False,
                                )
                            ],
                            targets=[],
                            conditions=cxcombo_conditions,
                        )
                    )
                    stats.parsed_lines += 1
                    stats.emitted_defs["Auto.ReverseBattleOpponent.Level.OnReverse"] += 1
                    continue

                match = re.match(
                    r"^if this card's battle opponent is cost (\d+) or lower, you may 【REVERSE】 that character\.?$",
                    effect,
                    re.I,
                )
                if match:
                    ability_defs.append(
                        _ability_def(
                            "Auto",
                            "OnReverse",
                            effects=[
                                _template(
                                    "BattleOpponentReverseIf",
                                    max_level=None,
                                    max_cost=int(match.group(1)),
                                    level_gt_opponent_level=False,
                                )
                            ],
                            targets=[],
                            conditions=cxcombo_conditions,
                        )
                    )
                    stats.parsed_lines += 1
                    stats.emitted_defs["Auto.ReverseBattleOpponent.Cost.OnReverse"] += 1
                    continue

                match = re.match(
                    r"^if the level of this card's battle opponent is higher than your opponent's level, you may 【REVERSE】 that character\.?$",
                    effect,
                    re.I,
                )
                if match:
                    ability_defs.append(
                        _ability_def(
                            "Auto",
                            "OnReverse",
                            effects=[
                                _template(
                                    "BattleOpponentReverseIf",
                                    max_level=None,
                                    max_cost=None,
                                    level_gt_opponent_level=True,
                                )
                            ],
                            targets=[],
                            conditions=cxcombo_conditions,
                        )
                    )
                    stats.parsed_lines += 1
                    stats.emitted_defs["Auto.ReverseBattleOpponent.OpponentLevel.OnReverse"] += 1
                    continue

                match = re.match(
                    r"^if this card's battle opponent is level (\d+) or lower, you may put that character at the bottom of your opponent's deck\.?$",
                    effect,
                    re.I,
                )
                if match:
                    ability_defs.append(
                        _ability_def(
                            "Auto",
                            "OnReverse",
                            effects=[
                                _template(
                                    "BattleOpponentMoveToDeckBottomIf",
                                    max_level=int(match.group(1)),
                                    max_cost=None,
                                    level_gt_opponent_level=False,
                                )
                            ],
                            targets=[],
                            conditions=cxcombo_conditions,
                        )
                    )
                    stats.parsed_lines += 1
                    stats.emitted_defs["Auto.BottomDeckBattleOpponent.Level.OnReverse"] += 1
                    continue

                match = re.match(
                    r"^if this card's battle opponent is cost (\d+) or lower, you may put that character at the bottom of your opponent's deck\.?$",
                    effect,
                    re.I,
                )
                if match:
                    ability_defs.append(
                        _ability_def(
                            "Auto",
                            "OnReverse",
                            effects=[
                                _template(
                                    "BattleOpponentMoveToDeckBottomIf",
                                    max_level=None,
                                    max_cost=int(match.group(1)),
                                    level_gt_opponent_level=False,
                                )
                            ],
                            targets=[],
                            conditions=cxcombo_conditions,
                        )
                    )
                    stats.parsed_lines += 1
                    stats.emitted_defs["Auto.BottomDeckBattleOpponent.Cost.OnReverse"] += 1
                    continue

                match = re.match(
                    r"^if the level of this card's battle opponent is higher than your opponent's level, you may put that character at the bottom of your opponent's deck\.?$",
                    effect,
                    re.I,
                )
                if match:
                    ability_defs.append(
                        _ability_def(
                            "Auto",
                            "OnReverse",
                            effects=[
                                _template(
                                    "BattleOpponentMoveToDeckBottomIf",
                                    max_level=None,
                                    max_cost=None,
                                    level_gt_opponent_level=True,
                                )
                            ],
                            targets=[],
                            conditions=cxcombo_conditions,
                        )
                    )
                    stats.parsed_lines += 1
                    stats.emitted_defs["Auto.BottomDeckBattleOpponent.OpponentLevel.OnReverse"] += 1
                    continue

                match = re.match(
                    r"^if this card's battle opponent is level (\d+) or lower, you may put that character into your opponent's memory\.?$",
                    effect,
                    re.I,
                )
                if match:
                    ability_defs.append(
                        _ability_def(
                            "Auto",
                            "OnReverse",
                            effects=[
                                _template(
                                    "BattleOpponentMoveToMemoryIf",
                                    max_level=int(match.group(1)),
                                    max_cost=None,
                                    level_gt_opponent_level=False,
                                )
                            ],
                            targets=[],
                            conditions=cxcombo_conditions,
                        )
                    )
                    stats.parsed_lines += 1
                    stats.emitted_defs["Auto.MemoryBattleOpponent.Level.OnReverse"] += 1
                    continue

                match = re.match(
                    r"^if this card's battle opponent is cost (\d+) or lower, you may put that character into your opponent's memory\.?$",
                    effect,
                    re.I,
                )
                if match:
                    ability_defs.append(
                        _ability_def(
                            "Auto",
                            "OnReverse",
                            effects=[
                                _template(
                                    "BattleOpponentMoveToMemoryIf",
                                    max_level=None,
                                    max_cost=int(match.group(1)),
                                    level_gt_opponent_level=False,
                                )
                            ],
                            targets=[],
                            conditions=cxcombo_conditions,
                        )
                    )
                    stats.parsed_lines += 1
                    stats.emitted_defs["Auto.MemoryBattleOpponent.Cost.OnReverse"] += 1
                    continue

                match = re.match(
                    r"^if the level of this card's battle opponent is higher than your opponent's level, you may put that character into your opponent's memory\.?$",
                    effect,
                    re.I,
                )
                if match:
                    ability_defs.append(
                        _ability_def(
                            "Auto",
                            "OnReverse",
                            effects=[
                                _template(
                                    "BattleOpponentMoveToMemoryIf",
                                    max_level=None,
                                    max_cost=None,
                                    level_gt_opponent_level=True,
                                )
                            ],
                            targets=[],
                            conditions=cxcombo_conditions,
                        )
                    )
                    stats.parsed_lines += 1
                    stats.emitted_defs["Auto.MemoryBattleOpponent.OpponentLevel.OnReverse"] += 1
                    continue

                match = re.match(
                    r"^you may put that character into your opponent's clock\.?$",
                    effect,
                    re.I,
                )
                if match:
                    ability_defs.append(
                        _ability_def(
                            "Auto",
                            "OnReverse",
                            effects=[
                                _template(
                                    "BattleOpponentMoveToClockIf",
                                    max_level=None,
                                    max_cost=None,
                                    level_gt_opponent_level=False,
                                )
                            ],
                            targets=[],
                            conditions=cxcombo_conditions,
                        )
                    )
                    stats.parsed_lines += 1
                    stats.emitted_defs["Auto.ClockBattleOpponent.Any.OnReverse"] += 1
                    continue

                match = re.match(
                    r"^if this card's battle opponent is level (\d+) or lower, you may put that character into your opponent's clock\.?$",
                    effect,
                    re.I,
                )
                if match:
                    ability_defs.append(
                        _ability_def(
                            "Auto",
                            "OnReverse",
                            effects=[
                                _template(
                                    "BattleOpponentMoveToClockIf",
                                    max_level=int(match.group(1)),
                                    max_cost=None,
                                    level_gt_opponent_level=False,
                                )
                            ],
                            targets=[],
                            conditions=cxcombo_conditions,
                        )
                    )
                    stats.parsed_lines += 1
                    stats.emitted_defs["Auto.ClockBattleOpponent.Level.OnReverse"] += 1
                    continue

                match = re.match(
                    r"^if this card's battle opponent is cost (\d+) or lower, you may put that character into your opponent's clock\.?$",
                    effect,
                    re.I,
                )
                if match:
                    ability_defs.append(
                        _ability_def(
                            "Auto",
                            "OnReverse",
                            effects=[
                                _template(
                                    "BattleOpponentMoveToClockIf",
                                    max_level=None,
                                    max_cost=int(match.group(1)),
                                    level_gt_opponent_level=False,
                                )
                            ],
                            targets=[],
                            conditions=cxcombo_conditions,
                        )
                    )
                    stats.parsed_lines += 1
                    stats.emitted_defs["Auto.ClockBattleOpponent.Cost.OnReverse"] += 1
                    continue

                match = re.match(
                    r"^if the level of this card's battle opponent is higher than your opponent's level, you may put that character into your opponent's clock\.?$",
                    effect,
                    re.I,
                )
                if match:
                    ability_defs.append(
                        _ability_def(
                            "Auto",
                            "OnReverse",
                            effects=[
                                _template(
                                    "BattleOpponentMoveToClockIf",
                                    max_level=None,
                                    max_cost=None,
                                    level_gt_opponent_level=True,
                                )
                            ],
                            targets=[],
                            conditions=cxcombo_conditions,
                        )
                    )
                    stats.parsed_lines += 1
                    stats.emitted_defs["Auto.ClockBattleOpponent.OpponentLevel.OnReverse"] += 1
                    continue

                match = re.match(
                    r"^if this card's battle opponent is level (\d+) or lower, you may put that character into your opponent's stock\. If you do, put (?:the bottom card|a card from the bottom) of your opponent's stock into (?:their|his or her) waiting room\.?$",
                    effect,
                    re.I,
                )
                if match:
                    ability_defs.append(
                        _ability_def(
                            "Auto",
                            "OnReverse",
                            effects=[
                                _template(
                                    "BattleOpponentMoveToStockThenBottomStockToWaitingRoomIf",
                                    max_level=int(match.group(1)),
                                    max_cost=None,
                                    level_gt_opponent_level=False,
                                )
                            ],
                            targets=[],
                            conditions=cxcombo_conditions,
                        )
                    )
                    stats.parsed_lines += 1
                    stats.emitted_defs["Auto.StockSwapBattleOpponent.Level.OnReverse"] += 1
                    continue

                match = re.match(
                    r"^if this card's battle opponent is cost (\d+) or lower, you may put that character into your opponent's stock\. If you do, put (?:the bottom card|a card from the bottom) of your opponent's stock into (?:their|his or her) waiting room\.?$",
                    effect,
                    re.I,
                )
                if match:
                    ability_defs.append(
                        _ability_def(
                            "Auto",
                            "OnReverse",
                            effects=[
                                _template(
                                    "BattleOpponentMoveToStockThenBottomStockToWaitingRoomIf",
                                    max_level=None,
                                    max_cost=int(match.group(1)),
                                    level_gt_opponent_level=False,
                                )
                            ],
                            targets=[],
                            conditions=cxcombo_conditions,
                        )
                    )
                    stats.parsed_lines += 1
                    stats.emitted_defs["Auto.StockSwapBattleOpponent.Cost.OnReverse"] += 1
                    continue

                match = re.match(
                    r"^if the level of this card's battle opponent is higher than your opponent's level, you may put that character into your opponent's stock\. If you do, put (?:the bottom card|a card from the bottom) of your opponent's stock into (?:their|his or her) waiting room\.?$",
                    effect,
                    re.I,
                )
                if match:
                    ability_defs.append(
                        _ability_def(
                            "Auto",
                            "OnReverse",
                            effects=[
                                _template(
                                    "BattleOpponentMoveToStockThenBottomStockToWaitingRoomIf",
                                    max_level=None,
                                    max_cost=None,
                                    level_gt_opponent_level=True,
                                )
                            ],
                            targets=[],
                            conditions=cxcombo_conditions,
                        )
                    )
                    stats.parsed_lines += 1
                    stats.emitted_defs["Auto.StockSwapBattleOpponent.OpponentLevel.OnReverse"] += 1
                    continue

                match = re.match(
                    r"^if this card's battle opponent is level (\d+) or lower, you may put the top card of your opponent's clock into (?:their|his or her) waiting room\. If you do, put that character into your opponent's clock\.?$",
                    effect,
                    re.I,
                )
                if match:
                    ability_defs.append(
                        _ability_def(
                            "Auto",
                            "OnReverse",
                            effects=[
                                _template(
                                    "BattleOpponentMoveToClockAfterClockTopToWaitingRoomIf",
                                    max_level=int(match.group(1)),
                                    max_cost=None,
                                    level_gt_opponent_level=False,
                                )
                            ],
                            targets=[],
                            conditions=cxcombo_conditions,
                        )
                    )
                    stats.parsed_lines += 1
                    stats.emitted_defs["Auto.ClockSwapBattleOpponent.Level.OnReverse"] += 1
                    continue

                match = re.match(
                    r"^if this card's battle opponent is cost (\d+) or lower, you may put the top card of your opponent's clock into (?:their|his or her) waiting room\. If you do, put that character into your opponent's clock\.?$",
                    effect,
                    re.I,
                )
                if match:
                    ability_defs.append(
                        _ability_def(
                            "Auto",
                            "OnReverse",
                            effects=[
                                _template(
                                    "BattleOpponentMoveToClockAfterClockTopToWaitingRoomIf",
                                    max_level=None,
                                    max_cost=int(match.group(1)),
                                    level_gt_opponent_level=False,
                                )
                            ],
                            targets=[],
                            conditions=cxcombo_conditions,
                        )
                    )
                    stats.parsed_lines += 1
                    stats.emitted_defs["Auto.ClockSwapBattleOpponent.Cost.OnReverse"] += 1
                    continue

                match = re.match(
                    r"^if the level of this card's battle opponent is higher than your opponent's level, you may put the top card of your opponent's clock into (?:their|his or her) waiting room\. If you do, put that character into your opponent's clock\.?$",
                    effect,
                    re.I,
                )
                if match:
                    ability_defs.append(
                        _ability_def(
                            "Auto",
                            "OnReverse",
                            effects=[
                                _template(
                                    "BattleOpponentMoveToClockAfterClockTopToWaitingRoomIf",
                                    max_level=None,
                                    max_cost=None,
                                    level_gt_opponent_level=True,
                                )
                            ],
                            targets=[],
                            conditions=cxcombo_conditions,
                        )
                    )
                    stats.parsed_lines += 1
                    stats.emitted_defs["Auto.ClockSwapBattleOpponent.OpponentLevel.OnReverse"] += 1
                    continue

                if "may" in effect.lower() and "choose" not in effect.lower():
                    mark_unsupported(line)
                    continue
                draw = re.match(r"^draw (\d+) card(?:s)?\.?$", effect, re.I)
                if draw and "may" not in effect.lower() and "up to" not in effect.lower():
                    abilities.append(_template("AutoOnReverseDraw", count=int(draw.group(1))))
                    stats.parsed_lines += 1
                    stats.emitted_templates["AutoOnReverseDraw"] += 1
                    continue

                salv = re.match(
                    r"^(you may )?choose (up to )?(\d+) ([a-z ]+?) in your waiting room, and return (?:it|them) to your hand\.?$",
                    effect,
                    re.I,
                )
                if salv:
                    optional = bool(salv.group(1)) or bool(salv.group(2))
                    count = int(salv.group(3))
                    type_text = salv.group(4).strip().lower()
                    card_type_hint = None
                    if "character" in type_text:
                        card_type_hint = "Character"
                    elif "climax" in type_text:
                        card_type_hint = "Climax"
                    elif "event" in type_text:
                        card_type_hint = "Event"
                    abilities.append(
                        _template(
                            "AutoOnReverseSalvage",
                            count=count,
                            optional=optional,
                            card_type=card_type_hint,
                        )
                    )
                    stats.parsed_lines += 1
                    stats.emitted_templates["AutoOnReverseSalvage"] += 1
                    continue

            match = re.match(
                r"^At the (?:beginning of )?your end phase, draw (\d+) card(?:s)?\.?$",
                remainder,
                re.I,
            )
            if match and "may" not in remainder.lower():
                abilities.append(_template("AutoEndPhaseDraw", count=int(match.group(1))))
                stats.parsed_lines += 1
                stats.emitted_templates["AutoEndPhaseDraw"] += 1
                continue

            level_up_self_to_waiting = re.match(
                r"^When you level up, put this card into your waiting room\.?$",
                remainder,
                re.I,
            )
            if level_up_self_to_waiting:
                if not cost_is_empty(cost):
                    mark_unsupported(line)
                    continue
                ability_defs.append(
                    _ability_def(
                        "Auto",
                        "LevelUp",
                        effects=[_template("MoveToWaitingRoom")],
                        targets=["This"],
                    )
                )
                stats.parsed_lines += 1
                stats.emitted_defs["Auto.MoveSelfToWaitingRoom.OnLevelUp"] += 1
                continue

            look_reorder_auto_exact = re.match(
                rf"^When .+, look at up to ({COUNT_TOKEN_RE}) cards? from the top of your deck, and put them on the top of your deck in any order\.?$",
                remainder,
                re.I,
            )
            if look_reorder_auto_exact:
                count = parse_count_token(look_reorder_auto_exact.group(1))
                if count is None:
                    mark_unsupported(line)
                    continue
                timing: Optional[str] = None
                conditions: Optional[Dict[str, Any]] = None
                if re.match(r"^When this card attacks,", remainder, re.I):
                    timing = "AttackDeclaration"
                elif re.match(
                    r"^When this card's battle opponent becomes 【REVERSE】,",
                    remainder,
                    re.I,
                ):
                    timing = "BattleOpponentReverse"
                elif re.match(
                    r"^When (?:your|a) climax is placed on your climax area,",
                    remainder,
                    re.I,
                ):
                    timing = "AfterClimaxPhase"
                    conditions = {"climax_area": {"side": "SelfSide", "card_ids": []}}
                elif re.match(
                    r"^When this card is placed on (?:the )?stage from your hand,",
                    remainder,
                    re.I,
                ):
                    timing = "OnPlay"
                if timing is not None:
                    ability_defs.append(
                        _ability_def(
                            "Auto",
                            timing,
                            effects=[_template("LookTopDeckReorder", count=count)],
                            targets=["SelfDeckTop"],
                            conditions=conditions,
                            target_limit=count,
                        )
                    )
                    stats.parsed_lines += 1
                    stats.emitted_defs["Auto.LookTopReorder.Exact"] += 1
                    continue

            begin_main_look_top_decision_exact = re.match(
                r"^At the beginning of your main phase, look at the top card of your deck, and put it on the top of your deck or into your waiting room\.?$",
                remainder,
                re.I,
            )
            if begin_main_look_top_decision_exact:
                ability_defs.append(
                    _ability_def(
                        "Auto",
                        "BeginMainPhase",
                        effects=[_template("LookTopCardTopOrWaitingRoom")],
                        targets=["SelfDeckTop"],
                        target_limit=1,
                    )
                )
                stats.parsed_lines += 1
                stats.emitted_defs["Auto.BeginMainPhase.LookTopDecision.Exact"] += 1
                continue

            paid_attack_named_climax_search_exact = re.match(
                r'^When this card attacks, if a card named "([^"]+)" is in your climax area, you may pay the cost\. If you do, search your deck for up to one 《([^》]+)》 character, reveal it to your opponent, and put it into your hand\. Shuffle your deck.*$',
                remainder,
                re.I,
            )
            if paid_attack_named_climax_search_exact:
                climax_name = paid_attack_named_climax_search_exact.group(1).strip()
                climax_ids = sorted(set((name_to_ids or {}).get(climax_name, [])))
                target_trait = (trait_map or {}).get(
                    paid_attack_named_climax_search_exact.group(2).strip()
                )
                if cost_is_empty(cost) or not climax_ids or target_trait is None:
                    mark_unsupported(line)
                    continue
                ability_defs.append(
                    _ability_def(
                        "Auto",
                        "AttackDeclaration",
                        effects=[_template("MoveToHand")],
                        targets=["SelfDeckTop"],
                        cost=cost,
                        conditions={"climax_area": {"side": "SelfSide", "card_ids": climax_ids}},
                        effect_optional=[True],
                        target_card_type="Character",
                        target_trait=target_trait,
                        target_limit=1,
                    )
                )
                stats.parsed_lines += 1
                stats.emitted_defs["Auto.AttackNamedClimaxSearch.Paid"] += 1
                continue

            on_play_memory_at_most_named_to_memory_exact = re.match(
                rf'^When this card is placed on (?:the )?stage from your hand, if your memory has ({COUNT_TOKEN_RE}) or less cards, you may choose ({COUNT_TOKEN_RE}) "[^"]+" in your waiting room, and put it into your memory\.?$',
                remainder,
                re.I,
            )
            if on_play_memory_at_most_named_to_memory_exact:
                memory_cap = parse_count_token(
                    on_play_memory_at_most_named_to_memory_exact.group(1)
                )
                choose_count = parse_count_token(
                    on_play_memory_at_most_named_to_memory_exact.group(2)
                )
                target_ids = resolve_exact_quoted_name_ids(remainder)
                if memory_cap is None or choose_count is None or not target_ids:
                    mark_unsupported(line)
                    continue
                ability_defs.append(
                    _ability_def(
                        "Auto",
                        "OnPlay",
                        effects=[_template("MoveToMemory")],
                        targets=["SelfWaitingRoom"],
                        conditions={"self_memory_at_most": memory_cap},
                        effect_optional=[True],
                        target_card_ids=target_ids,
                        target_limit=choose_count,
                    )
                )
                stats.parsed_lines += 1
                stats.emitted_defs["Auto.OnPlayMemoryAtMostNamedToMemory"] += 1
                continue

            on_play_opp_wr_climax_rest_exact = re.match(
                rf"^When this card is placed on (?:the )?stage from your hand, if your opponent's waiting room has ({COUNT_TOKEN_RE}) or more climax, 【REST】 this card\.?$",
                remainder,
                re.I,
            )
            if on_play_opp_wr_climax_rest_exact:
                threshold = parse_count_token(on_play_opp_wr_climax_rest_exact.group(1))
                if threshold is None:
                    mark_unsupported(line)
                    continue
                ability_defs.append(
                    _ability_def(
                        "Auto",
                        "OnPlay",
                        effects=[_template("RestTarget")],
                        targets=["This"],
                        conditions={
                            "zone_count": {
                                "side": "Opponent",
                                "zone": "WaitingRoomClimax",
                                "cmp": "AtLeast",
                                "value": threshold,
                            }
                        },
                    )
                )
                stats.parsed_lines += 1
                stats.emitted_defs["Auto.OnPlayOppWrClimaxRest"] += 1
                continue

            on_play_heal_then_self_power_exact = re.match(
                rf"^When this card is placed on (?:the )?stage from your hand, put up to ({COUNT_TOKEN_RE}) card from the top of your clock into your waiting room, and this card gets \+([+-]?\d+) power until end of turn\.?$",
                remainder,
                re.I,
            )
            if on_play_heal_then_self_power_exact:
                heal_count = parse_count_token(on_play_heal_then_self_power_exact.group(1))
                if heal_count is None:
                    mark_unsupported(line)
                    continue
                ability_defs.append(
                    _ability_def(
                        "Auto",
                        "OnPlay",
                        effects=[
                            _template("MoveToWaitingRoom"),
                            _template(
                                "AddPower",
                                amount=int(on_play_heal_then_self_power_exact.group(2)),
                                duration_turn=True,
                            ),
                        ],
                        targets=["SelfClock", "This"],
                        target_limit=heal_count,
                    )
                )
                stats.parsed_lines += 1
                stats.emitted_defs["Auto.OnPlayHealThenSelfPower"] += 1
                continue

            on_play_if_opp_center_count_remove_exact = re.match(
                rf"^When this card is placed on (?:the )?stage from your hand, if your opponent's center stage has ({COUNT_TOKEN_RE}) or less characters, you may choose ({COUNT_TOKEN_RE}) cost ({COUNT_TOKEN_RE}) or lower character in your opponent's center stage, and put it into their waiting room\.?$",
                remainder,
                re.I,
            )
            if on_play_if_opp_center_count_remove_exact:
                center_max = parse_count_token(on_play_if_opp_center_count_remove_exact.group(1))
                choose_count = parse_count_token(on_play_if_opp_center_count_remove_exact.group(2))
                cost_max = parse_count_token(on_play_if_opp_center_count_remove_exact.group(3))
                if center_max is None or choose_count is None or cost_max is None:
                    mark_unsupported(line)
                    continue
                ability_defs.append(
                    _ability_def(
                        "Auto",
                        "OnPlay",
                        effects=[_template("MoveToWaitingRoom")],
                        targets=["OppFrontRow"],
                        conditions={
                            "zone_count": {
                                "side": "Opponent",
                                "zone": "FrontRow",
                                "cmp": "AtMost",
                                "value": center_max,
                            }
                        },
                        effect_optional=[True],
                        target_card_type="Character",
                        target_cost_max=cost_max,
                        target_limit=choose_count,
                    )
                )
                stats.parsed_lines += 1
                stats.emitted_defs["Auto.OnPlayOppCenterCountCostRemove"] += 1
                continue

            paid_frontal_attack_reduce_level_exact = re.match(
                rf"^When this card frontal attacks, you may pay the cost\. If you do, choose ({COUNT_TOKEN_RE}) of your opponent's characters, and that character gets -({COUNT_TOKEN_RE}) level until end of turn\.?$",
                remainder,
                re.I,
            )
            if paid_frontal_attack_reduce_level_exact:
                choose_count = parse_count_token(paid_frontal_attack_reduce_level_exact.group(1))
                level_delta = parse_count_token(paid_frontal_attack_reduce_level_exact.group(2))
                if choose_count is None or level_delta is None or cost_is_empty(cost):
                    mark_unsupported(line)
                    continue
                ability_defs.append(
                    _ability_def(
                        "Auto",
                        "AttackDeclaration",
                        effects=[_template("AddLevel", amount=-level_delta, duration_turn=True)],
                        targets=["OppStage"],
                        cost=cost,
                        effect_optional=[True],
                        target_limit=choose_count,
                    )
                )
                stats.parsed_lines += 1
                stats.emitted_defs["Auto.PaidFrontalAttackReduceLevel"] += 1
                continue

            if allow_approx_effects:
                look_reorder_auto = re.match(
                    rf"^When .+, look at up to ({COUNT_TOKEN_RE}) cards? from the top of your deck, and put them on the top of your deck in any order\.?$",
                    remainder,
                    re.I,
                )
                if look_reorder_auto:
                    timing: Optional[str] = None
                    conditions: Optional[Dict[str, Any]] = None
                    if re.match(r"^When this card attacks,", remainder, re.I):
                        timing = "AttackDeclaration"
                    elif re.match(
                        r"^When this card's battle opponent becomes 【REVERSE】,",
                        remainder,
                        re.I,
                    ):
                        timing = "BattleOpponentReverse"
                    elif re.match(
                        r"^When (?:your|a) climax is placed on your climax area,",
                        remainder,
                        re.I,
                    ):
                        timing = "AfterClimaxPhase"
                        conditions = {"climax_area": {"side": "SelfSide", "card_ids": []}}
                    elif re.match(
                        r"^When this card is placed on (?:the )?stage from your hand,",
                        remainder,
                        re.I,
                    ):
                        timing = "OnPlay"
                    elif re.match(r"^When you play an event,", remainder, re.I):
                        timing = "OnPlay"
                    if timing is not None:
                        ability_defs.append(
                            _ability_def(
                                "Auto",
                                timing,
                                effects=[_template("Draw", count=0)],
                                targets=[],
                                conditions=with_approx_condition(conditions),
                            )
                        )
                        stats.parsed_lines += 1
                        stats.emitted_defs["Auto.LookTopReorder.ApproxNoop"] += 1
                        continue

                begin_main_look_top_decision = re.match(
                    r"^At the beginning of your main phase, look at the top card of your deck, and put it on the top of your deck or into your waiting room\.?$",
                    remainder,
                    re.I,
                )
                if begin_main_look_top_decision:
                    ability_defs.append(
                        _ability_def(
                            "Auto",
                            "BeginMainPhase",
                            effects=[_template("Draw", count=0)],
                            targets=[],
                            conditions=with_approx_condition(),
                        )
                    )
                    stats.parsed_lines += 1
                    stats.emitted_defs["Auto.BeginMainPhase.LookTopDecision.ApproxNoop"] += 1
                    continue

                begin_opp_draw_center_team_power = re.match(
                    rf"^At the beginning of your opponent's draw phase, if this card is in your center stage, choose ({COUNT_TOKEN_RE}) of your characters, and that character gets \+([+-]?\d+) power until end of turn\.?$",
                    remainder,
                    re.I,
                )
                if begin_opp_draw_center_team_power:
                    ability_defs.append(
                        _ability_def(
                            "Auto",
                            "BeginDrawPhase",
                            effects=[_template("Draw", count=0)],
                            targets=[],
                            conditions=with_approx_condition({"turn": "OpponentTurn"}),
                        )
                    )
                    stats.parsed_lines += 1
                    stats.emitted_defs["Auto.BeginOpponentDraw.CenterTeamPower.ApproxNoop"] += 1
                    continue

                on_play_event_self_power = re.match(
                    r"^When you play an event, this card gets \+([+-]?\d+) power until end of turn\.?$",
                    remainder,
                    re.I,
                )
                if on_play_event_self_power:
                    ability_defs.append(
                        _ability_def(
                            "Auto",
                            "OnPlay",
                            effects=[_template("Draw", count=0)],
                            targets=[],
                            conditions=with_approx_condition(),
                        )
                    )
                    stats.parsed_lines += 1
                    stats.emitted_defs["Auto.OnPlayEventSelfPower.ApproxNoop"] += 1
                    continue

                begin_climax_opp_may_stock_mill = re.match(
                    rf"^At the beginning of your climax phase, your opponent may put the top ({COUNT_TOKEN_RE}) cards? of their stock into their waiting room\. If they do, this card cannot frontal attack until end of turn\.?$",
                    remainder,
                    re.I,
                )
                if begin_climax_opp_may_stock_mill:
                    ability_defs.append(
                        _ability_def(
                            "Auto",
                            "BeginClimaxPhase",
                            effects=[_template("Draw", count=0)],
                            targets=[],
                            conditions=with_approx_condition(),
                        )
                    )
                    stats.parsed_lines += 1
                    stats.emitted_defs["Auto.BeginClimaxOpponentMayStockMill.ApproxNoop"] += 1
                    continue

                begin_opp_attack_move_opp_center = re.match(
                    rf"^At the beginning of your opponent's attack phase, you may choose ({COUNT_TOKEN_RE}) character in your opponent's center stage, and move it to another open position of their center stage\.?$",
                    remainder,
                    re.I,
                )
                if begin_opp_attack_move_opp_center:
                    ability_defs.append(
                        _ability_def(
                            "Auto",
                            "BeginAttackPhase",
                            effects=[_template("Draw", count=0)],
                            targets=[],
                            conditions=with_approx_condition({"turn": "OpponentTurn"}),
                            effect_optional=[True],
                        )
                    )
                    stats.parsed_lines += 1
                    stats.emitted_defs["Auto.BeginOpponentAttackMoveOppCenter.ApproxNoop"] += 1
                    continue

                begin_opp_attack_mill_trait_move_center = re.match(
                    r"^At the beginning of your opponent's attack phase, you may put the top card of your deck into your waiting room\. If that card is a 《[^》]+》 character, you may move this card to an open position of your center stage\.?$",
                    remainder,
                    re.I,
                )
                if begin_opp_attack_mill_trait_move_center:
                    ability_defs.append(
                        _ability_def(
                            "Auto",
                            "BeginAttackPhase",
                            effects=[_template("Draw", count=0)],
                            targets=[],
                            conditions=with_approx_condition({"turn": "OpponentTurn"}),
                            effect_optional=[True],
                        )
                    )
                    stats.parsed_lines += 1
                    stats.emitted_defs[
                        "Auto.BeginOpponentAttackMillTraitMoveCenter.ApproxNoop"
                    ] += 1
                    continue

                other_named_on_play_stock = re.match(
                    r'^When your other "[^"]+" is placed on the stage from your hand, you may put the top card of your deck into your stock\.?$',
                    remainder,
                    re.I,
                )
                if other_named_on_play_stock:
                    ability_defs.append(
                        _ability_def(
                            "Auto",
                            "OnPlay",
                            effects=[_template("Draw", count=0)],
                            targets=[],
                            conditions=with_approx_condition(),
                            effect_optional=[True],
                        )
                    )
                    stats.parsed_lines += 1
                    stats.emitted_defs["Auto.OtherNamedOnPlayStockTop.ApproxNoop"] += 1
                    continue

                paid_attack_named_climax_search = re.match(
                    r'^When this card attacks, if a card named "[^"]+" is in your climax area, you may pay the cost\. If you do, search your deck for up to one 《[^》]+》 character, reveal it to your opponent, and put it into your hand\. Shuffle your deck.*$',
                    remainder,
                    re.I,
                )
                if paid_attack_named_climax_search:
                    ability_defs.append(
                        _ability_def(
                            "Auto",
                            "AttackDeclaration",
                            effects=[_template("Draw", count=0)],
                            targets=[],
                            cost=cost,
                            conditions=with_approx_condition(),
                            effect_optional=[True],
                        )
                    )
                    stats.parsed_lines += 1
                    stats.emitted_defs["Auto.AttackNamedClimaxSearch.Paid.ApproxNoop"] += 1
                    continue

                paid_on_play_named_wr_to_stage = re.match(
                    rf'^When this card is placed on (?:the )?stage from your hand, you may pay the cost\. If you do, choose ({COUNT_TOKEN_RE}) "[^"]+" in your waiting room, and put it on any position of your stage\.?$',
                    remainder,
                    re.I,
                )
                if paid_on_play_named_wr_to_stage:
                    ability_defs.append(
                        _ability_def(
                            "Auto",
                            "OnPlay",
                            effects=[_template("Draw", count=0)],
                            targets=[],
                            cost=cost,
                            conditions=with_approx_condition(),
                            effect_optional=[True],
                        )
                    )
                    stats.parsed_lines += 1
                    stats.emitted_defs["Auto.OnPlayNamedWrToStage.Paid.ApproxNoop"] += 1
                    continue

                paid_on_play_discard_character_or_move_self = re.match(
                    r"^When this card is placed on the stage from your hand, you may pay the cost\. If you do not, put this card into your waiting room\.?$",
                    remainder,
                    re.I,
                )
                if paid_on_play_discard_character_or_move_self:
                    ability_defs.append(
                        _ability_def(
                            "Auto",
                            "OnPlay",
                            effects=[_template("Draw", count=0)],
                            targets=[],
                            cost=cost,
                            conditions=with_approx_condition(),
                            effect_optional=[True],
                        )
                    )
                    stats.parsed_lines += 1
                    stats.emitted_defs["Auto.OnPlayPayOrMoveSelfToWr.ApproxNoop"] += 1
                    continue

                cxcombo_reverse_stock_salvage = re.match(
                    rf'^When this card\'s battle opponent becomes 【REVERSE】, if "[^"]+" is in your climax area, put up to ({COUNT_TOKEN_RE}) card from the top of your deck into your stock, choose up to ({COUNT_TOKEN_RE}) "[^"]+" in your waiting room, and return it to your hand\.?$',
                    remainder,
                    re.I,
                )
                if cxcombo_reverse_stock_salvage:
                    ability_defs.append(
                        _ability_def(
                            "Auto",
                            "BattleOpponentReverse",
                            effects=[_template("Draw", count=0)],
                            targets=[],
                            conditions=with_approx_condition(
                                {"climax_area": {"side": "SelfSide", "card_ids": []}}
                            ),
                        )
                    )
                    stats.parsed_lines += 1
                    stats.emitted_defs["Auto.CxcomboReverseStockSalvage.ApproxNoop"] += 1
                    continue

                another_reverse_choose_one_power = re.match(
                    rf"^When another of your characters becomes 【REVERSE】 in battle, choose ({COUNT_TOKEN_RE}) of your characters, and that character gets \+([+-]?\d+) power until end of turn\.?$",
                    remainder,
                    re.I,
                )
                if another_reverse_choose_one_power:
                    ability_defs.append(
                        _ability_def(
                            "Auto",
                            "OnReverse",
                            effects=[_template("Draw", count=0)],
                            targets=[],
                            conditions=with_approx_condition(),
                        )
                    )
                    stats.parsed_lines += 1
                    stats.emitted_defs["Auto.AnotherReverseTeamPower.ApproxNoop"] += 1
                    continue

                attack_choose_one_other_level_power = re.match(
                    r"^When this card attacks, choose one of your other characters, and that character gets \+([+-]?\d+) level and \+([+-]?\d+) power until end of turn\.?$",
                    remainder,
                    re.I,
                )
                if attack_choose_one_other_level_power:
                    ability_defs.append(
                        _ability_def(
                            "Auto",
                            "AttackDeclaration",
                            effects=[_template("Draw", count=0)],
                            targets=[],
                            conditions=with_approx_condition(),
                        )
                    )
                    stats.parsed_lines += 1
                    stats.emitted_defs["Auto.AttackChooseOneOtherLevelPower.ApproxNoop"] += 1
                    continue

                attack_choose_two_trait_power = re.match(
                    r"^When this card attacks, choose up to two of your 《[^》]+》 characters, and those characters get \+([+-]?\d+) power until end of turn\.?$",
                    remainder,
                    re.I,
                )
                if attack_choose_two_trait_power:
                    ability_defs.append(
                        _ability_def(
                            "Auto",
                            "AttackDeclaration",
                            effects=[_template("Draw", count=0)],
                            targets=[],
                            conditions=with_approx_condition(),
                        )
                    )
                    stats.parsed_lines += 1
                    stats.emitted_defs["Auto.AttackChooseTwoTraitPower.ApproxNoop"] += 1
                    continue

                attack_facing_level_opp_next_turn_power = re.match(
                    rf"^When this card attacks, if the character facing this card is level ({COUNT_TOKEN_RE}) or higher, this card gets \+([+-]?\d+) power until the end of your opponent's next turn\.?$",
                    remainder,
                    re.I,
                )
                if attack_facing_level_opp_next_turn_power:
                    ability_defs.append(
                        _ability_def(
                            "Auto",
                            "AttackDeclaration",
                            effects=[_template("Draw", count=0)],
                            targets=[],
                            conditions=with_approx_condition(),
                        )
                    )
                    stats.parsed_lines += 1
                    stats.emitted_defs["Auto.AttackFacingLevelPowerOppNextTurn.ApproxNoop"] += 1
                    continue

                on_play_memory_at_most_named_to_memory = re.match(
                    rf'^When this card is placed on (?:the )?stage from your hand, if your memory has ({COUNT_TOKEN_RE}) or less cards, you may choose ({COUNT_TOKEN_RE}) "[^"]+" in your waiting room, and put it into your memory\.?$',
                    remainder,
                    re.I,
                )
                if on_play_memory_at_most_named_to_memory:
                    ability_defs.append(
                        _ability_def(
                            "Auto",
                            "OnPlay",
                            effects=[_template("Draw", count=0)],
                            targets=[],
                            conditions=with_approx_condition(),
                            effect_optional=[True],
                        )
                    )
                    stats.parsed_lines += 1
                    stats.emitted_defs["Auto.OnPlayMemoryAtMostNamedToMemory.ApproxNoop"] += 1
                    continue

                on_play_opp_wr_climax_rest = re.match(
                    rf"^When this card is placed on (?:the )?stage from your hand, if your opponent's waiting room has ({COUNT_TOKEN_RE}) or more climax, 【REST】 this card\.?$",
                    remainder,
                    re.I,
                )
                if on_play_opp_wr_climax_rest:
                    ability_defs.append(
                        _ability_def(
                            "Auto",
                            "OnPlay",
                            effects=[_template("Draw", count=0)],
                            targets=[],
                            conditions=with_approx_condition(),
                        )
                    )
                    stats.parsed_lines += 1
                    stats.emitted_defs["Auto.OnPlayOppWrClimaxRest.ApproxNoop"] += 1
                    continue

                on_play_mill2_if_climax_rest = re.match(
                    r"^When this card is placed on (?:the )?stage from your hand, put the top two cards of your deck into your waiting room\. If there is a climax among those cards, 【REST】 this card\.?$",
                    remainder,
                    re.I,
                )
                if on_play_mill2_if_climax_rest:
                    ability_defs.append(
                        _ability_def(
                            "Auto",
                            "OnPlay",
                            effects=[_template("Draw", count=0)],
                            targets=[],
                            conditions=with_approx_condition(),
                        )
                    )
                    stats.parsed_lines += 1
                    stats.emitted_defs["Auto.OnPlayMillIfClimaxRest.ApproxNoop"] += 1
                    continue

                on_play_heal_then_self_power = re.match(
                    rf"^When this card is placed on (?:the )?stage from your hand, put up to ({COUNT_TOKEN_RE}) card from the top of your clock into your waiting room, and this card gets \+([+-]?\d+) power until end of turn\.?$",
                    remainder,
                    re.I,
                )
                if on_play_heal_then_self_power:
                    ability_defs.append(
                        _ability_def(
                            "Auto",
                            "OnPlay",
                            effects=[_template("Draw", count=0)],
                            targets=[],
                            conditions=with_approx_condition(),
                        )
                    )
                    stats.parsed_lines += 1
                    stats.emitted_defs["Auto.OnPlayHealThenSelfPower.ApproxNoop"] += 1
                    continue

                on_play_if_opp_center_count_remove = re.match(
                    rf"^When this card is placed on (?:the )?stage from your hand, if your opponent's center stage has ({COUNT_TOKEN_RE}) or less characters, you may choose ({COUNT_TOKEN_RE}) cost ({COUNT_TOKEN_RE}) or lower character in your opponent's center stage, and put it into their waiting room\.?$",
                    remainder,
                    re.I,
                )
                if on_play_if_opp_center_count_remove:
                    ability_defs.append(
                        _ability_def(
                            "Auto",
                            "OnPlay",
                            effects=[_template("Draw", count=0)],
                            targets=[],
                            conditions=with_approx_condition(),
                            effect_optional=[True],
                        )
                    )
                    stats.parsed_lines += 1
                    stats.emitted_defs["Auto.OnPlayOppCenterCountCostRemove.ApproxNoop"] += 1
                    continue

                on_play_look_top_opp_deck = re.match(
                    rf"^When this card is placed on (?:the )?stage from your hand, look at up to ({COUNT_TOKEN_RE}) cards? from the top of your opponent's deck, and put them on the top of (?:his or her|their) deck in the original order\.?$",
                    remainder,
                    re.I,
                )
                if on_play_look_top_opp_deck:
                    ability_defs.append(
                        _ability_def(
                            "Auto",
                            "OnPlay",
                            effects=[_template("Draw", count=0)],
                            targets=[],
                            conditions=with_approx_condition(),
                        )
                    )
                    stats.parsed_lines += 1
                    stats.emitted_defs["Auto.OnPlayLookTopOpponentDeck.ApproxNoop"] += 1
                    continue

                on_play_look_x_opp_count = re.match(
                    rf"^When this card is placed on (?:the )?stage from your hand, look at up to X cards? from the top of your deck, choose up to ({COUNT_TOKEN_RE}) card from among them, put it into your hand, and put the rest into your waiting room\. X is equal to the number of characters your opponent has\.?$",
                    remainder,
                    re.I,
                )
                if on_play_look_x_opp_count:
                    ability_defs.append(
                        _ability_def(
                            "Auto",
                            "OnPlay",
                            effects=[_template("Draw", count=0)],
                            targets=[],
                            conditions=with_approx_condition(),
                        )
                    )
                    stats.parsed_lines += 1
                    stats.emitted_defs["Auto.OnPlayLookXOppCount.ApproxNoop"] += 1
                    continue

                on_play_mill_if_level_no_side_penalty_revealed = re.match(
                    rf"^When this card is placed on (?:the )?stage from your hand, put the top ({COUNT_TOKEN_RE}) cards? of your deck into your waiting room\. If there is a level ({COUNT_TOKEN_RE}) or lower character revealed among those cards, this card's soul does not decrease by side attacking until end of turn\.?$",
                    remainder,
                    re.I,
                )
                if on_play_mill_if_level_no_side_penalty_revealed:
                    ability_defs.append(
                        _ability_def(
                            "Auto",
                            "OnPlay",
                            effects=[_template("Draw", count=0)],
                            targets=[],
                            conditions=with_approx_condition(),
                        )
                    )
                    stats.parsed_lines += 1
                    stats.emitted_defs[
                        "Auto.OnPlayMillIfLevelRevealedNoSidePenalty.ApproxNoop"
                    ] += 1
                    continue

                on_play_mill_top_if_climax_stock_self = re.match(
                    r"^When this card is placed on the stage from your hand, put the top card of your deck into your waiting room\. If that card is a climax, put this card into your stock\.?$",
                    remainder,
                    re.I,
                )
                if on_play_mill_top_if_climax_stock_self:
                    ability_defs.append(
                        _ability_def(
                            "Auto",
                            "OnPlay",
                            effects=[_template("Draw", count=0)],
                            targets=[],
                            conditions=with_approx_condition(),
                        )
                    )
                    stats.parsed_lines += 1
                    stats.emitted_defs["Auto.OnPlayMillTopIfClimaxStockSelf.ApproxNoop"] += 1
                    continue

                other_battle_opp_reverse_team_power = re.match(
                    rf"^When your other character's battle opponent becomes 【REVERSE】, choose ({COUNT_TOKEN_RE}) of your characters, and that character gets \+([+-]?\d+) power until end of turn\.?$",
                    remainder,
                    re.I,
                )
                if other_battle_opp_reverse_team_power:
                    ability_defs.append(
                        _ability_def(
                            "Auto",
                            "BattleOpponentReverse",
                            effects=[_template("Draw", count=0)],
                            targets=[],
                            conditions=with_approx_condition(),
                        )
                    )
                    stats.parsed_lines += 1
                    stats.emitted_defs["Auto.OtherBattleOpponentReverseTeamPower.ApproxNoop"] += 1
                    continue

                reverse_cannot_use_quoted = re.match(
                    r'^When this card or this card\'s battle opponent becomes 【REVERSE】, that character cannot use "[^"]+" until end of turn\. \(The "[^"]+" rule cannot be used either\)\.?$',
                    remainder,
                    re.I,
                )
                if reverse_cannot_use_quoted:
                    ability_defs.append(
                        _ability_def(
                            "Auto",
                            "OnReverse",
                            effects=[_template("Draw", count=0)],
                            targets=[],
                            conditions=with_approx_condition(),
                        )
                    )
                    stats.parsed_lines += 1
                    stats.emitted_defs["Auto.ReverseCannotUseQuoted.ApproxNoop"] += 1
                    continue

            climax_following = re.match(
                rf'^When (?:your|a) climax is placed on your climax area, choose (up to )?({COUNT_TOKEN_RE}) of your characters, and that character gets the following ability until end of turn\.\s*"(.+)"\.?$',
                remainder,
                re.I,
            )
            if climax_following:
                choose_count = parse_count_token(climax_following.group(2))
                if choose_count is None:
                    mark_unsupported(line)
                    continue
                nested_effect = parse_following_effect_or_grant(
                    climax_following.group(3),
                    duration_turn=True,
                    grant_duration="UntilEndOfTurn",
                )
                if nested_effect is not None:
                    ability_defs.append(
                        _ability_def(
                            "Auto",
                            "AfterClimaxPhase",
                            effects=[nested_effect],
                            targets=["SelfStage"],
                            effect_optional=([True] if climax_following.group(1) else []),
                            target_limit=choose_count,
                            conditions={"climax_area": {"side": "SelfSide", "card_ids": []}},
                        )
                    )
                    stats.parsed_lines += 1
                    stats.emitted_defs["Auto.ClimaxFollowingAbility.Flattened"] += 1
                    continue
                if allow_approx_effects:
                    ability_defs.append(
                        _ability_def(
                            "Auto",
                            "AfterClimaxPhase",
                            effects=[_template("Draw", count=0)],
                            targets=[],
                            conditions=with_approx_condition(
                                {"climax_area": {"side": "SelfSide", "card_ids": []}}
                            ),
                        )
                    )
                    stats.parsed_lines += 1
                    stats.emitted_defs["Auto.ClimaxFollowingAbility.ApproxNoop"] += 1
                    continue

            if allow_approx_effects:
                on_play_reveal_trait_draw_discard = re.match(
                    r"^When this card is placed on the stage from your hand, reveal the top card of your deck\. If that card is a 《[^》]+》 character, put it into your hand, choose a card in your hand, and put it into your waiting room\. \(If it is not, return the revealed card to its original place\)\.?$",
                    remainder,
                    re.I,
                )
                if on_play_reveal_trait_draw_discard:
                    ability_defs.append(
                        _ability_def(
                            "Auto",
                            "OnPlay",
                            effects=[_template("Draw", count=0)],
                            targets=[],
                            conditions=with_approx_condition(),
                        )
                    )
                    stats.parsed_lines += 1
                    stats.emitted_defs["Auto.RevealTraitDrawDiscard.OnPlay.ApproxNoop"] += 1
                    continue

                on_play_reveal_level_stock = re.match(
                    rf"^When this card is placed on the stage from your hand, reveal the top card of your deck\. If that card is level ({COUNT_TOKEN_RE}) or lower, put it into your stock\. \(Otherwise, return it to its original place\. Climax are regarded as level ({COUNT_TOKEN_RE})\)\.?$",
                    remainder,
                    re.I,
                )
                if on_play_reveal_level_stock:
                    ability_defs.append(
                        _ability_def(
                            "Auto",
                            "OnPlay",
                            effects=[_template("Draw", count=0)],
                            targets=[],
                            conditions=with_approx_condition(),
                        )
                    )
                    stats.parsed_lines += 1
                    stats.emitted_defs["Auto.RevealLevelStock.OnPlay.ApproxNoop"] += 1
                    continue

                reverse_self_memory_return = re.match(
                    rf'^When this card\'s battle opponent becomes 【REVERSE】, you may put this card into your memory\. If you do, at the beginning of your next draw phase, choose ({COUNT_TOKEN_RE}) "[^"]+" in your memory, and put it (?:on|in) any position of your stage\.?$',
                    remainder,
                    re.I,
                )
                if reverse_self_memory_return:
                    ability_defs.append(
                        _ability_def(
                            "Auto",
                            "BattleOpponentReverse",
                            effects=[_template("Draw", count=0)],
                            targets=[],
                            conditions=with_approx_condition(),
                            effect_optional=[True],
                        )
                    )
                    stats.parsed_lines += 1
                    stats.emitted_defs["Auto.ReverseSelfMemoryReturn.ApproxNoop"] += 1
                    continue

                other_reverse_team_power = re.match(
                    rf"^When your other character becomes 【REVERSE】 in battle, choose ({COUNT_TOKEN_RE}) of your characters, and that character gets \+([+-]?\d+) power until end of turn\.?$",
                    remainder,
                    re.I,
                )
                if other_reverse_team_power:
                    ability_defs.append(
                        _ability_def(
                            "Auto",
                            None,
                            effects=[_template("Draw", count=0)],
                            targets=[],
                            conditions=with_approx_condition(),
                        )
                    )
                    stats.parsed_lines += 1
                    stats.emitted_defs["Auto.OtherReverseTeamPower.ApproxNoop"] += 1
                    continue

                paid_frontal_attack_reduce_level = re.match(
                    rf"^When this card frontal attacks, you may pay the cost\. If you do, choose ({COUNT_TOKEN_RE}) of your opponent's characters, and that character gets -({COUNT_TOKEN_RE}) level until end of turn\.?$",
                    remainder,
                    re.I,
                )
                if paid_frontal_attack_reduce_level:
                    ability_defs.append(
                        _ability_def(
                            "Auto",
                            "AttackDeclaration",
                            effects=[_template("Draw", count=0)],
                            targets=[],
                            conditions=with_approx_condition(),
                            effect_optional=[True],
                        )
                    )
                    stats.parsed_lines += 1
                    stats.emitted_defs["Auto.PaidFrontalAttackReduceLevel.ApproxNoop"] += 1
                    continue

                begin_opp_draw_mill_gate_return = re.match(
                    rf"^At the beginning of your opponent's draw phase, put the top ({COUNT_TOKEN_RE}) cards? of your deck into your waiting room\. If there is a level ({COUNT_TOKEN_RE}) or higher card among those cards, you may return this card to your hand\. \(Climax are regarded as level ({COUNT_TOKEN_RE})\)\.?$",
                    remainder,
                    re.I,
                )
                if begin_opp_draw_mill_gate_return:
                    ability_defs.append(
                        _ability_def(
                            "Auto",
                            "BeginDrawPhase",
                            effects=[_template("Draw", count=0)],
                            targets=[],
                            conditions=with_approx_condition({"turn": "OpponentTurn"}),
                        )
                    )
                    stats.parsed_lines += 1
                    stats.emitted_defs[
                        "Auto.MillGateMoveSelfToHand.BeginOpponentDraw.ApproxNoop"
                    ] += 1
                    continue

                damage_received_not_canceled_center_look = re.match(
                    r"^During your opponent's turn, when the damage you received is not canceled, if this card is in your center stage, look at the top card of your deck, and put it on the top of your deck or into your waiting room\.?$",
                    remainder,
                    re.I,
                )
                if damage_received_not_canceled_center_look:
                    ability_defs.append(
                        _ability_def(
                            "Auto",
                            "DamageReceivedNotCanceled",
                            effects=[_template("Draw", count=0)],
                            targets=[],
                            conditions=with_approx_condition({"turn": "OpponentTurn"}),
                        )
                    )
                    stats.parsed_lines += 1
                    stats.emitted_defs[
                        "Auto.DamageReceivedNotCanceled.CenterLookTop.ApproxNoop"
                    ] += 1
                    continue

                on_play_hand_to_stage_then_power = re.match(
                    rf"^When this card is placed on (?:the )?stage from your hand, choose up to ({COUNT_TOKEN_RE}) character with level equal to or lower than your level in your hand, put it in any position of your stage, and that character gets \+([+-]?\d+) power until end of turn\.?$",
                    remainder,
                    re.I,
                )
                if on_play_hand_to_stage_then_power:
                    ability_defs.append(
                        _ability_def(
                            "Auto",
                            "OnPlay",
                            effects=[_template("Draw", count=0)],
                            targets=[],
                            conditions=with_approx_condition(),
                        )
                    )
                    stats.parsed_lines += 1
                    stats.emitted_defs["Auto.HandToStageThenPower.OnPlay.ApproxNoop"] += 1
                    continue

                on_play_repeat_following = re.match(
                    rf'^When this card is placed on (?:the )?stage from your hand, perform the following action ({COUNT_TOKEN_RE}) times?\.\s*"[^"]+"\.?$',
                    remainder,
                    re.I,
                )
                if on_play_repeat_following:
                    ability_defs.append(
                        _ability_def(
                            "Auto",
                            "OnPlay",
                            effects=[_template("Draw", count=0)],
                            targets=[],
                            conditions=with_approx_condition(),
                        )
                    )
                    stats.parsed_lines += 1
                    stats.emitted_defs["Auto.RepeatFollowingAction.OnPlay.ApproxNoop"] += 1
                    continue

                on_play_choose_following_effects = re.match(
                    rf'^When this card is placed on (?:the )?stage from your hand, choose ({COUNT_TOKEN_RE}) of the following effects, and perform it\.\s*"[^"]+"\s*"[^"]+"\.?$',
                    remainder,
                    re.I,
                )
                if on_play_choose_following_effects:
                    ability_defs.append(
                        _ability_def(
                            "Auto",
                            "OnPlay",
                            effects=[_template("Draw", count=0)],
                            targets=[],
                            conditions=with_approx_condition(),
                        )
                    )
                    stats.parsed_lines += 1
                    stats.emitted_defs["Auto.OnPlayChooseFollowingEffects.ApproxNoop"] += 1
                    continue

                on_play_mill_if_climax_opp_center_remove = re.match(
                    rf"^When this card is placed on (?:the )?stage from your hand, put the top ({COUNT_TOKEN_RE}) cards? of your deck into your waiting room\. If there is a climax among those cards, you may choose ({COUNT_TOKEN_RE}) level ({COUNT_TOKEN_RE}) or lower character in your opponent's center stage, and put it into their waiting room\.?$",
                    remainder,
                    re.I,
                )
                if on_play_mill_if_climax_opp_center_remove:
                    ability_defs.append(
                        _ability_def(
                            "Auto",
                            "OnPlay",
                            effects=[_template("Draw", count=0)],
                            targets=[],
                            conditions=with_approx_condition(),
                            effect_optional=[True],
                        )
                    )
                    stats.parsed_lines += 1
                    stats.emitted_defs["Auto.MillIfClimaxRemoveOppCenter.OnPlay.ApproxNoop"] += 1
                    continue

                on_play_mill_if_level_side_no_penalty = re.match(
                    rf"^When this card is placed on (?:the )?stage from your hand, put the top ({COUNT_TOKEN_RE}) cards? of your deck into your waiting room\. If there is a level ({COUNT_TOKEN_RE}) or lower character among those cards, this card's soul does not decrease by side attacking until end of turn\.?$",
                    remainder,
                    re.I,
                )
                if on_play_mill_if_level_side_no_penalty:
                    ability_defs.append(
                        _ability_def(
                            "Auto",
                            "OnPlay",
                            effects=[_template("Draw", count=0)],
                            targets=[],
                            conditions=with_approx_condition(),
                        )
                    )
                    stats.parsed_lines += 1
                    stats.emitted_defs["Auto.MillIfLevelNoSideSoulLoss.OnPlay.ApproxNoop"] += 1
                    continue

                on_climax_soul_icon_salvage_and_team_soul = re.match(
                    rf"^When this card is placed on your climax area from your hand, choose up to ({COUNT_TOKEN_RE}) character with\s*(?:\[SOUL\]\s*)?in its trigger icon in your waiting room, return it to your hand, choose up to ({COUNT_TOKEN_RE}) of your characters, and those characters get \+([+-]?\d+) soul until end of turn\.?$",
                    remainder,
                    re.I,
                )
                if on_climax_soul_icon_salvage_and_team_soul:
                    ability_defs.append(
                        _ability_def(
                            "Auto",
                            "OnPlay",
                            effects=[_template("Draw", count=0)],
                            targets=[],
                            conditions=with_approx_condition(),
                        )
                    )
                    stats.parsed_lines += 1
                    stats.emitted_defs["Auto.ClimaxSoulIconSalvageAndSoulBuff.ApproxNoop"] += 1
                    continue

                paid_replace_prev_slot = re.match(
                    r"^When your other character is put into your waiting room from the stage, if this card is in your back stage, you may pay the cost\. If you do, return that character to its previous stage position as 【REST】\.?$",
                    remainder,
                    re.I,
                )
                if paid_replace_prev_slot:
                    ability_defs.append(
                        _ability_def(
                            "Auto",
                            None,
                            effects=[_template("Draw", count=0)],
                            targets=[],
                            cost=cost,
                            conditions=with_approx_condition(),
                            effect_optional=[True],
                        )
                    )
                    stats.parsed_lines += 1
                    stats.emitted_defs["Auto.ReturnToPreviousSlot.ApproxNoop"] += 1
                    continue

                paid_side_no_penalty = re.match(
                    r"^When this card is placed on (?:the )?stage from your hand, you may pay the cost\. If you do, this card's soul does not decrease by side attacking until end of turn\.?$",
                    remainder,
                    re.I,
                )
                if paid_side_no_penalty:
                    ability_defs.append(
                        _ability_def(
                            "Auto",
                            "OnPlay",
                            effects=[_template("Draw", count=0)],
                            targets=[],
                            cost=cost,
                            conditions=with_approx_condition(),
                            effect_optional=[True],
                        )
                    )
                    stats.parsed_lines += 1
                    stats.emitted_defs["Auto.NoSideSoulLoss.OnPlay.Paid.ApproxNoop"] += 1
                    continue

                paid_on_play_turn_reverse_clock = re.match(
                    r"^During the turn that this card is placed on the stage from your hand, when this card's battle opponent becomes 【REVERSE】, you may pay the cost\. If you do, put that character into your opponent's clock\.?$",
                    remainder,
                    re.I,
                )
                if paid_on_play_turn_reverse_clock:
                    ability_defs.append(
                        _ability_def(
                            "Auto",
                            "BattleOpponentReverse",
                            effects=[_template("Draw", count=0)],
                            targets=[],
                            cost=cost,
                            conditions=with_approx_condition(),
                            effect_optional=[True],
                        )
                    )
                    stats.parsed_lines += 1
                    stats.emitted_defs["Auto.ReverseClock.OnPlayTurn.Paid.ApproxNoop"] += 1
                    continue

                cxcombo_climax_named_following = re.match(
                    rf'^When "[^"]+" is placed on your climax area, choose (up to )?({COUNT_TOKEN_RE}) of your other characters, and that character gets the following ability until end of turn\.\s*"[^"]+"\.?$',
                    remainder,
                    re.I,
                )
                if cxcombo_climax_named_following:
                    ability_defs.append(
                        _ability_def(
                            "Auto",
                            "AfterClimaxPhase",
                            effects=[_template("Draw", count=0)],
                            targets=[],
                            conditions=with_approx_condition(
                                {"climax_area": {"side": "SelfSide", "card_ids": []}}
                            ),
                        )
                    )
                    stats.parsed_lines += 1
                    stats.emitted_defs["Auto.CxcomboFollowingAbility.ApproxNoop"] += 1
                    continue

            if allow_approx_effects and "following ability" in remainder.lower():
                fallback_timing = infer_auto_timing_from_remainder(remainder)
                optional = bool(re.search(r"\byou may\b|\bup to\b", remainder, re.I))
                ability_defs.append(
                    _ability_def(
                        "Auto",
                        fallback_timing,
                        effects=[_template("Draw", count=0)],
                        targets=[],
                        cost=cost if not cost_is_empty(cost) else None,
                        conditions=with_approx_condition(),
                        effect_optional=[optional] if optional else [],
                    )
                )
                stats.parsed_lines += 1
                stats.emitted_defs["Auto.FollowingAbility.Generic.ApproxNoop"] += 1
                continue

            if try_parser_v2_fallback(line):
                continue
            mark_unsupported(line)
            continue

        if line_clean.startswith("【ACT】"):
            remainder = line_clean[len("【ACT】") :].strip()
            if "【" in remainder and "following ability" not in remainder.lower():
                if try_parser_v2_fallback(line):
                    continue
                mark_unsupported(line)
                continue

            handled_act_rule = False
            for rule in ACT_RULES:
                match = rule.pattern.match(remainder)
                if not match:
                    continue
                if not rule_enabled(rule):
                    continue
                if rule.id == "Activated.Brainstorm.CustomAction.ApproxDraw":
                    reveal_count = parse_count_token(match.group(1))
                    if reveal_count is None:
                        mark_unsupported(line)
                        handled_act_rule = True
                        break
                    ability_defs.append(
                        _ability_def(
                            "Activated",
                            None,
                            effects=[
                                _template(
                                    "Brainstorm",
                                    reveal_count=reveal_count,
                                    per_climax=1,
                                    mode="Draw",
                                )
                            ],
                            targets=[],
                            cost=cost,
                            conditions=with_approx_condition(),
                        )
                    )
                    stats.parsed_lines += 1
                    stats.emitted_defs[rule.id] += 1
                    handled_act_rule = True
                    break
            if handled_act_rule:
                continue

            act_heal_top_clock = re.match(
                r"^put the top card of your clock into your waiting room\.?$",
                remainder,
                re.I,
            )
            if act_heal_top_clock:
                ability_defs.append(
                    _ability_def(
                        "Activated",
                        None,
                        effects=[_template("Heal")],
                        targets=["SelfClock"],
                        cost=cost,
                        target_limit=1,
                    )
                )
                stats.parsed_lines += 1
                stats.emitted_defs["Activated.HealTopClock"] += 1
                continue

            act_look_top_top_or_bottom = re.match(
                r"^look at the top card of your deck, and put it on the top or at the bottom of your deck\.?$",
                remainder,
                re.I,
            )
            if act_look_top_top_or_bottom:
                ability_defs.append(
                    _ability_def(
                        "Activated",
                        None,
                        effects=[_template("MoveToDeckBottom")],
                        targets=["SelfDeckTop"],
                        cost=cost,
                        effect_optional=[True],
                        target_limit=1,
                    )
                )
                stats.parsed_lines += 1
                stats.emitted_defs["Activated.LookTopTopOrBottom"] += 1
                continue

            act_look_top_reorder = re.match(
                rf"^look at up to ({COUNT_TOKEN_RE}) cards? from the top of your deck, and put them on the top of your deck in any order\.?$",
                remainder,
                re.I,
            )
            if act_look_top_reorder:
                count = parse_count_token(act_look_top_reorder.group(1))
                if count is None:
                    mark_unsupported(line)
                    continue
                ability_defs.append(
                    _ability_def(
                        "Activated",
                        None,
                        effects=[_template("LookTopDeckReorder", count=count)],
                        targets=["SelfDeckTop"],
                        cost=cost,
                        target_limit=count,
                    )
                )
                stats.parsed_lines += 1
                stats.emitted_defs["Activated.LookTopDeckReorder"] += 1
                continue

            brainstorm_draw = re.match(
                rf"^Brainstorm\s+(?:Flip over|Reveal)\s+({COUNT_TOKEN_RE})\s+cards?\s+from the top of your deck, and put them into your waiting room\.\s+For each climax revealed(?: among those cards)?,\s+draw up to\s+({COUNT_TOKEN_RE})\s+card(?:s)?\.?$",
                remainder,
                re.I,
            )
            if brainstorm_draw:
                reveal_count = parse_count_token(brainstorm_draw.group(1))
                per_climax = parse_count_token(brainstorm_draw.group(2))
                if reveal_count is None or per_climax is None:
                    mark_unsupported(line)
                    continue
                ability_defs.append(
                    _ability_def(
                        "Activated",
                        None,
                        effects=[
                            _template(
                                "Brainstorm",
                                reveal_count=reveal_count,
                                per_climax=per_climax,
                                mode="Draw",
                            )
                        ],
                        targets=[],
                        cost=cost,
                    )
                )
                stats.parsed_lines += 1
                stats.emitted_defs["Activated.Brainstorm.Draw"] += 1
                continue

            brainstorm_salvage = re.match(
                rf"^Brainstorm\s+(?:Flip over|Reveal)\s+({COUNT_TOKEN_RE})\s+cards?\s+from the top of your deck, and put them into your waiting room\.\s+For each climax revealed(?: among those cards)?,\s+choose up to\s+({COUNT_TOKEN_RE})\s+character in your waiting room, and return (?:it|them) to your hand\.?$",
                remainder,
                re.I,
            )
            if brainstorm_salvage:
                reveal_count = parse_count_token(brainstorm_salvage.group(1))
                per_climax = parse_count_token(brainstorm_salvage.group(2))
                if reveal_count is None or per_climax is None:
                    mark_unsupported(line)
                    continue
                ability_defs.append(
                    _ability_def(
                        "Activated",
                        None,
                        effects=[
                            _template(
                                "Brainstorm",
                                reveal_count=reveal_count,
                                per_climax=per_climax,
                                mode="SalvageCharacter",
                            )
                        ],
                        targets=[],
                        cost=cost,
                    )
                )
                stats.parsed_lines += 1
                stats.emitted_defs["Activated.Brainstorm.SalvageCharacter"] += 1
                continue

            brainstorm_search_to_hand = re.match(
                rf"^Brainstorm\s+(?:Flip over|Reveal)\s+({COUNT_TOKEN_RE})\s+cards?\s+from the top of your deck, and put them into your waiting room\.\s+For each climax revealed(?: among those cards)?,\s+search your deck for up to\s+({COUNT_TOKEN_RE})\s+.+?\s+character, reveal (?:it|them) to your opponent, put (?:it|them) into your hand, and shuffle your deck(?: afterwards)?\.?$",
                remainder,
                re.I,
            )
            if brainstorm_search_to_hand:
                reveal_count = parse_count_token(brainstorm_search_to_hand.group(1))
                per_climax = parse_count_token(brainstorm_search_to_hand.group(2))
                if reveal_count is None or per_climax is None:
                    mark_unsupported(line)
                    continue
                if allow_approx_effects:
                    ability_defs.append(
                        _ability_def(
                            "Activated",
                            None,
                            effects=[
                                _template(
                                    "Brainstorm",
                                    reveal_count=reveal_count,
                                    per_climax=per_climax,
                                    mode="Draw",
                                )
                            ],
                            targets=[],
                            cost=cost,
                            conditions=with_approx_condition(),
                        )
                    )
                    stats.parsed_lines += 1
                    stats.emitted_defs["Activated.Brainstorm.SearchToHand.ApproxDraw"] += 1
                    continue
                mark_unsupported(line)
                continue

            brainstorm_trigger_icon_salvage = re.match(
                rf"^Brainstorm\s+(?:Flip over|Reveal)\s+({COUNT_TOKEN_RE})\s+cards?\s+from the top of your deck, and put them into your waiting room\.\s+For each climax with \[[^\]]+\] in its trigger icon revealed among those cards,\s+choose up to\s+({COUNT_TOKEN_RE})\s+character in your waiting room, and return (?:it|them) to your hand\.?$",
                remainder,
                re.I,
            )
            if brainstorm_trigger_icon_salvage:
                reveal_count = parse_count_token(brainstorm_trigger_icon_salvage.group(1))
                per_climax = parse_count_token(brainstorm_trigger_icon_salvage.group(2))
                if reveal_count is None or per_climax is None:
                    mark_unsupported(line)
                    continue
                if allow_approx_effects:
                    ability_defs.append(
                        _ability_def(
                            "Activated",
                            None,
                            effects=[
                                _template(
                                    "Brainstorm",
                                    reveal_count=reveal_count,
                                    per_climax=per_climax,
                                    mode="SalvageCharacter",
                                )
                            ],
                            targets=[],
                            cost=cost,
                            conditions=with_approx_condition(),
                        )
                    )
                    stats.parsed_lines += 1
                    stats.emitted_defs["Activated.Brainstorm.TriggerIconSalvage.Approx"] += 1
                    continue
                mark_unsupported(line)
                continue

            brainstorm_team_power = re.match(
                rf"^Brainstorm\s+(?:Flip over|Reveal)\s+({COUNT_TOKEN_RE})\s+cards?\s+from the top of your deck, and put them into your waiting room\.\s+For each climax revealed(?: among those cards)?,\s+choose (?:one|1) of your characters, and that character gets \+([+-]?\d+) power until end of turn\.?$",
                remainder,
                re.I,
            )
            if brainstorm_team_power:
                if allow_approx_effects:
                    ability_defs.append(
                        _ability_def(
                            "Activated",
                            None,
                            effects=[_template("Draw", count=0)],
                            targets=[],
                            cost=cost,
                            conditions=with_approx_condition(),
                        )
                    )
                    stats.parsed_lines += 1
                    stats.emitted_defs["Activated.Brainstorm.TeamPower.ApproxNoop"] += 1
                    continue
                mark_unsupported(line)
                continue

            brainstorm_following = re.match(
                rf'^Brainstorm\s+(?:Flip over|Reveal)\s+({COUNT_TOKEN_RE})\s+cards?\s+from the top of your deck, and put them into your waiting room\.\s+For each climax revealed(?: among those cards)?,\s+perform the following action\.\s+"([^"]+)"\.?$',
                remainder,
                re.I,
            )
            if brainstorm_following:
                reveal_count = parse_count_token(brainstorm_following.group(1))
                nested = brainstorm_following.group(2).strip()
                if reveal_count is None:
                    mark_unsupported(line)
                    continue

                draw_match = re.match(
                    rf"^draw up to ({COUNT_TOKEN_RE}) card(?:s)?\.?$",
                    nested,
                    re.I,
                )
                if draw_match:
                    per_climax = parse_count_token(draw_match.group(1))
                    if per_climax is None:
                        mark_unsupported(line)
                        continue
                    ability_defs.append(
                        _ability_def(
                            "Activated",
                            None,
                            effects=[
                                _template(
                                    "Brainstorm",
                                    reveal_count=reveal_count,
                                    per_climax=per_climax,
                                    mode="Draw",
                                )
                            ],
                            targets=[],
                            cost=cost,
                        )
                    )
                    stats.parsed_lines += 1
                    stats.emitted_defs["Activated.Brainstorm.Draw.FollowingAbility"] += 1
                    continue

                draw_match = re.match(
                    rf"^draw ({COUNT_TOKEN_RE}) card(?:s)?\.?$",
                    nested,
                    re.I,
                )
                if draw_match:
                    per_climax = parse_count_token(draw_match.group(1))
                    if per_climax is None:
                        mark_unsupported(line)
                        continue
                    ability_defs.append(
                        _ability_def(
                            "Activated",
                            None,
                            effects=[
                                _template(
                                    "Brainstorm",
                                    reveal_count=reveal_count,
                                    per_climax=per_climax,
                                    mode="Draw",
                                )
                            ],
                            targets=[],
                            cost=cost,
                        )
                    )
                    stats.parsed_lines += 1
                    stats.emitted_defs["Activated.Brainstorm.Draw.FollowingAbility"] += 1
                    continue

                salvage_match = re.match(
                    rf"^choose up to ({COUNT_TOKEN_RE}) character in your waiting room, and return (?:it|them) to your hand\.?$",
                    nested,
                    re.I,
                )
                if salvage_match:
                    per_climax = parse_count_token(salvage_match.group(1))
                    if per_climax is None:
                        mark_unsupported(line)
                        continue
                    ability_defs.append(
                        _ability_def(
                            "Activated",
                            None,
                            effects=[
                                _template(
                                    "Brainstorm",
                                    reveal_count=reveal_count,
                                    per_climax=per_climax,
                                    mode="SalvageCharacter",
                                )
                            ],
                            targets=[],
                            cost=cost,
                        )
                    )
                    stats.parsed_lines += 1
                    stats.emitted_defs[
                        "Activated.Brainstorm.SalvageCharacter.FollowingAbility"
                    ] += 1
                    continue

                look_top_match = re.match(
                    rf"^look at up to ({COUNT_TOKEN_RE}) cards? from the top of your deck, choose up to ({COUNT_TOKEN_RE}) card from among them, put it into your hand, and put the rest into your waiting room\.?$",
                    nested,
                    re.I,
                )
                if look_top_match:
                    look_count = parse_count_token(look_top_match.group(1))
                    choose_count = parse_count_token(look_top_match.group(2))
                    if (
                        look_count is None
                        or choose_count is None
                        or look_count != 3
                        or choose_count != 1
                    ):
                        mark_unsupported(line)
                        continue
                    ability_defs.append(
                        _ability_def(
                            "Activated",
                            None,
                            effects=[
                                _template(
                                    "Brainstorm",
                                    reveal_count=reveal_count,
                                    per_climax=1,
                                    mode="LookTopToHand",
                                )
                            ],
                            targets=[],
                            cost=cost,
                        )
                    )
                    stats.parsed_lines += 1
                    stats.emitted_defs["Activated.Brainstorm.LookTopToHand.FollowingAbility"] += 1
                    continue

                look_top_discard_match = re.match(
                    rf"^look at up to ({COUNT_TOKEN_RE}) cards? from the top of your deck, choose up to ({COUNT_TOKEN_RE}) card from among them, put it into your hand, put the rest into your waiting room, choose 1 card in your hand, and put it into your waiting room\.?$",
                    nested,
                    re.I,
                )
                if look_top_discard_match:
                    look_count = parse_count_token(look_top_discard_match.group(1))
                    choose_count = parse_count_token(look_top_discard_match.group(2))
                    if (
                        look_count is None
                        or choose_count is None
                        or look_count != 3
                        or choose_count != 1
                    ):
                        mark_unsupported(line)
                        continue
                    ability_defs.append(
                        _ability_def(
                            "Activated",
                            None,
                            effects=[
                                _template(
                                    "Brainstorm",
                                    reveal_count=reveal_count,
                                    per_climax=1,
                                    mode="LookTopToHandThenDiscard",
                                )
                            ],
                            targets=[],
                            cost=cost,
                        )
                    )
                    stats.parsed_lines += 1
                    stats.emitted_defs[
                        "Activated.Brainstorm.LookTopToHandThenDiscard.FollowingAbility"
                    ] += 1
                    continue

                salvage_discard_match = re.match(
                    r"^choose 1 (?:《[^》]+》(?: or 《[^》]+》)? )?character in your waiting room, return it to your hand, choose 1 card in your hand, and put it into your waiting room\.?$",
                    nested,
                    re.I,
                )
                if salvage_discard_match:
                    ability_defs.append(
                        _ability_def(
                            "Activated",
                            None,
                            effects=[
                                _template(
                                    "Brainstorm",
                                    reveal_count=reveal_count,
                                    per_climax=1,
                                    mode="SalvageCharacterThenDiscard",
                                )
                            ],
                            targets=[],
                            cost=cost,
                        )
                    )
                    stats.parsed_lines += 1
                    stats.emitted_defs[
                        "Activated.Brainstorm.SalvageCharacterThenDiscard.FollowingAbility"
                    ] += 1
                    continue

            act_draw_then_discard = re.match(
                rf"^draw ({COUNT_TOKEN_RE}) card(?:s)?, choose ({COUNT_TOKEN_RE}) card in your hand, and put it into your waiting room\.?$",
                remainder,
                re.I,
            )
            if act_draw_then_discard:
                draw_count = parse_count_token(act_draw_then_discard.group(1))
                discard_count = parse_count_token(act_draw_then_discard.group(2))
                if draw_count is None or discard_count is None:
                    mark_unsupported(line)
                    continue
                ability_defs.append(
                    _ability_def(
                        "Activated",
                        None,
                        effects=[
                            _template("Draw", count=draw_count),
                            _template("MoveToWaitingRoom"),
                        ],
                        targets=["This", "SelfHand"],
                        cost=cost,
                        target_limit=discard_count,
                    )
                )
                stats.parsed_lines += 1
                stats.emitted_defs["Activated.DrawThenDiscardFromHand"] += 1
                continue

            act_salvage = re.match(
                rf"^choose (up to )?({COUNT_TOKEN_RE}) (.+?) in your waiting room, and return (?:it|them) to your hand\.?$",
                remainder,
                re.I,
            )
            if act_salvage:
                choose_count = parse_count_token(act_salvage.group(2))
                if choose_count is None:
                    mark_unsupported(line)
                    continue
                optional = bool(act_salvage.group(1))
                selector = act_salvage.group(3).strip()
                selector_lower = selector.lower()
                card_type_hint = parse_card_type_hint(selector)
                target_trait = None
                trait_matches = [trait.strip() for trait in re.findall(r"《([^》]+)》", selector)]
                if len(trait_matches) == 1:
                    target_trait = (trait_map or {}).get(trait_matches[0])
                    if target_trait is None:
                        mark_unsupported(line)
                        continue
                target_level_max = None
                level_match = re.search(r"level (\d+) or lower", selector_lower)
                if level_match:
                    target_level_max = int(level_match.group(1))
                ability_defs.append(
                    _ability_def(
                        "Activated",
                        None,
                        effects=[_template("MoveToHand")],
                        targets=["SelfWaitingRoom"],
                        cost=cost,
                        effect_optional=[optional] if optional else [],
                        target_card_type=card_type_hint,
                        target_trait=target_trait,
                        target_level_max=target_level_max,
                        target_limit=choose_count,
                    )
                )
                stats.parsed_lines += 1
                stats.emitted_defs["Activated.SalvageWaitingRoom"] += 1
                continue

            act_wr_to_source_slot = re.match(
                rf'^Choose ({COUNT_TOKEN_RE}) "([^"]+)" in your waiting room, and put it on the stage position that this card was on\.?$',
                remainder,
                re.I,
            )
            if act_wr_to_source_slot:
                choose_count = parse_count_token(act_wr_to_source_slot.group(1))
                if choose_count is None:
                    mark_unsupported(line)
                    continue
                named = act_wr_to_source_slot.group(2).strip()
                target_ids = sorted(set((name_to_ids or {}).get(named, [])))
                if not target_ids:
                    mark_unsupported(line)
                    continue
                ability_defs.append(
                    _ability_def(
                        "Activated",
                        None,
                        effects=[
                            _template("MoveWaitingRoomCardToSourceSlot", target_ids=target_ids)
                        ],
                        targets=["SelfWaitingRoom"],
                        cost=cost,
                        target_limit=choose_count,
                    )
                )
                stats.parsed_lines += 1
                stats.emitted_defs["Activated.MoveWaitingRoomCardToSourceSlot"] += 1
                continue

            act_following = re.match(
                rf'^choose (up to )?({COUNT_TOKEN_RE}) of your characters, and that character gets the following ability until end of turn\.\s*"(.+)"\.?$',
                remainder,
                re.I,
            )
            if act_following:
                choose_count = parse_count_token(act_following.group(2))
                if choose_count is None:
                    mark_unsupported(line)
                    continue
                nested_effect = parse_following_effect_or_grant(
                    act_following.group(3),
                    duration_turn=True,
                    grant_duration="UntilEndOfTurn",
                )
                if nested_effect is not None:
                    ability_defs.append(
                        _ability_def(
                            "Activated",
                            None,
                            effects=[nested_effect],
                            targets=["SelfStage"],
                            cost=cost,
                            effect_optional=([True] if act_following.group(1) else []),
                            target_limit=choose_count,
                        )
                    )
                    stats.parsed_lines += 1
                    stats.emitted_defs["Activated.FollowingAbility.Flattened"] += 1
                    continue
                if allow_approx_effects:
                    ability_defs.append(
                        _ability_def(
                            "Activated",
                            None,
                            effects=[_template("Draw", count=0)],
                            targets=[],
                            cost=cost,
                            conditions=with_approx_condition(),
                        )
                    )
                    stats.parsed_lines += 1
                    stats.emitted_defs["Activated.FollowingAbility.ApproxNoop"] += 1
                    continue

            match = re.match(
                rf"^choose (?:up to )?({COUNT_TOKEN_RE}) of your(?: (.*?))? characters, and that character gets \+([+-]?\d+) power until end of turn\.?$",
                remainder,
                re.I,
            )
            if match:
                choose_count = parse_count_token(match.group(1))
                if choose_count is None:
                    mark_unsupported(line)
                    continue
                selector = (match.group(2) or "").strip()
                selector_other = False
                selector_base = selector
                if selector.lower().startswith("other "):
                    selector_other = True
                    selector_base = selector[len("other ") :].strip()
                target_trait = resolve_single_trait_selector(selector_base)
                target_card_ids = []
                selector_compact = re.sub(r"\s+", " ", selector_base.lower()).strip()
                generic_selector = selector_compact == ""
                if not generic_selector and target_trait is None:
                    target_card_ids = resolve_stage_selector_card_ids(selector_base) or []
                    if not target_card_ids:
                        if allow_approx_effects:
                            ability_defs.append(
                                _ability_def(
                                    "Activated",
                                    None,
                                    effects=[_template("Draw", count=0)],
                                    targets=[],
                                    cost=cost,
                                    conditions=with_approx_condition(),
                                )
                            )
                            stats.parsed_lines += 1
                            stats.emitted_defs["Activated.AddPower.Selector.ApproxNoop"] += 1
                            continue
                        mark_unsupported(line)
                        continue
                effects: List[Any]
                if selector_other:
                    effects = [
                        _template(
                            "TimedConditionalAddPower",
                            amount=int(match.group(3)),
                            duration_turn=True,
                            turn=None,
                            zone_count=None,
                            require_source_marker=False,
                            per_source_marker=False,
                            per_zone_count=False,
                            exclude_source=True,
                            target_ids=[],
                        )
                    ]
                else:
                    effects = [
                        _template("AddPower", amount=int(match.group(3)), duration_turn=True)
                    ]
                ability_defs.append(
                    _ability_def(
                        "Activated",
                        None,
                        effects=effects,
                        targets=["SelfStage"],
                        cost=cost,
                        effect_optional=(
                            [True] if re.search(r"\bup to\b", remainder, re.I) else []
                        ),
                        target_trait=target_trait,
                        target_card_ids=target_card_ids,
                        target_limit=choose_count,
                    )
                )
                stats.parsed_lines += 1
                stats.emitted_defs["Activated.AddPower"] += 1
                continue

            match = re.match(
                rf"^choose (?:up to )?({COUNT_TOKEN_RE}) of your(?: (.*?))? characters, and that character gets \+([+-]?\d+) level and \+([+-]?\d+) power until end of turn\.?$",
                remainder,
                re.I,
            )
            if match:
                choose_count = parse_count_token(match.group(1))
                if choose_count is None:
                    mark_unsupported(line)
                    continue
                selector = (match.group(2) or "").strip()
                selector_base = (
                    selector[len("other ") :].strip()
                    if selector.lower().startswith("other ")
                    else selector
                )
                target_trait = resolve_single_trait_selector(selector_base)
                target_card_ids = []
                selector_compact = re.sub(r"\s+", " ", selector.lower()).strip()
                if selector_compact not in {"", "other"} and target_trait is None:
                    target_card_ids = resolve_stage_selector_card_ids(selector_base) or []
                    if not target_card_ids:
                        if allow_approx_effects:
                            ability_defs.append(
                                _ability_def(
                                    "Activated",
                                    None,
                                    effects=[_template("Draw", count=0)],
                                    targets=[],
                                    cost=cost,
                                    conditions=with_approx_condition(),
                                )
                            )
                            stats.parsed_lines += 1
                            stats.emitted_defs["Activated.AddLevelPower.Selector.ApproxNoop"] += 1
                            continue
                        mark_unsupported(line)
                        continue
                ability_defs.append(
                    _ability_def(
                        "Activated",
                        None,
                        effects=[
                            _template(
                                "AddLevel",
                                amount=int(match.group(3)),
                                duration_turn=True,
                            ),
                            _template(
                                "AddPower",
                                amount=int(match.group(4)),
                                duration_turn=True,
                            ),
                        ],
                        targets=["SelfStage", "SelfStage"],
                        cost=cost,
                        target_trait=target_trait,
                        target_card_ids=target_card_ids,
                        target_limit=choose_count,
                    )
                )
                stats.parsed_lines += 1
                stats.emitted_defs["Activated.AddLevelPower"] += 1
                continue

            match = re.match(
                rf"^choose (?:up to )?({COUNT_TOKEN_RE}) of your characters, and that character gets \+(\d+) power until the end of your opponent's next turn\.?$",
                remainder,
                re.I,
            )
            if match:
                choose_count = parse_count_token(match.group(1))
                if choose_count is None:
                    mark_unsupported(line)
                    continue
                granted = _ability_def(
                    "Continuous",
                    None,
                    effects=[
                        _template(
                            "AddPower",
                            amount=int(match.group(2)),
                            duration_turn=False,
                        )
                    ],
                    targets=["This"],
                )
                ability_defs.append(
                    _ability_def(
                        "Activated",
                        None,
                        effects=[
                            _template(
                                "GrantAbilityDef",
                                ability=granted,
                                duration="UntilEndOfOpponentsNextTurn",
                            )
                        ],
                        targets=["SelfStage"],
                        cost=cost,
                        effect_optional=(
                            [True] if re.search(r"\bup to\b", remainder, re.I) else []
                        ),
                        target_limit=choose_count,
                    )
                )
                stats.parsed_lines += 1
                stats.emitted_defs["Activated.AddPower.OpponentNextTurn"] += 1
                continue

            match = re.match(
                rf"^choose (?:up to )?({COUNT_TOKEN_RE}) of your opponent's characters, and that character gets -(\d+) power until end of turn\.?$",
                remainder,
                re.I,
            )
            if match:
                choose_count = parse_count_token(match.group(1))
                if choose_count is None:
                    mark_unsupported(line)
                    continue
                ability_defs.append(
                    _ability_def(
                        "Activated",
                        None,
                        effects=[
                            _template("AddPower", amount=-int(match.group(2)), duration_turn=True)
                        ],
                        targets=["OppStage"],
                        cost=cost,
                        effect_optional=(
                            [True] if re.search(r"\bup to\b", remainder, re.I) else []
                        ),
                        target_limit=choose_count,
                    )
                )
                stats.parsed_lines += 1
                stats.emitted_defs["Activated.ReducePower.OppStage"] += 1
                continue

            match = re.match(
                rf"^choose (?:up to )?({COUNT_TOKEN_RE}) character in your opponent's center stage, and that character gets -(\d+) power until end of turn\.?$",
                remainder,
                re.I,
            )
            if match:
                choose_count = parse_count_token(match.group(1))
                if choose_count is None:
                    mark_unsupported(line)
                    continue
                ability_defs.append(
                    _ability_def(
                        "Activated",
                        None,
                        effects=[
                            _template("AddPower", amount=-int(match.group(2)), duration_turn=True)
                        ],
                        targets=["OppFrontRow"],
                        cost=cost,
                        effect_optional=(
                            [True] if re.search(r"\bup to\b", remainder, re.I) else []
                        ),
                        target_limit=choose_count,
                    )
                )
                stats.parsed_lines += 1
                stats.emitted_defs["Activated.ReducePower.OppCenter"] += 1
                continue

            match = re.match(
                rf"^choose (up to )?({COUNT_TOKEN_RE}) of your opponent's characters, and return it to their hand\.?$",
                remainder,
                re.I,
            )
            if match:
                choose_count = parse_count_token(match.group(2))
                if choose_count is None:
                    mark_unsupported(line)
                    continue
                ability_defs.append(
                    _ability_def(
                        "Activated",
                        None,
                        effects=[_template("MoveToHand")],
                        targets=["OppStage"],
                        cost=cost,
                        effect_optional=([True] if match.group(1) else []),
                        target_limit=choose_count,
                    )
                )
                stats.parsed_lines += 1
                stats.emitted_defs["Activated.BounceOpponentStage"] += 1
                continue

            match = re.match(
                r"^this card gets \+(\d+) power until end of turn\.?$",
                remainder,
                re.I,
            )
            if match:
                ability_defs.append(
                    _ability_def(
                        "Activated",
                        None,
                        effects=[
                            _template("AddPower", amount=int(match.group(1)), duration_turn=True)
                        ],
                        targets=["This"],
                        cost=cost,
                    )
                )
                stats.parsed_lines += 1
                stats.emitted_defs["Activated.AddPower.Self"] += 1
                continue

            match = re.match(
                r"^this card gets \+(\d+) soul until end of turn\.?$",
                remainder,
                re.I,
            )
            if match:
                ability_defs.append(
                    _ability_def(
                        "Activated",
                        None,
                        effects=[
                            _template("AddSoul", amount=int(match.group(1)), duration_turn=True)
                        ],
                        targets=["This"],
                        cost=cost,
                    )
                )
                stats.parsed_lines += 1
                stats.emitted_defs["Activated.AddSoul.Self"] += 1
                continue

            match = re.match(
                r'^choose one of your characters with "([^"]+)" in its card name, and that character gets \+(\d+) power until end of turn\.?$',
                remainder,
                re.I,
            )
            if match:
                target_ids = resolve_name_fragment_ids(name_to_ids, match.group(1).strip())
                if not target_ids:
                    mark_unsupported(line)
                    continue
                ability_defs.append(
                    _ability_def(
                        "Activated",
                        None,
                        effects=[
                            _template(
                                "TimedConditionalAddPower",
                                amount=int(match.group(2)),
                                duration_turn=True,
                                turn=None,
                                zone_count=None,
                                require_source_marker=False,
                                per_source_marker=False,
                                per_zone_count=False,
                                exclude_source=False,
                                target_ids=target_ids,
                            )
                        ],
                        targets=["SelfStage"],
                        cost=cost,
                        target_limit=1,
                    )
                )
                stats.parsed_lines += 1
                stats.emitted_defs["Activated.AddPower.NamedFragment"] += 1
                continue

            match = re.match(
                r"^choose one of your 《([^》]+)》 characters, and that character gets \+(\d+) power until end of turn\.?$",
                remainder,
                re.I,
            )
            if match:
                trait_id = (trait_map or {}).get(match.group(1).strip())
                if trait_id is None:
                    mark_unsupported(line)
                    continue
                ability_defs.append(
                    _ability_def(
                        "Activated",
                        None,
                        effects=[
                            _template("AddPower", amount=int(match.group(2)), duration_turn=True)
                        ],
                        targets=["SelfStage"],
                        cost=cost,
                        target_trait=trait_id,
                        target_limit=1,
                    )
                )
                stats.parsed_lines += 1
                stats.emitted_defs["Activated.AddPower.TraitOne"] += 1
                continue

            if allow_approx_effects:
                match = re.match(
                    r"^choose another character, and that character gets \+(\d+) level until the end of your opponent's next turn\.?$",
                    remainder,
                    re.I,
                )
                if match:
                    ability_defs.append(
                        _ability_def(
                            "Activated",
                            None,
                            effects=[_template("Draw", count=0)],
                            targets=[],
                            cost=cost,
                            conditions=with_approx_condition(),
                        )
                    )
                    stats.parsed_lines += 1
                    stats.emitted_defs["Activated.AddLevelAnother.OppNextTurn.ApproxNoop"] += 1
                    continue

            act_look_reorder = re.match(
                rf"^Look at up to ({COUNT_TOKEN_RE}) cards? from the top of your deck, and put them on the top of your deck in any order\.?$",
                remainder,
                re.I,
            )
            if act_look_reorder:
                look_count = parse_count_token(act_look_reorder.group(1))
                if look_count is None:
                    mark_unsupported(line)
                    continue
                ability_defs.append(
                    _ability_def(
                        "Activated",
                        None,
                        effects=[_template("LookTopDeckReorder", count=look_count)],
                        targets=["SelfDeckTop"],
                        cost=cost,
                        target_limit=look_count,
                    )
                )
                stats.parsed_lines += 1
                stats.emitted_defs["Activated.LookTopReorder"] += 1
                continue

            if try_parser_v2_fallback(line):
                continue
            mark_unsupported(line)
            continue

        if try_parser_v2_fallback(line):
            continue
        mark_unsupported(line)

    # Event fallback: simple damage-only event text without tags
    if card_type == "Event" and not abilities and not ability_defs:
        if text.strip() and not any(line.startswith("【") for line in lines):
            dmg = re.match(
                r"^(?:you )?deal (\d+) damage to your opponent\.?$",
                text.strip(),
                re.I,
            )
            if dmg:
                cancelable = "cannot be canceled" not in text.lower()
                abilities.append(
                    _template(
                        "EventDealDamage",
                        amount=int(dmg.group(1)),
                        cancelable=cancelable,
                    )
                )
                stats.emitted_templates["EventDealDamage"] += 1

    return abilities, ability_defs, counter_timing
