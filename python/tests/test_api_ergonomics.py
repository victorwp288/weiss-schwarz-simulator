from __future__ import annotations

import inspect
from pathlib import Path

import weiss_sim


def _assert_has_doc(obj: object, *, name: str) -> None:
    doc = inspect.getdoc(obj)
    assert doc is not None, f"{name} is missing a docstring"
    assert len(doc.strip()) >= 16, f"{name} docstring is too short"


def _assert_has_return_annotation(fn: object, *, name: str) -> None:
    sig = inspect.signature(fn)
    assert sig.return_annotation is not inspect.Signature.empty, (
        f"{name} is missing a return annotation"
    )


def test_package_is_typed_and_extension_stub_is_shipped() -> None:
    pkg_dir = Path(weiss_sim.__file__).resolve().parent
    assert (pkg_dir / "py.typed").is_file()

    stub_path = pkg_dir / "weiss_sim.pyi"
    assert stub_path.is_file()
    stub_text = stub_path.read_text(encoding="utf-8")
    assert "__version__: str" in stub_text


def test_high_level_entrypoints_include_docstrings_and_types() -> None:
    _assert_has_doc(weiss_sim.make, name="weiss_sim.make")
    _assert_has_doc(weiss_sim.fast, name="weiss_sim.fast")
    _assert_has_doc(weiss_sim.inspect, name="weiss_sim.inspect")

    _assert_has_return_annotation(weiss_sim.make, name="weiss_sim.make")
    _assert_has_return_annotation(weiss_sim.fast, name="weiss_sim.fast")
    _assert_has_return_annotation(weiss_sim.inspect, name="weiss_sim.inspect")

    make_params = inspect.signature(weiss_sim.make).parameters
    for key in ("mode", "deck", "opponent_deck", "num_envs", "seed"):
        assert key in make_params


def test_cards_namespace_has_typed_documented_helpers() -> None:
    for name in (
        "search",
        "suggest",
        "get",
        "presets",
        "preset_metadata",
        "preset_min_rules_profile",
        "builder",
        "resolve_deck",
        "validate_deck",
        "describe_deck",
        "export_deck",
        "save_deck",
        "load_deck",
    ):
        fn = getattr(weiss_sim.cards, name)
        _assert_has_doc(fn, name=f"weiss_sim.cards.{name}")
        _assert_has_return_annotation(fn, name=f"weiss_sim.cards.{name}")


def test_deck_builder_methods_have_docstrings_for_ide_descriptions() -> None:
    for name in (
        "add",
        "remove",
        "set_count",
        "count",
        "total_cards",
        "remaining_slots",
        "to_id_map",
        "to_card_no_map",
        "to_id_list",
        "describe",
        "validate",
        "build",
    ):
        fn = getattr(weiss_sim.DeckBuilder, name)
        _assert_has_doc(fn, name=f"DeckBuilder.{name}")
        _assert_has_return_annotation(fn, name=f"DeckBuilder.{name}")


def test_public_exports_cover_core_autocomplete_surface() -> None:
    exported = set(weiss_sim.__all__)
    for symbol in (
        "make",
        "fast",
        "inspect",
        "cards",
        "DeckBuilder",
        "RewardOverrides",
        "LegalActions",
        "ResetBatch",
        "StepBatch",
        "__version__",
    ):
        assert symbol in exported
