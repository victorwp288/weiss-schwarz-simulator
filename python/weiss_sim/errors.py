from __future__ import annotations


class WeissSimError(Exception):
    """Base error for high-level weiss_sim API failures."""


class DeckSpecError(WeissSimError):
    """Deck input format is invalid or cannot be resolved."""


class CardLookupError(DeckSpecError):
    """Card identifier could not be resolved in the packaged catalog."""


class DeckValidationError(DeckSpecError):
    """Resolved deck violates high-level validation rules."""


class ConfigConflictError(WeissSimError):
    """Requested high-level options are mutually incompatible."""


class DbMismatchError(WeissSimError):
    """`card_pool=\"parsed_only\"` was requested against a mismatched DB hash."""

    def __init__(
        self,
        *,
        expected_db_sha256: str,
        actual_db_sha256: str,
        remediation: str | None = None,
    ) -> None:
        self.expected_db_sha256 = expected_db_sha256
        self.actual_db_sha256 = actual_db_sha256
        self.remediation = remediation or (
            "Use the packaged default DB, switch card_pool='all', "
            "or regenerate catalog artifacts for the external DB."
        )
        super().__init__(
            "card_pool='parsed_only' requires DB hash match with packaged catalog "
            f"(expected {self.expected_db_sha256}, got {self.actual_db_sha256}). "
            f"Remediation: {self.remediation}"
        )
