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
    action_desc_for_id, action_id_for, decode_action_id, ActionIdDesc, ActionParam,
    ActionParamValue,
};
pub use action_ids::{ACTION_META_UNUSED, ACTION_META_WIDTH};
pub use constants::*;
pub use mask::{build_action_mask, fill_action_mask, fill_action_mask_sparse};
pub use observation::encode_observation;
pub use spec::{
    action_spec, action_spec_json, observation_spec, observation_spec_json, ActionFamilySpec,
    ActionSpec, ObsFieldSpec, ObsSliceSpec, ObservationSpec, PlayerBlockSpec,
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
    const ACTION_SPEC_HASH: u64 = 2958398628847112153;

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

    fn param(name: &'static str, value: ActionParamValue) -> ActionParam {
        ActionParam { name, value }
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
