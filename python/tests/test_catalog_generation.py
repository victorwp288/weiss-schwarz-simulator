from __future__ import annotations

import json
import subprocess
import sys
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
DATA_DIR = ROOT / "python" / "weiss_sim" / "data"


def test_build_python_catalog_preserves_bundled_deck_presets(tmp_path: Path) -> None:
    result = subprocess.run(
        [
            sys.executable,
            "scripts/build_python_catalog.py",
            "--out-dir",
            str(tmp_path),
        ],
        cwd=ROOT,
        check=True,
        capture_output=True,
        text=True,
    )
    assert "deck_presets" in result.stdout

    generated = json.loads((tmp_path / "deck_presets.json").read_text(encoding="utf-8"))
    bundled = json.loads((DATA_DIR / "deck_presets.json").read_text(encoding="utf-8"))
    assert generated == bundled

    generated_meta = json.loads((tmp_path / "deck_preset_meta.json").read_text(encoding="utf-8"))
    bundled_meta = json.loads((DATA_DIR / "deck_preset_meta.json").read_text(encoding="utf-8"))
    assert generated_meta == bundled_meta

    meta = json.loads((tmp_path / "catalog_meta.json").read_text(encoding="utf-8"))
    assert meta["deck_preset_count"] == len(bundled)
    assert meta["deck_preset_names"] == sorted(bundled)
    assert len(meta["deck_presets_sha256"]) == 64
    assert len(meta["deck_preset_meta_sha256"]) == 64
