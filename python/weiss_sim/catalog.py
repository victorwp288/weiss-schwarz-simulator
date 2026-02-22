from __future__ import annotations

from collections import Counter
from difflib import SequenceMatcher
import gzip
import hashlib
import json
from functools import lru_cache
from importlib import resources
from pathlib import Path

from ._presets import get_preset as _get_preset, preset_names as _preset_names
from .config_types import CardPoolMode, DeckInput, RulesProfile
from .errors import CardLookupError, DbMismatchError, DeckSpecError
from .types import CardRef

_CATALOG_FILE = "card_catalog.json.gz"
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
    return _preset_names()


def get_preset(name: str) -> list[int]:
    return _get_preset(name)


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


def suggest_cards(query: str, *, limit: int = 5) -> list[CardRef]:
    token = query.strip().lower()
    if not token:
        return []
    max_items = max(1, int(limit))
    by_id, _ = _catalog_maps()
    cards = sorted(by_id.values(), key=lambda c: c.id)

    ranked_exact_or_substring: list[tuple[int, str, CardRef]] = []
    seen_ids: set[int] = set()
    for card in cards:
        card_no = card.card_no.lower()
        name = card.name.lower()
        if token == card_no or token == name:
            ranked_exact_or_substring.append((3, card.card_no, card))
            seen_ids.add(card.id)
            continue
        if token in card_no or token in name:
            ranked_exact_or_substring.append((2, card.card_no, card))
            seen_ids.add(card.id)

    ranked_exact_or_substring.sort(key=lambda item: (-item[0], item[1]))
    results = [card for _, _, card in ranked_exact_or_substring[:max_items]]
    if len(results) >= max_items:
        return results

    fuzzy_ranked: list[tuple[float, str, CardRef]] = []
    for card in cards:
        if card.id in seen_ids:
            continue
        card_no = card.card_no.lower()
        name = card.name.lower()
        score = max(
            SequenceMatcher(None, token, card_no).ratio(),
            SequenceMatcher(None, token, name).ratio(),
        )
        if score <= 0.0:
            continue
        fuzzy_ranked.append((score, card.card_no, card))
    fuzzy_ranked.sort(key=lambda item: (-item[0], item[1]))
    for _, _, card in fuzzy_ranked:
        results.append(card)
        if len(results) >= max_items:
            break
    return results


def _sha256_hex(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def default_wsdb_bytes() -> bytes:
    return _read_data_bytes(_DEFAULT_WSDB_FILE)


def catalog_db_sha256() -> str:
    return str(load_catalog_meta().get("catalog_db_sha256", ""))


@lru_cache(maxsize=1)
def _default_db_sha256() -> str:
    return _sha256_hex(default_wsdb_bytes())


def compute_db_sha256(db_path: str | Path | None = None) -> str:
    if db_path is None:
        return _default_db_sha256()
    resolved_path = Path(db_path).expanduser().resolve(strict=True)
    return _sha256_hex(resolved_path.read_bytes())


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
    """Namespace wrapper exposed as `weiss_sim.cards`."""

    def search(self, query: str, *, limit: int = 20) -> list[CardRef]:
        """Search the packaged catalog by card name or card number."""
        return search_cards(query, limit=limit)

    def suggest(self, query: str, *, limit: int = 5) -> list[CardRef]:
        """Suggest likely card matches for a possibly mistyped query."""
        return suggest_cards(query, limit=limit)

    def get(self, identifier: int | str) -> CardRef:
        """Look up a card by numeric id or card number."""
        return get_card(identifier)

    def presets(self) -> list[str]:
        """List available deck preset names."""
        return presets()

    def builder(self, initial: DeckInput | None = None):
        """Create a fluent deck builder."""
        from .deck_builder import DeckBuilder

        return DeckBuilder(initial=initial)

    def resolve_deck(
        self,
        deck_input: DeckInput,
        *,
        rules_profile: RulesProfile,
        card_pool: CardPoolMode,
        db_path: str | Path | None = None,
    ) -> list[int]:
        """Resolve a `DeckInput` into a validated list of 50 card ids."""
        from .decks import resolve_deck

        return resolve_deck(
            deck_input,
            rules_profile=rules_profile,
            card_pool=card_pool,
            db_path=db_path,
        )

    def validate_deck(
        self,
        deck_input: DeckInput,
        *,
        rules_profile: RulesProfile,
        card_pool: CardPoolMode,
        db_path: str | Path | None = None,
        deck_size: int = 50,
    ):
        """Validate a deck and return a structured report without raising."""
        from .decks import validate_deck

        return validate_deck(
            deck_input,
            rules_profile=rules_profile,
            card_pool=card_pool,
            db_path=db_path,
            deck_size=deck_size,
        )

    def describe_deck(
        self,
        deck_input: DeckInput,
        *,
        rules_profile: RulesProfile,
        card_pool: CardPoolMode,
        db_path: str | Path | None = None,
    ) -> dict[str, object]:
        """Resolve a `DeckInput` and return ids plus per-card metadata."""
        from .decks import resolve_deck_details

        return resolve_deck_details(
            deck_input,
            rules_profile=rules_profile,
            card_pool=card_pool,
            db_path=db_path,
        )

    def export_deck(
        self,
        deck_input: DeckInput,
        *,
        format: str = "card_no_map",
        rules_profile: RulesProfile,
        card_pool: CardPoolMode,
        db_path: str | Path | None = None,
        include_meta: bool = True,
    ) -> object:
        """Export a resolved deck to a deterministic JSON-serializable payload."""
        from .decks import resolve_deck

        ids = resolve_deck(
            deck_input,
            rules_profile=rules_profile,
            card_pool=card_pool,
            db_path=db_path,
        )
        encoding = format
        counts = Counter(ids)
        if encoding == "id_list":
            payload: object = list(ids)
        elif encoding == "id_map":
            payload = {
                card_id: int(count)
                for card_id, count in sorted(counts.items(), key=lambda item: item[0])
            }
        elif encoding == "card_no_map":
            entries = sorted(
                ((get_card(card_id).card_no, int(count)) for card_id, count in counts.items()),
                key=lambda item: item[0],
            )
            payload = {card_no: count for card_no, count in entries}
        else:
            raise DeckSpecError("export format must be one of: card_no_map, id_map, id_list")

        if not include_meta:
            return payload

        return {
            "format": "wsim_deck_v1",
            "encoding": encoding,
            "deck_size": 50,
            "cards": payload,
            "generated_by": "weiss-sim",
            "catalog_db_sha256": catalog_db_sha256(),
        }

    def save_deck(
        self,
        path: str | Path,
        deck_input: DeckInput,
        *,
        format: str = "card_no_map",
        rules_profile: RulesProfile,
        card_pool: CardPoolMode,
        db_path: str | Path | None = None,
        include_meta: bool = True,
        indent: int = 2,
    ) -> str:
        """Serialize a resolved deck payload to disk."""
        payload = self.export_deck(
            deck_input,
            format=format,
            rules_profile=rules_profile,
            card_pool=card_pool,
            db_path=db_path,
            include_meta=include_meta,
        )
        out_path = Path(path)
        out_path.write_text(json.dumps(payload, indent=indent), encoding="utf-8")
        return str(out_path)

    def load_deck(self, path: str | Path):
        """Load a deck payload from disk (v1 envelope or raw map/list)."""
        try:
            payload = json.loads(Path(path).read_text(encoding="utf-8"))
        except FileNotFoundError as exc:
            raise DeckSpecError(f"deck file not found: {path}") from exc
        except OSError as exc:
            raise DeckSpecError(f"failed to read deck file {path}: {exc}") from exc
        except json.JSONDecodeError as exc:
            raise DeckSpecError(f"invalid deck JSON in {path}: {exc}") from exc

        if isinstance(payload, dict) and payload.get("format") == "wsim_deck_v1":
            encoding = payload.get("encoding")
            cards = payload.get("cards")
            if encoding not in {"card_no_map", "id_map", "id_list"}:
                raise DeckSpecError(
                    "deck envelope encoding must be one of: card_no_map, id_map, id_list"
                )
            if not isinstance(cards, (dict, list)):
                raise DeckSpecError("deck envelope cards payload must be a mapping or list")
            return cards

        if isinstance(payload, (dict, list)):
            return payload
        raise DeckSpecError("deck JSON must be a mapping/list or a wsim_deck_v1 envelope")


cards = _CardsNamespace()
