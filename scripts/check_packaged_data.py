#!/usr/bin/env python3
from __future__ import annotations

import argparse
import hashlib
import json
import subprocess
import sys
import tempfile
from pathlib import Path
from typing import Any


ROOT = Path(__file__).resolve().parents[1]


def _sha256_bytes(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def _canonical_json_bytes(value: Any) -> bytes:
    return json.dumps(value, separators=(",", ":"), sort_keys=True, ensure_ascii=True).encode(
        "utf-8"
    )


def _read_json(path: Path) -> Any:
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except FileNotFoundError as exc:
        raise SystemExit(f"missing required packaged data file: {path}") from exc
    except json.JSONDecodeError as exc:
        raise SystemExit(f"invalid JSON in {path}: {exc}") from exc


def _read_bytes(path: Path) -> bytes:
    try:
        return path.read_bytes()
    except FileNotFoundError as exc:
        raise SystemExit(f"missing required packaged data file: {path}") from exc


def _rel(path: Path) -> str:
    try:
        return path.resolve().relative_to(ROOT).as_posix()
    except ValueError:
        return str(path)


def _fail_bytes_mismatch(path: Path, actual: bytes, expected: bytes) -> None:
    raise SystemExit(
        "\n".join(
            [
                f"packaged DB is stale: {_rel(path)}",
                f"  expected sha256: {_sha256_bytes(expected)}",
                f"  actual sha256:   {_sha256_bytes(actual)}",
                f"  expected bytes:  {len(expected)}",
                f"  actual bytes:    {len(actual)}",
                "Regenerate with:",
                "  cargo run -p weiss_core --bin carddb_pack -- scraper/out/cards.json target/cards.wsdb",
                "then copy the generated file to scraper/out/cards.wsdb, "
                "python/weiss_sim/data/default_cards.wsdb, and weiss_py/src/default_cards.wsdb.",
            ]
        )
    )


def _check_presets(path: Path) -> dict[str, list[int]]:
    payload = _read_json(path)
    if not isinstance(payload, dict):
        raise SystemExit(f"expected object in {_rel(path)}")
    out: dict[str, list[int]] = {}
    for name, values in payload.items():
        if not isinstance(name, str) or not isinstance(values, list):
            raise SystemExit(f"invalid deck preset entry in {_rel(path)}: {name!r}")
        try:
            deck = [int(value) for value in values]
        except Exception as exc:
            raise SystemExit(f"invalid card id in deck preset {name!r}: {exc}") from exc
        if len(deck) != 50:
            raise SystemExit(f"deck preset {name!r} must contain 50 cards, got {len(deck)}")
        out[name] = deck
    return out


def _check_preset_meta(path: Path, preset_names: set[str]) -> dict[str, dict[str, str]]:
    payload = _read_json(path)
    if not isinstance(payload, dict):
        raise SystemExit(f"expected object in {_rel(path)}")
    unknown = set(payload) - preset_names
    missing = preset_names - set(payload)
    if unknown:
        raise SystemExit(f"deck preset metadata has unknown presets: {sorted(unknown)}")
    if missing:
        raise SystemExit(f"deck preset metadata is missing presets: {sorted(missing)}")

    out: dict[str, dict[str, str]] = {}
    for name, meta in payload.items():
        if not isinstance(name, str) or not isinstance(meta, dict):
            raise SystemExit(f"invalid deck preset metadata entry in {_rel(path)}: {name!r}")
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


def _check_meta_hashes(
    catalog_meta_path: Path,
    default_wsdb_path: Path,
    presets: dict[str, list[int]],
    preset_meta: dict[str, dict[str, str]],
) -> None:
    meta = _read_json(catalog_meta_path)
    if not isinstance(meta, dict):
        raise SystemExit(f"expected object in {_rel(catalog_meta_path)}")

    expected = {
        "catalog_db_sha256": _sha256_bytes(_read_bytes(default_wsdb_path)),
        "deck_presets_sha256": _sha256_bytes(_canonical_json_bytes(presets)),
        "deck_preset_meta_sha256": _sha256_bytes(_canonical_json_bytes(preset_meta)),
    }
    for key, value in expected.items():
        actual = meta.get(key)
        if actual != value:
            raise SystemExit(
                f"{_rel(catalog_meta_path)} has stale {key}: expected {value}, got {actual!r}"
            )

    names = sorted(presets)
    if meta.get("deck_preset_count") != len(names):
        raise SystemExit(f"{_rel(catalog_meta_path)} has stale deck_preset_count")
    if meta.get("deck_preset_names") != names:
        raise SystemExit(f"{_rel(catalog_meta_path)} has stale deck_preset_names")


def _generate_wsdb(cards_json_path: Path, out_path: Path) -> bytes:
    if not cards_json_path.exists():
        raise SystemExit(f"missing source card JSON: {_rel(cards_json_path)}")
    cmd = [
        "cargo",
        "run",
        "-p",
        "weiss_core",
        "--bin",
        "carddb_pack",
        "--",
        str(cards_json_path),
        str(out_path),
    ]
    subprocess.run(cmd, cwd=ROOT, check=True)
    return out_path.read_bytes()


def main() -> None:
    parser = argparse.ArgumentParser(
        description="Check regenerated card DB bytes and packaged preset metadata."
    )
    parser.add_argument("--cards-json", default="scraper/out/cards.json")
    parser.add_argument("--scraper-wsdb", default="scraper/out/cards.wsdb")
    parser.add_argument("--python-wsdb", default="python/weiss_sim/data/default_cards.wsdb")
    parser.add_argument("--pyo3-wsdb", default="weiss_py/src/default_cards.wsdb")
    parser.add_argument("--catalog-meta", default="python/weiss_sim/data/catalog_meta.json")
    parser.add_argument("--deck-presets", default="python/weiss_sim/data/deck_presets.json")
    parser.add_argument("--deck-preset-meta", default="python/weiss_sim/data/deck_preset_meta.json")
    args = parser.parse_args()

    cards_json_path = ROOT / args.cards_json
    scraper_wsdb_path = ROOT / args.scraper_wsdb
    python_wsdb_path = ROOT / args.python_wsdb
    pyo3_wsdb_path = ROOT / args.pyo3_wsdb
    catalog_meta_path = ROOT / args.catalog_meta
    deck_presets_path = ROOT / args.deck_presets
    deck_preset_meta_path = ROOT / args.deck_preset_meta

    presets = _check_presets(deck_presets_path)
    preset_meta = _check_preset_meta(deck_preset_meta_path, set(presets))

    with tempfile.TemporaryDirectory(prefix="wss-packaged-data-") as tmp:
        generated = _generate_wsdb(cards_json_path, Path(tmp) / "cards.wsdb")

    for path in (scraper_wsdb_path, python_wsdb_path, pyo3_wsdb_path):
        actual = _read_bytes(path)
        if actual != generated:
            _fail_bytes_mismatch(path, actual, generated)

    _check_meta_hashes(catalog_meta_path, python_wsdb_path, presets, preset_meta)
    print(
        json.dumps(
            {
                "status": "ok",
                "cards_wsdb_sha256": _sha256_bytes(generated),
                "cards_wsdb_bytes": len(generated),
                "deck_preset_count": len(presets),
            },
            indent=2,
        )
    )


if __name__ == "__main__":
    try:
        main()
    except subprocess.CalledProcessError as exc:
        sys.exit(exc.returncode)
