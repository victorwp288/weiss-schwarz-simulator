# Project State

Implementation-facing snapshot of current simulator behavior.

If this file and code disagree, code is authoritative.

## Current posture

- deterministic, RL-first engine with advance-until-decision semantics
- fixed action space and fixed-length observation contract
- Rust core (`weiss_core`) + PyO3 extension (`weiss_py`) + Python API (`python/weiss_sim`)
- replay + fingerprint surfaces used for reproducibility and drift diagnosis

## Compatibility boundaries

Current versioned constants:

- `OBS_ENCODING_VERSION = 2`
- `ACTION_ENCODING_VERSION = 1`
- `POLICY_VERSION = 2`
- `SPEC_HASH = 8590000130`
- `REPLAY_SCHEMA_VERSION = 2`
- `WSDB_SCHEMA_VERSION = 2`

Policy:

- these values define compatibility boundaries
- changing them requires coordinated code/tests/docs updates

## Determinism guarantees

Core determinism properties expected to remain stable:

- canonical action legality and id mapping
- deterministic trigger/stack/priority ordering
- deterministic choice paging (`CHOICE_COUNT=16`)
- explicit bounded loops (`STACK_AUTO_RESOLVE_CAP=256`, `CHECK_TIMING_QUIESCENCE_CAP=256`)
- stable fingerprint algorithm (`postcard+blake3+u64le v1`)

## Runtime fault model

Per-env runtime faults are latched and surfaced through `engine_status`.

Codes:

- `0` none
- `1` stack auto-resolve cap
- `2` trigger quiescence cap
- `3` panic trapped in step/runtime
- `4` action application error
- `5` invariant violation
- `6` reset error
- `7` reset panic

Batch stepping continues for other envs when one env faults.

## Visibility/replay behavior

- public observation mode masks hidden information at output boundaries
- replay visibility mode controls raw (`Full`) vs sanitized (`Public`) replay payloads
- replay public sanitization is tied to replay visibility mode

## API-surface defaults and caveats

There are multiple config entry paths:

- low-level `EnvPool.new_rl_train/new_rl_eval/new_debug`
- high-level `weiss_sim.create/train/evaluate`

Important caveat:

- serialized curriculum payloads use `serde` field defaults for omitted fields
- this can differ from `CurriculumConfig::default()` values
- when behavior matters, set curriculum fields explicitly rather than relying on implicit defaults

## Coverage and parser posture

- parser-v2/rule-pack conversion drives WSDB content and ability template emission
- strict/approx profile coverage is enforced against `scripts/ability_coverage_baseline.json`
- approx-only ability defs are runtime-gated by `enable_approx_effects`

## Known partial areas

- full card-text effect coverage remains in progress
- rule 1.2.5-style direct win/lose-by-effect handling is still a tracked gap
- advanced replacement/prevention layering remains incremental

## Maintenance rules

For behavior changes:

1. update code + tests
2. update relevant docs under `docs/`
3. if contract changed, update:
   - `docs/rl_contract.md`
   - `docs/encodings_changelog.md`
4. run:

```bash
python scripts/check_docs_links.py
python scripts/check_docs_constants.py
```
