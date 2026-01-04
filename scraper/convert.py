#!/usr/bin/env python3
import argparse
import json
import re
from collections import Counter, defaultdict
from dataclasses import dataclass
from pathlib import Path
from typing import Any, Dict, Iterable, List, Optional, Tuple


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
    "Choice": None,
    "Pool": None,
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


@dataclass
class AbilityParseStats:
    total_lines: int = 0
    parsed_lines: int = 0
    emitted_templates: Counter = None
    emitted_defs: Counter = None
    unsupported_signatures: Counter = None

    def __post_init__(self) -> None:
        if self.emitted_templates is None:
            self.emitted_templates = Counter()
        if self.emitted_defs is None:
            self.emitted_defs = Counter()
        if self.unsupported_signatures is None:
            self.unsupported_signatures = Counter()


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


def ability_signature(text: str) -> str:
    sig = text.strip()
    sig = re.sub(r'"[^"]+"', '"<Q>"', sig)
    sig = re.sub(r"\d+", "#", sig)
    sig = re.sub(r"\s+", " ", sig).strip()
    return sig


def parse_cost(line: str) -> Tuple[Dict[str, Any], bool, str]:
    cost = {
        "stock": 0,
        "rest_self": False,
        "rest_other": 0,
        "discard_from_hand": 0,
        "clock_from_hand": 0,
        "clock_from_deck_top": 0,
        "reveal_from_hand": 0,
    }
    supported = True
    segments = re.findall(r"\[([^\]]+)\]", line)
    if not segments:
        return cost, True, line

    for seg in segments:
        seg_low = seg.lower()
        if re.search(r"put this card|this card into", seg_low):
            supported = False
            continue

        for match in re.finditer(r"\((\d+)\)", seg_low):
            cost["stock"] += int(match.group(1))
        seg_low = re.sub(r"\(\d+\)", " ", seg_low)

        if re.search(r"【rest】\s*this card(?: from 【stand】)?", seg_low):
            cost["rest_self"] = True
            seg_low = re.sub(r"【rest】\s*this card(?: from 【stand】)?", " ", seg_low)

        for match in re.finditer(
            r"put (\d+) card(?:s)? from your hand into your waiting room", seg_low
        ):
            cost["discard_from_hand"] += int(match.group(1))
        seg_low = re.sub(
            r"put \d+ card(?:s)? from your hand into your waiting room",
            " ",
            seg_low,
        )

        for match in re.finditer(
            r"put (\d+) card(?:s)? from your hand into your clock", seg_low
        ):
            cost["clock_from_hand"] += int(match.group(1))
        seg_low = re.sub(
            r"put \d+ card(?:s)? from your hand into your clock",
            " ",
            seg_low,
        )

        for match in re.finditer(
            r"put the top (\d+) card(?:s)? of your deck into your clock", seg_low
        ):
            cost["clock_from_deck_top"] += int(match.group(1))
        seg_low = re.sub(
            r"put the top \d+ card(?:s)? of your deck into your clock",
            " ",
            seg_low,
        )

        for match in re.finditer(r"reveal (\d+) card(?:s)? from your hand", seg_low):
            cost["reveal_from_hand"] += int(match.group(1))
        seg_low = re.sub(r"reveal \d+ card(?:s)? from your hand", " ", seg_low)

        seg_low = re.sub(r"\b(and|&|then)\b", " ", seg_low)
        residue = re.sub(r"[^a-z]+", " ", seg_low).strip()
        if residue:
            supported = False

    line_clean = re.sub(r"\s*\[[^\]]+\]\s*", " ", line)
    line_clean = re.sub(r"\s+", " ", line_clean).strip()
    return cost, supported, line_clean


def cost_is_empty(cost: Dict[str, Any]) -> bool:
    return not any(
        (
            cost.get("stock", 0),
            cost.get("rest_self", False),
            cost.get("rest_other", 0),
            cost.get("discard_from_hand", 0),
            cost.get("clock_from_hand", 0),
            cost.get("clock_from_deck_top", 0),
            cost.get("reveal_from_hand", 0),
        )
    )


def _template(name: str, **fields: Any) -> Any:
    if fields:
        return {name: fields}
    return name


def _ability_def(
    kind: str,
    timing: Optional[str],
    effects: List[Any],
    targets: List[str],
    cost: Optional[Dict[str, Any]] = None,
) -> Dict[str, Any]:
    return {
        "kind": kind,
        "timing": timing,
        "effects": effects,
        "targets": targets,
        "cost": cost
        or {
            "stock": 0,
            "rest_self": False,
            "rest_other": 0,
            "discard_from_hand": 0,
            "clock_from_hand": 0,
            "clock_from_deck_top": 0,
            "reveal_from_hand": 0,
        },
        "target_card_type": None,
        "target_trait": None,
        "target_level_max": None,
        "target_cost_max": None,
        "target_limit": None,
    }


def parse_abilities(text: str, card_type: str, stats: AbilityParseStats) -> Tuple[List[Any], List[Any], bool]:
    abilities: List[Any] = []
    ability_defs: List[Any] = []
    counter_timing = False

    if not text:
        return abilities, ability_defs, counter_timing

    lines = [line.strip() for line in text.split("\n") if line.strip()]
    for line in lines:
        if not line.startswith("【"):
            continue
        stats.total_lines += 1

        if "Brainstorm" in line or "BRAINSTORM" in line:
            stats.unsupported_signatures[ability_signature(line)] += 1
            continue

        if "CXCOMBO" in line:
            stats.unsupported_signatures[ability_signature(line)] += 1
            continue

        if "【COUNTER】" in line and re.search(r"Backup\s+\d+", line, re.IGNORECASE):
            match = re.search(r"Backup\s+(\d+)", line, re.IGNORECASE)
            if match:
                power = int(match.group(1))
                abilities.append(_template("CounterBackup", power=power))
                counter_timing = True
                stats.parsed_lines += 1
                stats.emitted_templates["CounterBackup"] += 1
                continue

        cost, cost_supported, line_clean = parse_cost(line)
        if not cost_supported:
            stats.unsupported_signatures[ability_signature(line)] += 1
            continue

        if line_clean.startswith("【CONT】"):
            remainder = line_clean[len("【CONT】") :].strip()
            if "【" in remainder:
                stats.unsupported_signatures[ability_signature(line)] += 1
                continue
            match = re.match(r"^This card gets \+(\d+) power\.?$", remainder, re.I)
            if match:
                abilities.append(_template("ContinuousPower", amount=int(match.group(1))))
                stats.parsed_lines += 1
                stats.emitted_templates["ContinuousPower"] += 1
                continue

        if line_clean.startswith("【AUTO】"):
            if not cost_is_empty(cost):
                stats.unsupported_signatures[ability_signature(line)] += 1
                continue
            remainder = line_clean[len("【AUTO】") :].strip()
            if "【" in remainder:
                stats.unsupported_signatures[ability_signature(line)] += 1
                continue

            if remainder.lower().startswith(
                "when this card is placed on the stage from your hand,"
            ):
                effect = remainder.split(",", 1)[1].strip()
                if "may" in effect.lower() and "choose" not in effect.lower():
                    stats.unsupported_signatures[ability_signature(line)] += 1
                    continue

                match = re.match(r"^draw (\d+) card(?:s)?\.?$", effect, re.I)
                if match and "may" not in effect.lower() and "up to" not in effect.lower():
                    abilities.append(_template("AutoOnPlayDraw", count=int(match.group(1))))
                    stats.parsed_lines += 1
                    stats.emitted_templates["AutoOnPlayDraw"] += 1
                    continue

                match = re.match(
                    r"^put the top (\d+) card(?:s)? of your deck into your waiting room\.?$",
                    effect,
                    re.I,
                )
                if match and "may" not in effect.lower():
                    abilities.append(_template("AutoOnPlayMillTop", count=int(match.group(1))))
                    stats.parsed_lines += 1
                    stats.emitted_templates["AutoOnPlayMillTop"] += 1
                    continue

                match = re.match(
                    r"^put the top (\d+) card(?:s)? of your deck into your stock\.?$",
                    effect,
                    re.I,
                )
                if match and "may" not in effect.lower():
                    abilities.append(
                        _template("AutoOnPlayStockCharge", count=int(match.group(1)))
                    )
                    stats.parsed_lines += 1
                    stats.emitted_templates["AutoOnPlayStockCharge"] += 1
                    continue

                match = re.match(
                    r"^put the top card of your clock into your waiting room\.?$",
                    effect,
                    re.I,
                )
                if match and "may" not in effect.lower():
                    abilities.append(_template("AutoOnPlayHeal", count=1))
                    stats.parsed_lines += 1
                    stats.emitted_templates["AutoOnPlayHeal"] += 1
                    continue

                match = re.match(
                    r"^(you may )?choose (up to )?(\d+) ([a-z ]+?) in your waiting room, and return (?:it|them) to your hand\.?$",
                    effect,
                    re.I,
                )
                if match:
                    optional = bool(match.group(1)) or bool(match.group(2))
                    count = int(match.group(3))
                    type_text = match.group(4).strip().lower()
                    card_type_hint = None
                    if "character" in type_text:
                        card_type_hint = "Character"
                    elif "climax" in type_text:
                        card_type_hint = "Climax"
                    elif "event" in type_text:
                        card_type_hint = "Event"
                    abilities.append(
                        _template(
                            "AutoOnPlaySalvage",
                            count=count,
                            optional=optional,
                            card_type=card_type_hint,
                        )
                    )
                    stats.parsed_lines += 1
                    stats.emitted_templates["AutoOnPlaySalvage"] += 1
                    continue

                match = re.match(
                    r"^choose 1 of your characters, and that character gets \+(\d+) power until end of turn\.?$",
                    effect,
                    re.I,
                )
                if match:
                    ability_defs.append(
                        _ability_def(
                            "Auto",
                            "OnPlay",
                            effects=[
                                _template(
                                    "AddPower", amount=int(match.group(1)), duration_turn=True
                                )
                            ],
                            targets=["SelfStage"],
                        )
                    )
                    stats.parsed_lines += 1
                    stats.emitted_defs["Auto.AddPower.OnPlay"] += 1
                    continue

            match = re.match(
                r"^When this card attacks, (.+)$",
                remainder,
                re.I,
            )
            if match:
                effect = match.group(1).strip()
                if "may" in effect.lower():
                    stats.unsupported_signatures[ability_signature(line)] += 1
                    continue
                dmg = re.match(
                    r"^(?:you )?deal (\d+) damage to your opponent\.?$",
                    effect,
                    re.I,
                )
                if dmg:
                    cancelable = "cannot be canceled" not in effect.lower()
                    abilities.append(
                        _template(
                            "AutoOnAttackDealDamage",
                            amount=int(dmg.group(1)),
                            cancelable=cancelable,
                        )
                    )
                    stats.parsed_lines += 1
                    stats.emitted_templates["AutoOnAttackDealDamage"] += 1
                    continue

                buff = re.match(
                    r"^choose 1 of your characters, and that character gets \+(\d+) power until end of turn\.?$",
                    effect,
                    re.I,
                )
                if buff:
                    ability_defs.append(
                        _ability_def(
                            "Auto",
                            "AttackDeclaration",
                            effects=[
                                _template(
                                    "AddPower", amount=int(buff.group(1)), duration_turn=True
                                )
                            ],
                            targets=["SelfStage"],
                        )
                    )
                    stats.parsed_lines += 1
                    stats.emitted_defs["Auto.AddPower.AttackDeclaration"] += 1
                    continue

            match = re.match(
                r"^When this card becomes 【REVERSE】(?: in battle)?, (.+)$",
                remainder,
                re.I,
            )
            if match:
                effect = match.group(1).strip()
                if "may" in effect.lower() and "choose" not in effect.lower():
                    stats.unsupported_signatures[ability_signature(line)] += 1
                    continue
                draw = re.match(r"^draw (\d+) card(?:s)?\.?$", effect, re.I)
                if draw and "may" not in effect.lower() and "up to" not in effect.lower():
                    abilities.append(_template("AutoOnReverseDraw", count=int(draw.group(1))))
                    stats.parsed_lines += 1
                    stats.emitted_templates["AutoOnReverseDraw"] += 1
                    continue

                salv = re.match(
                    r"^(you may )?choose (up to )?(\d+) ([a-z ]+?) in your waiting room, and return (?:it|them) to your hand\.?$",
                    effect,
                    re.I,
                )
                if salv:
                    optional = bool(salv.group(1)) or bool(salv.group(2))
                    count = int(salv.group(3))
                    type_text = salv.group(4).strip().lower()
                    card_type_hint = None
                    if "character" in type_text:
                        card_type_hint = "Character"
                    elif "climax" in type_text:
                        card_type_hint = "Climax"
                    elif "event" in type_text:
                        card_type_hint = "Event"
                    abilities.append(
                        _template(
                            "AutoOnReverseSalvage",
                            count=count,
                            optional=optional,
                            card_type=card_type_hint,
                        )
                    )
                    stats.parsed_lines += 1
                    stats.emitted_templates["AutoOnReverseSalvage"] += 1
                    continue

            match = re.match(
                r"^At the (?:beginning of )?your end phase, draw (\d+) card(?:s)?\.?$",
                remainder,
                re.I,
            )
            if match and "may" not in remainder.lower():
                abilities.append(_template("AutoEndPhaseDraw", count=int(match.group(1))))
                stats.parsed_lines += 1
                stats.emitted_templates["AutoEndPhaseDraw"] += 1
                continue

            stats.unsupported_signatures[ability_signature(line)] += 1
            continue

        if line_clean.startswith("【ACT】"):
            remainder = line_clean[len("【ACT】") :].strip()
            if "【" in remainder:
                stats.unsupported_signatures[ability_signature(line)] += 1
                continue

            if remainder.lower().startswith("choose 1 of your characters"):
                match = re.match(
                    r"^choose 1 of your characters, and that character gets \+(\d+) power until end of turn\.?$",
                    remainder,
                    re.I,
                )
                if match:
                    ability_defs.append(
                        _ability_def(
                            "Activated",
                            None,
                            effects=[
                                _template(
                                    "AddPower", amount=int(match.group(1)), duration_turn=True
                                )
                            ],
                            targets=["SelfStage"],
                            cost=cost,
                        )
                    )
                    stats.parsed_lines += 1
                    stats.emitted_defs["Activated.AddPower"] += 1
                    continue

            match = re.match(
                r"^this card gets \+(\d+) power until end of turn\.?$",
                remainder,
                re.I,
            )
            if match:
                ability_defs.append(
                    _ability_def(
                        "Activated",
                        None,
                        effects=[
                            _template(
                                "AddPower", amount=int(match.group(1)), duration_turn=True
                            )
                        ],
                        targets=["This"],
                        cost=cost,
                    )
                )
                stats.parsed_lines += 1
                stats.emitted_defs["Activated.AddPower.Self"] += 1
                continue

            stats.unsupported_signatures[ability_signature(line)] += 1
            continue

        stats.unsupported_signatures[ability_signature(line)] += 1

    # Event fallback: simple damage-only event text without tags
    if card_type == "Event" and not abilities and not ability_defs:
        if text.strip() and not any(line.startswith("【") for line in lines):
            dmg = re.match(
                r"^(?:you )?deal (\d+) damage to your opponent\.?$",
                text.strip(),
                re.I,
            )
            if dmg:
                cancelable = "cannot be canceled" not in text.lower()
                abilities.append(
                    _template(
                        "EventDealDamage",
                        amount=int(dmg.group(1)),
                        cancelable=cancelable,
                    )
                )
                stats.emitted_templates["EventDealDamage"] += 1

    return abilities, ability_defs, counter_timing


def convert(
    records: List[Dict[str, Any]],
    out_dir: Path,
    input_path: Path,
) -> None:
    out_dir.mkdir(parents=True, exist_ok=True)

    stats: Dict[str, Any] = {
        "input_path": str(input_path),
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

    ordered = sorted(deduped.values(), key=lambda r: (r.get("card_no") or ""))
    trait_map = build_trait_map(ordered)
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
            text, card_type, ability_stats
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
    args = parser.parse_args()
    input_path = Path(args.input)
    out_dir = Path(args.out_dir)
    records = load_jsonl(input_path)
    convert(records, out_dir, input_path)


if __name__ == "__main__":
    main()
