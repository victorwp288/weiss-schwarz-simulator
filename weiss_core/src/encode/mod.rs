//! Observation/action encoding and spec helpers.
//!
//! Related docs:
//! - <https://github.com/victorwp288/weiss-schwarz-simulator/blob/main/docs/README.md>
//! - <https://github.com/victorwp288/weiss-schwarz-simulator/blob/main/docs/encodings.md>
//! - <https://github.com/victorwp288/weiss-schwarz-simulator/blob/main/docs/encodings_changelog.md>

mod action_ids;
mod constants;
mod mask;
mod observation;
mod spec;

pub(crate) use action_ids::action_meta_for_id;
pub use action_ids::{
    action_desc_for_id, action_id_for, decode_action_id, decode_factorized_action_id,
    encode_factorized_action, ActionIdDesc, ActionParam, ActionParamValue, FactorizedActionDesc,
};
pub use action_ids::{ACTION_META_UNUSED, ACTION_META_WIDTH};
pub use constants::*;
pub use mask::{build_action_mask, fill_action_mask, fill_action_mask_sparse};
pub use observation::encode_observation;
pub use spec::{
    action_spec, action_spec_json, observation_spec, observation_spec_json,
    ActionFactorizationSpec, ActionFamilySpec, ActionSpec, ObsFieldSpec, ObsSliceSpec,
    ObservationSpec, PlayerBlockSpec,
};

pub(crate) use observation::{
    encode_obs_context, encode_obs_header, encode_obs_player_block_into, encode_obs_reason,
    encode_obs_reveal, encode_observation_with_slot_power,
};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ActionDesc;

    const OBS_SPEC_HASH: u64 = 3922564485128559020;
    const ACTION_SPEC_HASH: u64 = 11305511342814019290;

    #[test]
    fn observation_spec_json_snapshot_hash() {
        let json = observation_spec_json();
        let hash = crate::fingerprint::hash_bytes(json.as_bytes());
        assert_eq!(hash, OBS_SPEC_HASH, "obs spec JSON hash changed");
    }

    #[test]
    fn action_spec_json_snapshot_hash() {
        let json = action_spec_json();
        let hash = crate::fingerprint::hash_bytes(json.as_bytes());
        assert_eq!(hash, ACTION_SPEC_HASH, "action spec JSON hash changed");
    }

    #[test]
    fn action_spec_factorization_schema_smoke_test() {
        let spec = action_spec();
        assert_eq!(spec.factorization.meta_version, "action_meta_v1");
        assert_eq!(
            spec.factorization.meta_fields,
            vec!["family_id", "arg0", "arg1", "arg2"]
        );
        assert_eq!(spec.factorization.families.len(), spec.families.len());
        assert_eq!(spec.factorization.families[0].name, "mulligan_confirm");
    }

    fn param(name: &'static str, value: ActionParamValue) -> ActionParam {
        ActionParam { name, value }
    }

    #[test]
    fn factorized_action_id_roundtrip_samples() {
        let samples = vec![
            (
                FactorizedActionDesc {
                    family: "mulligan_confirm",
                    arg0: None,
                    arg1: None,
                    arg2: None,
                },
                MULLIGAN_CONFIRM_ID,
                ActionDesc::MulliganConfirm,
            ),
            (
                FactorizedActionDesc {
                    family: "mulligan_select",
                    arg0: Some(2),
                    arg1: None,
                    arg2: None,
                },
                MULLIGAN_SELECT_BASE + 2,
                ActionDesc::MulliganSelect { hand_index: 2 },
            ),
            (
                FactorizedActionDesc {
                    family: "main_play_character",
                    arg0: Some(1),
                    arg1: Some(2),
                    arg2: None,
                },
                MAIN_PLAY_CHAR_BASE + MAX_STAGE + 2,
                ActionDesc::MainPlayCharacter {
                    hand_index: 1,
                    stage_slot: 2,
                },
            ),
            (
                FactorizedActionDesc {
                    family: "main_move",
                    arg0: Some(0),
                    arg1: Some(1),
                    arg2: None,
                },
                MAIN_MOVE_BASE,
                ActionDesc::MainMove {
                    from_slot: 0,
                    to_slot: 1,
                },
            ),
            (
                FactorizedActionDesc {
                    family: "attack",
                    arg0: Some(1),
                    arg1: Some(1),
                    arg2: None,
                },
                ATTACK_BASE + 4,
                ActionDesc::Attack {
                    slot: 1,
                    attack_type: crate::state::AttackType::Side,
                },
            ),
            (
                FactorizedActionDesc {
                    family: "choice_select",
                    arg0: Some(3),
                    arg1: None,
                    arg2: None,
                },
                CHOICE_BASE + 3,
                ActionDesc::ChoiceSelect { index: 3 },
            ),
            (
                FactorizedActionDesc {
                    family: "concede",
                    arg0: None,
                    arg1: None,
                    arg2: None,
                },
                CONCEDE_ID,
                ActionDesc::Concede,
            ),
        ];

        for (factorized, expected_id, action) in samples {
            let id = encode_factorized_action(&factorized).expect("factorized id");
            assert_eq!(id, expected_id);
            let decoded = decode_factorized_action_id(id).expect("factorized decode");
            assert_eq!(decoded, factorized);
            assert_eq!(encode_factorized_action(&decoded), Some(id));
            assert_eq!(action_id_for(&action), Some(id));
        }
    }

    #[test]
    fn factorized_action_rejects_out_of_range_params() {
        assert_eq!(
            encode_factorized_action(&FactorizedActionDesc {
                family: "mulligan_select",
                arg0: Some(258),
                arg1: None,
                arg2: None,
            }),
            None
        );
        assert_eq!(
            encode_factorized_action(&FactorizedActionDesc {
                family: "attack",
                arg0: Some(1),
                arg1: Some(9),
                arg2: None,
            }),
            None
        );
    }

    #[test]
    fn action_id_decode_roundtrip_samples() {
        let samples = vec![
            (
                ActionDesc::MulliganConfirm,
                ActionIdDesc {
                    family: "mulligan_confirm",
                    params: vec![],
                },
            ),
            (
                ActionDesc::MulliganSelect { hand_index: 2 },
                ActionIdDesc {
                    family: "mulligan_select",
                    params: vec![param("hand_index", ActionParamValue::Int(2))],
                },
            ),
            (
                ActionDesc::Pass,
                ActionIdDesc {
                    family: "pass",
                    params: vec![],
                },
            ),
            (
                ActionDesc::Clock { hand_index: 3 },
                ActionIdDesc {
                    family: "clock_from_hand",
                    params: vec![param("hand_index", ActionParamValue::Int(3))],
                },
            ),
            (
                ActionDesc::MainPlayCharacter {
                    hand_index: 1,
                    stage_slot: 2,
                },
                ActionIdDesc {
                    family: "main_play_character",
                    params: vec![
                        param("hand_index", ActionParamValue::Int(1)),
                        param("stage_slot", ActionParamValue::Int(2)),
                    ],
                },
            ),
            (
                ActionDesc::MainPlayEvent { hand_index: 4 },
                ActionIdDesc {
                    family: "main_play_event",
                    params: vec![param("hand_index", ActionParamValue::Int(4))],
                },
            ),
            (
                ActionDesc::MainMove {
                    from_slot: 0,
                    to_slot: 1,
                },
                ActionIdDesc {
                    family: "main_move",
                    params: vec![
                        param("from_slot", ActionParamValue::Int(0)),
                        param("to_slot", ActionParamValue::Int(1)),
                    ],
                },
            ),
            (
                ActionDesc::ClimaxPlay { hand_index: 2 },
                ActionIdDesc {
                    family: "climax_play",
                    params: vec![param("hand_index", ActionParamValue::Int(2))],
                },
            ),
            (
                ActionDesc::Attack {
                    slot: 1,
                    attack_type: crate::state::AttackType::Side,
                },
                ActionIdDesc {
                    family: "attack",
                    params: vec![
                        param("slot", ActionParamValue::Int(1)),
                        param("attack_type", ActionParamValue::Str("side")),
                    ],
                },
            ),
            (
                ActionDesc::LevelUp { index: 3 },
                ActionIdDesc {
                    family: "level_up",
                    params: vec![param("index", ActionParamValue::Int(3))],
                },
            ),
            (
                ActionDesc::EncorePay { slot: 2 },
                ActionIdDesc {
                    family: "encore_pay",
                    params: vec![param("slot", ActionParamValue::Int(2))],
                },
            ),
            (
                ActionDesc::EncoreDecline { slot: 2 },
                ActionIdDesc {
                    family: "encore_decline",
                    params: vec![param("slot", ActionParamValue::Int(2))],
                },
            ),
            (
                ActionDesc::TriggerOrder { index: 5 },
                ActionIdDesc {
                    family: "trigger_order",
                    params: vec![param("index", ActionParamValue::Int(5))],
                },
            ),
            (
                ActionDesc::ChoiceSelect { index: 3 },
                ActionIdDesc {
                    family: "choice_select",
                    params: vec![param("index", ActionParamValue::Int(3))],
                },
            ),
            (
                ActionDesc::ChoicePrevPage,
                ActionIdDesc {
                    family: "choice_prev_page",
                    params: vec![],
                },
            ),
            (
                ActionDesc::ChoiceNextPage,
                ActionIdDesc {
                    family: "choice_next_page",
                    params: vec![],
                },
            ),
            (
                ActionDesc::Concede,
                ActionIdDesc {
                    family: "concede",
                    params: vec![],
                },
            ),
        ];

        for (action, expected) in samples {
            let id = action_id_for(&action).expect("id");
            let decoded = decode_action_id(id).expect("decode");
            assert_eq!(decoded, expected);
            let back = action_desc_for_id(id).expect("back");
            assert_eq!(back, action);
        }
    }
}
