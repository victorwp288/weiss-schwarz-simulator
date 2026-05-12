#!/usr/bin/env python3
"""Verify that compatibility constants in docs match Rust constants."""

from __future__ import annotations

import ast
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
CONSTANTS_PATH = ROOT / "weiss_core" / "src" / "encode" / "constants.rs"
STATE_REVEAL_PATH = ROOT / "weiss_core" / "src" / "state" / "reveal.rs"
REPLAY_PATH = ROOT / "weiss_core" / "src" / "replay.rs"
WSDB_PATH = ROOT / "weiss_core" / "src" / "db" / "serialization.rs"
DOC_PATH = ROOT / "docs" / "rl_contract.md"
DOCS_TO_SCAN = [
    ROOT / "README.md",
    ROOT / "docs" / "README.md",
    ROOT / "docs" / "architecture.md",
    ROOT / "docs" / "rl_contract.md",
]

TARGET_FIELDS = {
    "OBS_LEN",
    "ACTION_SPACE_SIZE",
    "OBS_ENCODING_VERSION",
    "ACTION_ENCODING_VERSION",
    "SPEC_HASH",
}
VERSION_FIELDS = {
    "REPLAY_SCHEMA_VERSION": REPLAY_PATH,
    "WSDB_SCHEMA_VERSION": WSDB_PATH,
}

CONST_RE = re.compile(r"^\s*pub const (\w+): [^=]+ = (.+)$")


def safe_eval(expr: str, env: dict[str, int]) -> int:
    expr = re.sub(r"\s+as\s+\w+", "", expr)
    expr = re.sub(r"(\w+)\.div_ceil\((\d+)\)", r"(\1 + \2 - 1)//\2", expr)
    tree = ast.parse(expr, mode="eval")

    def _eval(node):
        if isinstance(node, ast.Expression):
            return _eval(node.body)
        if isinstance(node, ast.Constant) and isinstance(node.value, int):
            return node.value
        if isinstance(node, ast.Name):
            if node.id not in env:
                raise ValueError(f"unknown name {node.id}")
            return env[node.id]
        if isinstance(node, ast.BinOp):
            left = _eval(node.left)
            right = _eval(node.right)
            if isinstance(node.op, ast.Add):
                return left + right
            if isinstance(node.op, ast.Sub):
                return left - right
            if isinstance(node.op, ast.Mult):
                return left * right
            if isinstance(node.op, ast.FloorDiv):
                return left // right
            if isinstance(node.op, ast.Mod):
                return left % right
            if isinstance(node.op, ast.LShift):
                return left << right
            if isinstance(node.op, ast.RShift):
                return left >> right
            if isinstance(node.op, ast.BitOr):
                return left | right
            if isinstance(node.op, ast.BitAnd):
                return left & right
        if isinstance(node, ast.UnaryOp):
            val = _eval(node.operand)
            if isinstance(node.op, ast.UAdd):
                return +val
            if isinstance(node.op, ast.USub):
                return -val
        raise ValueError(f"unsupported expression: {expr}")

    return _eval(tree)


def parse_state_constants() -> dict[str, int]:
    env: dict[str, int] = {}
    text = STATE_REVEAL_PATH.read_text()
    m = re.search(r"pub const REVEAL_HISTORY_LEN: usize = (\d+);", text)
    if not m:
        raise ValueError("REVEAL_HISTORY_LEN not found")
    env["REVEAL_HISTORY_LEN"] = int(m.group(1))
    return env


def parse_encode_constants() -> dict[str, int]:
    env = parse_state_constants()
    lines = CONSTANTS_PATH.read_text().splitlines()
    idx = 0
    while idx < len(lines):
        raw = lines[idx]
        line = raw.split("//", 1)[0].rstrip()
        if not line.strip():
            idx += 1
            continue
        m = CONST_RE.match(line)
        if not m:
            idx += 1
            continue
        name, expr = m.group(1), m.group(2).strip()
        idx += 1
        while not expr.rstrip().endswith(";") and idx < len(lines):
            next_line = lines[idx].split("//", 1)[0].strip()
            if next_line:
                expr = f"{expr} {next_line}"
            idx += 1
        if expr.endswith(";"):
            expr = expr[:-1].strip()
        try:
            env[name] = safe_eval(expr, env)
        except Exception:
            # Skip constants we cannot evaluate; only target fields are required.
            continue
    return env


def parse_contract_table() -> dict[str, int]:
    text = DOC_PATH.read_text().splitlines()
    table: dict[str, int] = {}
    in_table = False
    for line in text:
        if line.strip().startswith("| Field | Value |"):
            in_table = True
            continue
        if in_table:
            if not line.strip().startswith("|"):
                break
            cols = [c.strip() for c in line.strip().strip("|").split("|")]
            if len(cols) < 2:
                continue
            field, value = cols[0], cols[1]
            if field in TARGET_FIELDS:
                try:
                    table[field] = int(value)
                except ValueError:
                    raise ValueError(f"Invalid integer for {field}: {value}")
    return table


def parse_named_constant(path: Path, name: str) -> int:
    pattern = re.compile(rf"^\s*pub const {re.escape(name)}: [^=]+ = (\d+);")
    for line in path.read_text().splitlines():
        match = pattern.match(line)
        if match:
            return int(match.group(1))
    raise ValueError(f"{name} not found in {path.relative_to(ROOT)}")


def find_documented_versions(path: Path, name: str) -> list[tuple[int, int]]:
    pattern = re.compile(rf"\b{re.escape(name)}\s*[=`|]\s*`?(\d+)")
    found: list[tuple[int, int]] = []
    for line_no, line in enumerate(path.read_text(encoding="utf-8").splitlines(), start=1):
        for match in pattern.finditer(line):
            found.append((line_no, int(match.group(1))))
    return found


def main() -> int:
    env = parse_encode_constants()
    table = parse_contract_table()
    missing = TARGET_FIELDS - set(table.keys())
    if missing:
        print(f"Missing fields in contract table: {sorted(missing)}")
        return 1
    errors = []
    for field in sorted(TARGET_FIELDS):
        expected = env.get(field)
        actual = table.get(field)
        if expected is None:
            errors.append(f"Could not compute {field} from constants")
            continue
        if expected != actual:
            errors.append(f"{field} mismatch: docs={actual} constants={expected}")
    for field, path in VERSION_FIELDS.items():
        expected = parse_named_constant(path, field)
        occurrences = []
        for doc_path in DOCS_TO_SCAN:
            occurrences.extend(
                (doc_path, line_no, value)
                for line_no, value in find_documented_versions(doc_path, field)
            )
        if not occurrences:
            errors.append(f"{field} is not documented in checked docs")
            continue
        for doc_path, line_no, actual in occurrences:
            if actual != expected:
                rel = doc_path.relative_to(ROOT)
                errors.append(
                    f"{field} mismatch in {rel}:{line_no}: docs={actual} constants={expected}"
                )
    if errors:
        for err in errors:
            print(err)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
