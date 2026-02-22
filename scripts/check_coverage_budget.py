#!/usr/bin/env python3
import argparse
import sys
from pathlib import Path
from typing import Any, List, Optional, Tuple

from coverage_common import family_coverage_from_metrics, load_json, resolve_profile_metrics


def metric(report: dict[str, Any], profile: str, key: str) -> float:
    metrics = resolve_profile_metrics(report, profile)
    if key not in metrics:
        raise ValueError(f"profile '{profile}' missing metric '{key}'")
    return float(metrics[key])


def parse_family_floor(value: str) -> Tuple[str, float]:
    if "=" not in value:
        raise argparse.ArgumentTypeError(f"invalid family floor '{value}'; expected FAMILY=FLOOR")
    family, floor_raw = value.split("=", 1)
    family = family.strip()
    if not family:
        raise argparse.ArgumentTypeError(f"invalid family floor '{value}'; FAMILY cannot be empty")
    try:
        floor = float(floor_raw.strip())
    except ValueError as exc:
        raise argparse.ArgumentTypeError(
            f"invalid family floor '{value}'; FLOOR must be a number"
        ) from exc
    if floor < 0.0 or floor > 1.0:
        raise argparse.ArgumentTypeError(
            f"invalid family floor '{value}'; FLOOR must be between 0.0 and 1.0"
        )
    return family, floor


def family_coverage_map(report: dict[str, Any], profile: str) -> Optional[dict[str, Any]]:
    try:
        profile_metrics = resolve_profile_metrics(report, profile)
    except ValueError:
        return None
    by_family = family_coverage_from_metrics(profile_metrics)
    return by_family or None


def enforce_family_floors(
    report: dict[str, Any],
    profile: str,
    floors: List[Tuple[str, float]],
    failures: List[str],
) -> None:
    if not floors:
        return
    by_family = family_coverage_map(report, profile)
    if by_family is None:
        failures.append(
            f"{profile} family coverage unavailable; report missing rule_family_coverage/family_cluster_coverage"
        )
        return
    for family, floor in floors:
        entry = by_family.get(family)
        if not isinstance(entry, dict):
            failures.append(
                f"{profile} family '{family}' missing from report; cannot enforce floor={floor:.6f}"
            )
            continue
        coverage = float(entry.get("coverage", 0.0))
        if coverage < floor:
            failures.append(
                f"{profile} family '{family}' coverage below floor: current={coverage:.6f}, floor={floor:.6f}"
            )


def main() -> None:
    parser = argparse.ArgumentParser(description="Fail CI when ability coverage regresses.")
    parser.add_argument("--report", required=True, help="Current coverage report JSON")
    parser.add_argument(
        "--baseline",
        default="scripts/ability_coverage_baseline.json",
        help="Baseline coverage report JSON",
    )
    parser.add_argument(
        "--min-parse-line-coverage-strict",
        type=float,
        default=None,
        help="Optional hard floor for strict-profile parse line coverage",
    )
    parser.add_argument(
        "--min-card-coverage-strict",
        type=float,
        default=None,
        help="Optional hard floor for strict-profile card-level all-lines-supported coverage",
    )
    parser.add_argument(
        "--min-card-coverage-approx",
        type=float,
        default=None,
        help="Optional hard floor for approx-profile card-level all-lines-supported coverage",
    )
    parser.add_argument(
        "--max-unsupported-lines-strict",
        type=float,
        default=None,
        help="Optional hard ceiling for strict-profile unsupported line count",
    )
    parser.add_argument(
        "--min-family-coverage-strict",
        action="append",
        type=parse_family_floor,
        default=[],
        metavar="FAMILY=FLOOR",
        help="Optional repeatable floors for strict-profile family coverage",
    )
    parser.add_argument(
        "--min-family-coverage-approx",
        action="append",
        type=parse_family_floor,
        default=[],
        metavar="FAMILY=FLOOR",
        help="Optional repeatable floors for approx-profile family coverage",
    )
    parser.add_argument(
        "--tolerance",
        type=float,
        default=0.0,
        help="Allowed regression tolerance for baseline comparisons",
    )
    args = parser.parse_args()

    report = load_json(Path(args.report))
    baseline = load_json(Path(args.baseline))
    tol = float(args.tolerance)
    failures = []

    min_parse_line_coverage_strict = args.min_parse_line_coverage_strict
    min_card_coverage_strict = args.min_card_coverage_strict
    min_card_coverage_approx = args.min_card_coverage_approx
    max_unsupported_lines_strict = args.max_unsupported_lines_strict
    min_family_coverage_strict = args.min_family_coverage_strict
    min_family_coverage_approx = args.min_family_coverage_approx

    current_strict_parse = metric(report, "strict", "parse_line_coverage")
    baseline_strict_parse = metric(baseline, "strict", "parse_line_coverage")
    if current_strict_parse + tol < baseline_strict_parse:
        failures.append(
            f"strict parse_line_coverage regressed: current={current_strict_parse:.6f}, baseline={baseline_strict_parse:.6f}"
        )

    current_strict_card = metric(report, "strict", "card_level_all_lines_supported_coverage")
    current_approx_card = metric(report, "approx", "card_level_all_lines_supported_coverage")
    baseline_approx_card = metric(baseline, "approx", "card_level_all_lines_supported_coverage")
    if current_approx_card + tol < baseline_approx_card:
        failures.append(
            f"approx card coverage regressed: current={current_approx_card:.6f}, baseline={baseline_approx_card:.6f}"
        )

    current_strict_unsupported = metric(report, "strict", "unsupported_lines")
    baseline_strict_unsupported = metric(baseline, "strict", "unsupported_lines")
    if current_strict_unsupported - tol > baseline_strict_unsupported:
        failures.append(
            f"strict unsupported_lines regressed: current={current_strict_unsupported:.0f}, baseline={baseline_strict_unsupported:.0f}"
        )

    if (
        min_parse_line_coverage_strict is not None
        and current_strict_parse < min_parse_line_coverage_strict
    ):
        failures.append(
            f"strict parse_line_coverage below floor: current={current_strict_parse:.6f}, floor={min_parse_line_coverage_strict:.6f}"
        )
    if min_card_coverage_strict is not None and current_strict_card < min_card_coverage_strict:
        failures.append(
            f"strict card coverage below floor: current={current_strict_card:.6f}, floor={min_card_coverage_strict:.6f}"
        )
    if min_card_coverage_approx is not None and current_approx_card < min_card_coverage_approx:
        failures.append(
            f"approx card coverage below floor: current={current_approx_card:.6f}, floor={min_card_coverage_approx:.6f}"
        )
    if (
        max_unsupported_lines_strict is not None
        and current_strict_unsupported > max_unsupported_lines_strict
    ):
        failures.append(
            f"strict unsupported_lines above ceiling: current={current_strict_unsupported:.0f}, ceiling={max_unsupported_lines_strict:.0f}"
        )
    enforce_family_floors(report, "strict", min_family_coverage_strict, failures)
    enforce_family_floors(report, "approx", min_family_coverage_approx, failures)

    if failures:
        for failure in failures:
            print(f"[coverage-budget] {failure}", file=sys.stderr)
        sys.exit(1)

    print("[coverage-budget] OK")


if __name__ == "__main__":
    main()
