#!/usr/bin/env python3
from __future__ import annotations

import argparse
import gzip
import hashlib
import json
from pathlib import Path
from typing import Any

from coverage_common import load_json, load_json_any


def _profile_supported_ids(report: dict[str, Any], profile: str) -> set[int]:
    profiles = report.get("profiles")
    if not isinstance(profiles, dict):
        return set()
    payload = profiles.get(profile)
    if not isinstance(payload, dict):
        return set()
    ids = payload.get("cards_all_lines_supported_ids")
    if isinstance(ids, list):
        out: set[int] = set()
        for value in ids:
            if isinstance(value, int):
                out.add(value)
        return out
    return set()


def _sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def _sha256_bytes(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def _canonical_json_bytes(value: Any) -> bytes:
    return json.dumps(value, separators=(",", ":"), sort_keys=True, ensure_ascii=True).encode(
        "utf-8"
    )


def _load_deck_presets(path: Path) -> dict[str, list[int]]:
    if not path.exists():
        raise SystemExit(f"missing deck presets file: {path}")
    payload = load_json(path)
    if not isinstance(payload, dict):
        raise SystemExit(f"expected object in {path}")
    presets: dict[str, list[int]] = {}
    for name, values in payload.items():
        if not isinstance(name, str) or not isinstance(values, list):
            raise SystemExit(f"invalid deck preset entry in {path}: {name!r}")
        try:
            deck = [int(value) for value in values]
        except Exception as exc:
            raise SystemExit(f"invalid card id in deck preset {name!r}: {exc}") from exc
        if len(deck) != 50:
            raise SystemExit(f"deck preset {name!r} must contain 50 cards, got {len(deck)}")
        presets[name] = deck
    return presets


def _load_deck_preset_meta(path: Path, preset_names: set[str]) -> dict[str, dict[str, str]]:
    if not path.exists():
        raise SystemExit(f"missing deck preset metadata file: {path}")
    payload = load_json(path)
    if not isinstance(payload, dict):
        raise SystemExit(f"expected object in {path}")
    unknown = set(payload) - preset_names
    missing = preset_names - set(payload)
    if unknown:
        raise SystemExit(f"deck preset metadata has unknown presets: {sorted(unknown)}")
    if missing:
        raise SystemExit(f"deck preset metadata is missing presets: {sorted(missing)}")

    out: dict[str, dict[str, str]] = {}
    for name, meta in payload.items():
        if not isinstance(name, str) or not isinstance(meta, dict):
            raise SystemExit(f"invalid deck preset metadata entry in {path}: {name!r}")
        profile = meta.get("min_rules_profile")
        if profile not in {"strict", "approx"}:
            raise SystemExit(
                f"deck preset metadata {name!r} must have min_rules_profile strict or approx"
            )
        source = meta.get("source", "")
        if source is not None and not isinstance(source, str):
            raise SystemExit(f"deck preset metadata {name!r} has non-string source")
        cleaned = {"min_rules_profile": str(profile)}
        if source:
            cleaned["source"] = source
        out[name] = cleaned
    return out


def main() -> None:
    parser = argparse.ArgumentParser(description="Build packaged Python card catalog metadata.")
    parser.add_argument(
        "--cards-raw",
        default="scraper/out/cards_raw.json",
        help="cards_raw JSON input path",
    )
    parser.add_argument(
        "--coverage-report",
        default="scraper/out/ability_coverage_report.json",
        help="ability coverage report path",
    )
    parser.add_argument(
        "--default-wsdb",
        default="python/weiss_sim/data/default_cards.wsdb",
        help="path to packaged default wsdb file",
    )
    parser.add_argument(
        "--deck-presets",
        default="python/weiss_sim/data/deck_presets.json",
        help="deck preset JSON to preserve in packaged data",
    )
    parser.add_argument(
        "--deck-preset-meta",
        default="python/weiss_sim/data/deck_preset_meta.json",
        help="deck preset metadata JSON to preserve in packaged data",
    )
    parser.add_argument(
        "--out-dir",
        default="python/weiss_sim/data",
        help="output directory for packaged data files",
    )
    args = parser.parse_args()

    cards_raw_path = Path(args.cards_raw)
    coverage_path = Path(args.coverage_report)
    default_wsdb_path = Path(args.default_wsdb)
    deck_presets_path = Path(args.deck_presets)
    deck_preset_meta_path = Path(args.deck_preset_meta)
    out_dir = Path(args.out_dir)
    out_dir.mkdir(parents=True, exist_ok=True)

    cards_raw = load_json_any(cards_raw_path)
    if not isinstance(cards_raw, list):
        raise SystemExit(f"expected list in {cards_raw_path}")
    report = load_json(coverage_path)
    if not isinstance(report, dict):
        raise SystemExit(f"expected object in {coverage_path}")

    strict_ids = _profile_supported_ids(report, "strict")
    approx_ids = _profile_supported_ids(report, "approx")

    all_card_ids = {int(rec["id"]) for rec in cards_raw if isinstance(rec, dict) and "id" in rec}
    if not strict_ids:
        strict_ids = set(all_card_ids)
    if not approx_ids:
        approx_ids = set(all_card_ids)

    rows: list[dict[str, Any]] = []
    for rec in cards_raw:
        if not isinstance(rec, dict):
            continue
        card_id = rec.get("id")
        if not isinstance(card_id, int):
            continue
        rows.append(
            {
                "id": card_id,
                "card_no": str(rec.get("card_no", "")),
                "name": str(rec.get("name", "")),
                "card_type": str(rec.get("card_type", "Unknown")),
                "card_set": rec.get("card_set"),
                # Keep synthetic fallback/toy deck ids (1..13) available in both profiles.
                "strict_ok": (card_id in strict_ids) or card_id <= 13,
                "approx_ok": (card_id in approx_ids) or card_id <= 13,
            }
        )
    rows.sort(key=lambda r: int(r["id"]))

    presets = _load_deck_presets(deck_presets_path)
    preset_meta = _load_deck_preset_meta(deck_preset_meta_path, set(presets))
    presets_json = _canonical_json_bytes(presets)
    preset_meta_json = _canonical_json_bytes(preset_meta)

    catalog_meta = {
        "schema_version": 1,
        "catalog_db_sha256": _sha256(default_wsdb_path),
        "cards_raw_path": cards_raw_path.as_posix(),
        "coverage_report_path": coverage_path.as_posix(),
        "strict_supported_count": len(strict_ids),
        "approx_supported_count": len(approx_ids),
        "catalog_count": len(rows),
        "deck_preset_count": len(presets),
        "deck_preset_names": sorted(presets),
        "deck_preset_meta_sha256": _sha256_bytes(preset_meta_json),
        "deck_presets_sha256": _sha256_bytes(presets_json),
    }

    catalog_path = out_dir / "card_catalog.json.gz"
    catalog_path.write_bytes(gzip.compress(_canonical_json_bytes(rows), mtime=0))

    (out_dir / "deck_presets.json").write_text(
        json.dumps(presets, indent=2, sort_keys=True) + "\n", encoding="utf-8"
    )
    (out_dir / "deck_preset_meta.json").write_text(
        json.dumps(preset_meta, indent=2, sort_keys=True) + "\n", encoding="utf-8"
    )
    (out_dir / "catalog_meta.json").write_text(
        json.dumps(catalog_meta, indent=2, sort_keys=True) + "\n", encoding="utf-8"
    )

    print(
        json.dumps(
            {
                "card_catalog": str(catalog_path),
                "deck_presets": str(out_dir / "deck_presets.json"),
                "deck_preset_meta": str(out_dir / "deck_preset_meta.json"),
                "catalog_meta": str(out_dir / "catalog_meta.json"),
                "catalog_count": len(rows),
            },
            indent=2,
        )
    )


if __name__ == "__main__":
    main()
