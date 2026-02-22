#!/usr/bin/env python3
import argparse
import json
from collections import Counter
from pathlib import Path
import sys
from typing import Any, Dict, Iterable, List

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

from coverage_common import matching_families
from scraper.convert import (
    APPROX_PROFILE_APPROX,
    APPROX_PROFILE_STRICT,
    AbilityParseStats,
    ability_signature,
    build_trait_map,
    normalize_ability_line,
    parse_abilities,
)


def load_cards(path: Path) -> List[Dict[str, Any]]:
    data = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(data, list):
        raise ValueError(f"expected list in {path}")
    return data


def build_name_to_ids(records: Iterable[Dict[str, Any]]) -> Dict[str, List[int]]:
    mapping: Dict[str, List[int]] = {}
    for rec in records:
        name = (rec.get("name") or "").strip()
        if not name:
            continue
        card_id = rec.get("id")
        if not isinstance(card_id, int):
            continue
        mapping.setdefault(name, []).append(card_id)
    return mapping


def build_trait_to_ids(records: Iterable[Dict[str, Any]]) -> Dict[str, List[int]]:
    mapping: Dict[str, List[int]] = {}
    for rec in records:
        card_id = rec.get("id")
        if not isinstance(card_id, int):
            continue
        for trait in rec.get("traits") or []:
            if not isinstance(trait, str):
                continue
            key = trait.strip()
            if not key:
                continue
            mapping.setdefault(key, []).append(card_id)
    return {trait: sorted(set(ids)) for trait, ids in mapping.items()}


def tag_for_line(line: str) -> str:
    if line.startswith("【CONT】"):
        return "CONT"
    if line.startswith("【AUTO】"):
        return "AUTO"
    if line.startswith("【ACT】"):
        return "ACT"
    return "OTHER"


def has_approx_provenance(payload: Any) -> bool:
    if isinstance(payload, dict):
        if payload.get("approx") is True:
            return True
        conditions = payload.get("conditions")
        if isinstance(conditions, dict) and conditions.get("requires_approx_effects") is True:
            return True
        return any(has_approx_provenance(value) for value in payload.values())
    if isinstance(payload, list):
        return any(has_approx_provenance(item) for item in payload)
    return False


def classify_supported_line(
    profile: str,
    abilities: List[Dict[str, Any]],
    ability_defs: List[Dict[str, Any]],
) -> str:
    _ = profile
    if has_approx_provenance(abilities) or has_approx_provenance(ability_defs):
        return "approx"
    return "exact"


def normalize_family_coverage_entry(entry: Any) -> Dict[str, Any]:
    if not isinstance(entry, dict):
        return {"total": 0, "supported": 0, "coverage": 0.0}
    try:
        total = int(entry.get("total", 0))
    except (TypeError, ValueError):
        total = 0
    try:
        supported = int(entry.get("supported", 0))
    except (TypeError, ValueError):
        supported = 0
    try:
        coverage = float(entry.get("coverage", 0.0))
    except (TypeError, ValueError):
        coverage = 0.0
    return {"total": total, "supported": supported, "coverage": coverage}


def family_coverage_for_profile(metrics: Dict[str, Any]) -> Dict[str, Dict[str, Any]]:
    coverage = metrics.get("rule_family_coverage")
    if not isinstance(coverage, dict):
        coverage = metrics.get("family_cluster_coverage")
    if not isinstance(coverage, dict):
        return {}
    return {
        family: normalize_family_coverage_entry(entry)
        for family, entry in coverage.items()
        if isinstance(family, str)
    }


def build_rule_family_coverage_section(
    strict_metrics: Dict[str, Any], approx_metrics: Dict[str, Any]
) -> Dict[str, Dict[str, Any]]:
    strict_families = family_coverage_for_profile(strict_metrics)
    approx_families = family_coverage_for_profile(approx_metrics)
    families = sorted(set(strict_families.keys()) | set(approx_families.keys()))
    out: Dict[str, Dict[str, Any]] = {}
    for family in families:
        strict_entry = strict_families.get(family, {"total": 0, "supported": 0, "coverage": 0.0})
        approx_entry = approx_families.get(family, {"total": 0, "supported": 0, "coverage": 0.0})
        strict_unsupported = max(0, strict_entry["total"] - strict_entry["supported"])
        approx_unsupported = max(0, approx_entry["total"] - approx_entry["supported"])
        out[family] = {
            APPROX_PROFILE_STRICT: strict_entry,
            APPROX_PROFILE_APPROX: approx_entry,
            "coverage_delta_approx_minus_strict": (
                approx_entry["coverage"] - strict_entry["coverage"]
            ),
            "unsupported_lines_delta_approx_minus_strict": (
                approx_unsupported - strict_unsupported
            ),
        }
    return out


def analyze_profile(
    records: List[Dict[str, Any]],
    profile: str,
    name_to_ids: Dict[str, List[int]],
    trait_map: Dict[str, int],
    trait_to_ids: Dict[str, List[int]],
) -> Dict[str, Any]:
    total_lines = 0
    supported_lines = 0
    tag_total: Counter = Counter()
    tag_supported: Counter = Counter()
    unsupported: Counter = Counter()
    family_total: Counter = Counter()
    family_supported: Counter = Counter()
    supported_by_provenance: Counter = Counter()
    cards_with_tagged_lines = 0
    cards_all_lines_supported = 0
    cards_all_lines_supported_ids: set[int] = set()

    for rec in records:
        text = rec.get("raw_text") or rec.get("text") or ""
        if not isinstance(text, str) or not text.strip():
            continue
        card_type = rec.get("card_type") or "Character"
        lines = [normalize_ability_line(line.strip()) for line in text.split("\n") if line.strip()]
        tagged = [line for line in lines if line.startswith("【")]
        if not tagged:
            continue
        cards_with_tagged_lines += 1
        card_ok = True
        for line in tagged:
            total_lines += 1
            tag = tag_for_line(line)
            tag_total[tag] += 1
            line_families = matching_families(line)
            for family in line_families:
                family_total[family] += 1

            line_stats = AbilityParseStats()
            abilities, ability_defs, _counter_timing = parse_abilities(
                line,
                card_type,
                line_stats,
                name_to_ids,
                trait_map,
                profile,
                trait_to_ids,
                rec.get("id") if isinstance(rec.get("id"), int) else None,
                parser_version="v2",
            )
            parsed = line_stats.parsed_lines > 0
            if parsed:
                supported_lines += 1
                tag_supported[tag] += 1
                supported_by_provenance[
                    classify_supported_line(profile, abilities, ability_defs)
                ] += 1
                for family in line_families:
                    family_supported[family] += 1
            else:
                card_ok = False
                unsupported[ability_signature(line)] += 1
        if card_ok:
            cards_all_lines_supported += 1
            card_id = rec.get("id")
            if isinstance(card_id, int):
                cards_all_lines_supported_ids.add(card_id)

    by_tag: Dict[str, Any] = {}
    for tag, count in sorted(tag_total.items()):
        sup = int(tag_supported.get(tag, 0))
        by_tag[tag] = {
            "total": int(count),
            "supported": sup,
            "coverage": (float(sup) / float(count)) if count else 0.0,
        }

    by_family: Dict[str, Any] = {}
    for family, count in sorted(family_total.items()):
        sup = int(family_supported.get(family, 0))
        by_family[family] = {
            "total": int(count),
            "supported": sup,
            "coverage": (float(sup) / float(count)) if count else 0.0,
        }

    unsupported_sorted = sorted(unsupported.items(), key=lambda kv: (-kv[1], kv[0]))
    unsupported_lines = int(sum(unsupported.values()))
    approx_supported = int(supported_by_provenance.get("approx", 0))
    exact_supported = int(supported_by_provenance.get("exact", 0))
    unknown_supported = max(0, int(supported_lines) - approx_supported - exact_supported)
    provenance_known = approx_supported + exact_supported

    return {
        "profile": profile,
        "total_lines": int(total_lines),
        "supported_lines": int(supported_lines),
        "parse_line_coverage": (float(supported_lines) / float(total_lines))
        if total_lines
        else 0.0,
        "unsupported_lines": unsupported_lines,
        "distinct_unsupported_signatures": len(unsupported_sorted),
        "top_unsupported_signatures": [
            {"signature": sig, "count": int(count)} for sig, count in unsupported_sorted[:200]
        ],
        "supported_lines_by_tag": by_tag,
        "family_cluster_coverage": by_family,
        "rule_family_coverage": by_family,
        "supported_line_contributions": {
            "approx": {
                "supported_lines": approx_supported,
                "share_of_supported_lines": (
                    (float(approx_supported) / float(supported_lines)) if supported_lines else 0.0
                ),
            },
            "exact": {
                "supported_lines": exact_supported,
                "share_of_supported_lines": (
                    (float(exact_supported) / float(supported_lines)) if supported_lines else 0.0
                ),
            },
            "unknown": {
                "supported_lines": unknown_supported,
                "share_of_supported_lines": (
                    (float(unknown_supported) / float(supported_lines)) if supported_lines else 0.0
                ),
            },
            "provenance_known_share": (
                (float(provenance_known) / float(supported_lines)) if supported_lines else 0.0
            ),
        },
        "cards_with_tagged_lines": int(cards_with_tagged_lines),
        "cards_all_lines_supported": int(cards_all_lines_supported),
        "cards_all_lines_supported_ids": sorted(cards_all_lines_supported_ids),
        "card_level_all_lines_supported_coverage": (
            (float(cards_all_lines_supported) / float(cards_with_tagged_lines))
            if cards_with_tagged_lines
            else 0.0
        ),
    }


def main() -> None:
    parser = argparse.ArgumentParser(description="Build machine-readable ability coverage metrics.")
    parser.add_argument(
        "--input",
        default="scraper/out/cards_raw.json",
        help="Input cards_raw JSON path",
    )
    parser.add_argument(
        "--output",
        default="scraper/out/ability_coverage_report.json",
        help="Output report path",
    )
    args = parser.parse_args()

    input_path = Path(args.input)
    output_path = Path(args.output)

    records = load_cards(input_path)
    name_to_ids = build_name_to_ids(records)
    trait_map = build_trait_map(records)
    trait_to_ids = build_trait_to_ids(records)

    strict_metrics = analyze_profile(
        records, APPROX_PROFILE_STRICT, name_to_ids, trait_map, trait_to_ids
    )
    approx_metrics = analyze_profile(
        records, APPROX_PROFILE_APPROX, name_to_ids, trait_map, trait_to_ids
    )

    report = {
        "input_path": str(input_path),
        "profiles": {
            APPROX_PROFILE_STRICT: strict_metrics,
            APPROX_PROFILE_APPROX: approx_metrics,
        },
        "rule_family_coverage": build_rule_family_coverage_section(strict_metrics, approx_metrics),
        "comparison": {
            "parse_line_coverage_delta_approx_minus_strict": (
                approx_metrics["parse_line_coverage"] - strict_metrics["parse_line_coverage"]
            ),
            "card_level_supported_delta_approx_minus_strict": (
                approx_metrics["card_level_all_lines_supported_coverage"]
                - strict_metrics["card_level_all_lines_supported_coverage"]
            ),
            "unsupported_lines_delta_approx_minus_strict": (
                approx_metrics["unsupported_lines"] - strict_metrics["unsupported_lines"]
            ),
            "approx_supported_lines_delta_approx_minus_strict": (
                approx_metrics["supported_line_contributions"]["approx"]["supported_lines"]
                - strict_metrics["supported_line_contributions"]["approx"]["supported_lines"]
            ),
            "exact_supported_lines_delta_approx_minus_strict": (
                approx_metrics["supported_line_contributions"]["exact"]["supported_lines"]
                - strict_metrics["supported_line_contributions"]["exact"]["supported_lines"]
            ),
            "unknown_supported_lines_delta_approx_minus_strict": (
                approx_metrics["supported_line_contributions"]["unknown"]["supported_lines"]
                - strict_metrics["supported_line_contributions"]["unknown"]["supported_lines"]
            ),
        },
    }

    output_path.parent.mkdir(parents=True, exist_ok=True)
    output_path.write_text(json.dumps(report, indent=2, sort_keys=True), encoding="utf-8")


if __name__ == "__main__":
    main()
