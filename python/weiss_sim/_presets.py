from __future__ import annotations

import json
from functools import lru_cache
from importlib import resources
from pathlib import Path

from .errors import CardLookupError

_PRESETS_FILE = "deck_presets.json"


def _data_root():
    return resources.files(__package__).joinpath("data")


def _read_data_bytes(name: str) -> bytes:
    with resources.as_file(_data_root().joinpath(name)) as p:
        return Path(p).read_bytes()


@lru_cache(maxsize=1)
def load_presets() -> dict[str, list[int]]:
    try:
        raw = _read_data_bytes(_PRESETS_FILE)
        payload = json.loads(raw.decode("utf-8"))
    except FileNotFoundError:
        payload = {"starter_v1": (list(range(1, 14)) * 4)[:50]}
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


def preset_names() -> list[str]:
    return sorted(load_presets().keys())


def get_preset(name: str) -> list[int]:
    key = name.strip()
    try:
        return list(load_presets()[key])
    except KeyError as exc:
        raise CardLookupError(f"unknown preset '{name}'") from exc
