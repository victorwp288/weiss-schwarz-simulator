# Documentation Hub

[![Docs checks](https://img.shields.io/badge/docs-checks%20in%20CI-brightgreen)](../.github/workflows/ci.yml)
[![Rustdoc](https://img.shields.io/badge/rustdoc-online-blue)](https://victorwp288.github.io/weiss-schwarz-simulator/rustdoc/)

This is the canonical map for repository docs.

## Overview

Start here:

1. [Beginner happy path](beginner_happy_path.md)
2. [Quickstart](quickstart.md)
3. [How it works](how_it_works.md)
4. [RL Contract](rl_contract.md)

Keep handy:

- [Glossary](glossary.md)
- [Python API Reference (generated)](python_api_reference.md)

## Tutorials (RL)

- [PPO tutorial (masked discrete actions)](tutorials/ppo.md)
- [IMPALA + V-trace tutorial (actor/learner)](tutorials/impala_vtrace.md)

## Read by goal

### I want to run training quickly

1. [Beginner happy path](beginner_happy_path.md)
2. [Quickstart](quickstart.md)
3. [PPO tutorial](tutorials/ppo.md)
4. [RL Contract](rl_contract.md)

### I want to integrate or extend Python tooling

1. [Python API Guide](python_api.md)
2. [Python API Reference (generated)](python_api_reference.md)
3. [Troubleshooting](troubleshooting.md)

### I want to modify engine behavior

1. [Engine Architecture](engine_architecture.md)
2. [Rules Coverage](rules_coverage.md)
3. [Invariants & Validation](invariants_validation.md)
4. [Project State](../PROJECT_STATE.md)

### I want to investigate determinism or replay drift

1. [Replays & Determinism](replays_determinism.md)
2. [RL Contract](rl_contract.md)
3. [Invariants & Validation](invariants_validation.md)

### I want to profile performance

1. [Performance & Benchmarks](performance_benchmarks.md)
2. [Engine Architecture](engine_architecture.md)
3. [Contributing](contributing.md)

## Docs graph

```mermaid
flowchart TD
  BG["beginner_happy_path.md"] --> QS["quickstart.md"]
  QS["quickstart.md"] --> HIT["how_it_works.md"]
  QS --> PPO["tutorials/ppo.md"]
  PPO --> IMP["tutorials/impala_vtrace.md"]
  QS --> RL["rl_contract.md"]
  RL --> ENC["encodings.md"]
  ENC --> CHG["encodings_changelog.md"]
  RL --> REP["replays_determinism.md"]
  QS --> PY["python_api.md"]
  PY --> PYREF["python_api_reference.md"]
  HIT --> RL
  GLO["glossary.md"] --> QS

  ENG["engine_architecture.md"] --> RL
  ENG --> COV["rules_coverage.md"]
  COV --> APR["approximation_policy.md"]
  ENG --> INV["invariants_validation.md"]
  INV --> PS["../PROJECT_STATE.md"]
  PERF["performance_benchmarks.md"] --> ENG
  TRO["troubleshooting.md"] --> QS
  CON["contributing.md"] --> ENG
  CON --> RL
```

## Full docs index

- [Beginner happy path](beginner_happy_path.md): long-form start-to-finish guide from deck building to a minimal reset/step loop and engine mental model.
- [Quickstart](quickstart.md): install paths, first reset/step, and integration sanity checks.
- [How it works](how_it_works.md): decision boundaries, legality pipeline, determinism surfaces, and the high/low-level Python layers.
- [Glossary](glossary.md): contract-facing terms and stable definitions used across docs.
- [Python API Guide](python_api.md): `make/fast/inspect`, `WeissEnv`, `batch.legal`, and integration patterns.
- [Python API Reference (generated)](python_api_reference.md): generated names/signatures for the public `weiss_sim` surface.
- [PPO tutorial](tutorials/ppo.md): minimal masked-discrete PPO loop using `weiss_sim` + PyTorch.
- [IMPALA + V-trace tutorial](tutorials/impala_vtrace.md): minimal actor/learner loop with V-trace corrections.
- [RL Contract](rl_contract.md): step semantics, output schema, and compatibility checksum table.
- [Encodings](encodings.md): observation/action spec model and compatibility process.
- [Encodings Changelog](encodings_changelog.md): append-only encoding/schema history.
- [Engine Architecture](engine_architecture.md): runtime loop, layers, ordering, and safeguards.
- [Rules Coverage](rules_coverage.md): implemented areas, local policy choices, and coverage tooling.
- [Approximation Policy](approximation_policy.md): approved deterministic approximations and gating.
- [Replays & Determinism](replays_determinism.md): replay payloads, visibility modes, and drift workflows.
- [Performance & Benchmarks](performance_benchmarks.md): perf snapshots, budget gates, and baseline regeneration.
- [Invariants & Validation](invariants_validation.md): constants, fault codes, and validation paths.
- [Troubleshooting](troubleshooting.md): common setup/runtime/perf issues and exact fixes.
- [Contributing](contributing.md): branch/PR workflow and required quality gates.
- [Freeze Preflight 2/3/5](freeze_preflight_235.md): freeze artifact runbook.

Repository-level references:

- [Root README](../README.md)
- [Project State](../PROJECT_STATE.md)
- [CHANGELOG](../CHANGELOG.md)

## Doc update rules

When behavior changes, docs change in the same PR.

1. Update the relevant page under `docs/`.
2. If encoding/layout changed, update both:
   - [RL Contract checksum table](rl_contract.md)
   - [Encodings changelog](encodings_changelog.md)
3. Keep [Project State](../PROJECT_STATE.md) aligned with actual runtime behavior.
4. Regenerate generated docs:

```bash
python scripts/gen_docs_snippets.py --write
```

5. Run:

```bash
python scripts/check_docs_links.py
python scripts/check_docs_constants.py
python scripts/gen_docs_snippets.py --check
```
