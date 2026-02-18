from __future__ import annotations

import json
from pathlib import Path
from typing import Mapping, Sequence

from .catalog import (
    assert_parsed_only_catalog_match,
    get_card,
    get_preset,
    resolve_card_id,
)
from .config_types import CardPoolMode, DeckInput, RulesProfile
from .errors import DeckSpecError, DeckValidationError

_DECK_SIZE = 50


def _normalize_profile(rules_profile: RulesProfile | str) -> RulesProfile:
    token = str(rules_profile).strip().lower()
    if token not in {"strict", "approx"}:
        raise DeckSpecError(f"rules_profile must be strict or approx (got {rules_profile!r})")
    return token  # type: ignore[return-value]


def _normalize_card_pool(card_pool: CardPoolMode | str) -> CardPoolMode:
    token = str(card_pool).strip().lower()
    if token not in {"parsed_only", "all"}:
        raise DeckSpecError(f"card_pool must be parsed_only or all (got {card_pool!r})")
    return token  # type: ignore[return-value]


def _is_path_like(token: str) -> bool:
    return "/" in token or "\\" in token or token.endswith(".json")


def _load_json_path(path: Path):
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except FileNotFoundError as exc:
        raise DeckSpecError(f"deck file not found: {path}") from exc
    except json.JSONDecodeError as exc:
        raise DeckSpecError(f"invalid deck JSON in {path}: {exc}") from exc


def _resolve_string_input(token: str):
    raw = token.strip()
    if not raw:
        raise DeckSpecError("empty deck string")
    lower = raw.lower()
    if lower.startswith("preset:"):
        name = raw.split(":", 1)[1].strip()
        return get_preset(name)
    if lower.startswith("file:"):
        path = Path(raw.split(":", 1)[1].strip())
        return _load_json_path(path)
    if _is_path_like(raw):
        return _load_json_path(Path(raw))
    return get_preset(raw)


def _resolve_deck_input(deck_input: DeckInput):
    if isinstance(deck_input, Path):
        return _load_json_path(deck_input)
    if isinstance(deck_input, str):
        return _resolve_string_input(deck_input)
    return deck_input


def _expand_mapping(deck_map: Mapping[int | str, int]) -> list[int]:
    expanded: list[int] = []
    pairs: list[tuple[int, int]] = []
    for key, count in deck_map.items():
        try:
            n = int(count)
        except Exception as exc:
            raise DeckSpecError(f"invalid count for {key!r}: {count!r}") from exc
        if n < 0:
            raise DeckSpecError(f"count for {key!r} must be non-negative")
        card_id = key if isinstance(key, int) else resolve_card_id(str(key))
        pairs.append((int(card_id), n))
    pairs.sort(key=lambda p: p[0])
    for card_id, n in pairs:
        expanded.extend([card_id] * n)
    return expanded


def _normalize_sequence(ids: Sequence[int]) -> list[int]:
    out: list[int] = []
    for value in ids:
        try:
            out.append(int(value))
        except Exception as exc:
            raise DeckSpecError(f"deck list contains non-integer value {value!r}") from exc
    return out


def _coerce_to_id_list(deck_input: DeckInput) -> list[int]:
    resolved = _resolve_deck_input(deck_input)
    if isinstance(resolved, Mapping):
        return _expand_mapping(resolved)
    if isinstance(resolved, Sequence) and not isinstance(resolved, (str, bytes, bytearray)):
        return _normalize_sequence(resolved)
    raise DeckSpecError(
        "deck must resolve to a list of ids or a count map "
        "(accepted forms: list, map, preset string, file path)"
    )


def _validate_catalog_membership(ids: list[int]) -> None:
    for card_id in ids:
        try:
            get_card(card_id)
        except Exception as exc:
            raise DeckValidationError(f"unknown card id {card_id}") from exc


def _validate_profile_allowlist(ids: list[int], rules_profile: RulesProfile) -> None:
    strict = rules_profile == "strict"
    for card_id in ids:
        card = get_card(card_id)
        if strict and not card.strict_ok:
            raise DeckValidationError(
                f"card id {card_id} ({card.card_no}) is not strict-supported in parsed_only mode"
            )
        if not strict and not card.approx_ok:
            raise DeckValidationError(
                f"card id {card_id} ({card.card_no}) is not approx-supported in parsed_only mode"
            )


def resolve_deck(
    deck_input: DeckInput,
    *,
    rules_profile: RulesProfile,
    card_pool: CardPoolMode,
    db_path: str | Path | None = None,
) -> list[int]:
    rules_profile = _normalize_profile(rules_profile)
    card_pool = _normalize_card_pool(card_pool)
    ids = _coerce_to_id_list(deck_input)
    if len(ids) != _DECK_SIZE:
        raise DeckValidationError(f"deck must have exactly {_DECK_SIZE} cards (got {len(ids)})")
    _validate_catalog_membership(ids)
    assert_parsed_only_catalog_match(card_pool, db_path)
    if card_pool == "parsed_only":
        _validate_profile_allowlist(ids, rules_profile)
    return ids


def resolve_match_decks(
    deck: DeckInput,
    opponent_deck: DeckInput | None,
    *,
    rules_profile: RulesProfile,
    card_pool: CardPoolMode,
    db_path: str | Path | None = None,
) -> tuple[list[int], list[int]]:
    player = resolve_deck(deck, rules_profile=rules_profile, card_pool=card_pool, db_path=db_path)
    if opponent_deck is None:
        return player, list(player)
    opp = resolve_deck(
        opponent_deck,
        rules_profile=rules_profile,
        card_pool=card_pool,
        db_path=db_path,
    )
    return player, opp
