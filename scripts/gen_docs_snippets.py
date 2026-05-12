#!/usr/bin/env python3
"""Generate and validate in-repo Markdown snippets derived from code.

The compact docs keep only one generated region today: the public
``weiss_sim.make`` signature in ``docs/python_api.md``. This script stays small
on purpose so stale documentation surfaces are obvious.
"""

from __future__ import annotations

import argparse
import ast
import difflib
import sys
from dataclasses import dataclass
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
DOC_API_GUIDE = ROOT / "docs" / "python_api.md"
API_MODULE = ROOT / "python" / "weiss_sim" / "api.py"


@dataclass(frozen=True)
class ModuleInfo:
    source: str
    defs: dict[str, ast.AST]


def read_text(path: Path) -> str:
    try:
        return path.read_text(encoding="utf-8")
    except OSError as exc:
        raise SystemExit(f"Failed to read {path}: {exc}") from exc


def write_text(path: Path, content: str) -> None:
    try:
        path.write_text(content, encoding="utf-8")
    except OSError as exc:
        raise SystemExit(f"Failed to write {path}: {exc}") from exc


def start_marker(marker_id: str) -> str:
    return f"<!-- GENERATED:{marker_id}:START -->"


def end_marker(marker_id: str) -> str:
    return f"<!-- GENERATED:{marker_id}:END -->"


def replace_region(content: str, *, marker_id: str, body: str) -> str:
    start = start_marker(marker_id)
    end = end_marker(marker_id)
    if start not in content or end not in content:
        raise SystemExit(f"Missing markers for {marker_id}")
    before, rest = content.split(start, 1)
    _, after = rest.split(end, 1)
    body_norm = body.rstrip("\n")
    replacement = f"{start}\n{body_norm}\n{end}" if body_norm else f"{start}\n{end}"
    return before + replacement + after


def extract_region(content: str, *, marker_id: str) -> str:
    start = start_marker(marker_id)
    end = end_marker(marker_id)
    if start not in content or end not in content:
        raise SystemExit(f"Missing markers for {marker_id}")
    _, rest = content.split(start, 1)
    body, _ = rest.split(end, 1)
    return body.strip("\n")


def parse_module(path: Path) -> ModuleInfo:
    source = read_text(path)
    tree = ast.parse(source, filename=str(path))
    defs: dict[str, ast.AST] = {}
    for node in tree.body:
        if isinstance(node, (ast.FunctionDef, ast.AsyncFunctionDef, ast.ClassDef)):
            defs[node.name] = node
    return ModuleInfo(source=source, defs=defs)


def source_for(expr: ast.AST | None, *, source: str) -> str | None:
    if expr is None:
        return None
    segment = ast.get_source_segment(source, expr)
    if segment is not None and segment.strip():
        return segment.strip()
    try:
        return ast.unparse(expr).strip()
    except Exception:
        return None


def render_make_call_signature(api_mod: ModuleInfo) -> str:
    node = api_mod.defs.get("make")
    if not isinstance(node, ast.FunctionDef):
        raise SystemExit("python/weiss_sim/api.py must define make()")

    args = node.args
    if args.args or args.posonlyargs:
        raise SystemExit("make() is expected to be keyword-only (def make(*, ...))")
    if len(args.kwonlyargs) != len(args.kw_defaults):
        raise SystemExit("make() kwonlyargs/kw_defaults length mismatch")

    lines = ["```python", "weiss_sim.make("]
    for arg, default_node in zip(args.kwonlyargs, args.kw_defaults, strict=True):
        default = source_for(default_node, source=api_mod.source)
        if default is None:
            raise SystemExit(f"make() missing default for {arg.arg}")
        lines.append(f"    {arg.arg}={default},")
    lines.append(")")
    lines.append("```")
    return "\n".join(lines)


def unified_diff(a: str, b: str, *, fromfile: str, tofile: str) -> str:
    return "".join(
        difflib.unified_diff(
            a.splitlines(keepends=True),
            b.splitlines(keepends=True),
            fromfile=fromfile,
            tofile=tofile,
        )
    )


def main() -> int:
    parser = argparse.ArgumentParser(description="Generate/validate Markdown snippets.")
    mode = parser.add_mutually_exclusive_group(required=True)
    mode.add_argument("--write", action="store_true", help="Rewrite generated regions in place.")
    mode.add_argument("--check", action="store_true", help="Fail if generated regions are stale.")
    args = parser.parse_args()

    expected = {
        (DOC_API_GUIDE, "MAKE_SIGNATURE"): render_make_call_signature(parse_module(API_MODULE)),
    }

    failures: list[str] = []
    for (path, marker_id), body in expected.items():
        content = read_text(path)
        updated = replace_region(content, marker_id=marker_id, body=body)
        if args.write:
            if updated != content:
                write_text(path, updated)
            continue

        current_region = extract_region(content, marker_id=marker_id)
        expected_region = extract_region(updated, marker_id=marker_id)
        if current_region != expected_region:
            diff = unified_diff(
                current_region + "\n",
                expected_region + "\n",
                fromfile=f"{path}:{marker_id} (current)",
                tofile=f"{path}:{marker_id} (expected)",
            )
            failures.append(diff or f"{path}: {marker_id} differs")

    if failures:
        for item in failures:
            sys.stdout.write(item)
            if not item.endswith("\n"):
                sys.stdout.write("\n")
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
