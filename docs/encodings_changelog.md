# Encodings Changelog

Append-only history of compatibility-boundary changes.

Use with [Encodings](encodings.md) and [RL Contract](rl_contract.md).

## WSDB_SCHEMA_VERSION 2

- loader accepts WSDB v2 and rejects older schema payloads
- regeneration is required for legacy WSDB v1 artifacts
- optional ability provenance field: `conditions.source_rule_id` (`sourceRuleId` alias)
- optional selector narrowing field: `AbilityDef.target_card_ids` (`targetCardIds` alias)

## OBS_ENCODING_VERSION 2

- added per-front-slot effective soul exposure
- added per-front-slot side-attack-allowed flag
- observation length is now `OBS_LEN=378`

## OBS_ENCODING_VERSION 1

- introduced fixed-layout observation model (header + player blocks + tails)
- added reason bits
- added reveal-history tail
- added context tail for priority/choice/stack/encore indicators

## ACTION_ENCODING_VERSION 1

- fixed action id space with canonical action families
- explicit pass action id
- explicit choice pagination action ids
- encoded attack declarations by slot + attack type

## Related

- [Encodings](encodings.md)
- [RL Contract](rl_contract.md)
