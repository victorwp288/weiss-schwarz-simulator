# Documentation Hub

This repository has two main audiences:

- RL practitioners integrating the simulator into training pipelines
- Engine contributors extending rules/effects while preserving determinism

Use this page as the canonical map.

## Start Here by Goal

### I want to run training quickly

1. [Quickstart](quickstart.md)
2. [RL Contract](rl_contract.md)
3. [Encodings](encodings.md)

### I want to modify engine behavior

1. [Engine Architecture](engine_architecture.md)
2. [Project State](../PROJECT_STATE.md)
3. [Rules Coverage & Local Policy](rules_coverage.md)
4. [Invariants & Validation](invariants_validation.md)

### I want to debug determinism or replay drift

1. [Replays & Determinism](replays_determinism.md)
2. [RL Contract](rl_contract.md)
3. [Invariants & Validation](invariants_validation.md)

### I want to benchmark or optimize throughput

1. [Performance & Benchmarks](performance_benchmarks.md)
2. [RL Contract](rl_contract.md)
3. [Engine Architecture](engine_architecture.md)

## Full Docs Map

- [Quickstart](quickstart.md): local setup, first successful run, and common onboarding pitfalls.
- [Python API Guide](python_api.md): practical reference for pool constructors, buffer types, and stepping APIs.
- [Engine Architecture](engine_architecture.md): core layering and the advance-until-decision engine flow.
- [RL Contract](rl_contract.md): step semantics, output schema, and compatibility checksum values.
- [Encodings](encodings.md): observation/action encoding model and compatibility expectations.
- [Encodings Changelog](encodings_changelog.md): append-only historical changes by encoding version.
- [Performance & Benchmarks](performance_benchmarks.md): benchmark commands and interpretation guidance.
- [Replays & Determinism](replays_determinism.md): replay pipeline and determinism assumptions.
- [Rules Coverage & Local Policy](rules_coverage.md): implemented sections vs local policy decisions.
- [Approximation Policy](approximation_policy.md): approved deterministic approximation mappings and gates.
- [Invariants & Validation](invariants_validation.md): machine-checked constants and debug validation.
- [Troubleshooting](troubleshooting.md): common build, test, runtime, and determinism issues.
- [Contributing](contributing.md): PR workflow, quality gates, and documentation standards.

## Standard Reading Order

For first-time contributors, this sequence minimizes confusion:

1. [Quickstart](quickstart.md)
2. [RL Contract](rl_contract.md)
3. [Engine Architecture](engine_architecture.md)
4. [Project State](../PROJECT_STATE.md)
5. [Rules Coverage & Local Policy](rules_coverage.md)

Repository-level references:

- [Root README](../README.md)
- [Project State](../PROJECT_STATE.md)
- [CHANGELOG](../CHANGELOG.md)
- [Rust API Docs](https://victorwp288.github.io/weiss-schwarz-simulator/rustdoc/)

## Documentation Standards

When updating behavior, treat docs as part of the implementation:

1. Update the relevant behavioral doc in the same PR.
2. Update `rl_contract.md` and `encodings_changelog.md` for encoding changes.
3. Keep `PROJECT_STATE.md` aligned with current engine behavior.
4. Run local doc checks:

```bash
python scripts/check_docs_links.py
python scripts/check_docs_constants.py
```

## Style Guidelines for New Docs

- State scope and audience in the first section.
- Prefer concrete behavior and constraints over aspirational wording.
- Link to authoritative files or modules when claiming semantics.
- Separate "implemented now" from "future work".
- Avoid duplicating large tables when one canonical source already exists.
