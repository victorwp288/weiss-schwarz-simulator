from __future__ import annotations

import importlib.machinery
import sys
from pathlib import Path


_PYTHON_ROOT = Path(__file__).resolve().parents[1]
_PYTHON_ROOT_STR = str(_PYTHON_ROOT)


def _has_in_tree_extension() -> bool:
    package_root = _PYTHON_ROOT / "weiss_sim"
    return any(
        (package_root / f"weiss_sim{suffix}").exists()
        for suffix in importlib.machinery.EXTENSION_SUFFIXES
    )


if _PYTHON_ROOT_STR in sys.path:
    sys.path.remove(_PYTHON_ROOT_STR)

if _has_in_tree_extension():
    # Prefer source tree when the compiled extension is available in-place.
    sys.path.insert(0, _PYTHON_ROOT_STR)
else:
    # Keep local tests importable without shadowing wheel-installed packages.
    sys.path.append(_PYTHON_ROOT_STR)
