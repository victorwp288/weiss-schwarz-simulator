#!/usr/bin/env python3
import argparse
import json
import re
from pathlib import Path
from typing import Any, Dict, List, Tuple


FAMILY_PATTERNS: List[Tuple[str, re.Pattern[str]]] = [
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

PROFILE_ALIASES: Dict[str, List[str]] = {
    "strict": ["strict", "none"],
    "approx": ["approx", "rl_v1"],
    "none": ["strict", "none"],
    "rl_v1": ["approx", "rl_v1"],
}


def load_json(path: Path) -> Dict[str, Any]:
    data = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(data, dict):
        raise ValueError(f"expected object in {path}")
    return data


def normalize_profile_name(profile: str) -> str:
    token = (profile or "strict").strip().lower()
    aliases = PROFILE_ALIASES.get(token)
    if aliases is None:
        raise ValueError(
            f"unsupported profile '{profile}', expected one of: {sorted(PROFILE_ALIASES.keys())}"
        )
    return aliases[0]


def get_profile_metrics(report: Dict[str, Any], profile: str) -> Dict[str, Any]:
    profiles = report.get("profiles")
    if not isinstance(profiles, dict):
        raise ValueError("report missing profiles object")
    normalized = normalize_profile_name(profile)
    for candidate in PROFILE_ALIASES.get(normalized, [normalized]):
        metrics = profiles.get(candidate)
        if isinstance(metrics, dict):
            return metrics
    raise ValueError(
        f"report missing profile '{profile}' (accepted aliases: {PROFILE_ALIASES.get(normalized, [normalized])})"
    )


def family_coverage_map(profile_metrics: Dict[str, Any]) -> Dict[str, Dict[str, Any]]:
    by_family = profile_metrics.get("rule_family_coverage")
    if not isinstance(by_family, dict):
        by_family = profile_metrics.get("family_cluster_coverage")
    if not isinstance(by_family, dict):
        return {}

    normalized: Dict[str, Dict[str, Any]] = {}
    for family, entry in by_family.items():
        if not isinstance(family, str) or not isinstance(entry, dict):
            continue
        total = int(entry.get("total", 0))
        supported = int(entry.get("supported", 0))
        coverage = float(entry.get("coverage", (float(supported) / float(total)) if total else 0.0))
        normalized[family] = {"total": total, "supported": supported, "coverage": coverage}
    return normalized


def matching_families(text: str) -> List[str]:
    return [family for family, pattern in FAMILY_PATTERNS if pattern.search(text)]


def prioritized_families(
    by_family: Dict[str, Dict[str, Any]], max_families: int
) -> List[Dict[str, Any]]:
    ranked: List[Dict[str, Any]] = []
    for family, entry in by_family.items():
        total = int(entry.get("total", 0))
        supported = int(entry.get("supported", 0))
        coverage = float(entry.get("coverage", 0.0))
        unsupported = max(0, total - supported)
        ranked.append(
            {
                "family": family,
                "total_lines": total,
                "supported_lines": supported,
                "unsupported_lines": unsupported,
                "coverage": coverage,
            }
        )

    ranked.sort(
        key=lambda row: (
            -row["unsupported_lines"],
            row["coverage"],
            -row["total_lines"],
            row["family"],
        )
    )
    limited = ranked[: max(0, max_families)]
    for index, row in enumerate(limited, start=1):
        row["priority_rank"] = index
    return limited


def prioritized_signatures(
    profile_metrics: Dict[str, Any],
    family_backlog: Dict[str, int],
    max_signatures: int,
) -> List[Dict[str, Any]]:
    top = profile_metrics.get("top_unsupported_signatures")
    if not isinstance(top, list):
        return []

    ranked: List[Dict[str, Any]] = []
    for entry in top:
        if not isinstance(entry, dict):
            continue
        signature = entry.get("signature")
        if not isinstance(signature, str):
            continue
        count = int(entry.get("count", 0))
        families = matching_families(signature)
        family_priority_hint = max(
            (family_backlog.get(family, 0) for family in families), default=0
        )
        priority_score = (count * 1000) + family_priority_hint
        ranked.append(
            {
                "signature": signature,
                "count": count,
                "matched_families": families,
                "family_priority_hint": family_priority_hint,
                "priority_score": priority_score,
            }
        )

    ranked.sort(
        key=lambda row: (
            -row["priority_score"],
            -row["count"],
            row["signature"],
        )
    )
    limited = ranked[: max(0, max_signatures)]
    for index, row in enumerate(limited, start=1):
        row["priority_rank"] = index
    return limited


def family_backlog_map(by_family: Dict[str, Dict[str, Any]]) -> Dict[str, int]:
    return {
        family: max(0, int(entry.get("total", 0)) - int(entry.get("supported", 0)))
        for family, entry in by_family.items()
    }


def main() -> None:
    parser = argparse.ArgumentParser(
        description="Generate prioritized ability coverage targets from ability_coverage_report.json."
    )
    parser.add_argument(
        "--report",
        default="scraper/out/ability_coverage_report.json",
        help="Input ability_coverage_report.json path",
    )
    parser.add_argument(
        "--output",
        default="-",
        help="Output JSON path; use '-' to print to stdout",
    )
    parser.add_argument(
        "--profile",
        default="strict",
        help="Profile to prioritize (default: strict; aliases: none, rl_v1)",
    )
    parser.add_argument(
        "--max-families",
        type=int,
        default=20,
        help="Maximum number of family targets to emit",
    )
    parser.add_argument(
        "--max-signatures",
        type=int,
        default=50,
        help="Maximum number of signature targets to emit",
    )
    args = parser.parse_args()

    report_path = Path(args.report)
    report = load_json(report_path)
    normalized_profile = normalize_profile_name(args.profile)
    profile_metrics = get_profile_metrics(report, normalized_profile)
    by_family = family_coverage_map(profile_metrics)
    families = prioritized_families(by_family, args.max_families)
    family_backlog = family_backlog_map(by_family)
    signatures = prioritized_signatures(profile_metrics, family_backlog, args.max_signatures)

    payload: Dict[str, Any] = {
        "input_report_path": str(report_path),
        "profile": normalized_profile,
        "requested_profile": args.profile,
        "summary": {
            "parse_line_coverage": float(profile_metrics.get("parse_line_coverage", 0.0)),
            "supported_lines": int(profile_metrics.get("supported_lines", 0)),
            "unsupported_lines": int(profile_metrics.get("unsupported_lines", 0)),
            "distinct_unsupported_signatures": int(
                profile_metrics.get("distinct_unsupported_signatures", 0)
            ),
        },
        "family_targets": families,
        "signature_targets": signatures,
    }

    serialized = json.dumps(payload, indent=2, sort_keys=True)
    if args.output == "-":
        print(serialized)
        return

    output_path = Path(args.output)
    output_path.parent.mkdir(parents=True, exist_ok=True)
    output_path.write_text(serialized, encoding="utf-8")
    print(f"[ability-coverage-targets] wrote {output_path}")


if __name__ == "__main__":
    main()
