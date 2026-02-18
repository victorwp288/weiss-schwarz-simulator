from __future__ import annotations

import re
from typing import Any, Dict, List, Optional, Tuple


COUNT_TOKEN_RE = r"(?:\d+|a|an|one|two|three|four|five|six|seven|eight|nine|ten)"
_STAGED_COST_STEP_ORDER = (
    "RestOther",
    "SacrificeFromStage",
    "DiscardFromHand",
    "ClockFromHand",
    "ClockFromDeckTop",
    "RevealFromHand",
)


def default_cost() -> Dict[str, Any]:
    return {
        "stock": 0,
        "rest_self": False,
        "rest_other": 0,
        "sacrifice_from_stage": 0,
        "discard_from_hand": 0,
        "clock_from_hand": 0,
        "clock_from_deck_top": 0,
        "reveal_from_hand": 0,
        "move_self_to_waiting_room": False,
        "return_self_to_hand": False,
    }


def parse_count_token(token: str) -> Optional[int]:
    value = token.strip().lower()
    if not value:
        return None
    if value.isdigit():
        return int(value)
    words = {
        "a": 1,
        "an": 1,
        "one": 1,
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
    return words.get(value)


def _extract_bracket_segments(line: str) -> List[Tuple[int, int]]:
    spans: List[Tuple[int, int]] = []
    in_quotes = False
    depth = 0
    start_idx: Optional[int] = None
    for idx, ch in enumerate(line):
        if ch == '"':
            in_quotes = not in_quotes
            continue
        if in_quotes:
            continue
        if ch == "[":
            if depth == 0:
                start_idx = idx
            depth += 1
            continue
        if ch == "]" and depth > 0:
            depth -= 1
            if depth == 0 and start_idx is not None:
                spans.append((start_idx, idx))
                start_idx = None
    return spans


def _consume_counted(cost: Dict[str, Any], seg: str, pattern: str, field: str) -> str:
    def repl(match: re.Match[str]) -> str:
        count = parse_count_token(match.group(1))
        if count is None:
            return match.group(0)
        cost[field] = int(cost.get(field, 0)) + count
        return " "

    return re.sub(pattern, repl, seg, flags=re.I)


def _build_cost_step(name: str, count: Optional[int] = None) -> Dict[str, Any]:
    if count is None:
        return {name: {}}
    return {name: {"count": int(count)}}


def _extract_ordered_cost_steps(seg: str) -> List[Dict[str, Any]]:
    text = seg.lower()

    candidates: List[Tuple[int, int, int, Dict[str, Any]]] = []

    def add_simple(pattern: str, step_name: str, *, priority: int = 10) -> None:
        for match in re.finditer(pattern, text, re.I):
            candidates.append(
                (
                    match.start(),
                    match.end(),
                    priority,
                    _build_cost_step(step_name),
                )
            )

    def add_counted(pattern: str, step_name: str, *, priority: int = 10) -> None:
        for match in re.finditer(pattern, text, re.I):
            count = parse_count_token(match.group(1))
            if count is None:
                continue
            candidates.append(
                (
                    match.start(),
                    match.end(),
                    priority,
                    _build_cost_step(step_name, count=count),
                )
            )

    for match in re.finditer(r"\((\d+)\)", text):
        candidates.append(
            (
                match.start(),
                match.end(),
                30,
                _build_cost_step("PayStock", count=int(match.group(1))),
            )
        )

    add_simple(r"【rest】\s*this card(?: from 【stand】)?", "RestSelf", priority=25)
    add_counted(
        rf"【rest】\s*({COUNT_TOKEN_RE})\s*of your(?: [^\]]*?)? characters",
        "RestOther",
        priority=24,
    )
    add_simple(r"put this card into your waiting room", "MoveSelfToWaitingRoom", priority=22)
    add_simple(r"put this card in your waiting room", "MoveSelfToWaitingRoom", priority=22)
    add_simple(r"return this card to your hand", "ReturnSelfToHand", priority=22)
    add_simple(
        r"put another [^\]]+ from your stage into your waiting room",
        "SacrificeFromStage",
        priority=20,
    )
    add_counted(
        rf"put ({COUNT_TOKEN_RE}) [^\]]*? from your stage into your waiting room",
        "SacrificeFromStage",
        priority=19,
    )
    add_counted(
        rf"put ({COUNT_TOKEN_RE}) .*? from your hand into your waiting room",
        "DiscardFromHand",
        priority=18,
    )
    add_counted(
        rf"put ({COUNT_TOKEN_RE}) card(?:s)? from your hand into your clock",
        "ClockFromHand",
        priority=17,
    )
    add_counted(
        rf"put the top ({COUNT_TOKEN_RE}) card(?:s)? of your deck into your clock",
        "ClockFromDeckTop",
        priority=16,
    )
    add_counted(
        rf"put ({COUNT_TOKEN_RE}) card(?:s)? from the top of your deck into your clock",
        "ClockFromDeckTop",
        priority=16,
    )
    add_simple(
        r"put the top card of your deck into your clock",
        "ClockFromDeckTop",
        priority=16,
    )
    add_counted(
        rf"reveal ({COUNT_TOKEN_RE}) .*? from your hand",
        "RevealFromHand",
        priority=15,
    )

    if not candidates:
        return []

    occupied = [False] * len(text)
    selected: List[Tuple[int, int, Dict[str, Any]]] = []
    for start, end, priority, step in sorted(
        candidates,
        key=lambda item: (-item[2], item[0], -(item[1] - item[0])),
    ):
        if any(occupied[idx] for idx in range(start, end)):
            continue
        for idx in range(start, end):
            occupied[idx] = True
        selected.append((start, end, step))

    selected.sort(key=lambda item: item[0])
    return [step for _, _, step in selected]


def _extract_explicit_step_order(
    cost: Dict[str, Any],
    cost_steps: List[Dict[str, Any]],
) -> List[str]:
    ordered_stage_steps: List[str] = []
    for step in cost_steps:
        if not isinstance(step, dict) or len(step) != 1:
            continue
        step_name = next(iter(step))
        if step_name not in _STAGED_COST_STEP_ORDER:
            continue
        ordered_stage_steps.append(step_name)
    if len(ordered_stage_steps) < 2:
        return []

    default_stage_steps: List[str] = []
    if int(cost.get("rest_other", 0)) > 0:
        default_stage_steps.append("RestOther")
    if int(cost.get("sacrifice_from_stage", 0)) > 0:
        default_stage_steps.append("SacrificeFromStage")
    if int(cost.get("discard_from_hand", 0)) > 0:
        default_stage_steps.append("DiscardFromHand")
    if int(cost.get("clock_from_hand", 0)) > 0:
        default_stage_steps.append("ClockFromHand")
    if int(cost.get("clock_from_deck_top", 0)) > 0:
        default_stage_steps.append("ClockFromDeckTop")
    if int(cost.get("reveal_from_hand", 0)) > 0:
        default_stage_steps.append("RevealFromHand")

    if ordered_stage_steps == default_stage_steps:
        return []
    return ordered_stage_steps


def parse_cost(line: str) -> Tuple[Dict[str, Any], bool, str]:
    cost = default_cost()
    cost_steps: List[Dict[str, Any]] = []
    spans = _extract_bracket_segments(line)
    if not spans:
        return cost, True, line.strip()

    supported = True
    segments = [line[start + 1 : end] for start, end in spans]
    for seg in segments:
        cost_steps.extend(_extract_ordered_cost_steps(seg))
        seg = seg.lower()
        for match in re.finditer(r"\((\d+)\)", seg):
            cost["stock"] += int(match.group(1))
        seg = re.sub(r"\(\d+\)", " ", seg)

        if re.search(r"【rest】\s*this card(?: from 【stand】)?", seg):
            cost["rest_self"] = True
            seg = re.sub(r"【rest】\s*this card(?: from 【stand】)?", " ", seg)

        def rest_other_repl(match: re.Match[str]) -> str:
            count = parse_count_token(match.group(1))
            if count is None:
                return match.group(0)
            cost["rest_other"] += count
            return " "

        seg = re.sub(
            rf"【rest】\s*({COUNT_TOKEN_RE})\s*of your(?: [^\]]*?)? characters",
            rest_other_repl,
            seg,
            flags=re.I,
        )

        seg = _consume_counted(
            cost,
            seg,
            rf"put ({COUNT_TOKEN_RE}) [^\]]*? from your stage into your waiting room",
            "sacrifice_from_stage",
        )
        if re.search(r"put another [^\]]+ from your stage into your waiting room", seg):
            cost["sacrifice_from_stage"] += 1
            seg = re.sub(
                r"put another [^\]]+ from your stage into your waiting room",
                " ",
                seg,
            )

        seg = _consume_counted(
            cost,
            seg,
            rf"put ({COUNT_TOKEN_RE}) .*? from your hand into your waiting room",
            "discard_from_hand",
        )
        seg = _consume_counted(
            cost,
            seg,
            rf"put ({COUNT_TOKEN_RE}) card(?:s)? from your hand into your clock",
            "clock_from_hand",
        )
        seg = _consume_counted(
            cost,
            seg,
            rf"put the top ({COUNT_TOKEN_RE}) card(?:s)? of your deck into your clock",
            "clock_from_deck_top",
        )
        seg = _consume_counted(
            cost,
            seg,
            rf"put ({COUNT_TOKEN_RE}) card(?:s)? from the top of your deck into your clock",
            "clock_from_deck_top",
        )
        if re.search(r"put the top card of your deck into your clock", seg):
            cost["clock_from_deck_top"] += 1
            seg = re.sub(r"put the top card of your deck into your clock", " ", seg)
        seg = _consume_counted(
            cost,
            seg,
            rf"reveal ({COUNT_TOKEN_RE}) .*? from your hand",
            "reveal_from_hand",
        )

        if re.search(r"put this card into your waiting room", seg):
            cost["move_self_to_waiting_room"] = True
            seg = re.sub(r"put this card into your waiting room", " ", seg)
        if re.search(r"put this card in your waiting room", seg):
            cost["move_self_to_waiting_room"] = True
            seg = re.sub(r"put this card in your waiting room", " ", seg)
        if re.search(r"return this card to your hand", seg):
            cost["return_self_to_hand"] = True
            seg = re.sub(r"return this card to your hand", " ", seg)

        seg = re.sub(r"\b(and|then|&)\b", " ", seg)
        residue = re.sub(r"[^a-z]+", " ", seg).strip()
        if residue:
            supported = False

    out_parts: List[str] = []
    cursor = 0
    for start, end in spans:
        out_parts.append(line[cursor:start])
        out_parts.append(" ")
        cursor = end + 1
    out_parts.append(line[cursor:])
    clean_line = re.sub(r"\s+", " ", "".join(out_parts)).strip()
    if cost_steps:
        cost["cost_steps"] = cost_steps
        explicit_step_order = _extract_explicit_step_order(cost, cost_steps)
        if explicit_step_order:
            cost["step_order"] = explicit_step_order
    return cost, supported, clean_line
