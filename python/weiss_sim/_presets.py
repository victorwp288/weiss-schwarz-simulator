from __future__ import annotations

import json
from functools import lru_cache
from importlib import resources
from pathlib import Path

from .errors import CardLookupError

_PRESETS_FILE = "deck_presets.json"
_PRESET_META_FILE = "deck_preset_meta.json"
_VALID_RULES_PROFILES = frozenset({"strict", "approx"})


def _data_root():
    return resources.files(__package__).joinpath("data")


def _read_data_bytes(name: str) -> bytes:
    with resources.as_file(_data_root().joinpath(name)) as p:
        return Path(p).read_bytes()


@lru_cache(maxsize=1)
def _load_presets_cached() -> dict[str, list[int]]:
    try:
        raw = _read_data_bytes(_PRESETS_FILE)
        payload = json.loads(raw.decode("utf-8"))
    except FileNotFoundError:
        payload = {"starter_deck_ws02_v1": (list(range(1, 14)) * 4)[:50]}
    if not isinstance(payload, dict):
        raise ValueError(f"{_PRESETS_FILE} must decode to an object")
    out: dict[str, list[int]] = {}
    for key, value in payload.items():
        if not isinstance(key, str):
            continue
        if not isinstance(value, list):
            continue
        out[key] = [int(v) for v in value]
    return out


def load_presets() -> dict[str, list[int]]:
    return {k: list(v) for k, v in _load_presets_cached().items()}


@lru_cache(maxsize=1)
def _load_preset_metadata_cached() -> dict[str, dict[str, object]]:
    presets = _load_presets_cached()
    out: dict[str, dict[str, object]] = {name: {"min_rules_profile": "strict"} for name in presets}
    try:
        raw = _read_data_bytes(_PRESET_META_FILE)
        payload = json.loads(raw.decode("utf-8"))
    except FileNotFoundError:
        return out
    if not isinstance(payload, dict):
        raise ValueError(f"{_PRESET_META_FILE} must decode to an object")
    for name, meta in payload.items():
        if not isinstance(name, str) or name not in presets:
            continue
        if not isinstance(meta, dict):
            continue
        copied = dict(meta)
        profile = str(copied.get("min_rules_profile", "strict"))
        if profile not in _VALID_RULES_PROFILES:
            raise ValueError(
                f"{_PRESET_META_FILE} entry {name!r} has invalid min_rules_profile {profile!r}"
            )
        copied["min_rules_profile"] = profile
        out[name] = copied
    return out


def load_preset_metadata() -> dict[str, dict[str, object]]:
    return {k: dict(v) for k, v in _load_preset_metadata_cached().items()}


def get_preset_metadata(name: str) -> dict[str, object]:
    key = name.strip()
    try:
        return dict(load_preset_metadata()[key])
    except KeyError as exc:
        raise CardLookupError(f"unknown preset '{name}'") from exc


def preset_min_rules_profile(name: str) -> str:
    return str(get_preset_metadata(name).get("min_rules_profile", "strict"))


def preset_names() -> list[str]:
    return sorted(load_presets().keys())


def get_preset(name: str) -> list[int]:
    key = name.strip()
    try:
        return list(load_presets()[key])
    except KeyError as exc:
        raise CardLookupError(f"unknown preset '{name}'") from exc
