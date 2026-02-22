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
fn action_id_for_family_offset(family: ActionFamily, offset: usize) -> Option<usize> {
    if offset < action_family_count(family) {
        Some(action_family_base(family) + offset)
    } else {
        None
    }
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
fn family_offset_for_action_key(action: ActionKey) -> Option<(ActionFamily, usize)> {
    match action {
        ActionKey::MulliganConfirm => Some((ActionFamily::MulliganConfirm, 0)),
        ActionKey::MulliganSelect { hand_index } if hand_index < MULLIGAN_SELECT_COUNT => {
            Some((ActionFamily::MulliganSelect, hand_index))
        }
        ActionKey::Pass => Some((ActionFamily::Pass, 0)),
        ActionKey::ClockFromHand { hand_index } if hand_index < MAX_HAND => {
            Some((ActionFamily::ClockFromHand, hand_index))
        }
        ActionKey::MainPlayCharacter {
            hand_index,
            stage_slot,
        } if hand_index < MAX_HAND && stage_slot < MAX_STAGE => Some((
            ActionFamily::MainPlayCharacter,
            hand_index * MAX_STAGE + stage_slot,
        )),
        ActionKey::MainPlayEvent { hand_index } if hand_index < MAX_HAND => {
            Some((ActionFamily::MainPlayEvent, hand_index))
        }
        ActionKey::MainMove { from_slot, to_slot }
            if from_slot < MAX_STAGE && to_slot < MAX_STAGE && from_slot != to_slot =>
        {
            let to_index = if to_slot < from_slot {
                to_slot
            } else {
                to_slot - 1
            };
            Some((
                ActionFamily::MainMove,
                from_slot * (MAX_STAGE - 1) + to_index,
            ))
        }
        ActionKey::ClimaxPlay { hand_index } if hand_index < MAX_HAND => {
            Some((ActionFamily::ClimaxPlay, hand_index))
        }
        ActionKey::Attack {
            slot,
            attack_type_code,
        } if slot < ATTACK_SLOT_COUNT && attack_type_code < 3 => {
            Some((ActionFamily::Attack, slot * 3 + attack_type_code))
        }
        ActionKey::LevelUp { index } if index < LEVEL_UP_COUNT => {
            Some((ActionFamily::LevelUp, index))
        }
        ActionKey::EncorePay { slot } if slot < ENCORE_PAY_COUNT => {
            Some((ActionFamily::EncorePay, slot))
        }
        ActionKey::EncoreDecline { slot } if slot < ENCORE_DECLINE_COUNT => {
            Some((ActionFamily::EncoreDecline, slot))
        }
        ActionKey::TriggerOrder { index } if index < TRIGGER_ORDER_COUNT => {
            Some((ActionFamily::TriggerOrder, index))
        }
        ActionKey::ChoiceSelect { index } if index < CHOICE_COUNT => {
            Some((ActionFamily::ChoiceSelect, index))
        }
        ActionKey::ChoicePrevPage => Some((ActionFamily::ChoicePrevPage, 0)),
        ActionKey::ChoiceNextPage => Some((ActionFamily::ChoiceNextPage, 0)),
        ActionKey::Concede => Some((ActionFamily::Concede, 0)),
        _ => None,
    }
}

#[inline]
fn action_key_for_id(id: usize) -> Option<ActionKey> {
    let (family, offset) = action_family_offset_for_id(id)?;
    Some(action_key_from_family_offset(family, offset))
}

#[inline]
fn action_id_for_key(action: ActionKey) -> Option<usize> {
    let (family, offset) = family_offset_for_action_key(action)?;
    action_id_for_family_offset(family, offset)
}

#[inline]
fn action_key_for_desc(action: &ActionDesc) -> Option<ActionKey> {
    match action {
        ActionDesc::MulliganConfirm => Some(ActionKey::MulliganConfirm),
        ActionDesc::MulliganSelect { hand_index } => Some(ActionKey::MulliganSelect {
            hand_index: *hand_index as usize,
        }),
        ActionDesc::Pass => Some(ActionKey::Pass),
        ActionDesc::Clock { hand_index } => Some(ActionKey::ClockFromHand {
            hand_index: *hand_index as usize,
        }),
        ActionDesc::MainPlayCharacter {
            hand_index,
            stage_slot,
        } => Some(ActionKey::MainPlayCharacter {
            hand_index: *hand_index as usize,
            stage_slot: *stage_slot as usize,
        }),
        ActionDesc::MainPlayEvent { hand_index } => Some(ActionKey::MainPlayEvent {
            hand_index: *hand_index as usize,
        }),
        ActionDesc::MainMove { from_slot, to_slot } => Some(ActionKey::MainMove {
            from_slot: *from_slot as usize,
            to_slot: *to_slot as usize,
        }),
        ActionDesc::MainActivateAbility { .. } => None,
        ActionDesc::ClimaxPlay { hand_index } => Some(ActionKey::ClimaxPlay {
            hand_index: *hand_index as usize,
        }),
        ActionDesc::Attack { slot, attack_type } => Some(ActionKey::Attack {
            slot: *slot as usize,
            attack_type_code: attack_type_to_i32(*attack_type) as usize,
        }),
        ActionDesc::CounterPlay { .. } => None,
        ActionDesc::LevelUp { index } => Some(ActionKey::LevelUp {
            index: *index as usize,
        }),
        ActionDesc::EncorePay { slot } => Some(ActionKey::EncorePay {
            slot: *slot as usize,
        }),
        ActionDesc::EncoreDecline { slot } => Some(ActionKey::EncoreDecline {
            slot: *slot as usize,
        }),
        ActionDesc::TriggerOrder { index } => Some(ActionKey::TriggerOrder {
            index: *index as usize,
        }),
        ActionDesc::ChoiceSelect { index } => Some(ActionKey::ChoiceSelect {
            index: *index as usize,
        }),
        ActionDesc::ChoicePrevPage => Some(ActionKey::ChoicePrevPage),
        ActionDesc::ChoiceNextPage => Some(ActionKey::ChoiceNextPage),
        ActionDesc::Concede => Some(ActionKey::Concede),
    }
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

/// Decode an action id into a human-readable description.
pub fn decode_action_id(id: usize) -> Option<ActionIdDesc> {
    let action = action_key_for_id(id)?;
    Some(action_id_desc_for_key(action))
}

/// Decode an action id into a canonical action descriptor.
pub fn action_desc_for_id(id: usize) -> Option<ActionDesc> {
    let action = action_key_for_id(id)?;
    Some(action_desc_for_key(action))
}

/// Encode a canonical action descriptor into an action id.
pub fn action_id_for(action: &ActionDesc) -> Option<usize> {
    let action = action_key_for_desc(action)?;
    action_id_for_key(action)
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
