from __future__ import annotations

from dataclasses import dataclass, field
from typing import Any, Dict, List, Optional


RULE_MODE_EXACT = "exact"
RULE_MODE_APPROX = "approx"
RULE_MODES = {RULE_MODE_EXACT, RULE_MODE_APPROX}


@dataclass(frozen=True)
class AbilityLine:
    raw: str
    normalized: str
    tag: Optional[str]
    body: str
    has_cxcombo_tag: bool = False


@dataclass(frozen=True)
class Clause:
    raw: str
    normalized: str


@dataclass(frozen=True)
class ParseContext:
    card_type: str
    line: AbilityLine
    source_card_id: Optional[int] = None
    emit_trace: bool = False


@dataclass(frozen=True)
class RuleMatch:
    rule_id: str
    mode: str
    priority: int
    pattern: str
    metadata: Dict[str, Any] = field(default_factory=dict)
    groups: Dict[str, str] = field(default_factory=dict)


@dataclass
class ParseOutcome:
    matched: bool
    ability_def: Optional[Dict[str, Any]] = None
    rule_match: Optional[RuleMatch] = None
    trace: List[Dict[str, Any]] = field(default_factory=list)
