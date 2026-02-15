#!/usr/bin/env python3
from __future__ import annotations

import argparse
from pathlib import Path
import sys

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

from scraper.parser_v2.registry import (  # noqa: E402
    build_compiled_payload,
    write_compiled_rulepack,
)


def main() -> None:
    parser = argparse.ArgumentParser(
        description="Compile parser-v2 rulepack YAML into deterministic runtime JSON."
    )
    parser.add_argument(
        "--rules-dir",
        default="scraper/rules_v2",
        help="Directory containing parser-v2 YAML rulepacks.",
    )
    parser.add_argument(
        "--output",
        default="scraper/rules_v2_compiled.json",
        help="Output path for compiled runtime JSON.",
    )
    parser.add_argument(
        "--tmp-output",
        action="store_true",
        help="Write compiled runtime JSON to /tmp/rules_v2_compiled.json.",
    )
    args = parser.parse_args()

    rules_dir = Path(args.rules_dir)
    output_path = Path("/tmp/rules_v2_compiled.json") if args.tmp_output else Path(args.output)

    payload = build_compiled_payload(rules_dir=rules_dir)
    out_path = write_compiled_rulepack(output_path=output_path, rules_dir=rules_dir)
    print(f"compiled {len(payload.get('rules', []))} rules from {rules_dir} -> {out_path}")


if __name__ == "__main__":
    main()
