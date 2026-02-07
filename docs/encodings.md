# Encodings (Observation + Action)

**TL;DR**
- Encodings are a compatibility contract, not an implementation detail.
- Treat spec JSON as authoritative layout metadata.
- Any semantic layout change must be versioned and logged.

[Overview](README.md) | [Quickstart](quickstart.md) | [Engine](engine_architecture.md) | [RL Contract](rl_contract.md) | Encodings | [Performance](performance_benchmarks.md) | [Replays](replays_determinism.md) | [Rules](rules_coverage.md) | [Invariants](invariants_validation.md) | [Contributing](contributing.md)

---

## Contract principles

Encodings are designed for long-lived RL pipelines where model artifacts outlive a single commit.

Rules:

1. Do not silently change field meaning.
2. If semantics/layout change, bump the corresponding encoding version.
3. Keep docs, constants, and tests aligned in the same PR.
4. Keep [Encodings changelog](encodings_changelog.md) append-only.

Primary constant source: `weiss_core/src/encode/constants.rs`.

---

## Observation encoding

Current observation contract:

- dtype: `int32`
- length: `OBS_LEN`
- structure: header + two player blocks + reason/context tails

Top-level observation segments:

- header (`OBS_HEADER_LEN`): phase/decision/attack context fields
- player block x2 (`PER_PLAYER_BLOCK_LEN` each): counts + zone slices
- reason bits (`OBS_REASON_LEN`): coarse gating hints
- reveal history (`OBS_REVEAL_LEN`)
- context bits (`OBS_CONTEXT_LEN`): priority/choice/stack/encore flags

Access runtime spec JSON from Python:

```python
import json
import weiss_sim

obs_spec = json.loads(weiss_sim.observation_spec_json())
print(obs_spec["obs_encoding_version"], obs_spec["obs_len"], obs_spec["dtype"])
```

---

## Action encoding

Current action contract:

- fixed action id space
- total size: `ACTION_SPACE_SIZE`
- pass action id: `PASS_ACTION_ID`
- legal action surfaces: mask and/or packed legal ids

Representative action families:

- mulligan confirm/select
- pass
- clock hand
- main play/move/activate
- climax play
- attack declarations
- level-up and encore decisions
- trigger order
- paged choice actions
- concede

Access runtime action spec JSON:

```python
import json
import weiss_sim

action_spec = json.loads(weiss_sim.action_spec_json())
print(action_spec["action_encoding_version"], action_spec["action_space_size"])
```

---

## Spec bundle handshake pattern

Use the combined bundle when wiring training infra:

```python
import weiss_sim

bundle = weiss_sim.spec_bundle()
print(bundle["policy_version"], bundle["spec_hash"])
```

Recommended production behavior:

- persist `spec_hash` with training checkpoints
- reject loading incompatible checkpoints unless explicitly migrated
- log observation/action encoding versions at run start

---

## Visibility and sanitization interaction

Encoding layout remains stable across visibility modes; values are sanitized/masked based on visibility policy.

Meaning:

- consumers can rely on shape/layout stability
- hidden information may be replaced by sentinel/masked values in public mode
- replay sanitization is separate from in-memory engine internals

---

## Change process (required)

When encoding behavior changes:

1. update `weiss_core/src/encode/constants.rs` and/or encode logic
2. update `docs/rl_contract.md` checksum table
3. append entry in `docs/encodings_changelog.md`
4. run tests and doc checks

```bash
python scripts/check_docs_constants.py
python scripts/check_docs_links.py
cargo test --workspace --features test-harness
pytest -q python/tests
```

---

## Related

- [Encodings changelog](encodings_changelog.md)
- [RL contract](rl_contract.md)
- [Invariants & validation](invariants_validation.md)
