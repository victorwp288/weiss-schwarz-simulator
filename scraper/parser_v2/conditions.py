from __future__ import annotations

import re
from typing import Any, Dict, List, Optional

from .cost import COUNT_TOKEN_RE, parse_count_token
from .models import RULE_MODE_APPROX, RULE_MODE_EXACT


def kind_from_tag(tag: Optional[str]) -> str:
    if tag == "CONT":
        return "Continuous"
    if tag == "AUTO":
        return "Auto"
    if tag == "ACT":
        return "Activated"
    return "Auto"


def infer_timing(tag: Optional[str], body: str) -> Optional[str]:
    if tag == "CONT":
        return None
    if tag == "ACT":
        return None
    lower = body.lower().strip()
    if lower.startswith("when this card is placed on the stage from your hand") or lower.startswith(
        "when this card is placed on stage from your hand"
    ):
        return "OnPlay"
    if lower.startswith("when this card attacks") or lower.startswith(
        "when this card direct attacks"
    ):
        return "AttackDeclaration"
    if lower.startswith("when your other ") and " attacks" in lower:
        return "OtherAttackDeclaration"
    if lower.startswith("when this card's battle opponent becomes 【reverse】"):
        return "BattleOpponentReverse"
    if lower.startswith("when this card becomes 【reverse】"):
        return "OnReverse"
    if lower.startswith("at the beginning of your opponent's attack phase"):
        return "BeginAttackPhase"
    if lower.startswith("at the beginning of your opponent's draw phase"):
        return "BeginDrawPhase"
    if lower.startswith("at the beginning of your climax phase"):
        return "BeginClimaxPhase"
    if lower.startswith("when you use an 【act】"):
        return "UseAct"
    if lower.startswith("when you use this card's"):
        return "UseAct"
    return None


def infer_targets(body: str) -> List[str]:
    lower = body.lower()
    if "all of your opponent's characters" in lower:
        return ["OppStage"]
    if "all of your characters" in lower:
        return ["SelfStage"]
    if "the character facing this card" in lower:
        return ["OppFrontRow"]
    if "choose" in lower and "your opponent's" in lower and "character" in lower:
        return ["OppStage"]
    if "choose" in lower and "your characters" in lower:
        return ["SelfStage"]
    if (
        "look at the top card of your deck" in lower
        and "put it on the top of your deck or into your waiting room" in lower
    ):
        return ["SelfDeckTop"]
    if (
        "put this card into your memory" in lower
        or "put this card at the bottom of your deck" in lower
    ):
        return ["This"]
    if "this card gets" in lower:
        return ["This"]
    return []


def infer_target_limit(body: str) -> Optional[int]:
    match = re.search(rf"\bchoose (?:up to )?({COUNT_TOKEN_RE})\b", body, re.I)
    if not match:
        return None
    return parse_count_token(match.group(1))


def infer_effect_optional(body: str) -> List[bool]:
    return [True] if re.search(r"\byou may\b|\bup to\b", body, re.I) else []


def build_conditions(
    mode: str,
    rule_id: str,
    extra: Optional[Dict[str, Any]] = None,
) -> Dict[str, Any]:
    if mode not in {RULE_MODE_EXACT, RULE_MODE_APPROX}:
        mode = RULE_MODE_APPROX
    out: Dict[str, Any] = dict(extra or {})
    if mode == RULE_MODE_APPROX:
        out["requires_approx_effects"] = True
    out["source_rule_id"] = rule_id
    return out
