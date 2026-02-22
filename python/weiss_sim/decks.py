from __future__ import annotations

import json
import re
from pathlib import Path

from .weiss_sim import EnvPool
from ._config_norm import normalize_card_pool, normalize_rules_profile
from ._deck_inputs import coerce_to_id_list
from .catalog import assert_parsed_only_catalog_match, get_card
from .config_types import CardPoolMode, DeckInput, RulesProfile
from .errors import DeckSpecError, DeckValidationError

_DECK_SIZE = 50
_DB_PROBE_CURRICULUM_JSON = json.dumps(
    {
        "enable_visibility_policies": True,
        "enforce_color_requirement": False,
        "enforce_cost_requirement": False,
    },
    separators=(",", ":"),
)
_UNKNOWN_CARD_ID_RE = re.compile(r"unknown card id\s+(\d+)", re.I)
_DB_PROBE_NON_MEMBERSHIP_ERR_FRAGMENTS = (
    "deck length invalid",
    "too many climax cards",
    "too many copies of card",
)


def _validate_catalog_membership(ids: list[int]) -> None:
    for card_id in ids:
        try:
            get_card(card_id)
        except Exception as exc:
            raise DeckValidationError(f"unknown card id {card_id}") from exc


def _validate_db_membership(ids: list[int], db_path: str | Path | None) -> None:
    db_path_str = None if db_path is None else str(db_path)
    try:
        EnvPool.new_debug(
            1,
            db_path=db_path_str,
            deck_lists=[ids, ids],
            deck_ids=[0, 1],
            max_decisions=1,
            max_ticks=1,
            seed=0,
            curriculum_json=_DB_PROBE_CURRICULUM_JSON,
            reward_json=None,
            end_condition_policy_json=None,
            error_policy="lenient_terminate",
            observation_visibility="public",
            num_threads=1,
            debug_fingerprint_every_n=0,
            debug_event_ring_capacity=128,
        )
        return
    except Exception as exc:
        detail = str(exc)
        missing = sorted({int(m.group(1)) for m in _UNKNOWN_CARD_ID_RE.finditer(detail)})
        if not missing:
            detail_lower = detail.lower()
            if any(fragment in detail_lower for fragment in _DB_PROBE_NON_MEMBERSHIP_ERR_FRAGMENTS):
                return
            raise DeckValidationError(
                f"failed to validate deck against selected DB: {exc}"
            ) from exc
        preview = ", ".join(str(card_id) for card_id in missing[:8])
        suffix = "" if len(missing) <= 8 else f", ... (+{len(missing) - 8} more)"
        raise DeckValidationError(
            f"card id(s) not present in selected DB: {preview}{suffix}"
        ) from exc


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
    rules_profile = normalize_rules_profile(rules_profile, error_cls=DeckSpecError)
    card_pool = normalize_card_pool(card_pool, error_cls=DeckSpecError)
    ids = coerce_to_id_list(deck_input)
    if len(ids) != _DECK_SIZE:
        raise DeckValidationError(f"deck must have exactly {_DECK_SIZE} cards (got {len(ids)})")
    _validate_catalog_membership(ids)
    assert_parsed_only_catalog_match(card_pool, db_path)
    _validate_db_membership(ids, db_path)
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
