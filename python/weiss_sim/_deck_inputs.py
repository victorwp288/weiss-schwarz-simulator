from __future__ import annotations

import json
from pathlib import Path
from typing import Mapping, Sequence

from ._presets import get_preset
from .catalog import resolve_card_id, suggest_cards
from .config_types import DeckInput
from .errors import DeckSpecError


def _is_path_like(token: str) -> bool:
    return "/" in token or "\\" in token or token.endswith(".json")


def _working_root() -> Path:
    return Path.cwd().resolve()


def _is_within_root(path: Path, root: Path) -> bool:
    try:
        path.relative_to(root)
        return True
    except ValueError:
        return False


def _resolve_untrusted_string_path(
    raw_path: str,
    *,
    source: str,
    allow_absolute: bool,
) -> Path:
    token = raw_path.strip()
    if not token:
        raise DeckSpecError(f"{source} deck path is empty")
    try:
        candidate = Path(token).expanduser()
    except Exception as exc:
        raise DeckSpecError(f"{source} deck path is invalid: {raw_path!r}") from exc

    if candidate.is_absolute():
        if not allow_absolute:
            raise DeckSpecError(
                f"{source} absolute deck paths are not allowed; "
                "pass a Path(...) object for trusted absolute paths"
            )
        return candidate.resolve(strict=False)

    root = _working_root()
    resolved = (root / candidate).resolve(strict=False)
    if not _is_within_root(resolved, root):
        raise DeckSpecError(f"{source} deck path escapes working directory: {raw_path!r}")
    return resolved


def _load_json_path(path: Path):
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except FileNotFoundError as exc:
        raise DeckSpecError(f"deck file not found: {path}") from exc
    except OSError as exc:
        raise DeckSpecError(f"failed to read deck file {path}: {exc}") from exc
    except json.JSONDecodeError as exc:
        raise DeckSpecError(f"invalid deck JSON in {path}: {exc}") from exc


def _resolve_string_input(token: str):
    raw = token.strip()
    if not raw:
        raise DeckSpecError("empty deck string")
    lower = raw.lower()
    if lower.startswith("preset:"):
        name = raw.split(":", 1)[1].strip()
        return get_preset(name)
    if lower.startswith("file:"):
        path = _resolve_untrusted_string_path(
            raw.split(":", 1)[1],
            source="file:",
            allow_absolute=True,
        )
        return _load_json_path(path)
    if _is_path_like(raw):
        return _load_json_path(
            _resolve_untrusted_string_path(
                raw,
                source="path-like",
                allow_absolute=True,
            )
        )
    return get_preset(raw)


def _resolve_deck_input(deck_input: DeckInput):
    if isinstance(deck_input, Path):
        return _load_json_path(deck_input)
    if isinstance(deck_input, str):
        return _resolve_string_input(deck_input)
    return deck_input


def _expand_mapping(deck_map: Mapping[int | str, int]) -> list[int]:
    expanded: list[int] = []
    pairs: list[tuple[int, int]] = []
    for key, count in deck_map.items():
        try:
            n = int(count)
        except Exception as exc:
            raise DeckSpecError(f"invalid count for {key!r}: {count!r}") from exc
        if n < 0:
            raise DeckSpecError(f"count for {key!r} must be non-negative")
        if isinstance(key, int):
            card_id = key
        else:
            token = str(key)
            try:
                card_id = resolve_card_id(token)
            except Exception as exc:
                suggestions = [card.card_no for card in suggest_cards(token, limit=5)]
                if suggestions:
                    hint = ", ".join(suggestions)
                    raise DeckSpecError(
                        f"unknown card identifier {token!r}; suggestions: {hint}"
                    ) from exc
                raise DeckSpecError(f"unknown card identifier {token!r}") from exc
        pairs.append((int(card_id), n))
    pairs.sort(key=lambda p: p[0])
    for card_id, n in pairs:
        expanded.extend([card_id] * n)
    return expanded


def _normalize_sequence(ids: Sequence[int]) -> list[int]:
    out: list[int] = []
    for value in ids:
        try:
            out.append(int(value))
        except Exception as exc:
            raise DeckSpecError(f"deck list contains non-integer value {value!r}") from exc
    return out


def coerce_to_id_list(deck_input: DeckInput) -> list[int]:
    resolved = _resolve_deck_input(deck_input)
    if isinstance(resolved, Mapping):
        return _expand_mapping(resolved)
    if isinstance(resolved, Sequence) and not isinstance(resolved, (str, bytes, bytearray)):
        return _normalize_sequence(resolved)
    raise DeckSpecError(
        "deck must resolve to a list of ids or a count map "
        "(accepted forms: list, map, preset string, file path)"
    )
