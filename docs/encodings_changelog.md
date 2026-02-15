# Encodings Changelog

**TL;DR**
- Append-only history of encoding changes tied to version constants.
- Use this with [encodings.md](encodings.md) to interpret semantics.
- Headings are stable anchors for rustdoc links.

[Overview](README.md) | [Quickstart](quickstart.md) | [Engine](engine_architecture.md) | [RL Contract](rl_contract.md) | [Encodings](encodings.md) | [Performance](performance_benchmarks.md) | [Replays](replays_determinism.md) | [Rules](rules_coverage.md) | [Invariants](invariants_validation.md) | [Contributing](contributing.md)

## WSDB_SCHEMA_VERSION 2

- Breaking change: loader now accepts WSDB v2 only and rejects older schema payloads.
- Regeneration is required for existing `.wsdb` artifacts produced under v1.
- Added optional ability provenance field `conditions.source_rule_id` (alias `sourceRuleId`) for parser-v2/rule-pack emission traceability.
- Added optional `AbilityDef.target_card_ids` (alias `targetCardIds`) selector narrowing for exact named/dual-trait targeting.

---

## OBS_ENCODING_VERSION 2

- Added per-front-slot effective soul exposure.
- Added per-front-slot side-attack-allowed flag.
- Observation length changed to `OBS_LEN=378`.

---

## OBS_ENCODING_VERSION 1

- Added public layout with fixed header + two player blocks.
- Added reason bits (phase/resource/target gating).
- Added reveal history buffer (last 8 revealed card ids for the observing player; see `REVEAL_HISTORY_LEN`).
- Added context bits (priority window, choice active, stack non-empty, encore pending).

---

## ACTION_ENCODING_VERSION 1

- Fixed action space with canonical action families.
- Added pass action id and explicit pagination actions for choices.
- Encoded attacks by slot and attack type.

---

## Related

- [Encodings](encodings.md)
- [RL contract](rl_contract.md)
