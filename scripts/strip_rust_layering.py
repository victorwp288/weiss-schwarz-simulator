#!/usr/bin/env python3
from __future__ import annotations

import sys


def _raw_string_end(text: str, start: int) -> int | None:
    """Return exclusive end index of a Rust raw string starting at `start`."""
    n = len(text)
    prefix_len = 0
    if text.startswith("br", start):
        prefix_len = 2
    elif text.startswith("r", start):
        prefix_len = 1
    else:
        return None
    i = start + prefix_len
    hash_count = 0
    while i < n and text[i] == "#":
        hash_count += 1
        i += 1
    if i >= n or text[i] != '"':
        return None
    i += 1
    closing = '"' + ("#" * hash_count)
    pos = text.find(closing, i)
    if pos == -1:
        return n
    return pos + len(closing)


def _string_end(text: str, start: int) -> int | None:
    """Return exclusive end index of a Rust quoted string starting at `start`."""
    n = len(text)
    quote = start
    if text.startswith('b"', start):
        quote = start + 1
    elif not text.startswith('"', start):
        return None
    i = quote + 1
    escaped = False
    while i < n:
        ch = text[i]
        if escaped:
            escaped = False
            i += 1
            continue
        if ch == "\\":
            escaped = True
            i += 1
            continue
        if ch == '"':
            return i + 1
        i += 1
    return n


def _char_end(text: str, start: int) -> int | None:
    """Return exclusive end index of a Rust char literal starting at `start`."""
    n = len(text)
    quote = start
    if text.startswith("b'", start):
        quote = start + 1
    elif not text.startswith("'", start):
        return None
    i = quote + 1
    if i >= n:
        return None
    escaped = False
    # Char literals are short; keep a tight window to avoid matching lifetimes.
    limit = min(n, quote + 12)
    while i < limit:
        ch = text[i]
        if ch == "\n":
            return None
        if escaped:
            escaped = False
            i += 1
            continue
        if ch == "\\":
            escaped = True
            i += 1
            continue
        if ch == "'":
            return i + 1
        i += 1
    return None


def strip_rust(text: str) -> str:
    out: list[str] = []
    i = 0
    n = len(text)
    while i < n:
        char_end = _char_end(text, i)
        if char_end is not None:
            out.append("''")
            i = char_end
            continue
        if text.startswith("//", i):
            i += 2
            while i < n and text[i] != "\n":
                i += 1
            continue
        if text.startswith("/*", i):
            i += 2
            depth = 1
            while i < n and depth > 0:
                if text.startswith("/*", i):
                    depth += 1
                    i += 2
                elif text.startswith("*/", i):
                    depth -= 1
                    i += 2
                else:
                    if text[i] == "\n":
                        out.append("\n")
                    i += 1
            continue
        raw_end = _raw_string_end(text, i)
        if raw_end is not None:
            newlines = text[i:raw_end].count("\n")
            out.append('""' + ("\n" * newlines))
            i = raw_end
            continue
        string_end = _string_end(text, i)
        if string_end is not None:
            newlines = text[i:string_end].count("\n")
            out.append('""' + ("\n" * newlines))
            i = string_end
            continue
        out.append(text[i])
        i += 1
    return "".join(out)


def main() -> int:
    if len(sys.argv) < 2:
        print("usage: strip_rust_layering.py <path>", file=sys.stderr)
        return 1
    path = sys.argv[1]
    try:
        with open(path, "r", encoding="utf-8") as f:
            text = f.read()
    except OSError as exc:
        print(f"error reading {path}: {exc}", file=sys.stderr)
        return 1
    sys.stdout.write(strip_rust(text))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
