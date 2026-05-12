import re
from collections import Counter
from dataclasses import dataclass
from typing import Any, Dict, List, Optional

COUNT_TOKEN_RE = r"(?:\d+|a|an|one|two|three|four|five|six|seven|eight|nine|ten)"
NON_COST_BRACKET_TOKENS = {
    "soul",
    "soul2",
    "draw",
    "shot",
    "gate",
    "bounce",
    "standby",
    "treasure",
    "comeback",
    "salvage",
    "return",
    "choice",
    "pool",
}


@dataclass
class AbilityParseStats:
    total_lines: int = 0
    parsed_lines: int = 0
    emitted_templates: Counter = None
    emitted_defs: Counter = None
    unsupported_signatures: Counter = None
    parse_traces: List[Dict[str, Any]] = None

    def __post_init__(self) -> None:
        if self.emitted_templates is None:
            self.emitted_templates = Counter()
        if self.emitted_defs is None:
            self.emitted_defs = Counter()
        if self.unsupported_signatures is None:
            self.unsupported_signatures = Counter()
        if self.parse_traces is None:
            self.parse_traces = []


RULE_MODE_EXACT = "exact"
RULE_MODE_APPROX = "approx"
APPROX_PROFILE_STRICT = "strict"
APPROX_PROFILE_APPROX = "approx"
APPROX_PROFILES = {APPROX_PROFILE_STRICT, APPROX_PROFILE_APPROX}
APPROX_PROFILE_CLI_CHOICES = sorted(APPROX_PROFILES)


@dataclass(frozen=True)
class ParseRule:
    id: str
    pattern: re.Pattern[str]
    mode: str
    risk_class: str


def ability_signature(text: str) -> str:
    sig = text.strip()
    sig = re.sub(r'"[^"]+"', '"<Q>"', sig)
    sig = re.sub(r"\d+", "#", sig)
    sig = re.sub(r"\s+", " ", sig).strip()
    return sig


def resolve_name_fragment_ids(
    name_to_ids: Optional[Dict[str, List[int]]],
    fragment: str,
) -> List[int]:
    if not name_to_ids:
        return []
    raw = fragment.strip().lower()
    if not raw:
        return []
    stripped = re.sub(r"\([^)]*\)|（[^）]*）", "", raw).strip()
    variants = {raw}
    if stripped:
        variants.add(stripped)
    compact_variants = {re.sub(r"[^0-9a-z]+", "", token) for token in variants if token}
    matched: List[int] = []
    for name, ids in name_to_ids.items():
        name_lower = name.lower()
        name_compact = re.sub(r"[^0-9a-z]+", "", name_lower)
        if any(token in name_lower for token in variants if token):
            matched.extend(ids)
            continue
        if any(token and token in name_compact for token in compact_variants):
            matched.extend(ids)
    return sorted(set(matched))


def parse_count_token(token: str) -> Optional[int]:
    t = token.strip().lower()
    if not t:
        return None
    if t.isdigit():
        return int(t)
    if t in {"a", "an", "one"}:
        return 1
    words = {
        "two": 2,
        "three": 3,
        "four": 4,
        "five": 5,
        "six": 6,
        "seven": 7,
        "eight": 8,
        "nine": 9,
        "ten": 10,
    }
    return words.get(t)


def normalize_approx_profile(profile: Optional[str]) -> str:
    normalized = (profile or APPROX_PROFILE_STRICT).strip().lower()
    if normalized not in APPROX_PROFILES:
        raise ValueError(
            f"unsupported approx profile '{profile}', expected one of: {APPROX_PROFILE_CLI_CHOICES}"
        )
    return normalized


def _template(name: str, **fields: Any) -> Any:
    if fields:
        return {name: fields}
    return name


def _ability_def(
    kind: str,
    timing: Optional[str],
    effects: List[Any],
    targets: List[str],
    cost: Optional[Dict[str, Any]] = None,
    conditions: Optional[Dict[str, Any]] = None,
    effect_optional: Optional[List[bool]] = None,
    target_card_type: Optional[str] = None,
    target_trait: Optional[int] = None,
    target_level_max: Optional[int] = None,
    target_cost_max: Optional[int] = None,
    target_card_ids: Optional[List[int]] = None,
    target_limit: Optional[int] = None,
) -> Dict[str, Any]:
    out = {
        "kind": kind,
        "timing": timing,
        "effects": effects,
        "effect_optional": effect_optional or [],
        "targets": targets,
        "cost": cost
        or {
            "stock": 0,
            "rest_self": False,
            "rest_other": 0,
            "discard_from_hand": 0,
            "clock_from_hand": 0,
            "clock_from_deck_top": 0,
            "reveal_from_hand": 0,
        },
        "conditions": conditions or {},
        "target_card_type": target_card_type,
        "target_trait": target_trait,
        "target_level_max": target_level_max,
        "target_cost_max": target_cost_max,
        "target_limit": target_limit,
    }
    if target_card_ids:
        out["target_card_ids"] = target_card_ids
    return out
