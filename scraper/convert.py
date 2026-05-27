#!/usr/bin/env python3
import argparse
import json
import re
from collections import Counter, defaultdict
from pathlib import Path
from typing import Any, Dict, Iterable, List, Optional


TRIGGER_MAP = {
    "Soul": "Soul",
    "Soul2": "Soul",
    "Draw": "Draw",
    "Shot": "Shot",
    "Gate": "Gate",
    "Bounce": "Bounce",
    "Standby": "Standby",
    "Treasure": "Treasure",
    "Comeback": "Gate",
    "Salvage": "Gate",
    "Return": "Bounce",
    "Choice": "Choice",
    "Pool": "Pool",
}
CARD_TYPE_MAP = {
    "character": "Character",
    "event": "Event",
    "climax": "Climax",
}

COLOR_MAP = {
    "yellow": "Yellow",
    "green": "Green",
    "red": "Red",
    "blue": "Blue",
    "colorless": "Colorless",
}


try:
    from scraper.convert_abilities import (
        ACT_RULES,
        APPROX_PROFILE_APPROX,
        APPROX_PROFILE_CLI_CHOICES,
        APPROX_PROFILE_STRICT,
        APPROX_PROFILES,
        AUTO_RULES,
        CONT_RULES,
        PARSER_VERSIONS,
        RULE_MODE_APPROX,
        RULE_MODE_EXACT,
        AbilityParseStats,
        ParseRule,
        ability_signature,
        cost_is_empty,
        normalize_ability_line,
        normalize_approx_profile,
        parse_abilities,
        parse_cost,
        parse_count_token,
        resolve_name_fragment_ids,
    )
    from scraper.parser_v2.engine import PARSER_VERSION_V2
except ModuleNotFoundError:
    from convert_abilities import (
        ACT_RULES,
        APPROX_PROFILE_APPROX,
        APPROX_PROFILE_CLI_CHOICES,
        APPROX_PROFILE_STRICT,
        APPROX_PROFILES,
        AUTO_RULES,
        CONT_RULES,
        PARSER_VERSIONS,
        RULE_MODE_APPROX,
        RULE_MODE_EXACT,
        AbilityParseStats,
        ParseRule,
        ability_signature,
        cost_is_empty,
        normalize_ability_line,
        normalize_approx_profile,
        parse_abilities,
        parse_cost,
        parse_count_token,
        resolve_name_fragment_ids,
    )
    from parser_v2.engine import PARSER_VERSION_V2

__all__ = [
    "ACT_RULES",
    "APPROX_PROFILE_APPROX",
    "APPROX_PROFILE_CLI_CHOICES",
    "APPROX_PROFILE_STRICT",
    "APPROX_PROFILES",
    "AUTO_RULES",
    "CARD_TYPE_MAP",
    "COLOR_MAP",
    "CONT_RULES",
    "PARSER_VERSION_V2",
    "PARSER_VERSIONS",
    "RULE_MODE_APPROX",
    "RULE_MODE_EXACT",
    "TRIGGER_MAP",
    "AbilityParseStats",
    "ParseRule",
    "ability_signature",
    "build_trait_map",
    "convert",
    "cost_is_empty",
    "load_jsonl",
    "map_triggers",
    "normalize_ability_line",
    "normalize_approx_profile",
    "normalize_card_type",
    "normalize_color",
    "parse_abilities",
    "parse_cost",
    "parse_count_token",
    "pick_card_set",
    "resolve_name_fragment_ids",
]


def load_jsonl(path: Path) -> List[Dict[str, Any]]:
    records: List[Dict[str, Any]] = []
    with path.open("r", encoding="utf-8") as f:
        for line in f:
            line = line.strip()
            if not line:
                continue
            records.append(json.loads(line))
    return records


def normalize_card_type(value: Optional[str]) -> Optional[str]:
    if not value:
        return None
    token = value.strip()
    if not token:
        return None
    mapped = CARD_TYPE_MAP.get(token.lower())
    return mapped or token


def normalize_color(value: Optional[str]) -> Optional[str]:
    if not value:
        return None
    token = value.strip()
    token = re.sub(r"^[^A-Za-z]+", "", token)
    if not token:
        return None
    mapped = COLOR_MAP.get(token.lower())
    return mapped or token.title()


def pick_card_set(record: Dict[str, Any]) -> Optional[str]:
    for key in ("set_key", "expansion_filter_value", "expansion_raw"):
        value = record.get(key)
        if value is None:
            continue
        if isinstance(value, str) and value.strip():
            return value.strip()
        if isinstance(value, (int, float)):
            return str(value)
    return None


def build_trait_map(records: Iterable[Dict[str, Any]]) -> Dict[str, int]:
    traits = set()
    for rec in records:
        for trait in rec.get("traits") or []:
            if isinstance(trait, str) and trait.strip():
                traits.add(trait.strip())
    ordered = sorted(traits, key=lambda s: s.casefold())
    return {trait: idx + 1 for idx, trait in enumerate(ordered)}


def map_triggers(raw_triggers: Iterable[str], stats: Dict[str, Any]) -> List[str]:
    out: List[str] = []
    for trig in raw_triggers or []:
        if not trig:
            continue
        mapped = TRIGGER_MAP.get(trig)
        if mapped is None:
            stats["triggers_dropped"][trig] += 1
            continue
        if mapped != trig:
            stats["triggers_coerced"][f"{trig}->{mapped}"] += 1
        out.append(mapped)
    return out


def convert(
    records: List[Dict[str, Any]],
    out_dir: Path,
    input_path: Path,
    approx_profile: str = APPROX_PROFILE_STRICT,
    coverage_report: Optional[Path] = None,
    parser_version: str = PARSER_VERSION_V2,
    emit_parse_trace: bool = False,
) -> None:
    approx_profile = normalize_approx_profile(approx_profile)
    if parser_version not in PARSER_VERSIONS:
        raise ValueError(
            f"unsupported parser version '{parser_version}', expected one of: {sorted(PARSER_VERSIONS)}"
        )
    out_dir.mkdir(parents=True, exist_ok=True)

    stats: Dict[str, Any] = {
        "input_path": str(input_path),
        "approx_profile": approx_profile,
        "parser_version": parser_version,
        "input_count": len(records),
        "output_count": 0,
        "duplicate_card_no": 0,
        "missing_level_defaulted": 0,
        "missing_cost_defaulted": 0,
        "missing_power_defaulted": 0,
        "missing_soul_defaulted": 0,
        "unknown_colors": defaultdict(int),
        "triggers_dropped": defaultdict(int),
        "triggers_coerced": defaultdict(int),
        "ability": {
            "total_lines": 0,
            "parsed_lines": 0,
            "templates": Counter(),
            "defs": Counter(),
            "unsupported_signatures": Counter(),
        },
    }

    deduped: Dict[str, Dict[str, Any]] = {}
    for rec in records:
        card_no = (rec.get("card_no") or "").strip()
        if not card_no:
            continue
        if card_no in deduped:
            stats["duplicate_card_no"] += 1
            continue
        deduped[card_no] = rec

    ordered = sorted(deduped.values(), key=lambda r: r.get("card_no") or "")
    name_to_ids: Dict[str, List[int]] = defaultdict(list)
    for idx, rec in enumerate(ordered, start=1):
        name = rec.get("name")
        if isinstance(name, str):
            key = name.strip()
            if key:
                name_to_ids[key].append(idx)
    trait_map = build_trait_map(ordered)
    trait_to_ids: Dict[str, List[int]] = defaultdict(list)
    for idx, rec in enumerate(ordered, start=1):
        for trait in rec.get("traits") or []:
            if isinstance(trait, str) and trait.strip():
                trait_to_ids[trait.strip()].append(idx)
    trait_to_ids = {
        trait: sorted(set(card_ids)) for trait, card_ids in trait_to_ids.items() if card_ids
    }
    stats["trait_count"] = len(trait_map)
    stats["trait_id_map"] = trait_map

    cards_out: List[Dict[str, Any]] = []
    cards_raw: List[Dict[str, Any]] = []

    ability_stats = AbilityParseStats()

    for idx, rec in enumerate(ordered, start=1):
        card_no = (rec.get("card_no") or "").strip()
        card_type_raw = rec.get("card_type")
        card_type = normalize_card_type(card_type_raw)
        if not card_type:
            continue

        color_raw = rec.get("color")
        color = normalize_color(color_raw)
        if not color:
            color = "Colorless"
            stats["unknown_colors"][str(color_raw)] += 1
        elif color not in {"Yellow", "Green", "Red", "Blue", "Colorless"}:
            stats["unknown_colors"][color] += 1
            color = "Colorless"

        level = rec.get("level")
        cost = rec.get("cost")
        power = rec.get("power")
        soul = rec.get("soul")

        if level is None:
            level = 0
            stats["missing_level_defaulted"] += 1
        if cost is None:
            cost = 0
            stats["missing_cost_defaulted"] += 1
        if power is None:
            power = 0
            stats["missing_power_defaulted"] += 1
        if soul is None:
            soul = 0
            stats["missing_soul_defaulted"] += 1

        triggers_raw = rec.get("triggers") or []
        triggers = map_triggers(triggers_raw, stats)

        traits_raw = rec.get("traits") or []
        traits = [trait_map[t] for t in traits_raw if t in trait_map]

        text = rec.get("text") or ""
        abilities, ability_defs, counter_timing = parse_abilities(
            text,
            card_type,
            ability_stats,
            name_to_ids,
            trait_map,
            approx_profile,
            trait_to_ids,
            idx,
            parser_version=parser_version,
            emit_parse_trace=emit_parse_trace,
        )

        card_set = pick_card_set(rec)

        card_out = {
            "id": idx,
            "card_set": card_set,
            "card_type": card_type,
            "color": color,
            "level": int(level),
            "cost": int(cost),
            "power": int(power),
            "soul": int(soul),
            "triggers": triggers,
            "traits": traits,
            "abilities": abilities,
            "ability_defs": ability_defs,
            "counter_timing": counter_timing,
            "raw_text": text if text else None,
        }
        cards_out.append(card_out)

        raw_record = dict(rec)
        raw_record["id"] = idx
        raw_record["raw_text"] = rec.get("text")
        raw_record["raw_triggers"] = triggers_raw
        raw_record["raw_traits"] = traits_raw
        raw_record["card_set"] = card_set
        cards_raw.append(raw_record)

    stats["output_count"] = len(cards_out)
    stats["ability"]["total_lines"] = ability_stats.total_lines
    stats["ability"]["parsed_lines"] = ability_stats.parsed_lines
    stats["ability"]["templates"] = ability_stats.emitted_templates
    stats["ability"]["defs"] = ability_stats.emitted_defs
    stats["ability"]["unsupported_signatures"] = ability_stats.unsupported_signatures
    if emit_parse_trace:
        stats["ability"]["parse_trace"] = ability_stats.parse_traces

    stats["unknown_colors"] = dict(stats["unknown_colors"])
    stats["triggers_dropped"] = dict(stats["triggers_dropped"])
    stats["triggers_coerced"] = dict(stats["triggers_coerced"])
    stats["ability"]["templates"] = dict(stats["ability"]["templates"])
    stats["ability"]["defs"] = dict(stats["ability"]["defs"])

    unsupported_sorted = sorted(
        stats["ability"]["unsupported_signatures"].items(),
        key=lambda kv: (-kv[1], kv[0]),
    )
    stats["ability"]["unsupported_signatures"] = [
        {"signature": sig, "count": count} for sig, count in unsupported_sorted
    ]

    (out_dir / "cards.json").write_text(
        json.dumps(cards_out, indent=2, sort_keys=True), encoding="utf-8"
    )
    (out_dir / "cards_raw.json").write_text(
        json.dumps(cards_raw, indent=2, sort_keys=True), encoding="utf-8"
    )
    (out_dir / "conversion_stats.json").write_text(
        json.dumps(stats, indent=2, sort_keys=True), encoding="utf-8"
    )
    if coverage_report is not None:
        total_lines = stats["ability"]["total_lines"]
        parsed_lines = stats["ability"]["parsed_lines"]
        unsupported_count = sum(
            entry["count"] for entry in stats["ability"]["unsupported_signatures"]
        )
        coverage = {
            "approx_profile": approx_profile,
            "total_lines": total_lines,
            "parsed_lines": parsed_lines,
            "parse_line_coverage": (
                (float(parsed_lines) / float(total_lines)) if total_lines else 0.0
            ),
            "unsupported_lines": unsupported_count,
            "distinct_unsupported_signatures": len(stats["ability"]["unsupported_signatures"]),
            "top_unsupported_signatures": stats["ability"]["unsupported_signatures"][:100],
        }
        coverage_report.parent.mkdir(parents=True, exist_ok=True)
        coverage_report.write_text(json.dumps(coverage, indent=2, sort_keys=True), encoding="utf-8")


def main() -> None:
    parser = argparse.ArgumentParser(description="Convert scraped cards.jsonl into CardStatic JSON")
    parser.add_argument(
        "--input",
        default="out/cards.jsonl",
        help="Input JSONL file (default: out/cards.jsonl)",
    )
    parser.add_argument(
        "--out-dir",
        default="out",
        help="Output directory for cards.json, cards_raw.json, conversion_stats.json",
    )
    parser.add_argument(
        "--approx-profile",
        default=APPROX_PROFILE_STRICT,
        choices=APPROX_PROFILE_CLI_CHOICES,
        help="Approximation profile for conversion (default: strict).",
    )
    parser.add_argument(
        "--coverage-report",
        default=None,
        help="Optional path to emit machine-readable coverage report JSON",
    )
    parser.add_argument(
        "--parser-version",
        default=PARSER_VERSION_V2,
        choices=sorted(PARSER_VERSIONS),
        help="Parser version for unmatched-line fallback (default: v2)",
    )
    parser.add_argument(
        "--emit-parse-trace",
        action="store_true",
        help="Emit parser-v2 fallback trace data in conversion_stats.json",
    )
    args = parser.parse_args()
    input_path = Path(args.input)
    out_dir = Path(args.out_dir)
    coverage_report = Path(args.coverage_report) if args.coverage_report else None
    records = load_jsonl(input_path)
    convert(
        records,
        out_dir,
        input_path,
        args.approx_profile,
        coverage_report,
        parser_version=args.parser_version,
        emit_parse_trace=args.emit_parse_trace,
    )


if __name__ == "__main__":
    main()
