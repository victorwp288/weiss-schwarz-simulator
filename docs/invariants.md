# Engine Invariants

This file is intentionally short and machine-checked by tests. If code changes alter any of these,
update this file and the doc-invariants test together.

## Machine-checked
- action_space_size: 527
- choice_page_size: 16
- action_encoding_version: 1
- obs_encoding_version: 1
- replay_schema_version: 2
- wsdb_schema_version: 1
- fingerprint_algo: postcard+blake3+u64le v1
- observation_visibility_default: public
- visibility_policies_default: false
- priority_windows_default: false
- refresh_penalty_default: true
- replay_sanitization_requires_visibility_policies: true
- replay_sanitization_requires_public_visibility: true

## Notes (not machine-checked)
- Replay sanitization is global in public mode (viewer-agnostic).
- Choice paging is always enabled and uses deterministic ordering for candidates.
- Hidden-zone masks are enforced at output boundaries when policies are enabled.
