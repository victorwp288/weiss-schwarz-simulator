from __future__ import annotations

import re
from typing import List, Optional, Tuple

from .models import AbilityLine, Clause


FULLWIDTH_TRANSLATION = str.maketrans(
    {
        "［": "[",
        "］": "]",
        "／": "/",
        "（": "(",
        "）": ")",
        "，": ",",
        "．": ".",
        "：": ":",
        "；": ";",
        "　": " ",
    }
)
CONTROL_CHAR_RE = re.compile(r"[\x00-\x08\x0B\x0C\x0E-\x1F]")
JSX_FRAGMENT_RE = re.compile(r"""['"]?\s*/>\s*""")
TAG_RE = re.compile(r"^【([A-Z]+)】\s*(.*)$")


def normalize_ability_line(line: str) -> str:
    cleaned = line.translate(FULLWIDTH_TRANSLATION)
    cleaned = CONTROL_CHAR_RE.sub("", cleaned)
    cleaned = JSX_FRAGMENT_RE.sub(" ", cleaned)
    return re.sub(r"\s+", " ", cleaned).strip()


def strip_cxcombo_tag(text: str) -> str:
    return re.sub(r"【CXCOMBO】\s*", "", text, flags=re.I).strip()


def split_tag(line: str) -> Tuple[Optional[str], str]:
    match = TAG_RE.match(line.strip())
    if not match:
        return None, line.strip()
    return match.group(1).upper(), match.group(2).strip()


def split_clauses(text: str) -> List[Clause]:
    parts = [part.strip() for part in re.split(r"(?<=\.)\s+", text.strip()) if part.strip()]
    if not parts:
        return []
    return [Clause(raw=part, normalized=part.lower()) for part in parts]


def build_ability_line(raw_line: str) -> AbilityLine:
    normalized = normalize_ability_line(raw_line)
    tag, body = split_tag(strip_cxcombo_tag(normalized))
    return AbilityLine(
        raw=raw_line,
        normalized=normalized,
        tag=tag,
        body=body,
        has_cxcombo_tag=("【CXCOMBO】" in normalized.upper()),
    )
