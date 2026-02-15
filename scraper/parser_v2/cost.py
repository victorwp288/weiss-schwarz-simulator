from __future__ import annotations

import re
from typing import Any, Dict, List, Optional, Tuple


COUNT_TOKEN_RE = r"(?:\d+|a|an|one|two|three|four|five|six|seven|eight|nine|ten)"


def default_cost() -> Dict[str, Any]:
    return {
        "stock": 0,
        "rest_self": False,
        "rest_other": 0,
        "discard_from_hand": 0,
        "clock_from_hand": 0,
        "clock_from_deck_top": 0,
        "reveal_from_hand": 0,
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


def parse_cost(line: str) -> Tuple[Dict[str, Any], bool, str]:
    cost = default_cost()
    spans = _extract_bracket_segments(line)
    if not spans:
        return cost, True, line.strip()

    supported = True
    segments = [line[start + 1 : end].lower() for start, end in spans]
    for seg in segments:
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
        seg = _consume_counted(
            cost,
            seg,
            rf"reveal ({COUNT_TOKEN_RE}) .*? from your hand",
            "reveal_from_hand",
        )

        if re.search(r"put this card into your waiting room", seg):
            cost["move_self_to_waiting_room"] = True
            seg = re.sub(r"put this card into your waiting room", " ", seg)
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
    return cost, supported, clean_line
