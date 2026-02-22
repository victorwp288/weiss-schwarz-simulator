from __future__ import annotations

from collections import Counter

import pytest
import weiss_sim


def _starter_ids() -> list[int]:
    return weiss_sim.cards.resolve_deck(
        "starter_v1",
        rules_profile="approx",
        card_pool="all",
    )


def test_deck_builder_mutation_and_deterministic_serialization():
    starter_unique = sorted(set(_starter_ids()))
    assert len(starter_unique) >= 2
    id_a, id_b = starter_unique[0], starter_unique[1]

    builder = weiss_sim.DeckBuilder()
    builder.add(id_b, 3).add(id_a, 2).remove(id_b, 1).set_count(id_a, 4)

    assert builder.count(id_a) == 4
    assert builder.count(id_b) == 2
    assert builder.total_cards() == 6
    assert builder.remaining_slots(50) == 44
    assert builder.to_id_map() == {id_a: 4, id_b: 2}
    assert builder.to_id_list() == [id_a, id_a, id_a, id_a, id_b, id_b]

    card_no_map = builder.to_card_no_map()
    assert list(card_no_map.keys()) == sorted(card_no_map.keys())
    assert sum(card_no_map.values()) == 6


def test_deck_builder_validate_build_and_describe():
    builder = weiss_sim.cards.builder(initial="starter_v1")
    report = builder.validate(rules_profile="approx", card_pool="all")
    assert report.ok
    assert not report.errors
    assert len(report.resolved_ids) == 50

    built = builder.build(rules_profile="approx", card_pool="all")
    starter = _starter_ids()
    assert len(built) == 50
    assert Counter(built) == Counter(starter)

    details = builder.describe(rules_profile="approx", card_pool="all")
    assert details["ids"] == built
    assert len(details["cards"]) == 50
    assert details["counts"]


def test_deck_builder_build_raises_on_invalid_deck():
    builder = weiss_sim.DeckBuilder()
    starter_unique = sorted(set(_starter_ids()))
    builder.add(starter_unique[0], 4)

    report = builder.validate(rules_profile="approx", card_pool="all")
    assert not report.ok
    assert any(issue.code == "deck_length" for issue in report.errors)

    with pytest.raises(weiss_sim.DeckValidationError):
        builder.build(rules_profile="approx", card_pool="all")


def test_deck_builder_rejects_invalid_counts():
    builder = weiss_sim.DeckBuilder()
    with pytest.raises(weiss_sim.DeckSpecError):
        builder.add(1, 0)
    with pytest.raises(weiss_sim.DeckSpecError):
        builder.remove(1, 0)
    with pytest.raises(weiss_sim.DeckSpecError):
        builder.set_count(1, -1)
