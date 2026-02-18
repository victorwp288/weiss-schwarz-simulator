from __future__ import annotations

import gzip
import hashlib
import json
from functools import lru_cache
from importlib import resources
from pathlib import Path

from .config_types import CardPoolMode, DeckInput, RulesProfile
from .errors import CardLookupError, DbMismatchError
from .types import CardRef

_CATALOG_FILE = "card_catalog.json.gz"
_PRESETS_FILE = "deck_presets.json"
_META_FILE = "catalog_meta.json"
_DEFAULT_WSDB_FILE = "default_cards.wsdb"


def _data_root():
    return resources.files(__package__).joinpath("data")


def _read_data_bytes(name: str) -> bytes:
    with resources.as_file(_data_root().joinpath(name)) as p:
        return Path(p).read_bytes()


@lru_cache(maxsize=1)
def _load_catalog_rows() -> list[dict[str, object]]:
    try:
        raw = _read_data_bytes(_CATALOG_FILE)
    except FileNotFoundError:
        return _fallback_catalog_rows()
    rows = json.loads(gzip.decompress(raw).decode("utf-8"))
    if not isinstance(rows, list):
        raise ValueError(f"{_CATALOG_FILE} must decode to a list")
    return [r for r in rows if isinstance(r, dict)]


def _fallback_catalog_rows() -> list[dict[str, object]]:
    rows: list[dict[str, object]] = []
    for card_id in range(1, 14):
        rows.append(
            {
                "id": card_id,
                "card_no": f"CARD-{card_id}",
                "name": f"Card {card_id}",
                "card_type": "Character",
                "card_set": None,
                "strict_ok": True,
                "approx_ok": True,
            }
        )
    return rows


@lru_cache(maxsize=1)
def load_catalog_meta() -> dict[str, object]:
    try:
        raw = _read_data_bytes(_META_FILE)
    except FileNotFoundError:
        return {
            "catalog_db_sha256": _sha256_hex(default_wsdb_bytes()),
            "schema_version": 1,
        }
    meta = json.loads(raw.decode("utf-8"))
    if not isinstance(meta, dict):
        raise ValueError(f"{_META_FILE} must decode to an object")
    return meta


@lru_cache(maxsize=1)
def _load_presets() -> dict[str, list[int]]:
    try:
        raw = _read_data_bytes(_PRESETS_FILE)
        payload = json.loads(raw.decode("utf-8"))
    except FileNotFoundError:
        payload = {"starter_v1": (list(range(1, 14)) * 4)[:50]}
    if not isinstance(payload, dict):
        raise ValueError(f"{_PRESETS_FILE} must decode to an object")
    out: dict[str, list[int]] = {}
    for key, value in payload.items():
        if not isinstance(key, str):
            continue
        if not isinstance(value, list):
            continue
        ids = [int(v) for v in value]
        out[key] = ids
    return out


@lru_cache(maxsize=1)
def _catalog_maps() -> tuple[dict[int, CardRef], dict[str, CardRef]]:
    by_id: dict[int, CardRef] = {}
    by_card_no: dict[str, CardRef] = {}
    for row in _load_catalog_rows():
        try:
            card = CardRef(
                id=int(row["id"]),
                card_no=str(row.get("card_no", "")),
                name=str(row.get("name", "")),
                card_type=str(row.get("card_type", "Unknown")),
                card_set=(None if row.get("card_set") is None else str(row["card_set"])),
                strict_ok=bool(row.get("strict_ok", False)),
                approx_ok=bool(row.get("approx_ok", False)),
            )
        except Exception:
            continue
        by_id[card.id] = card
        by_card_no[card.card_no.lower()] = card
    return by_id, by_card_no


def presets() -> list[str]:
    return sorted(_load_presets().keys())


def get_preset(name: str) -> list[int]:
    key = name.strip()
    try:
        return list(_load_presets()[key])
    except KeyError as exc:
        raise CardLookupError(f"unknown preset '{name}'") from exc


def get_card(identifier: int | str) -> CardRef:
    by_id, by_card_no = _catalog_maps()
    if isinstance(identifier, int):
        if identifier in by_id:
            return by_id[identifier]
        raise CardLookupError(f"unknown card id '{identifier}'")
    token = str(identifier).strip()
    if not token:
        raise CardLookupError("empty card identifier")
    token_lower = token.lower()
    if token_lower in by_card_no:
        return by_card_no[token_lower]
    if token.isdigit():
        card_id = int(token)
        if card_id in by_id:
            return by_id[card_id]
    raise CardLookupError(f"unknown card identifier '{identifier}'")


def search_cards(query: str, *, limit: int = 20) -> list[CardRef]:
    token = query.strip().lower()
    if not token:
        return []
    out: list[CardRef] = []
    by_id, _ = _catalog_maps()
    cards = sorted(by_id.values(), key=lambda c: c.id)
    for card in cards:
        if token in card.name.lower() or token in card.card_no.lower():
            out.append(card)
            if len(out) >= max(1, int(limit)):
                break
    return out


def _sha256_hex(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def default_wsdb_bytes() -> bytes:
    return _read_data_bytes(_DEFAULT_WSDB_FILE)


def catalog_db_sha256() -> str:
    return str(load_catalog_meta().get("catalog_db_sha256", ""))


def compute_db_sha256(db_path: str | Path | None = None) -> str:
    if db_path is None:
        return _sha256_hex(default_wsdb_bytes())
    path = Path(db_path)
    return _sha256_hex(path.read_bytes())


def db_hash_matches_catalog(db_path: str | Path | None = None) -> bool:
    return compute_db_sha256(db_path) == catalog_db_sha256()


def db_info(db_path: str | Path | None = None) -> dict[str, object]:
    db_sha256 = compute_db_sha256(db_path)
    catalog_sha256 = catalog_db_sha256()
    return {
        "db_sha256": db_sha256,
        "catalog_db_sha256": catalog_sha256,
        "matches_catalog": db_sha256 == catalog_sha256,
    }


def assert_parsed_only_catalog_match(
    card_pool: CardPoolMode, db_path: str | Path | None = None
) -> dict[str, object]:
    info = db_info(db_path)
    if card_pool == "parsed_only" and not bool(info["matches_catalog"]):
        raise DbMismatchError(
            expected_db_sha256=str(info["catalog_db_sha256"]),
            actual_db_sha256=str(info["db_sha256"]),
            remediation=(
                "Pass db_path=None to use the bundled DB, switch to card_pool='all', "
                "or rebuild python/weiss_sim/data/catalog_meta.json for your DB."
            ),
        )
    return info


def resolve_card_id(identifier: int | str) -> int:
    return get_card(identifier).id


class _CardsNamespace:
    def search(self, query: str, *, limit: int = 20) -> list[CardRef]:
        return search_cards(query, limit=limit)

    def get(self, identifier: int | str) -> CardRef:
        return get_card(identifier)

    def presets(self) -> list[str]:
        return presets()

    def resolve_deck(
        self,
        deck_input: DeckInput,
        *,
        rules_profile: RulesProfile,
        card_pool: CardPoolMode,
        db_path: str | Path | None = None,
    ) -> list[int]:
        from .decks import resolve_deck

        return resolve_deck(
            deck_input,
            rules_profile=rules_profile,
            card_pool=card_pool,
            db_path=db_path,
        )


cards = _CardsNamespace()
