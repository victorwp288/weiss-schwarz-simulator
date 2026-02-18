# Invariants & Validation

**TL;DR**
- Invariants define non-negotiable contract properties.
- Some invariants are machine-checked in CI; others are policy assertions documented here.
- Debug validation can surface subtle state consistency bugs early.

[Overview](README.md) | [Quickstart](quickstart.md) | [Engine](engine_architecture.md) | [RL Contract](rl_contract.md) | [Encodings](encodings.md) | [Performance](performance_benchmarks.md) | [Replays](replays_determinism.md) | [Rules](rules_coverage.md) | Invariants | [Contributing](contributing.md)

---

## Why invariants matter

This simulator is used in deterministic RL workflows where small contract drift can silently poison training data.

Invariants protect against:

- accidental encoding changes
- non-deterministic behavior leaks
- visibility/sanitization regressions
- unsafe paging/ordering behavior in choices and actions

---

## Machine-checked invariants

These values are checked by repository tooling/tests and should be treated as authoritative:

- `action_space_size`: 527
- `choice_page_size`: 16
- `action_encoding_version`: 1
- `obs_encoding_version`: 1
- `replay_schema_version`: 2
- `wsdb_schema_version`: 2
- `fingerprint_algo`: `postcard+blake3+u64le v1`
- `observation_visibility_default`: `public`
- `visibility_policies_default`: `false`
- `priority_windows_default`: `false`
- `refresh_penalty_default`: `true`
- `replay_sanitization_requires_visibility_policies`: `false`
- `replay_sanitization_requires_public_visibility`: `true`

Primary checks:

```bash
python scripts/check_docs_constants.py
cargo test --workspace --features test-harness
pytest -q python/tests
```

WSDB compatibility is strict: the loader rejects non-v2 files. Regenerate card DB artifacts with the current parser-v2/rule-pack converter pipeline when this version changes.

---

## Debug validation path

Enable additional runtime assertions during debug runs:

```bash
WEISS_VALIDATE_STATE=1 cargo test --workspace --features test-harness
```

Validation covers areas such as:

- zone consistency and ownership assumptions
- decision-state coherence
- replay/visibility assumptions
- transition safety around timing windows

Use this mode during engine refactors and determinism bug hunts.

---

## Policy assertions (documented, not fully machine-checked)

Current policy-level guarantees:

- replay sanitization in public mode is viewer-agnostic
- choice paging is deterministic and always enabled
- hidden-zone masking is enforced at output boundaries when policies are enabled

If policy changes, update this page and [Project State](../PROJECT_STATE.md) in the same PR.

---

## Contributor checklist for invariant-sensitive changes

Before merging changes that touch encode/env/replay paths:

1. confirm version constants should or should not change
2. re-run docs constants/link checks
3. run deterministic test subsets and full suites as needed
4. update `rl_contract.md` and `encodings_changelog.md` when required

---

## Related

- [RL contract](rl_contract.md)
- [Encodings](encodings.md)
- [Replays & determinism](replays_determinism.md)
- [Project state](../PROJECT_STATE.md)
