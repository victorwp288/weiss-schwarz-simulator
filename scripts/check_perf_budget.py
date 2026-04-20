#!/usr/bin/env python3
from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path
from typing import Dict, List, Tuple


TIME_UNIT_TO_NS = {
    "ns": 1.0,
    "us": 1_000.0,
    "µs": 1_000.0,
    "ms": 1_000_000.0,
    "s": 1_000_000_000.0,
}


def parse_bencher(path: Path) -> Tuple[Dict[str, Tuple[float, float]], Dict[str, float]]:
    bench_ns: Dict[str, Tuple[float, float]] = {}
    allocs: Dict[str, float] = {}
    test_re = re.compile(
        r"^test\s+([A-Za-z0-9_]+)\s+\.\.\.\s+bench:\s+([0-9][0-9,]*(?:\.[0-9]+)?)\s+([a-zµ]+)\/iter(?:\s+\(\+/-\s+([0-9][0-9,]*(?:\.[0-9]+)?)\))?"
    )
    alloc_re = re.compile(r"^(alloc_[A-Za-z0-9_]+)\s+avg_allocs_per_iter=([0-9]+(?:\.[0-9]+)?)$")
    for raw in path.read_text(encoding="utf-8").splitlines():
        line = raw.strip()
        if not line:
            continue
        alloc_match = alloc_re.match(line)
        if alloc_match:
            allocs[alloc_match.group(1)] = float(alloc_match.group(2))
            continue
        test_match = test_re.match(line)
        if not test_match:
            continue
        name = test_match.group(1)
        value = float(test_match.group(2).replace(",", ""))
        unit = test_match.group(3)
        error = (
            float(test_match.group(4).replace(",", "")) if test_match.group(4) is not None else 0.0
        )
        multiplier = TIME_UNIT_TO_NS.get(unit)
        if multiplier is None:
            raise ValueError(f"unsupported bencher time unit '{unit}' in {path}")
        bench_ns[name] = (value * multiplier, error * multiplier)
    if not bench_ns:
        raise ValueError(f"no bencher test rows found in {path}")
    return bench_ns, allocs


def parse_python_throughput(path: Path) -> Dict[str, float]:
    throughput: Dict[str, float] = {}
    section = "default"
    line_re = re.compile(r"^([A-Za-z0-9_()/\-]+):.*\(([0-9][0-9,]*(?:\.[0-9]+)?)\s+env-steps/sec\)")

    def normalize_section(raw_section: str) -> str:
        token = raw_section.strip().lower()
        # Historical outputs use verbose labels like:
        # "default (auto-thread; ...):" and "explicit serial (...):".
        if token.startswith("default"):
            return "default"
        if token.startswith("explicit serial") or token.startswith("explicit_serial"):
            return "explicit_serial"
        return token.replace(" ", "_")

    for raw in path.read_text(encoding="utf-8").splitlines():
        line = raw.strip()
        if not line:
            continue
        if line.endswith(":") and "env-steps/sec" not in line:
            section = normalize_section(line[:-1])
            continue
        match = line_re.match(line)
        if not match:
            continue
        metric = match.group(1).strip().lower()
        value = float(match.group(2).replace(",", ""))
        throughput[f"{section}::{metric}"] = value
    if not throughput:
        raise ValueError(f"no python throughput rows found in {path}")
    return throughput


def pct_regression_higher_is_worse(baseline: float, current: float) -> float:
    if baseline <= 0:
        return 0.0
    return ((current - baseline) / baseline) * 100.0


def pct_regression_lower_is_worse(baseline: float, current: float) -> float:
    if baseline <= 0:
        return 0.0
    return ((baseline - current) / baseline) * 100.0


def parse_core_budget_overrides(values: List[str]) -> Dict[str, float]:
    overrides: Dict[str, float] = {}
    for raw in values:
        name, sep, pct = raw.partition("=")
        if sep == "" or not name.strip():
            raise ValueError(f"invalid --core-budget-override '{raw}'; expected BENCH_NAME=PERCENT")
        try:
            allowed = float(pct)
        except ValueError as exc:
            raise ValueError(
                f"invalid --core-budget-override '{raw}'; percent must be numeric"
            ) from exc
        if allowed < 0.0:
            raise ValueError(
                f"invalid --core-budget-override '{raw}'; percent must be non-negative"
            )
        overrides[name.strip()] = allowed
    return overrides


def main() -> int:
    parser = argparse.ArgumentParser(description="Enforce performance budget deltas")
    parser.add_argument("--baseline-benches", required=True, help="Baseline bencher output path")
    parser.add_argument("--current-benches", required=True, help="Current bencher output path")
    parser.add_argument(
        "--baseline-python", required=True, help="Baseline python throughput output path"
    )
    parser.add_argument(
        "--current-python", required=True, help="Current python throughput output path"
    )
    parser.add_argument(
        "--max-core-regression-pct",
        type=float,
        default=10.0,
        help="Maximum allowed regression percent for bencher timing rows (higher ns/iter is worse)",
    )
    parser.add_argument(
        "--max-python-regression-pct",
        type=float,
        default=6.0,
        help="Maximum allowed regression percent for python throughput rows (lower env-steps/sec is worse)",
    )
    parser.add_argument(
        "--require-zero-alloc",
        action="store_true",
        help="Fail if any allocation benchmark that was baseline-zero is now non-zero",
    )
    parser.add_argument(
        "--core-budget-override",
        action="append",
        default=[],
        metavar="BENCH_NAME=PERCENT",
        help=(
            "Override the default core regression budget for a specific benchmark row; "
            "repeat for multiple rows"
        ),
    )
    args = parser.parse_args()
    try:
        core_budget_overrides = parse_core_budget_overrides(args.core_budget_override)
    except ValueError as exc:
        parser.error(str(exc))

    baseline_benches, baseline_allocs = parse_bencher(Path(args.baseline_benches))
    current_benches, current_allocs = parse_bencher(Path(args.current_benches))
    baseline_py = parse_python_throughput(Path(args.baseline_python))
    current_py = parse_python_throughput(Path(args.current_python))

    failures: List[str] = []

    baseline_bench_names = set(baseline_benches.keys())
    current_bench_names = set(current_benches.keys())
    baseline_core_names = {name for name in baseline_bench_names if not name.startswith("alloc_")}
    current_core_names = {name for name in current_bench_names if not name.startswith("alloc_")}

    missing_core_benches = sorted(baseline_core_names - current_core_names)
    if missing_core_benches:
        failures.append(
            "missing core benchmarks in current output: " + ", ".join(missing_core_benches)
        )

    shared_benches = sorted(baseline_core_names & current_core_names)
    if not shared_benches:
        failures.append("no shared bencher test names between baseline and current outputs")
    for name in shared_benches:
        baseline, baseline_err = baseline_benches[name]
        current, current_err = current_benches[name]
        regression = pct_regression_higher_is_worse(baseline, current)
        allowed_regression = core_budget_overrides.get(name, args.max_core_regression_pct)
        baseline_upper = baseline + baseline_err
        current_lower = max(0.0, current - current_err)
        budget_upper = baseline_upper * (1.0 + allowed_regression / 100.0)
        print(
            f"[core] {name}: baseline={baseline:.3f}±{baseline_err:.3f}ns "
            f"current={current:.3f}±{current_err:.3f}ns regression={regression:.3f}% "
            f"budget={allowed_regression:.3f}%"
        )
        if current_lower > budget_upper:
            failures.append(
                f"core benchmark '{name}' regressed by {regression:.3f}% "
                f"(max {allowed_regression:.3f}%)"
            )

    baseline_py_names = set(baseline_py.keys())
    current_py_names = set(current_py.keys())
    missing_python_metrics = sorted(baseline_py_names - current_py_names)
    if missing_python_metrics:
        failures.append(
            "missing python throughput metrics in current output: "
            + ", ".join(missing_python_metrics)
        )

    shared_py = sorted(baseline_py_names & current_py_names)
    if not shared_py:
        failures.append("no shared python throughput metrics between baseline and current outputs")
    for name in shared_py:
        baseline = baseline_py[name]
        current = current_py[name]
        regression = pct_regression_lower_is_worse(baseline, current)
        print(
            f"[python] {name}: baseline={baseline:.3f}eps current={current:.3f}eps regression={regression:.3f}%"
        )
        if regression > args.max_python_regression_pct:
            failures.append(
                f"python throughput '{name}' regressed by {regression:.3f}% "
                f"(max {args.max_python_regression_pct:.3f}%)"
            )

    if args.require_zero_alloc:
        missing_alloc_metrics = sorted(set(baseline_allocs.keys()) - set(current_allocs.keys()))
        if missing_alloc_metrics:
            failures.append(
                "missing allocation metrics in current output: " + ", ".join(missing_alloc_metrics)
            )

        shared_allocs = sorted(set(baseline_allocs.keys()) & set(current_allocs.keys()))
        for name in shared_allocs:
            base = baseline_allocs[name]
            cur = current_allocs[name]
            print(f"[alloc] {name}: baseline={base:.3f} current={cur:.3f}")
            if abs(base) < 1e-12 and cur > 1e-12:
                failures.append(
                    f"allocation benchmark '{name}' regressed from zero baseline to {cur:.3f}"
                )

    if failures:
        print("[perf-budget] FAIL")
        for failure in failures:
            print(f" - {failure}")
        return 1

    print("[perf-budget] PASS")
    return 0


if __name__ == "__main__":
    sys.exit(main())
