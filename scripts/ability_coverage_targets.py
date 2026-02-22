#!/usr/bin/env python3
import argparse
import json
from pathlib import Path
from typing import Any, Dict, List

from coverage_common import (
    family_coverage_from_metrics,
    load_json,
    matching_families,
    normalize_profile_name,
    resolve_profile_metrics,
)


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
        help="Profile to prioritize (default: strict)",
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
    profile_metrics = resolve_profile_metrics(report, normalized_profile)
    by_family = family_coverage_from_metrics(profile_metrics)
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
