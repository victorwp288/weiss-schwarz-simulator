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

/// Decode an action id into a human-readable description.
pub fn decode_action_id(id: usize) -> Option<ActionIdDesc> {
    if id >= ACTION_SPACE_SIZE {
        return None;
    }
    if id == MULLIGAN_CONFIRM_ID {
        return Some(ActionIdDesc {
            family: "mulligan_confirm",
            params: vec![],
        });
    }
    if (MULLIGAN_SELECT_BASE..MULLIGAN_SELECT_BASE + MULLIGAN_SELECT_COUNT).contains(&id) {
        let hand_index = (id - MULLIGAN_SELECT_BASE) as i32;
        return Some(ActionIdDesc {
            family: "mulligan_select",
            params: vec![ActionParam {
                name: "hand_index",
                value: ActionParamValue::Int(hand_index),
            }],
        });
    }
    if id == PASS_ACTION_ID {
        return Some(ActionIdDesc {
            family: "pass",
            params: vec![],
        });
    }
    if (CLOCK_HAND_BASE..CLOCK_HAND_BASE + CLOCK_HAND_COUNT).contains(&id) {
        let hand_index = (id - CLOCK_HAND_BASE) as i32;
        return Some(ActionIdDesc {
            family: "clock_from_hand",
            params: vec![ActionParam {
                name: "hand_index",
                value: ActionParamValue::Int(hand_index),
            }],
        });
    }
    if (MAIN_PLAY_CHAR_BASE..MAIN_PLAY_CHAR_BASE + MAIN_PLAY_CHAR_COUNT).contains(&id) {
        let offset = id - MAIN_PLAY_CHAR_BASE;
        let hand_index = (offset / MAX_STAGE) as i32;
        let stage_slot = (offset % MAX_STAGE) as i32;
        return Some(ActionIdDesc {
            family: "main_play_character",
            params: vec![
                ActionParam {
                    name: "hand_index",
                    value: ActionParamValue::Int(hand_index),
                },
                ActionParam {
                    name: "stage_slot",
                    value: ActionParamValue::Int(stage_slot),
                },
            ],
        });
    }
    if (MAIN_PLAY_EVENT_BASE..MAIN_PLAY_EVENT_BASE + MAIN_PLAY_EVENT_COUNT).contains(&id) {
        let hand_index = (id - MAIN_PLAY_EVENT_BASE) as i32;
        return Some(ActionIdDesc {
            family: "main_play_event",
            params: vec![ActionParam {
                name: "hand_index",
                value: ActionParamValue::Int(hand_index),
            }],
        });
    }
    if (MAIN_MOVE_BASE..MAIN_MOVE_BASE + MAIN_MOVE_COUNT).contains(&id) {
        let offset = id - MAIN_MOVE_BASE;
        let from_slot = offset / (MAX_STAGE - 1);
        let to_index = offset % (MAX_STAGE - 1);
        let to_slot = if to_index >= from_slot {
            to_index + 1
        } else {
            to_index
        };
        return Some(ActionIdDesc {
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
        });
    }
    if (CLIMAX_PLAY_BASE..CLIMAX_PLAY_BASE + CLIMAX_PLAY_COUNT).contains(&id) {
        let hand_index = (id - CLIMAX_PLAY_BASE) as i32;
        return Some(ActionIdDesc {
            family: "climax_play",
            params: vec![ActionParam {
                name: "hand_index",
                value: ActionParamValue::Int(hand_index),
            }],
        });
    }
    if (ATTACK_BASE..ATTACK_BASE + ATTACK_COUNT).contains(&id) {
        let offset = id - ATTACK_BASE;
        let slot = (offset / 3) as i32;
        let attack_type = match (offset % 3) as i32 {
            0 => "frontal",
            1 => "side",
            _ => "direct",
        };
        return Some(ActionIdDesc {
            family: "attack",
            params: vec![
                ActionParam {
                    name: "slot",
                    value: ActionParamValue::Int(slot),
                },
                ActionParam {
                    name: "attack_type",
                    value: ActionParamValue::Str(attack_type),
                },
            ],
        });
    }
    if (LEVEL_UP_BASE..LEVEL_UP_BASE + LEVEL_UP_COUNT).contains(&id) {
        let index = (id - LEVEL_UP_BASE) as i32;
        return Some(ActionIdDesc {
            family: "level_up",
            params: vec![ActionParam {
                name: "index",
                value: ActionParamValue::Int(index),
            }],
        });
    }
    if (ENCORE_PAY_BASE..ENCORE_PAY_BASE + ENCORE_PAY_COUNT).contains(&id) {
        let slot = (id - ENCORE_PAY_BASE) as i32;
        return Some(ActionIdDesc {
            family: "encore_pay",
            params: vec![ActionParam {
                name: "slot",
                value: ActionParamValue::Int(slot),
            }],
        });
    }
    if (ENCORE_DECLINE_BASE..ENCORE_DECLINE_BASE + ENCORE_DECLINE_COUNT).contains(&id) {
        let slot = (id - ENCORE_DECLINE_BASE) as i32;
        return Some(ActionIdDesc {
            family: "encore_decline",
            params: vec![ActionParam {
                name: "slot",
                value: ActionParamValue::Int(slot),
            }],
        });
    }
    if (TRIGGER_ORDER_BASE..TRIGGER_ORDER_BASE + TRIGGER_ORDER_COUNT).contains(&id) {
        let index = (id - TRIGGER_ORDER_BASE) as i32;
        return Some(ActionIdDesc {
            family: "trigger_order",
            params: vec![ActionParam {
                name: "index",
                value: ActionParamValue::Int(index),
            }],
        });
    }
    if (CHOICE_BASE..CHOICE_BASE + CHOICE_COUNT).contains(&id) {
        let index = (id - CHOICE_BASE) as i32;
        return Some(ActionIdDesc {
            family: "choice_select",
            params: vec![ActionParam {
                name: "index",
                value: ActionParamValue::Int(index),
            }],
        });
    }
    if id == CHOICE_PREV_ID {
        return Some(ActionIdDesc {
            family: "choice_prev_page",
            params: vec![],
        });
    }
    if id == CHOICE_NEXT_ID {
        return Some(ActionIdDesc {
            family: "choice_next_page",
            params: vec![],
        });
    }
    if id == CONCEDE_ID {
        return Some(ActionIdDesc {
            family: "concede",
            params: vec![],
        });
    }
    None
}

/// Decode an action id into a canonical action descriptor.
pub fn action_desc_for_id(id: usize) -> Option<ActionDesc> {
    if id >= ACTION_SPACE_SIZE {
        return None;
    }
    if id == MULLIGAN_CONFIRM_ID {
        return Some(ActionDesc::MulliganConfirm);
    }
    if (MULLIGAN_SELECT_BASE..MULLIGAN_SELECT_BASE + MULLIGAN_SELECT_COUNT).contains(&id) {
        let hand_index = (id - MULLIGAN_SELECT_BASE) as u8;
        return Some(ActionDesc::MulliganSelect { hand_index });
    }
    if id == PASS_ACTION_ID {
        return Some(ActionDesc::Pass);
    }
    if (CLOCK_HAND_BASE..CLOCK_HAND_BASE + CLOCK_HAND_COUNT).contains(&id) {
        let hand_index = (id - CLOCK_HAND_BASE) as u8;
        return Some(ActionDesc::Clock { hand_index });
    }
    if (MAIN_PLAY_CHAR_BASE..MAIN_PLAY_CHAR_BASE + MAIN_PLAY_CHAR_COUNT).contains(&id) {
        let offset = id - MAIN_PLAY_CHAR_BASE;
        let hand_index = (offset / MAX_STAGE) as u8;
        let stage_slot = (offset % MAX_STAGE) as u8;
        return Some(ActionDesc::MainPlayCharacter {
            hand_index,
            stage_slot,
        });
    }
    if (MAIN_PLAY_EVENT_BASE..MAIN_PLAY_EVENT_BASE + MAIN_PLAY_EVENT_COUNT).contains(&id) {
        let hand_index = (id - MAIN_PLAY_EVENT_BASE) as u8;
        return Some(ActionDesc::MainPlayEvent { hand_index });
    }
    if (MAIN_MOVE_BASE..MAIN_MOVE_BASE + MAIN_MOVE_COUNT).contains(&id) {
        let offset = id - MAIN_MOVE_BASE;
        let from_slot = (offset / (MAX_STAGE - 1)) as u8;
        let to_index = (offset % (MAX_STAGE - 1)) as u8;
        let to_slot = if to_index < from_slot {
            to_index
        } else {
            to_index + 1
        };
        return Some(ActionDesc::MainMove { from_slot, to_slot });
    }
    if (CLIMAX_PLAY_BASE..CLIMAX_PLAY_BASE + CLIMAX_PLAY_COUNT).contains(&id) {
        let hand_index = (id - CLIMAX_PLAY_BASE) as u8;
        return Some(ActionDesc::ClimaxPlay { hand_index });
    }
    if (ATTACK_BASE..ATTACK_BASE + ATTACK_COUNT).contains(&id) {
        let offset = id - ATTACK_BASE;
        let slot = (offset / 3) as u8;
        let attack_type = match (offset % 3) as i32 {
            0 => AttackType::Frontal,
            1 => AttackType::Side,
            _ => AttackType::Direct,
        };
        return Some(ActionDesc::Attack { slot, attack_type });
    }
    if (LEVEL_UP_BASE..LEVEL_UP_BASE + LEVEL_UP_COUNT).contains(&id) {
        let index = (id - LEVEL_UP_BASE) as u8;
        return Some(ActionDesc::LevelUp { index });
    }
    if (ENCORE_PAY_BASE..ENCORE_PAY_BASE + ENCORE_PAY_COUNT).contains(&id) {
        let slot = (id - ENCORE_PAY_BASE) as u8;
        return Some(ActionDesc::EncorePay { slot });
    }
    if (ENCORE_DECLINE_BASE..ENCORE_DECLINE_BASE + ENCORE_DECLINE_COUNT).contains(&id) {
        let slot = (id - ENCORE_DECLINE_BASE) as u8;
        return Some(ActionDesc::EncoreDecline { slot });
    }
    if (TRIGGER_ORDER_BASE..TRIGGER_ORDER_BASE + TRIGGER_ORDER_COUNT).contains(&id) {
        let index = (id - TRIGGER_ORDER_BASE) as u8;
        return Some(ActionDesc::TriggerOrder { index });
    }
    if (CHOICE_BASE..CHOICE_BASE + CHOICE_COUNT).contains(&id) {
        let index = (id - CHOICE_BASE) as u8;
        return Some(ActionDesc::ChoiceSelect { index });
    }
    if id == CHOICE_PREV_ID {
        return Some(ActionDesc::ChoicePrevPage);
    }
    if id == CHOICE_NEXT_ID {
        return Some(ActionDesc::ChoiceNextPage);
    }
    if id == CONCEDE_ID {
        return Some(ActionDesc::Concede);
    }
    None
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
