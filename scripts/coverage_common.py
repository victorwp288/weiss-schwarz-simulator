from __future__ import annotations

import json
import re
from pathlib import Path
from typing import Any

FAMILY_PATTERNS: list[tuple[str, re.Pattern[str]]] = [
    ("Experience", re.compile(r"\bExperience\b", re.I)),
    (
        "AssistOrScalingPower",
        re.compile(r"\bAssist\b|for each|gets \+\d+ power|gets \+X power", re.I),
    ),
    ("FollowingAbilityGrant", re.compile(r"following ability", re.I)),
    (
        "PaidOnPlaySearchSalvage",
        re.compile(
            r"placed on (?:the )?stage from your hand.*pay the cost.*(?:look at|search|return .* to your hand)",
            re.I,
        ),
    ),
    (
        "OnReverseSelfMove",
        re.compile(
            r"becomes 【REVERSE】.*(?:put this card at the bottom of your deck|put this card into your memory)",
            re.I,
        ),
    ),
    ("ClimaxPlacedBuff", re.compile(r"climax is placed on your climax area", re.I)),
    ("BrainstormCustomAction", re.compile(r"Brainstorm .*perform the following action", re.I)),
    ("HandText", re.compile(r"while in your hand", re.I)),
    ("DeckConstructionRule", re.compile(r"put any number of cards with the same card name", re.I)),
]

SUPPORTED_PROFILES = frozenset({"strict", "approx"})


def load_json_any(path: Path) -> Any:
    return json.loads(path.read_text(encoding="utf-8"))


def load_json(path: Path) -> dict[str, Any]:
    data = load_json_any(path)
    if not isinstance(data, dict):
        raise ValueError(f"expected object in {path}")
    return data


def normalize_profile_name(profile: str) -> str:
    token = (profile or "strict").strip().lower()
    if token not in SUPPORTED_PROFILES:
        raise ValueError(
            f"unsupported profile '{profile}', expected one of: {sorted(SUPPORTED_PROFILES)}"
        )
    return token


def resolve_profile_metrics(report: dict[str, Any], profile: str) -> dict[str, Any]:
    profiles = report.get("profiles")
    if not isinstance(profiles, dict):
        raise ValueError("report missing profiles object")
    normalized = normalize_profile_name(profile)
    metrics = profiles.get(normalized)
    if not isinstance(metrics, dict):
        raise ValueError(f"report missing profile '{normalized}'")
    return metrics


def family_coverage_from_metrics(profile_metrics: dict[str, Any]) -> dict[str, dict[str, Any]]:
    by_family = profile_metrics.get("rule_family_coverage")
    if not isinstance(by_family, dict):
        by_family = profile_metrics.get("family_cluster_coverage")
    if not isinstance(by_family, dict):
        return {}
    out: dict[str, dict[str, Any]] = {}
    for family, entry in by_family.items():
        if not isinstance(family, str) or not isinstance(entry, dict):
            continue
        total = int(entry.get("total", 0))
        supported = int(entry.get("supported", 0))
        coverage = float(entry.get("coverage", (float(supported) / float(total)) if total else 0.0))
        out[family] = {"total": total, "supported": supported, "coverage": coverage}
    return out


def matching_families(text: str) -> list[str]:
    return [family for family, pattern in FAMILY_PATTERNS if pattern.search(text)]
