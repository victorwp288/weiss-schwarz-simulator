# Encodings

Observation/action encodings are long-lived compatibility contracts.

Use runtime JSON specs as the authoritative field layout source.

## Contract rules

1. Never silently change meaning of an encoded field.
2. If semantics/layout change, bump the corresponding encoding version.
3. Update code + docs + tests in one PR.
4. Keep [Encodings Changelog](encodings_changelog.md) append-only.

Primary constant source: `weiss_core/src/encode/constants.rs`.

## Observation encoding

Current contract values:

- `OBS_ENCODING_VERSION = 2`
- `OBS_LEN = 378`
- dtype in core spec: `int32`

High-level structure:

- header (`OBS_HEADER_LEN = 16`)
- player block x2 (`PER_PLAYER_BLOCK_LEN` each)
- reason tail (`OBS_REASON_LEN = 8`)
- reveal history tail (`OBS_REVEAL_LEN`)
- context tail (`OBS_CONTEXT_LEN = 4`)

Read current runtime spec:

```python
import json
import weiss_sim

spec = json.loads(weiss_sim.observation_spec_json())
print(spec["obs_encoding_version"], spec["obs_len"], spec["dtype"])
```

## Action encoding

Current contract values:

- `ACTION_ENCODING_VERSION = 1`
- `ACTION_SPACE_SIZE = 527`
- `PASS_ACTION_ID = 51`

Action ids are fixed-space and cover families such as mulligan, clock, main, attack, level-up, encore, trigger-order, choice pagination, and concede.

Read current runtime spec:

```python
import json
import weiss_sim

spec = json.loads(weiss_sim.action_spec_json())
print(spec["action_encoding_version"], spec["action_space_size"], spec["pass_action_id"])
```

Structured-policy helpers exposed by the action spec bundle:

- `spec["factorization"]` describes the stable family/`arg0`/`arg1`/`arg2` schema.
- `weiss_sim.decode_factorized_action_id(id)` / `weiss_sim.encode_factorized_action(...)` round-trip that schema.
- `weiss_sim.export_spec_bundle()["action_meta_v1"]` documents the packed `legal_action_meta` row layout.

## Spec bundle handshake

```python
import weiss_sim

bundle = weiss_sim.spec_bundle()
print(bundle["policy_version"], bundle["spec_hash"])
```

Recommended integration policy:

- persist `spec_hash` with checkpoints/artifacts
- fail fast on hash mismatch unless explicit migration is applied

## Visibility and sanitization

Layout is stable across visibility modes.

- shape/indices do not change between `public` and `full`
- values may be sanitized/masked in public mode
- replay sanitization is controlled by replay visibility mode, not by changing encoding layout

## Required change process

When encoding semantics/layout change:

1. update encode constants/logic
2. update [RL Contract checksum table](rl_contract.md)
3. append an entry to [Encodings Changelog](encodings_changelog.md)
4. run checks:

```bash
python scripts/check_docs_constants.py
python scripts/check_docs_links.py
cargo test --workspace --features test-harness
python -m pytest -q python/tests
```

## Related

- [RL Contract](rl_contract.md)
- [Encodings Changelog](encodings_changelog.md)
- [Invariants & Validation](invariants_validation.md)
