from __future__ import annotations

from typing import Dict, Iterable, List, Optional

from .cost import parse_cost
from .emit import emit_ability_def
from .models import ParseContext, ParseOutcome, RuleMatch
from .normalize import build_ability_line
from .registry import CompiledRule, load_compiled_rules


PARSER_VERSION_V2 = "v2"

_RULES_CACHE: Optional[List[CompiledRule]] = None


def _rules() -> List[CompiledRule]:
    global _RULES_CACHE
    if _RULES_CACHE is None:
        _RULES_CACHE = load_compiled_rules()
    return _RULES_CACHE


def _candidate_text(rule: CompiledRule, line_normalized: str, body: str) -> str:
    if rule.match_on == "line":
        return line_normalized
    return body


def parse_line(
    raw_line: str,
    card_type: str,
    source_card_id: Optional[int] = None,
    emit_trace: bool = False,
    allow_approx_rules: bool = True,
    rules: Optional[Iterable[CompiledRule]] = None,
) -> ParseOutcome:
    trace: List[Dict[str, object]] = []
    line = build_ability_line(raw_line)
    if not line.tag:
        return ParseOutcome(matched=False, trace=trace)

    parsed_cost, cost_supported, line_without_cost = parse_cost(line.normalized)
    clean_line = build_ability_line(line_without_cost)
    context = ParseContext(
        card_type=card_type,
        line=clean_line,
        source_card_id=source_card_id,
        emit_trace=emit_trace,
    )

    for rule in list(rules) if rules is not None else _rules():
        if rule.mode == "approx" and not allow_approx_rules:
            continue
        candidate = _candidate_text(rule, clean_line.normalized, clean_line.body)
        match = rule.regex.match(candidate)
        if emit_trace:
            trace.append(
                {
                    "rule_id": rule.id,
                    "matched": bool(match),
                    "candidate": candidate,
                }
            )
        if not match:
            continue
        groups = {key: value for key, value in match.groupdict().items() if value is not None}
        if not groups:
            for idx, value in enumerate(match.groups(), start=1):
                if value is not None:
                    groups[f"group_{idx}"] = value

        ability_def = emit_ability_def(
            context=context,
            rule=rule,
            groups=groups,
            parsed_cost=parsed_cost,
            cost_supported=cost_supported,
        )
        rule_match = RuleMatch(
            rule_id=rule.id,
            mode=rule.mode,
            priority=rule.priority,
            pattern=rule.pattern,
            metadata=dict(rule.metadata),
            groups=groups,
        )
        matched_trace = list(trace)
        if emit_trace:
            matched_trace.append(
                {
                    "selected_rule_id": rule.id,
                    "source_card_id": source_card_id,
                    "line": clean_line.normalized,
                }
            )
        return ParseOutcome(
            matched=True,
            ability_def=ability_def,
            rule_match=rule_match,
            trace=matched_trace,
        )

    return ParseOutcome(matched=False, trace=trace)
