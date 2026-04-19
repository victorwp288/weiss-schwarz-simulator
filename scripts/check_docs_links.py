#!/usr/bin/env python3
"""Validate local markdown links (files + anchors).

Rules:
- External links (http/https/mailto/tel) are ignored.
- Reference-style links are supported.
- Anchors are validated using GitHub-style slugging.
"""

from __future__ import annotations

import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
TARGETS = [ROOT / "README.md"] + sorted((ROOT / "docs").rglob("*.md"))

INLINE_LINK_RE = re.compile(r"!?\[([^\]]+)\]\(([^)]+)\)")
REF_LINK_RE = re.compile(r"\[([^\]]+)\]\[([^\]]*)\]")
REF_DEF_RE = re.compile(r"^\s*\[([^\]]+)\]:\s*(\S+)(?:\s+.*)?$")

EXTERNAL_PREFIXES = ("http://", "https://", "mailto:", "tel:")


def _read_text(path: Path) -> str:
    try:
        return path.read_text(encoding="utf-8")
    except UnicodeDecodeError as exc:
        raise SystemExit(f"{path}: failed to decode as UTF-8: {exc}") from exc


def _strip_title(target: str) -> str:
    target = target.strip()
    if target.startswith("<") and target.endswith(">"):
        return target[1:-1].strip()
    # split on whitespace to drop optional title
    parts = target.split()
    return parts[0].strip() if parts else target


def _slugify(text: str) -> str:
    text = text.strip().lower()
    text = re.sub(r"[^\w\s-]", "", text).replace("_", "")
    text = text.replace(" ", "-")
    text = re.sub(r"-+", "-", text).strip("-")
    return text


def _anchors_for(path: Path) -> set[str]:
    anchors: list[str] = []
    counts: dict[str, int] = {}
    in_code = False
    for line in _read_text(path).splitlines():
        stripped = line.strip()
        if stripped.startswith("```") or stripped.startswith("~~~"):
            in_code = not in_code
            continue
        if in_code:
            continue
        leading_spaces = len(line) - len(line.lstrip(" "))
        if leading_spaces > 3:
            continue
        heading_line = line[leading_spaces:]
        heading_match = re.match(r"^(#{1,6})\s+(.*?)\s*#*\s*$", heading_line)
        if not heading_match:
            continue
        heading = heading_match.group(2).strip()
        if not heading:
            continue
        slug = _slugify(heading)
        if slug in counts:
            counts[slug] += 1
            slug = f"{slug}-{counts[slug]}"
        else:
            counts[slug] = 0
        anchors.append(slug)
    return set(anchors)


def _collect_ref_defs(lines: list[str]) -> dict[str, str]:
    ref_defs: dict[str, str] = {}
    in_code = False
    for line in lines:
        stripped = line.strip()
        if stripped.startswith("```") or stripped.startswith("~~~"):
            in_code = not in_code
            continue
        if in_code:
            continue
        m = REF_DEF_RE.match(line)
        if not m:
            continue
        key = re.sub(r"\s+", " ", m.group(1).strip().lower())
        ref_defs[key] = _strip_title(m.group(2))
    return ref_defs


def _iter_links(lines: list[str]):
    in_code = False
    for line in lines:
        stripped = line.strip()
        if stripped.startswith("```") or stripped.startswith("~~~"):
            in_code = not in_code
            continue
        if in_code:
            continue
        for m in INLINE_LINK_RE.finditer(line):
            yield ("inline", m.group(1), _strip_title(m.group(2)))
        for m in REF_LINK_RE.finditer(line):
            yield ("ref", m.group(1), m.group(2))


def _normalize_ref_id(text: str) -> str:
    return re.sub(r"\s+", " ", text.strip().lower())


def main() -> int:
    errors: list[str] = []
    anchor_cache: dict[Path, set[str]] = {}

    for md_path in TARGETS:
        lines = _read_text(md_path).splitlines()
        ref_defs = _collect_ref_defs(lines)
        for kind, text, target in _iter_links(lines):
            resolved = target
            if kind == "ref":
                ref_id = target.strip() if target.strip() else text
                ref_id = _normalize_ref_id(ref_id)
                resolved = ref_defs.get(ref_id)
                if not resolved:
                    errors.append(f"{md_path}: missing reference definition for [{text}][{target}]")
                    continue
            if resolved.startswith(EXTERNAL_PREFIXES):
                continue
            if resolved.startswith("#"):
                anchor = resolved[1:]
                anchors = anchor_cache.setdefault(md_path, _anchors_for(md_path))
                if anchor and anchor not in anchors:
                    errors.append(f"{md_path}: missing anchor '#{anchor}'")
                continue
            if "#" in resolved:
                file_part, anchor = resolved.split("#", 1)
            else:
                file_part, anchor = resolved, ""
            target_path = (md_path.parent / file_part).resolve()
            try:
                target_path.relative_to(ROOT)
            except ValueError:
                errors.append(f"{md_path}: link points outside repo: {resolved}")
                continue
            if not target_path.exists():
                errors.append(f"{md_path}: missing target file {resolved}")
                continue
            if anchor:
                anchors = anchor_cache.setdefault(target_path, _anchors_for(target_path))
                if anchor not in anchors:
                    errors.append(f"{md_path}: missing anchor '#{anchor}' in {resolved}")

    if errors:
        for err in errors:
            print(err)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
