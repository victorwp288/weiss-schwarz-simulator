from __future__ import annotations

import json
from collections import Counter
from pathlib import Path

import pytest
import weiss_sim


def _starter_ids() -> list[int]:
    return weiss_sim.cards.resolve_deck(
        "starter_v1",
        rules_profile="approx",
        card_pool="all",
    )


def test_export_deck_default_envelope_contract():
    payload = weiss_sim.cards.export_deck(
        "starter_v1",
        rules_profile="approx",
        card_pool="all",
    )
    assert payload["format"] == "wsim_deck_v1"
    assert payload["encoding"] == "card_no_map"
    assert payload["deck_size"] == 50
    assert payload["generated_by"] == "weiss-sim"
    assert isinstance(payload["catalog_db_sha256"], str)
    assert isinstance(payload["cards"], dict)


def test_export_deck_raw_formats_roundtrip():
    expected = Counter(_starter_ids())
    for encoding in ("card_no_map", "id_map", "id_list"):
        payload = weiss_sim.cards.export_deck(
            "starter_v1",
            format=encoding,
            rules_profile="approx",
            card_pool="all",
            include_meta=False,
        )
        resolved = weiss_sim.cards.resolve_deck(
            payload,  # type: ignore[arg-type]
            rules_profile="approx",
            card_pool="all",
        )
        assert len(resolved) == 50
        assert Counter(resolved) == expected


def test_save_and_load_deck_supports_envelope_and_raw(tmp_path: Path):
    expected = Counter(_starter_ids())

    meta_path = tmp_path / "starter_meta.json"
    saved = weiss_sim.cards.save_deck(
        meta_path,
        "starter_v1",
        rules_profile="approx",
        card_pool="all",
        include_meta=True,
    )
    assert Path(saved) == meta_path

    loaded_meta = weiss_sim.cards.load_deck(meta_path)
    resolved_meta = weiss_sim.cards.resolve_deck(
        loaded_meta,
        rules_profile="approx",
        card_pool="all",
    )
    assert Counter(resolved_meta) == expected

    raw_path = tmp_path / "starter_raw.json"
    weiss_sim.cards.save_deck(
        raw_path,
        "starter_v1",
        format="id_list",
        rules_profile="approx",
        card_pool="all",
        include_meta=False,
    )
    loaded_raw = weiss_sim.cards.load_deck(raw_path)
    assert isinstance(loaded_raw, list)
    resolved_raw = weiss_sim.cards.resolve_deck(
        loaded_raw,  # type: ignore[arg-type]
        rules_profile="approx",
        card_pool="all",
    )
    assert Counter(resolved_raw) == expected


def test_load_deck_accepts_legacy_raw_json(tmp_path: Path):
    card_id = _starter_ids()[0]
    path = tmp_path / "legacy.json"
    path.write_text(json.dumps({str(card_id): 50}), encoding="utf-8")

    loaded = weiss_sim.cards.load_deck(path)
    assert isinstance(loaded, dict)
    resolved = weiss_sim.cards.resolve_deck(
        loaded,
        rules_profile="approx",
        card_pool="all",
    )
    assert resolved == [card_id] * 50


def test_resolve_deck_path_inputs_reject_directories_with_deck_spec_error(tmp_path: Path):
    for token in (str(tmp_path), f"file:{tmp_path}", "file:"):
        with pytest.raises(weiss_sim.DeckSpecError):
            weiss_sim.cards.resolve_deck(
                token,
                rules_profile="approx",
                card_pool="all",
            )


def test_load_deck_rejects_directory_path_with_deck_spec_error(tmp_path: Path):
    with pytest.raises(weiss_sim.DeckSpecError):
        weiss_sim.cards.load_deck(tmp_path)
