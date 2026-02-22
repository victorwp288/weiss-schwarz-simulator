from __future__ import annotations

import sys
from pathlib import Path


_PYTHON_ROOT = str(Path(__file__).resolve().parents[1])

if _PYTHON_ROOT in sys.path:
    sys.path.remove(_PYTHON_ROOT)
sys.path.insert(0, _PYTHON_ROOT)
