use serde::Serialize;

use crate::legal::ActionDesc;
use crate::state::AttackType;

use super::constants::*;

/// Parameter value for an action id description.
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(untagged)]
pub enum ActionParamValue {
    /// Integer parameter.
    Int(i32),
    /// String parameter.
    Str(&'static str),
}

/// Named parameter for an action id description.
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct ActionParam {
    /// Parameter name.
    pub name: &'static str,
    /// Parameter value.
    pub value: ActionParamValue,
}

/// Human-readable description of an action id.
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct ActionIdDesc {
    /// Action family name.
    pub family: &'static str,
    /// Parameters associated with the action.
    pub params: Vec<ActionParam>,
}

/// Machine-friendly factorized description of an action id.
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct FactorizedActionDesc {
    /// Action family name.
    pub family: &'static str,
    /// First factorized argument slot.
    pub arg0: Option<u16>,
    /// Second factorized argument slot.
    pub arg1: Option<u16>,
    /// Third factorized argument slot.
    pub arg2: Option<u16>,
}

/// Number of `u16` fields exported for each legal action metadata row.
pub const ACTION_META_WIDTH: usize = 4;
/// Sentinel value used for unused action metadata arguments.
pub const ACTION_META_UNUSED: u16 = u16::MAX;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
enum ActionFamily {
    MulliganConfirm,
    MulliganSelect,
    Pass,
    ClockFromHand,
    MainPlayCharacter,
    MainPlayEvent,
    MainMove,
    ClimaxPlay,
    Attack,
    LevelUp,
    EncorePay,
    EncoreDecline,
    TriggerOrder,
    ChoiceSelect,
    ChoicePrevPage,
    ChoiceNextPage,
    Concede,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ActionKey {
    MulliganConfirm,
    MulliganSelect {
        hand_index: usize,
    },
    Pass,
    ClockFromHand {
        hand_index: usize,
    },
    MainPlayCharacter {
        hand_index: usize,
        stage_slot: usize,
    },
    MainPlayEvent {
        hand_index: usize,
    },
    MainMove {
        from_slot: usize,
        to_slot: usize,
    },
    ClimaxPlay {
        hand_index: usize,
    },
    Attack {
        slot: usize,
        attack_type_code: usize,
    },
    LevelUp {
        index: usize,
    },
    EncorePay {
        slot: usize,
    },
    EncoreDecline {
        slot: usize,
    },
    TriggerOrder {
        index: usize,
    },
    ChoiceSelect {
        index: usize,
    },
    ChoicePrevPage,
    ChoiceNextPage,
    Concede,
}

const ACTION_FAMILY_ORDER: [ActionFamily; 17] = [
    ActionFamily::MulliganConfirm,
    ActionFamily::MulliganSelect,
    ActionFamily::Pass,
    ActionFamily::ClockFromHand,
    ActionFamily::MainPlayCharacter,
    ActionFamily::MainPlayEvent,
    ActionFamily::MainMove,
    ActionFamily::ClimaxPlay,
    ActionFamily::Attack,
    ActionFamily::LevelUp,
    ActionFamily::EncorePay,
    ActionFamily::EncoreDecline,
    ActionFamily::TriggerOrder,
    ActionFamily::ChoiceSelect,
    ActionFamily::ChoicePrevPage,
    ActionFamily::ChoiceNextPage,
    ActionFamily::Concede,
];

const ACTION_FAMILY_BASES: [usize; ACTION_FAMILY_ORDER.len()] = [
    MULLIGAN_CONFIRM_ID,
    MULLIGAN_SELECT_BASE,
    PASS_ACTION_ID,
    CLOCK_HAND_BASE,
    MAIN_PLAY_CHAR_BASE,
    MAIN_PLAY_EVENT_BASE,
    MAIN_MOVE_BASE,
    CLIMAX_PLAY_BASE,
    ATTACK_BASE,
    LEVEL_UP_BASE,
    ENCORE_PAY_BASE,
    ENCORE_DECLINE_BASE,
    TRIGGER_ORDER_BASE,
    CHOICE_BASE,
    CHOICE_PREV_ID,
    CHOICE_NEXT_ID,
    CONCEDE_ID,
];

const ACTION_FAMILY_COUNTS: [usize; ACTION_FAMILY_ORDER.len()] = [
    1,
    MULLIGAN_SELECT_COUNT,
    1,
    CLOCK_HAND_COUNT,
    MAIN_PLAY_CHAR_COUNT,
    MAIN_PLAY_EVENT_COUNT,
    MAIN_MOVE_COUNT,
    CLIMAX_PLAY_COUNT,
    ATTACK_COUNT,
    LEVEL_UP_COUNT,
    ENCORE_PAY_COUNT,
    ENCORE_DECLINE_COUNT,
    TRIGGER_ORDER_COUNT,
    CHOICE_COUNT,
    1,
    1,
    1,
];

#[inline]
const fn action_family_idx(family: ActionFamily) -> usize {
    family as usize
}

#[inline]
fn action_family_base(family: ActionFamily) -> usize {
    ACTION_FAMILY_BASES[action_family_idx(family)]
}

#[inline]
fn action_family_count(family: ActionFamily) -> usize {
    ACTION_FAMILY_COUNTS[action_family_idx(family)]
}

#[inline]
fn action_family_offset_for_id(id: usize) -> Option<(ActionFamily, usize)> {
    if id >= ACTION_SPACE_SIZE {
        return None;
    }
    for family in ACTION_FAMILY_ORDER {
        let base = action_family_base(family);
        if id < base {
            break;
        }
        let offset = id - base;
        if offset < action_family_count(family) {
            return Some((family, offset));
        }
    }
    None
}

#[inline]
fn action_key_from_family_offset(family: ActionFamily, offset: usize) -> ActionKey {
    match family {
        ActionFamily::MulliganConfirm => ActionKey::MulliganConfirm,
        ActionFamily::MulliganSelect => ActionKey::MulliganSelect { hand_index: offset },
        ActionFamily::Pass => ActionKey::Pass,
        ActionFamily::ClockFromHand => ActionKey::ClockFromHand { hand_index: offset },
        ActionFamily::MainPlayCharacter => ActionKey::MainPlayCharacter {
            hand_index: offset / MAX_STAGE,
            stage_slot: offset % MAX_STAGE,
        },
        ActionFamily::MainPlayEvent => ActionKey::MainPlayEvent { hand_index: offset },
        ActionFamily::MainMove => {
            let from_slot = offset / (MAX_STAGE - 1);
            let to_index = offset % (MAX_STAGE - 1);
            let to_slot = if to_index >= from_slot {
                to_index + 1
            } else {
                to_index
            };
            ActionKey::MainMove { from_slot, to_slot }
        }
        ActionFamily::ClimaxPlay => ActionKey::ClimaxPlay { hand_index: offset },
        ActionFamily::Attack => ActionKey::Attack {
            slot: offset / 3,
            attack_type_code: offset % 3,
        },
        ActionFamily::LevelUp => ActionKey::LevelUp { index: offset },
        ActionFamily::EncorePay => ActionKey::EncorePay { slot: offset },
        ActionFamily::EncoreDecline => ActionKey::EncoreDecline { slot: offset },
        ActionFamily::TriggerOrder => ActionKey::TriggerOrder { index: offset },
        ActionFamily::ChoiceSelect => ActionKey::ChoiceSelect { index: offset },
        ActionFamily::ChoicePrevPage => ActionKey::ChoicePrevPage,
        ActionFamily::ChoiceNextPage => ActionKey::ChoiceNextPage,
        ActionFamily::Concede => ActionKey::Concede,
    }
}

#[inline]
fn action_key_for_id(id: usize) -> Option<ActionKey> {
    let (family, offset) = action_family_offset_for_id(id)?;
    Some(action_key_from_family_offset(family, offset))
}

#[inline]
fn action_desc_for_key(action: ActionKey) -> ActionDesc {
    match action {
        ActionKey::MulliganConfirm => ActionDesc::MulliganConfirm,
        ActionKey::MulliganSelect { hand_index } => ActionDesc::MulliganSelect {
            hand_index: hand_index as u8,
        },
        ActionKey::Pass => ActionDesc::Pass,
        ActionKey::ClockFromHand { hand_index } => ActionDesc::Clock {
            hand_index: hand_index as u8,
        },
        ActionKey::MainPlayCharacter {
            hand_index,
            stage_slot,
        } => ActionDesc::MainPlayCharacter {
            hand_index: hand_index as u8,
            stage_slot: stage_slot as u8,
        },
        ActionKey::MainPlayEvent { hand_index } => ActionDesc::MainPlayEvent {
            hand_index: hand_index as u8,
        },
        ActionKey::MainMove { from_slot, to_slot } => ActionDesc::MainMove {
            from_slot: from_slot as u8,
            to_slot: to_slot as u8,
        },
        ActionKey::ClimaxPlay { hand_index } => ActionDesc::ClimaxPlay {
            hand_index: hand_index as u8,
        },
        ActionKey::Attack {
            slot,
            attack_type_code,
        } => ActionDesc::Attack {
            slot: slot as u8,
            attack_type: attack_type_from_code(attack_type_code),
        },
        ActionKey::LevelUp { index } => ActionDesc::LevelUp { index: index as u8 },
        ActionKey::EncorePay { slot } => ActionDesc::EncorePay { slot: slot as u8 },
        ActionKey::EncoreDecline { slot } => ActionDesc::EncoreDecline { slot: slot as u8 },
        ActionKey::TriggerOrder { index } => ActionDesc::TriggerOrder { index: index as u8 },
        ActionKey::ChoiceSelect { index } => ActionDesc::ChoiceSelect { index: index as u8 },
        ActionKey::ChoicePrevPage => ActionDesc::ChoicePrevPage,
        ActionKey::ChoiceNextPage => ActionDesc::ChoiceNextPage,
        ActionKey::Concede => ActionDesc::Concede,
    }
}

#[inline]
fn action_id_desc_for_key(action: ActionKey) -> ActionIdDesc {
    match action {
        ActionKey::MulliganConfirm => ActionIdDesc {
            family: "mulligan_confirm",
            params: vec![],
        },
        ActionKey::MulliganSelect { hand_index } => ActionIdDesc {
            family: "mulligan_select",
            params: vec![ActionParam {
                name: "hand_index",
                value: ActionParamValue::Int(hand_index as i32),
            }],
        },
        ActionKey::Pass => ActionIdDesc {
            family: "pass",
            params: vec![],
        },
        ActionKey::ClockFromHand { hand_index } => ActionIdDesc {
            family: "clock_from_hand",
            params: vec![ActionParam {
                name: "hand_index",
                value: ActionParamValue::Int(hand_index as i32),
            }],
        },
        ActionKey::MainPlayCharacter {
            hand_index,
            stage_slot,
        } => ActionIdDesc {
            family: "main_play_character",
            params: vec![
                ActionParam {
                    name: "hand_index",
                    value: ActionParamValue::Int(hand_index as i32),
                },
                ActionParam {
                    name: "stage_slot",
                    value: ActionParamValue::Int(stage_slot as i32),
                },
            ],
        },
        ActionKey::MainPlayEvent { hand_index } => ActionIdDesc {
            family: "main_play_event",
            params: vec![ActionParam {
                name: "hand_index",
                value: ActionParamValue::Int(hand_index as i32),
            }],
        },
        ActionKey::MainMove { from_slot, to_slot } => ActionIdDesc {
            family: "main_move",
            params: vec![
                ActionParam {
                    name: "from_slot",
                    value: ActionParamValue::Int(from_slot as i32),
                },
                ActionParam {
                    name: "to_slot",
                    value: ActionParamValue::Int(to_slot as i32),
                },
            ],
        },
        ActionKey::ClimaxPlay { hand_index } => ActionIdDesc {
            family: "climax_play",
            params: vec![ActionParam {
                name: "hand_index",
                value: ActionParamValue::Int(hand_index as i32),
            }],
        },
        ActionKey::Attack {
            slot,
            attack_type_code,
        } => ActionIdDesc {
            family: "attack",
            params: vec![
                ActionParam {
                    name: "slot",
                    value: ActionParamValue::Int(slot as i32),
                },
                ActionParam {
                    name: "attack_type",
                    value: ActionParamValue::Str(match attack_type_code {
                        0 => "frontal",
                        1 => "side",
                        _ => "direct",
                    }),
                },
            ],
        },
        ActionKey::LevelUp { index } => ActionIdDesc {
            family: "level_up",
            params: vec![ActionParam {
                name: "index",
                value: ActionParamValue::Int(index as i32),
            }],
        },
        ActionKey::EncorePay { slot } => ActionIdDesc {
            family: "encore_pay",
            params: vec![ActionParam {
                name: "slot",
                value: ActionParamValue::Int(slot as i32),
            }],
        },
        ActionKey::EncoreDecline { slot } => ActionIdDesc {
            family: "encore_decline",
            params: vec![ActionParam {
                name: "slot",
                value: ActionParamValue::Int(slot as i32),
            }],
        },
        ActionKey::TriggerOrder { index } => ActionIdDesc {
            family: "trigger_order",
            params: vec![ActionParam {
                name: "index",
                value: ActionParamValue::Int(index as i32),
            }],
        },
        ActionKey::ChoiceSelect { index } => ActionIdDesc {
            family: "choice_select",
            params: vec![ActionParam {
                name: "index",
                value: ActionParamValue::Int(index as i32),
            }],
        },
        ActionKey::ChoicePrevPage => ActionIdDesc {
            family: "choice_prev_page",
            params: vec![],
        },
        ActionKey::ChoiceNextPage => ActionIdDesc {
            family: "choice_next_page",
            params: vec![],
        },
        ActionKey::Concede => ActionIdDesc {
            family: "concede",
            params: vec![],
        },
    }
}

fn action_meta_for_key(action: ActionKey) -> [u16; ACTION_META_WIDTH] {
    let unused = ACTION_META_UNUSED;
    match action {
        ActionKey::MulliganConfirm => {
            [ActionFamily::MulliganConfirm as u16, unused, unused, unused]
        }
        ActionKey::MulliganSelect { hand_index } => [
            ActionFamily::MulliganSelect as u16,
            hand_index as u16,
            unused,
            unused,
        ],
        ActionKey::Pass => [ActionFamily::Pass as u16, unused, unused, unused],
        ActionKey::ClockFromHand { hand_index } => [
            ActionFamily::ClockFromHand as u16,
            hand_index as u16,
            unused,
            unused,
        ],
        ActionKey::MainPlayCharacter {
            hand_index,
            stage_slot,
        } => [
            ActionFamily::MainPlayCharacter as u16,
            hand_index as u16,
            stage_slot as u16,
            unused,
        ],
        ActionKey::MainPlayEvent { hand_index } => [
            ActionFamily::MainPlayEvent as u16,
            hand_index as u16,
            unused,
            unused,
        ],
        ActionKey::MainMove { from_slot, to_slot } => [
            ActionFamily::MainMove as u16,
            from_slot as u16,
            to_slot as u16,
            unused,
        ],
        ActionKey::ClimaxPlay { hand_index } => [
            ActionFamily::ClimaxPlay as u16,
            hand_index as u16,
            unused,
            unused,
        ],
        ActionKey::Attack {
            slot,
            attack_type_code,
        } => [
            ActionFamily::Attack as u16,
            slot as u16,
            attack_type_code as u16,
            unused,
        ],
        ActionKey::LevelUp { index } => {
            [ActionFamily::LevelUp as u16, index as u16, unused, unused]
        }
        ActionKey::EncorePay { slot } => {
            [ActionFamily::EncorePay as u16, slot as u16, unused, unused]
        }
        ActionKey::EncoreDecline { slot } => [
            ActionFamily::EncoreDecline as u16,
            slot as u16,
            unused,
            unused,
        ],
        ActionKey::TriggerOrder { index } => [
            ActionFamily::TriggerOrder as u16,
            index as u16,
            unused,
            unused,
        ],
        ActionKey::ChoiceSelect { index } => [
            ActionFamily::ChoiceSelect as u16,
            index as u16,
            unused,
            unused,
        ],
        ActionKey::ChoicePrevPage => [ActionFamily::ChoicePrevPage as u16, unused, unused, unused],
        ActionKey::ChoiceNextPage => [ActionFamily::ChoiceNextPage as u16, unused, unused, unused],
        ActionKey::Concede => [ActionFamily::Concede as u16, unused, unused, unused],
    }
}

#[inline]
fn factorized_action_desc_for_key(action: ActionKey) -> FactorizedActionDesc {
    match action {
        ActionKey::MulliganConfirm => FactorizedActionDesc {
            family: "mulligan_confirm",
            arg0: None,
            arg1: None,
            arg2: None,
        },
        ActionKey::MulliganSelect { hand_index } => FactorizedActionDesc {
            family: "mulligan_select",
            arg0: Some(hand_index as u16),
            arg1: None,
            arg2: None,
        },
        ActionKey::Pass => FactorizedActionDesc {
            family: "pass",
            arg0: None,
            arg1: None,
            arg2: None,
        },
        ActionKey::ClockFromHand { hand_index } => FactorizedActionDesc {
            family: "clock_from_hand",
            arg0: Some(hand_index as u16),
            arg1: None,
            arg2: None,
        },
        ActionKey::MainPlayCharacter {
            hand_index,
            stage_slot,
        } => FactorizedActionDesc {
            family: "main_play_character",
            arg0: Some(hand_index as u16),
            arg1: Some(stage_slot as u16),
            arg2: None,
        },
        ActionKey::MainPlayEvent { hand_index } => FactorizedActionDesc {
            family: "main_play_event",
            arg0: Some(hand_index as u16),
            arg1: None,
            arg2: None,
        },
        ActionKey::MainMove { from_slot, to_slot } => FactorizedActionDesc {
            family: "main_move",
            arg0: Some(from_slot as u16),
            arg1: Some(to_slot as u16),
            arg2: None,
        },
        ActionKey::ClimaxPlay { hand_index } => FactorizedActionDesc {
            family: "climax_play",
            arg0: Some(hand_index as u16),
            arg1: None,
            arg2: None,
        },
        ActionKey::Attack {
            slot,
            attack_type_code,
        } => FactorizedActionDesc {
            family: "attack",
            arg0: Some(slot as u16),
            arg1: Some(attack_type_code as u16),
            arg2: None,
        },
        ActionKey::LevelUp { index } => FactorizedActionDesc {
            family: "level_up",
            arg0: Some(index as u16),
            arg1: None,
            arg2: None,
        },
        ActionKey::EncorePay { slot } => FactorizedActionDesc {
            family: "encore_pay",
            arg0: Some(slot as u16),
            arg1: None,
            arg2: None,
        },
        ActionKey::EncoreDecline { slot } => FactorizedActionDesc {
            family: "encore_decline",
            arg0: Some(slot as u16),
            arg1: None,
            arg2: None,
        },
        ActionKey::TriggerOrder { index } => FactorizedActionDesc {
            family: "trigger_order",
            arg0: Some(index as u16),
            arg1: None,
            arg2: None,
        },
        ActionKey::ChoiceSelect { index } => FactorizedActionDesc {
            family: "choice_select",
            arg0: Some(index as u16),
            arg1: None,
            arg2: None,
        },
        ActionKey::ChoicePrevPage => FactorizedActionDesc {
            family: "choice_prev_page",
            arg0: None,
            arg1: None,
            arg2: None,
        },
        ActionKey::ChoiceNextPage => FactorizedActionDesc {
            family: "choice_next_page",
            arg0: None,
            arg1: None,
            arg2: None,
        },
        ActionKey::Concede => FactorizedActionDesc {
            family: "concede",
            arg0: None,
            arg1: None,
            arg2: None,
        },
    }
}

#[inline]
fn action_key_for_factorized_desc(desc: &FactorizedActionDesc) -> Option<ActionKey> {
    match desc.family {
        "mulligan_confirm" if desc.arg0.is_none() && desc.arg1.is_none() && desc.arg2.is_none() => {
            Some(ActionKey::MulliganConfirm)
        }
        "mulligan_select" if desc.arg1.is_none() && desc.arg2.is_none() => {
            let hand_index = usize::from(desc.arg0?);
            (hand_index < MULLIGAN_SELECT_COUNT).then_some(ActionKey::MulliganSelect { hand_index })
        }
        "pass" if desc.arg0.is_none() && desc.arg1.is_none() && desc.arg2.is_none() => {
            Some(ActionKey::Pass)
        }
        "clock_from_hand" if desc.arg1.is_none() && desc.arg2.is_none() => {
            let hand_index = usize::from(desc.arg0?);
            (hand_index < CLOCK_HAND_COUNT).then_some(ActionKey::ClockFromHand { hand_index })
        }
        "main_play_character" if desc.arg2.is_none() => match (desc.arg0, desc.arg1) {
            (Some(hand_index), Some(stage_slot)) => {
                let hand_index = usize::from(hand_index);
                let stage_slot = usize::from(stage_slot);
                (hand_index < MAX_HAND && stage_slot < MAX_STAGE).then_some(
                    ActionKey::MainPlayCharacter {
                        hand_index,
                        stage_slot,
                    },
                )
            }
            _ => None,
        },
        "main_play_event" if desc.arg1.is_none() && desc.arg2.is_none() => {
            let hand_index = usize::from(desc.arg0?);
            (hand_index < MAIN_PLAY_EVENT_COUNT).then_some(ActionKey::MainPlayEvent { hand_index })
        }
        "main_move" if desc.arg2.is_none() => match (desc.arg0, desc.arg1) {
            (Some(from_slot), Some(to_slot)) => {
                let from_slot = usize::from(from_slot);
                let to_slot = usize::from(to_slot);
                (from_slot < MAX_STAGE && to_slot < MAX_STAGE && from_slot != to_slot)
                    .then_some(ActionKey::MainMove { from_slot, to_slot })
            }
            _ => None,
        },
        "climax_play" if desc.arg1.is_none() && desc.arg2.is_none() => {
            let hand_index = usize::from(desc.arg0?);
            (hand_index < CLIMAX_PLAY_COUNT).then_some(ActionKey::ClimaxPlay { hand_index })
        }
        "attack" if desc.arg2.is_none() => match (desc.arg0, desc.arg1) {
            (Some(slot), Some(attack_type_code)) => {
                let slot = usize::from(slot);
                let attack_type_code = usize::from(attack_type_code);
                (slot < ATTACK_SLOT_COUNT && attack_type_code < 3).then_some(ActionKey::Attack {
                    slot,
                    attack_type_code,
                })
            }
            _ => None,
        },
        "level_up" if desc.arg1.is_none() && desc.arg2.is_none() => {
            let index = usize::from(desc.arg0?);
            (index < LEVEL_UP_COUNT).then_some(ActionKey::LevelUp { index })
        }
        "encore_pay" if desc.arg1.is_none() && desc.arg2.is_none() => {
            let slot = usize::from(desc.arg0?);
            (slot < ENCORE_PAY_COUNT).then_some(ActionKey::EncorePay { slot })
        }
        "encore_decline" if desc.arg1.is_none() && desc.arg2.is_none() => {
            let slot = usize::from(desc.arg0?);
            (slot < ENCORE_DECLINE_COUNT).then_some(ActionKey::EncoreDecline { slot })
        }
        "trigger_order" if desc.arg1.is_none() && desc.arg2.is_none() => {
            let index = usize::from(desc.arg0?);
            (index < TRIGGER_ORDER_COUNT).then_some(ActionKey::TriggerOrder { index })
        }
        "choice_select" if desc.arg1.is_none() && desc.arg2.is_none() => {
            let index = usize::from(desc.arg0?);
            (index < CHOICE_COUNT).then_some(ActionKey::ChoiceSelect { index })
        }
        "choice_prev_page" if desc.arg0.is_none() && desc.arg1.is_none() && desc.arg2.is_none() => {
            Some(ActionKey::ChoicePrevPage)
        }
        "choice_next_page" if desc.arg0.is_none() && desc.arg1.is_none() && desc.arg2.is_none() => {
            Some(ActionKey::ChoiceNextPage)
        }
        "concede" if desc.arg0.is_none() && desc.arg1.is_none() && desc.arg2.is_none() => {
            Some(ActionKey::Concede)
        }
        _ => None,
    }
}

/// Decode an action id into a human-readable description.
pub fn decode_action_id(id: usize) -> Option<ActionIdDesc> {
    let action = action_key_for_id(id)?;
    Some(action_id_desc_for_key(action))
}

/// Decode an action id into a factorized family/argument description.
pub fn decode_factorized_action_id(id: usize) -> Option<FactorizedActionDesc> {
    let action = action_key_for_id(id)?;
    Some(factorized_action_desc_for_key(action))
}

/// Encode a factorized family/argument description into an action id.
pub fn encode_factorized_action(desc: &FactorizedActionDesc) -> Option<usize> {
    let action = action_key_for_factorized_desc(desc)?;
    action_id_for(&action_desc_for_key(action))
}

/// Decode an action id into packed legal-action metadata.
pub(crate) fn action_meta_for_id(id: usize) -> Option<[u16; ACTION_META_WIDTH]> {
    let action = action_key_for_id(id)?;
    Some(action_meta_for_key(action))
}

/// Decode an action id into a canonical action descriptor.
pub fn action_desc_for_id(id: usize) -> Option<ActionDesc> {
    let action = action_key_for_id(id)?;
    Some(action_desc_for_key(action))
}

/// Encode a canonical action descriptor into an action id.
pub fn action_id_for(action: &ActionDesc) -> Option<usize> {
    match action {
        ActionDesc::MulliganConfirm => Some(MULLIGAN_CONFIRM_ID),
        ActionDesc::MulliganSelect { hand_index } => {
            let hi = *hand_index as usize;
            if hi < MULLIGAN_SELECT_COUNT {
                Some(MULLIGAN_SELECT_BASE + hi)
            } else {
                None
            }
        }
        ActionDesc::Pass => Some(PASS_ACTION_ID),
        ActionDesc::Clock { hand_index } => {
            let hi = *hand_index as usize;
            if hi < MAX_HAND {
                Some(CLOCK_HAND_BASE + hi)
            } else {
                None
            }
        }
        ActionDesc::MainPlayCharacter {
            hand_index,
            stage_slot,
        } => {
            let hi = *hand_index as usize;
            let ss = *stage_slot as usize;
            if hi < MAX_HAND && ss < MAX_STAGE {
                Some(MAIN_PLAY_CHAR_BASE + hi * MAX_STAGE + ss)
            } else {
                None
            }
        }
        ActionDesc::MainPlayEvent { hand_index } => {
            let hi = *hand_index as usize;
            if hi < MAX_HAND {
                Some(MAIN_PLAY_EVENT_BASE + hi)
            } else {
                None
            }
        }
        ActionDesc::MainMove { from_slot, to_slot } => {
            let fs = *from_slot as usize;
            let ts = *to_slot as usize;
            if fs < MAX_STAGE && ts < MAX_STAGE && fs != ts {
                let to_index = if ts < fs { ts } else { ts - 1 };
                Some(MAIN_MOVE_BASE + fs * (MAX_STAGE - 1) + to_index)
            } else {
                None
            }
        }
        ActionDesc::MainActivateAbility { .. } => None,
        ActionDesc::ClimaxPlay { hand_index } => {
            let hi = *hand_index as usize;
            if hi < MAX_HAND {
                Some(CLIMAX_PLAY_BASE + hi)
            } else {
                None
            }
        }
        ActionDesc::Attack { slot, attack_type } => {
            let s = *slot as usize;
            let t = attack_type_to_i32(*attack_type) as usize;
            if s < ATTACK_SLOT_COUNT && t < 3 {
                Some(ATTACK_BASE + s * 3 + t)
            } else {
                None
            }
        }
        ActionDesc::CounterPlay { .. } => None,
        ActionDesc::LevelUp { index } => {
            let idx = *index as usize;
            if idx < LEVEL_UP_COUNT {
                Some(LEVEL_UP_BASE + idx)
            } else {
                None
            }
        }
        ActionDesc::EncorePay { slot } => {
            let s = *slot as usize;
            if s < ENCORE_PAY_COUNT {
                Some(ENCORE_PAY_BASE + s)
            } else {
                None
            }
        }
        ActionDesc::EncoreDecline { slot } => {
            let s = *slot as usize;
            if s < ENCORE_DECLINE_COUNT {
                Some(ENCORE_DECLINE_BASE + s)
            } else {
                None
            }
        }
        ActionDesc::TriggerOrder { index } => {
            let idx = *index as usize;
            if idx < TRIGGER_ORDER_COUNT {
                Some(TRIGGER_ORDER_BASE + idx)
            } else {
                None
            }
        }
        ActionDesc::ChoiceSelect { index } => {
            let idx = *index as usize;
            if idx < CHOICE_COUNT {
                Some(CHOICE_BASE + idx)
            } else {
                None
            }
        }
        ActionDesc::ChoicePrevPage => Some(CHOICE_PREV_ID),
        ActionDesc::ChoiceNextPage => Some(CHOICE_NEXT_ID),
        ActionDesc::Concede => Some(CONCEDE_ID),
    }
}

fn attack_type_to_i32(attack_type: AttackType) -> i32 {
    match attack_type {
        AttackType::Frontal => 0,
        AttackType::Side => 1,
        AttackType::Direct => 2,
    }
}

#[inline]
fn attack_type_from_code(code: usize) -> AttackType {
    match code {
        0 => AttackType::Frontal,
        1 => AttackType::Side,
        _ => AttackType::Direct,
    }
}
