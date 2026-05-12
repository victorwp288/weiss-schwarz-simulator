#!/usr/bin/env python3
from __future__ import annotations

import argparse
import datetime as dt
import re
from pathlib import Path


BENCH_LINE_RE = re.compile(
    r"^test\s+(?P<name>.+?)\s+\.\.\.\s+bench:\s+(?P<value>[0-9,]+)\s+(?P<unit>\S+)"
)
START_MARKER = "<!-- BENCHMARKS:START -->"
END_MARKER = "<!-- BENCHMARKS:END -->"


def parse_benches(lines: list[str], prefix: str) -> list[tuple[str, str]]:
    benches: list[tuple[str, str]] = []
    for line in lines:
        match = BENCH_LINE_RE.match(line.strip())
        if not match:
            continue
        name = match.group("name").strip()
        value = match.group("value").replace(",", "")
        unit = match.group("unit").strip()
        benches.append((f"{prefix}{name}", f"{value} {unit}"))
    return benches


def parse_python_bench(lines: list[str], prefix: str) -> list[tuple[str, str]]:
    benches: list[tuple[str, str]] = []
    for line in lines:
        line = line.strip()
        if not line or ":" not in line or "(" not in line or not line.endswith(")"):
            continue
        name = line.split(":", 1)[0].strip()
        value = line.rsplit("(", 1)[1].rstrip(")").strip()
        benches.append((f"{prefix}{name}", value))
    return benches


def render_table(benches: list[tuple[str, str]], max_rows: int) -> str:
    rows = benches[:max_rows]
    if not rows:
        rows = [("pending", "-")]
    lines = [
        f"_Last updated: {dt.datetime.now(dt.timezone.utc):%Y-%m-%d %H:%M UTC}_",
        "",
        "| Benchmark | Time |",
        "| --- | --- |",
    ]
    lines.extend(f"| {name} | {value} |" for name, value in rows)
    return "\n".join(lines)


def read_text_file(path: Path, label: str) -> str:
    try:
        return path.read_text(encoding="utf-8")
    except (FileNotFoundError, PermissionError, UnicodeError, OSError) as err:
        raise SystemExit(f"Failed to read {label} at {path}: {err}") from err


def write_text_file(path: Path, label: str, content: str) -> None:
    try:
        path.write_text(content, encoding="utf-8")
    except (PermissionError, UnicodeError, OSError) as err:
        raise SystemExit(f"Failed to write {label} at {path}: {err}") from err


def update_readme(readme_path: Path, table_md: str) -> None:
    content = read_text_file(readme_path, "README")
    if START_MARKER not in content or END_MARKER not in content:
        raise SystemExit("README markers not found. Add BENCHMARKS:START/END markers first.")
    pattern = re.compile(
        re.escape(START_MARKER) + r".*?" + re.escape(END_MARKER),
        flags=re.DOTALL,
    )
    replacement = f"{START_MARKER}\n{table_md}\n{END_MARKER}"
    updated = pattern.sub(replacement, content, count=1)
    write_text_file(readme_path, "README", updated)


def main() -> int:
    parser = argparse.ArgumentParser(description="Update README benchmark snapshot table.")
    parser.add_argument("--input", required=True, help="Path to bencher output file.")
    parser.add_argument("--python-input", help="Path to python benchmark output file.")
    parser.add_argument("--readme", required=True, help="README to update.")
    parser.add_argument("--max", type=int, default=12, help="Max rows to include.")
    args = parser.parse_args()

    bench_text = read_text_file(Path(args.input), "benchmark output")
    bench_lines = bench_text.splitlines()
    benches = parse_benches(bench_lines, prefix="rust/")[: args.max]
    if args.python_input:
        python_text = read_text_file(Path(args.python_input), "python benchmark output")
        python_lines = python_text.splitlines()
        benches.extend(parse_python_bench(python_lines, prefix="python/"))
    table_md = render_table(benches, max_rows=len(benches))
    update_readme(Path(args.readme), table_md)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
