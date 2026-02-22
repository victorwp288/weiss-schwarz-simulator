from __future__ import annotations

import re
from collections import Counter
from collections.abc import Mapping
from pathlib import Path
from typing import Literal

from ._config_norm import normalize_card_pool, normalize_rules_profile
from ._deck_inputs import coerce_to_id_list
from .catalog import assert_parsed_only_catalog_match, get_card, suggest_cards
from .config_types import CardPoolMode, DeckInput, RulesProfile
from .errors import DbMismatchError, DeckSpecError, DeckValidationError
from .types import DeckValidationIssue, DeckValidationReport
from .weiss_sim import EnvPool

_DECK_SIZE = 50
_UNKNOWN_CARD_IDENTIFIER_RE = re.compile(
    r"unknown card identifier ['\"]?([^'\"]+)['\"]?(?:;\s*suggestions:\s*(.*))?",
    re.I,
)


def _issue(
    *,
    code: str,
    message: str,
    severity: Literal["error", "warning"],
    card_id: int | None = None,
    card_no: str | None = None,
    got: int | None = None,
    max_allowed: int | None = None,
    suggestions: list[str] | None = None,
) -> DeckValidationIssue:
    return DeckValidationIssue(
        code=code,
        message=message,
        severity=severity,
        card_id=card_id,
        card_no=card_no,
        got=got,
        max_allowed=max_allowed,
        suggestions=list(suggestions or []),
    )


def _build_report(
    *,
    deck_size: int,
    resolved_ids: list[int],
    errors: list[DeckValidationIssue],
    warnings: list[DeckValidationIssue],
) -> DeckValidationReport:
    counts = Counter(resolved_ids)
    return DeckValidationReport(
        ok=not errors,
        deck_size=deck_size,
        resolved_ids=list(resolved_ids),
        errors=list(errors),
        warnings=list(warnings),
        summary={
            "error_count": len(errors),
            "warning_count": len(warnings),
            "total_cards": len(resolved_ids),
            "unique_cards": len(counts),
        },
    )


def _parse_unknown_identifier_from_spec_error(
    message: str,
) -> tuple[str | None, list[str]]:
    match = _UNKNOWN_CARD_IDENTIFIER_RE.search(message)
    if not match:
        return None, []
    token = (match.group(1) or "").strip()
    suggestion_blob = (match.group(2) or "").strip()
    if not suggestion_blob:
        return token, []
    suggestions = [item.strip() for item in suggestion_blob.split(",") if item.strip()]
    return token, suggestions


def _db_validation_issues(
    deck_lists: list[list[int]],
    db_path: str | Path | None,
) -> list[Mapping[str, object]]:
    db_path_str = None if db_path is None else str(db_path)
    raw = EnvPool.validate_deck_issues(
        deck_lists=deck_lists,
        db_path=db_path_str,
        deck_ids=[0, 1],
    )
    return [entry for entry in raw if isinstance(entry, Mapping)]


def _issues_for_player(
    issues: list[Mapping[str, object]],
    player: int,
) -> list[Mapping[str, object]]:
    return [issue for issue in issues if int(issue.get("player", -1)) == int(player)]


def _validate_catalog_membership(ids: list[int]) -> None:
    for card_id in ids:
        try:
            get_card(card_id)
        except Exception as exc:
            raise DeckValidationError(f"unknown card id {card_id}") from exc


def _can_skip_db_probe(
    card_pool: CardPoolMode,
    db_hash_info: Mapping[str, object] | None,
    *,
    db_hash_mismatch: bool = False,
) -> bool:
    if card_pool != "parsed_only":
        return False
    if db_hash_mismatch:
        return True
    return bool(db_hash_info and db_hash_info.get("matches_catalog"))


def _validate_db_membership(
    ids: list[int],
    db_path: str | Path | None,
    *,
    skip_probe: bool = False,
) -> None:
    if skip_probe:
        return
    try:
        issues = _issues_for_player(_db_validation_issues([ids, ids], db_path), player=0)
    except Exception as exc:
        raise DeckValidationError(f"failed to validate deck against selected DB: {exc}") from exc

    missing: list[int] = sorted(
        {
            int(issue["card_id"])
            for issue in issues
            if issue.get("kind") == "unknown_card_id" and issue.get("card_id") is not None
        }
    )
    if missing:
        preview = ", ".join(str(card_id) for card_id in missing[:8])
        suffix = "" if len(missing) <= 8 else f", ... (+{len(missing) - 8} more)"
        raise DeckValidationError(f"card id(s) not present in selected DB: {preview}{suffix}")


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


def validate_deck(
    deck_input: DeckInput,
    *,
    rules_profile: RulesProfile,
    card_pool: CardPoolMode,
    db_path: str | Path | None = None,
    deck_size: int = _DECK_SIZE,
) -> DeckValidationReport:
    errors: list[DeckValidationIssue] = []
    warnings: list[DeckValidationIssue] = []
    resolved_ids: list[int] = []
    card_cache: dict[int, object] = {}

    try:
        normalized_deck_size = int(deck_size)
    except Exception:
        errors.append(
            _issue(
                code="invalid_input",
                message=f"deck_size must be an integer (got {deck_size!r})",
                severity="error",
            )
        )
        return _build_report(
            deck_size=_DECK_SIZE,
            resolved_ids=resolved_ids,
            errors=errors,
            warnings=warnings,
        )
    # Keep deck_size in the signature for compatibility, but enforce
    # the canonical Weiss Schwarz deck length.
    if normalized_deck_size != _DECK_SIZE:
        errors.append(
            _issue(
                code="invalid_input",
                message=f"deck_size is fixed at {_DECK_SIZE} (got {normalized_deck_size})",
                severity="error",
            )
        )
        return _build_report(
            deck_size=_DECK_SIZE,
            resolved_ids=resolved_ids,
            errors=errors,
            warnings=warnings,
        )

    try:
        rules_profile = normalize_rules_profile(rules_profile, error_cls=DeckSpecError)
        card_pool = normalize_card_pool(card_pool, error_cls=DeckSpecError)
    except DeckSpecError as exc:
        errors.append(
            _issue(
                code="invalid_input",
                message=str(exc),
                severity="error",
            )
        )
        return _build_report(
            deck_size=normalized_deck_size,
            resolved_ids=resolved_ids,
            errors=errors,
            warnings=warnings,
        )

    try:
        resolved_ids = coerce_to_id_list(deck_input)
    except DeckSpecError as exc:
        detail = str(exc)
        token, parsed_suggestions = _parse_unknown_identifier_from_spec_error(detail)
        if token is None:
            errors.append(
                _issue(
                    code="invalid_input",
                    message=detail,
                    severity="error",
                )
            )
            return _build_report(
                deck_size=normalized_deck_size,
                resolved_ids=resolved_ids,
                errors=errors,
                warnings=warnings,
            )
        suggestions = parsed_suggestions or [card.card_no for card in suggest_cards(token, limit=5)]
        errors.append(
            _issue(
                code="unknown_card",
                message=f"unknown card identifier {token!r}",
                severity="error",
                suggestions=suggestions,
            )
        )
        return _build_report(
            deck_size=normalized_deck_size,
            resolved_ids=resolved_ids,
            errors=errors,
            warnings=warnings,
        )

    if len(resolved_ids) != normalized_deck_size:
        errors.append(
            _issue(
                code="deck_length",
                message=f"deck must have exactly {normalized_deck_size} cards (got {len(resolved_ids)})",
                severity="error",
                got=len(resolved_ids),
                max_allowed=normalized_deck_size,
            )
        )
    has_local_deck_length_error = len(resolved_ids) != normalized_deck_size

    unknown_ids: set[int] = set()
    for card_id in sorted(set(resolved_ids)):
        try:
            card_cache[card_id] = get_card(card_id)
        except Exception:
            unknown_ids.add(card_id)
            suggestions = [card.card_no for card in suggest_cards(str(card_id), limit=5)]
            errors.append(
                _issue(
                    code="unknown_card",
                    message=f"unknown card id {card_id}",
                    severity="error",
                    card_id=card_id,
                    suggestions=suggestions,
                )
            )

    counts = Counter(resolved_ids)
    if not unknown_ids:
        climax_count = 0
        for card_id, count in counts.items():
            card = card_cache.get(card_id)
            if card is None:
                continue
            if getattr(card, "card_type", "").lower() == "climax":
                climax_count += int(count)

        if climax_count == 8:
            warnings.append(
                _issue(
                    code="climax_count_at_limit",
                    message="climax cards at limit: 8",
                    severity="warning",
                    got=climax_count,
                    max_allowed=8,
                )
            )

        for card_id, count in sorted(counts.items(), key=lambda item: item[0]):
            card = card_cache.get(card_id)
            if card is None:
                continue
            card_no = getattr(card, "card_no", None)
            if count == 4:
                warnings.append(
                    _issue(
                        code="copy_count_at_limit",
                        message=f"card {card_id} is at copy limit: 4",
                        severity="warning",
                        card_id=card_id,
                        card_no=card_no,
                        got=int(count),
                        max_allowed=4,
                    )
                )

    db_hash_info: Mapping[str, object] | None = None
    db_hash_mismatch = False
    db_hash_check_failed = False
    try:
        db_hash_info = assert_parsed_only_catalog_match(card_pool, db_path)
    except DbMismatchError as exc:
        db_hash_mismatch = True
        errors.append(
            _issue(
                code="db_hash_mismatch",
                message=str(exc),
                severity="error",
            )
        )
    except Exception as exc:
        db_hash_check_failed = True
        errors.append(
            _issue(
                code="db_validation_failed",
                message=f"failed to validate deck against selected DB: {exc}",
                severity="error",
            )
        )

    if (
        resolved_ids
        and not unknown_ids
        and not db_hash_check_failed
        and not _can_skip_db_probe(card_pool, db_hash_info, db_hash_mismatch=db_hash_mismatch)
    ):
        try:
            rust_issues = _issues_for_player(
                _db_validation_issues([resolved_ids, resolved_ids], db_path),
                player=0,
            )
        except Exception as exc:
            errors.append(
                _issue(
                    code="db_validation_failed",
                    message=f"failed to validate deck against selected DB: {exc}",
                    severity="error",
                )
            )
            rust_issues = []

        for issue in rust_issues:
            kind = str(issue.get("kind", ""))
            if kind == "unknown_card_id":
                card_id = int(issue.get("card_id", -1))
                card = card_cache.get(card_id)
                card_no = getattr(card, "card_no", None) if card is not None else None
                errors.append(
                    _issue(
                        code="db_card_missing",
                        message=f"card id {card_id} is not present in selected DB",
                        severity="error",
                        card_id=card_id,
                        card_no=card_no,
                    )
                )
                continue
            if kind == "deck_length":
                if has_local_deck_length_error:
                    continue
                got = int(issue.get("got", len(resolved_ids)))
                expected = int(issue.get("expected", _DECK_SIZE))
                errors.append(
                    _issue(
                        code="deck_length",
                        message=f"deck must have exactly {expected} cards (got {got})",
                        severity="error",
                        got=got,
                        max_allowed=expected,
                    )
                )
                has_local_deck_length_error = True
                continue
            if kind == "climax_count":
                got = int(issue.get("got", 0))
                max_allowed = int(issue.get("max", 8))
                errors.append(
                    _issue(
                        code="climax_count_exceeded",
                        message=f"too many climax cards: got {got}, max {max_allowed}",
                        severity="error",
                        got=got,
                        max_allowed=max_allowed,
                    )
                )
                continue
            if kind == "card_copy_count":
                card_id = int(issue.get("card_id", -1))
                card = card_cache.get(card_id)
                card_no = getattr(card, "card_no", None) if card is not None else None
                got = int(issue.get("got", 0))
                max_allowed = int(issue.get("max", 4))
                errors.append(
                    _issue(
                        code="copy_count_exceeded",
                        message=f"too many copies of card {card_id}: got {got}, max {max_allowed}",
                        severity="error",
                        card_id=card_id,
                        card_no=card_no,
                        got=got,
                        max_allowed=max_allowed,
                    )
                )

    if card_pool == "parsed_only" and not unknown_ids:
        strict = rules_profile == "strict"
        for card_id in sorted(set(resolved_ids)):
            card = card_cache.get(card_id)
            if card is None:
                continue
            card_no = getattr(card, "card_no", None)
            if strict and not bool(getattr(card, "strict_ok", False)):
                errors.append(
                    _issue(
                        code="profile_not_supported",
                        message=(
                            f"card id {card_id} ({card_no}) is not strict-supported in parsed_only mode"
                        ),
                        severity="error",
                        card_id=card_id,
                        card_no=card_no,
                    )
                )
            if not strict and not bool(getattr(card, "approx_ok", False)):
                errors.append(
                    _issue(
                        code="profile_not_supported",
                        message=(
                            f"card id {card_id} ({card_no}) is not approx-supported in parsed_only mode"
                        ),
                        severity="error",
                        card_id=card_id,
                        card_no=card_no,
                    )
                )

    return _build_report(
        deck_size=normalized_deck_size,
        resolved_ids=resolved_ids,
        errors=errors,
        warnings=warnings,
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
    db_hash_info = assert_parsed_only_catalog_match(card_pool, db_path)
    _validate_db_membership(
        ids,
        db_path,
        skip_probe=_can_skip_db_probe(card_pool, db_hash_info),
    )
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


def _card_payload(card_id: int) -> dict[str, object]:
    card = get_card(card_id)
    return {
        "id": card.id,
        "card_no": card.card_no,
        "name": card.name,
        "card_type": card.card_type,
        "card_set": card.card_set,
        "strict_ok": card.strict_ok,
        "approx_ok": card.approx_ok,
    }


def summarize_resolved_deck(ids: list[int]) -> dict[str, object]:
    counts = Counter(ids)
    return {
        "ids": list(ids),
        "cards": [_card_payload(card_id) for card_id in ids],
        "counts": [
            {
                **_card_payload(card_id),
                "count": int(count),
            }
            for card_id, count in sorted(counts.items(), key=lambda item: item[0])
        ],
    }


def resolve_deck_details(
    deck_input: DeckInput,
    *,
    rules_profile: RulesProfile,
    card_pool: CardPoolMode,
    db_path: str | Path | None = None,
) -> dict[str, object]:
    ids = resolve_deck(
        deck_input,
        rules_profile=rules_profile,
        card_pool=card_pool,
        db_path=db_path,
    )
    return summarize_resolved_deck(ids)
