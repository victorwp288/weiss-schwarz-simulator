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
        "--out-dir",
        default="python/weiss_sim/data",
        help="output directory for packaged data files",
    )
    args = parser.parse_args()

    cards_raw_path = Path(args.cards_raw)
    coverage_path = Path(args.coverage_report)
    default_wsdb_path = Path(args.default_wsdb)
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
                # Keep bundled starter preset ids (1..13) available in both profiles.
                "strict_ok": (card_id in strict_ids) or card_id <= 13,
                "approx_ok": (card_id in approx_ids) or card_id <= 13,
            }
        )
    rows.sort(key=lambda r: int(r["id"]))

    starter_v1 = (list(range(1, 14)) * 4)[:50]
    presets = {"starter_v1": starter_v1}

    catalog_meta = {
        "schema_version": 1,
        "catalog_db_sha256": _sha256(default_wsdb_path),
        "cards_raw_path": str(cards_raw_path),
        "coverage_report_path": str(coverage_path),
        "strict_supported_count": len(strict_ids),
        "approx_supported_count": len(approx_ids),
        "catalog_count": len(rows),
    }

    catalog_path = out_dir / "card_catalog.json.gz"
    with gzip.open(catalog_path, "wb") as f:
        f.write(json.dumps(rows, separators=(",", ":"), ensure_ascii=True).encode("utf-8"))

    (out_dir / "deck_presets.json").write_text(
        json.dumps(presets, indent=2, sort_keys=True), encoding="utf-8"
    )
    (out_dir / "catalog_meta.json").write_text(
        json.dumps(catalog_meta, indent=2, sort_keys=True), encoding="utf-8"
    )

    print(
        json.dumps(
            {
                "card_catalog": str(catalog_path),
                "deck_presets": str(out_dir / "deck_presets.json"),
                "catalog_meta": str(out_dir / "catalog_meta.json"),
                "catalog_count": len(rows),
            },
            indent=2,
        )
    )


if __name__ == "__main__":
    main()
