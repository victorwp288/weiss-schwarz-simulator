from __future__ import annotations

from collections import Counter
from pathlib import Path
from typing import Literal

from ._deck_inputs import coerce_to_id_list
from .catalog import get_card, resolve_card_id
from .config_types import CardPoolMode, DeckInput, RulesProfile
from .decks import summarize_resolved_deck, validate_deck
from .errors import DeckSpecError, DeckValidationError
from .types import DeckValidationReport


class DeckBuilder:
    """Fluent deck authoring helper backed by card-id counts."""

    def __init__(self, initial: DeckInput | None = None) -> None:
        self._counts: dict[int, int] = {}
        if initial is None:
            return
        ids = coerce_to_id_list(initial)
        self._counts = dict(sorted(Counter(ids).items(), key=lambda item: item[0]))

    def _resolve(self, card: int | str) -> int:
        if isinstance(card, int):
            return int(card)
        token = str(card).strip()
        if not token:
            raise DeckSpecError("card identifier must be non-empty")
        return int(resolve_card_id(token))

    def _coerce_positive_count(self, count: int, *, field_name: str) -> int:
        try:
            value = int(count)
        except Exception as exc:
            raise DeckSpecError(f"{field_name} must be an integer") from exc
        if value <= 0:
            raise DeckSpecError(f"{field_name} must be > 0")
        return value

    def add(self, card: int | str, count: int = 1) -> DeckBuilder:
        """Add `count` copies of a card to the builder."""
        n = self._coerce_positive_count(count, field_name="count")
        card_id = self._resolve(card)
        self._counts[card_id] = int(self._counts.get(card_id, 0) + n)
        return self

    def remove(self, card: int | str, count: int = 1) -> DeckBuilder:
        """Remove up to `count` copies of a card from the builder."""
        n = self._coerce_positive_count(count, field_name="count")
        card_id = self._resolve(card)
        current = int(self._counts.get(card_id, 0))
        remaining = current - n
        if remaining > 0:
            self._counts[card_id] = int(remaining)
        else:
            self._counts.pop(card_id, None)
        return self

    def set_count(self, card: int | str, count: int) -> DeckBuilder:
        """Set the exact copy count for a card (`0` removes it)."""
        try:
            n = int(count)
        except Exception as exc:
            raise DeckSpecError("count must be an integer") from exc
        if n < 0:
            raise DeckSpecError("count must be non-negative")
        card_id = self._resolve(card)
        if n == 0:
            self._counts.pop(card_id, None)
        else:
            self._counts[card_id] = int(n)
        return self

    def count(self, card: int | str) -> int:
        """Return current copy count for a card."""
        return int(self._counts.get(self._resolve(card), 0))

    def total_cards(self) -> int:
        """Return total cards currently in the builder."""
        return int(sum(self._counts.values()))

    def remaining_slots(self, deck_size: int = 50) -> int:
        """Return remaining cards needed to reach `deck_size`."""
        return int(deck_size) - self.total_cards()

    def to_id_map(self) -> dict[int, int]:
        """Return deterministic `{card_id: count}` payload."""
        return {card_id: int(self._counts[card_id]) for card_id in sorted(self._counts)}

    def to_card_no_map(self) -> dict[str, int]:
        """Return deterministic `{card_no: count}` payload."""
        entries = sorted(
            ((get_card(card_id).card_no, int(count)) for card_id, count in self._counts.items()),
            key=lambda item: item[0],
        )
        return {card_no: count for card_no, count in entries}

    def to_id_list(self, order: Literal["id_asc"] = "id_asc") -> list[int]:
        """Return expanded card-id list in deterministic order."""
        if order != "id_asc":
            raise DeckSpecError("order must be 'id_asc'")
        out: list[int] = []
        for card_id in sorted(self._counts):
            out.extend([card_id] * int(self._counts[card_id]))
        return out

    def describe(
        self,
        *,
        rules_profile: RulesProfile,
        card_pool: CardPoolMode,
        db_path: str | Path | None = None,
        deck_size: int = 50,
    ) -> dict[str, object]:
        """Validate/build and return resolved deck metadata summary."""
        ids = self.build(
            rules_profile=rules_profile,
            card_pool=card_pool,
            db_path=db_path,
            deck_size=deck_size,
        )
        return summarize_resolved_deck(ids)

    def validate(
        self,
        *,
        rules_profile: RulesProfile,
        card_pool: CardPoolMode,
        db_path: str | Path | None = None,
        deck_size: int = 50,
    ) -> DeckValidationReport:
        """Validate without raising and return a structured report."""
        return validate_deck(
            self.to_id_map(),
            rules_profile=rules_profile,
            card_pool=card_pool,
            db_path=db_path,
            deck_size=deck_size,
        )

    def build(
        self,
        *,
        rules_profile: RulesProfile,
        card_pool: CardPoolMode,
        db_path: str | Path | None = None,
        deck_size: int = 50,
    ) -> list[int]:
        """Validate and return final deck ids, raising on invalid decks."""
        report = self.validate(
            rules_profile=rules_profile,
            card_pool=card_pool,
            db_path=db_path,
            deck_size=deck_size,
        )
        if report.ok:
            return list(report.resolved_ids)
        message = (
            "; ".join(issue.message for issue in report.errors[:3]) or "deck validation failed"
        )
        raise DeckValidationError(message)
