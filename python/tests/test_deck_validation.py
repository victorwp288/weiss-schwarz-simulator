from __future__ import annotations

import gzip
import json
from collections import Counter
from pathlib import Path

import pytest
import weiss_sim
import weiss_sim.decks as decks_mod


def _catalog_rows() -> list[dict[str, object]]:
    catalog_path = (
        Path(__file__).resolve().parents[1] / "weiss_sim" / "data" / "card_catalog.json.gz"
    )
    rows = json.loads(gzip.decompress(catalog_path.read_bytes()).decode("utf-8"))
    return [row for row in rows if isinstance(row, dict)]


def _ids(
    rows: list[dict[str, object]],
    *,
    card_type: str | None = None,
    strict_ok: bool | None = None,
    approx_ok: bool | None = None,
) -> list[int]:
    out: list[int] = []
    for row in rows:
        rid = row.get("id")
        if rid is None:
            continue
        if card_type is not None and str(row.get("card_type", "")).lower() != card_type.lower():
            continue
        if strict_ok is not None and bool(row.get("strict_ok", False)) != strict_ok:
            continue
        if approx_ok is not None and bool(row.get("approx_ok", False)) != approx_ok:
            continue
        out.append(int(rid))
    return sorted(set(out))


def _deck_from_counts(pairs: list[tuple[int, int]]) -> list[int]:
    deck: list[int] = []
    for card_id, count in sorted(pairs, key=lambda item: item[0]):
        deck.extend([int(card_id)] * int(count))
    assert len(deck) == 50
    return deck


def _starter_ids() -> list[int]:
    return weiss_sim.cards.resolve_deck(
        "starter_deck_ws02_v1", rules_profile="approx", card_pool="all"
    )


def test_validate_deck_invalid_input_code():
    report = weiss_sim.cards.validate_deck(
        123,  # type: ignore[arg-type]
        rules_profile="approx",
        card_pool="all",
    )
    assert not report.ok
    assert any(issue.code == "invalid_input" for issue in report.errors)


def test_validate_deck_deck_length_code():
    starter = _starter_ids()
    report = weiss_sim.cards.validate_deck(
        starter[:49],
        rules_profile="approx",
        card_pool="all",
    )
    assert not report.ok
    assert any(issue.code == "deck_length" for issue in report.errors)


def test_validate_deck_deck_size_is_fixed_at_50():
    starter = _starter_ids()
    report = weiss_sim.cards.validate_deck(
        starter[:40],
        rules_profile="approx",
        card_pool="all",
        deck_size=40,
    )
    assert not report.ok
    assert report.deck_size == 50
    assert any(issue.code == "invalid_input" for issue in report.errors)
    assert any("fixed at 50" in issue.message for issue in report.errors)


def test_validate_deck_deck_length_error_is_not_duplicated():
    starter = _starter_ids()
    report = weiss_sim.cards.validate_deck(
        starter[:49],
        rules_profile="approx",
        card_pool="all",
    )
    length_errors = [issue for issue in report.errors if issue.code == "deck_length"]
    assert len(length_errors) == 1


def test_validate_deck_unknown_card_with_suggestions():
    seed_card = weiss_sim.cards.get(1).card_no
    report = weiss_sim.cards.validate_deck(
        {f"{seed_card}-typo": 50},
        rules_profile="approx",
        card_pool="all",
    )
    assert not report.ok
    unknown = [issue for issue in report.errors if issue.code == "unknown_card"]
    assert unknown
    assert unknown[0].suggestions


def test_validate_deck_db_hash_mismatch_code(tmp_path: Path):
    default_wsdb = Path(__file__).resolve().parents[1] / "weiss_sim" / "data" / "default_cards.wsdb"
    bad_wsdb = tmp_path / "bad.wsdb"
    bad_wsdb.write_bytes(default_wsdb.read_bytes() + b"\x00")

    report = weiss_sim.cards.validate_deck(
        "starter_deck_ws02_v1",
        rules_profile="strict",
        card_pool="parsed_only",
        db_path=str(bad_wsdb),
    )
    assert not report.ok
    assert any(issue.code == "db_hash_mismatch" for issue in report.errors)


def test_validate_deck_db_card_missing_code(monkeypatch):
    starter = _starter_ids()
    ids = sorted(set(starter))
    assert len(ids) >= 2
    missing_id = ids[1]

    def fake_validate_deck_issues(*, deck_lists, **_kwargs):
        assert deck_lists is not None
        return [{"kind": "unknown_card_id", "player": 0, "card_id": missing_id}]

    monkeypatch.setattr(decks_mod.EnvPool, "validate_deck_issues", fake_validate_deck_issues)

    report = weiss_sim.cards.validate_deck(
        starter,
        rules_profile="approx",
        card_pool="all",
    )
    assert not report.ok
    assert any(
        issue.code == "db_card_missing" and issue.card_id == missing_id for issue in report.errors
    )


def test_validate_deck_db_validation_failed_code(monkeypatch):
    def fake_validate_deck_issues(**_kwargs):
        raise RuntimeError("Failed to decode card db payload")

    monkeypatch.setattr(decks_mod.EnvPool, "validate_deck_issues", fake_validate_deck_issues)

    report = weiss_sim.cards.validate_deck(
        "starter_deck_ws02_v1",
        rules_profile="approx",
        card_pool="all",
    )
    assert not report.ok
    assert any(issue.code == "db_validation_failed" for issue in report.errors)


@pytest.mark.parametrize(("card_pool",), [("all",), ("parsed_only",)])
def test_validate_deck_missing_db_path_reports_error(tmp_path: Path, card_pool: str):
    missing_db = tmp_path / "missing.wsdb"
    report = weiss_sim.cards.validate_deck(
        "starter_deck_ws02_v1",
        rules_profile="approx",
        card_pool=card_pool,  # type: ignore[arg-type]
        db_path=str(missing_db),
    )
    assert not report.ok
    assert any(issue.code == "db_validation_failed" for issue in report.errors)


def test_validate_deck_copy_and_climax_limit_codes():
    starter_details = weiss_sim.cards.describe_deck(
        "starter_deck_ws02_v1",
        rules_profile="approx",
        card_pool="all",
    )
    counts_payload = starter_details["counts"]
    assert isinstance(counts_payload, list)
    climax_ids = sorted(
        {
            int(item["id"])
            for item in counts_payload
            if str(item.get("card_type", "")).lower() == "climax"
        }
    )
    non_climax_ids = sorted(
        {
            int(item["id"])
            for item in counts_payload
            if str(item.get("card_type", "")).lower() != "climax"
        }
    )
    if not climax_ids or len(non_climax_ids) < 13:
        pytest.skip("starter_deck_ws02_v1 does not expose enough ids for limit tests")

    copy_exceeded_deck = _deck_from_counts(
        [(non_climax_ids[0], 5)]
        + [(card_id, 4) for card_id in non_climax_ids[1:12]]
        + [(non_climax_ids[12], 1)]
    )
    copy_report = weiss_sim.cards.validate_deck(
        copy_exceeded_deck,
        rules_profile="approx",
        card_pool="all",
    )
    assert not copy_report.ok
    assert any(issue.code == "copy_count_exceeded" for issue in copy_report.errors)

    # Exceed total climax cap while keeping deck length fixed.
    climax_exceeded_deck = _deck_from_counts(
        [(climax_ids[0], 9)]
        + [(card_id, 4) for card_id in non_climax_ids[:10]]
        + [(non_climax_ids[10], 1)]
    )
    climax_report = weiss_sim.cards.validate_deck(
        climax_exceeded_deck,
        rules_profile="approx",
        card_pool="all",
    )
    assert not climax_report.ok
    assert any(issue.code == "climax_count_exceeded" for issue in climax_report.errors)


def test_validate_deck_profile_not_supported_code():
    rows = _catalog_rows()
    strict_unsupported = _ids(rows, strict_ok=False)
    if not strict_unsupported:
        pytest.skip("catalog has no strict_unsupported cards")
    target = strict_unsupported[0]
    all_ids = _ids(rows)
    others = [card_id for card_id in all_ids if card_id != target]
    if len(others) < 13:
        pytest.skip("catalog does not expose enough ids for profile test")

    deck = _deck_from_counts(
        [(target, 1)] + [(card_id, 4) for card_id in others[:12]] + [(others[12], 1)]
    )
    report = weiss_sim.cards.validate_deck(
        deck,
        rules_profile="strict",
        card_pool="parsed_only",
    )
    assert not report.ok
    assert any(
        issue.code == "profile_not_supported" and issue.card_id == target for issue in report.errors
    )


def test_validate_deck_warning_codes():
    starter_details = weiss_sim.cards.describe_deck(
        "starter_deck_ws02_v1",
        rules_profile="approx",
        card_pool="all",
    )
    counts_payload = starter_details["counts"]
    assert isinstance(counts_payload, list)
    climax_ids = sorted(
        {
            int(item["id"])
            for item in counts_payload
            if str(item.get("card_type", "")).lower() == "climax"
        }
    )
    non_climax_ids = sorted(
        {
            int(item["id"])
            for item in counts_payload
            if str(item.get("card_type", "")).lower() != "climax"
        }
    )
    if len(climax_ids) < 2 or len(non_climax_ids) < 11:
        pytest.skip("starter_deck_ws02_v1 does not expose enough ids for warning tests")

    warning_deck = _deck_from_counts(
        [(climax_ids[0], 4), (climax_ids[1], 4)]
        + [(card_id, 4) for card_id in non_climax_ids[:10]]
        + [(non_climax_ids[10], 2)]
    )
    report = weiss_sim.cards.validate_deck(
        warning_deck,
        rules_profile="approx",
        card_pool="all",
    )
    assert report.ok
    warning_codes = {issue.code for issue in report.warnings}
    assert "climax_count_at_limit" in warning_codes
    assert "copy_count_at_limit" in warning_codes

    resolved_counts = Counter(report.resolved_ids)
    assert sum(resolved_counts.values()) == 50
