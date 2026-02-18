# Invariants & Validation

This page lists contract-sensitive constants and validation paths.

## Canonical constants

Primary sources:

- `weiss_core/src/encode/constants.rs`
- `weiss_core/src/replay.rs`
- `weiss_core/src/db/serialization.rs`
- `weiss_core/src/fingerprint.rs`
- `weiss_core/src/env/constants.rs`

Current values:

| Invariant | Value |
| --- | --- |
| `OBS_LEN` | `378` |
| `ACTION_SPACE_SIZE` | `527` |
| `OBS_ENCODING_VERSION` | `2` |
| `ACTION_ENCODING_VERSION` | `1` |
| `POLICY_VERSION` | `2` |
| `SPEC_HASH` | `8590000130` |
| `REPLAY_SCHEMA_VERSION` | `2` |
| `WSDB_SCHEMA_VERSION` | `2` |
| `CHOICE_COUNT` | `16` |
| `STACK_AUTO_RESOLVE_CAP` | `256` |
| `CHECK_TIMING_QUIESCENCE_CAP` | `256` |
| `FINGERPRINT_ALGO` | `postcard+blake3+u64le v1` |

## Engine fault code contract

| Code | Name |
| --- | --- |
| `0` | `None` |
| `1` | `StackAutoResolveCap` |
| `2` | `TriggerQuiescenceCap` |
| `3` | `Panic` |
| `4` | `ActionError` |
| `5` | `InvariantViolation` |
| `6` | `ResetError` |
| `7` | `ResetPanic` |

## Checks you should run

```bash
python scripts/check_docs_constants.py
python scripts/check_docs_links.py
cargo test --workspace --features test-harness
pytest -q python/tests
```

Notes:

- `check_docs_constants.py` enforces the checksum table in [RL Contract](rl_contract.md).
- not every invariant in this page is automatically cross-checked by one script; tests + code constants remain authoritative.

## Debug validation path

Enable stricter runtime validation during Rust tests:

```bash
WEISS_VALIDATE_STATE=1 cargo test --workspace --features test-harness
```

Use this during engine refactors or determinism bug investigations.

## Change checklist for invariant-sensitive PRs

1. confirm whether constants/versions should change
2. update docs if contract-visible values changed
3. run checks/tests listed above
4. update related pages:
   - [RL Contract](rl_contract.md)
   - [Encodings Changelog](encodings_changelog.md)
   - [Project State](../PROJECT_STATE.md)

## Related

- [RL Contract](rl_contract.md)
- [Encodings](encodings.md)
- [Replays & Determinism](replays_determinism.md)
- [Project State](../PROJECT_STATE.md)
