use crate::config::{CurriculumConfig, ObservationVisibility};
use crate::db::CardDb;
use crate::legal::{ActionDesc, Decision, DecisionKind};
use crate::state::{
    AttackType, GameState, ModifierKind, Phase, StageStatus, TerminalResult, REVEAL_HISTORY_LEN,
};
use serde::Serialize;

pub const OBS_ENCODING_VERSION: u32 = 1;
pub const ACTION_ENCODING_VERSION: u32 = 1;
pub const POLICY_VERSION: u32 = 2;
pub const SPEC_HASH: u64 = ((OBS_ENCODING_VERSION as u64) << 32)
    | ((ACTION_ENCODING_VERSION as u64) << 16)
    | (POLICY_VERSION as u64);

pub const ACTOR_NONE: i8 = -1;
pub const DECISION_KIND_NONE: i8 = -1;

pub const MAX_HAND: usize = 50;
pub const MAX_DECK: usize = 50;
pub const MAX_STAGE: usize = 5;
pub const MAX_ABILITIES_PER_CARD: usize = 4;
pub const ATTACK_SLOT_COUNT: usize = 3;
pub const MAX_LEVEL: usize = 4;
pub const TOP_CLOCK: usize = 7;
pub const TOP_WAITING_ROOM: usize = 5;
pub const TOP_STOCK: usize = 5;
pub const TOP_RESOLUTION: usize = 5;

pub const MULLIGAN_CONFIRM_ID: usize = 0;
pub const MULLIGAN_SELECT_BASE: usize = MULLIGAN_CONFIRM_ID + 1;
pub const MULLIGAN_SELECT_COUNT: usize = MAX_HAND;

pub const PASS_ACTION_ID: usize = MULLIGAN_SELECT_BASE + MULLIGAN_SELECT_COUNT;
pub const CLOCK_HAND_BASE: usize = PASS_ACTION_ID + 1;
pub const CLOCK_HAND_COUNT: usize = MAX_HAND;

pub const MAIN_PLAY_CHAR_BASE: usize = CLOCK_HAND_BASE + CLOCK_HAND_COUNT;
pub const MAIN_PLAY_CHAR_COUNT: usize = MAX_HAND * MAX_STAGE;
pub const MAIN_PLAY_EVENT_BASE: usize = MAIN_PLAY_CHAR_BASE + MAIN_PLAY_CHAR_COUNT;
pub const MAIN_PLAY_EVENT_COUNT: usize = MAX_HAND;
pub const MAIN_MOVE_BASE: usize = MAIN_PLAY_EVENT_BASE + MAIN_PLAY_EVENT_COUNT;
pub const MAIN_MOVE_COUNT: usize = MAX_STAGE * (MAX_STAGE - 1);

pub const CLIMAX_PLAY_BASE: usize = MAIN_MOVE_BASE + MAIN_MOVE_COUNT;
pub const CLIMAX_PLAY_COUNT: usize = MAX_HAND;

pub const ATTACK_BASE: usize = CLIMAX_PLAY_BASE + CLIMAX_PLAY_COUNT;
pub const ATTACK_COUNT: usize = ATTACK_SLOT_COUNT * 3;

pub const LEVEL_UP_BASE: usize = ATTACK_BASE + ATTACK_COUNT;
pub const LEVEL_UP_COUNT: usize = 7;

pub const ENCORE_PAY_BASE: usize = LEVEL_UP_BASE + LEVEL_UP_COUNT;
pub const ENCORE_PAY_COUNT: usize = MAX_STAGE;
pub const ENCORE_DECLINE_BASE: usize = ENCORE_PAY_BASE + ENCORE_PAY_COUNT;
pub const ENCORE_DECLINE_COUNT: usize = MAX_STAGE;

pub const TRIGGER_ORDER_BASE: usize = ENCORE_DECLINE_BASE + ENCORE_DECLINE_COUNT;
pub const TRIGGER_ORDER_COUNT: usize = 10;

pub const CHOICE_BASE: usize = TRIGGER_ORDER_BASE + TRIGGER_ORDER_COUNT;
pub const CHOICE_COUNT: usize = 16;
pub const CHOICE_PREV_ID: usize = CHOICE_BASE + CHOICE_COUNT;
pub const CHOICE_NEXT_ID: usize = CHOICE_PREV_ID + 1;

pub const CONCEDE_ID: usize = CHOICE_NEXT_ID + 1;
pub const ACTION_SPACE_SIZE: usize = CONCEDE_ID + 1;
pub const ACTION_SPACE_WORDS: usize = ACTION_SPACE_SIZE.div_ceil(64);

pub const OBS_HEADER_LEN: usize = 16;
pub const OBS_REASON_LEN: usize = 8;
pub const OBS_REASON_IN_MAIN: usize = 0;
pub const OBS_REASON_IN_CLIMAX: usize = 1;
pub const OBS_REASON_IN_ATTACK: usize = 2;
pub const OBS_REASON_IN_COUNTER_WINDOW: usize = 3;
pub const OBS_REASON_NO_STOCK: usize = 4;
pub const OBS_REASON_NO_COLOR: usize = 5;
pub const OBS_REASON_NO_HAND: usize = 6;
pub const OBS_REASON_NO_TARGETS: usize = 7;
pub const OBS_REVEAL_LEN: usize = REVEAL_HISTORY_LEN;
pub const OBS_CONTEXT_LEN: usize = 4;
pub const OBS_CONTEXT_PRIORITY_WINDOW: usize = 0;
pub const OBS_CONTEXT_CHOICE_ACTIVE: usize = 1;
pub const OBS_CONTEXT_STACK_NONEMPTY: usize = 2;
pub const OBS_CONTEXT_ENCORE_PENDING: usize = 3;
pub const PER_PLAYER_COUNTS: usize = 9;
pub const PER_STAGE_SLOT: usize = 5;
pub const PER_PLAYER_STAGE: usize = MAX_STAGE * PER_STAGE_SLOT;
pub const PER_PLAYER_CLIMAX_TOP: usize = 1;
pub const PER_PLAYER_LEVEL: usize = MAX_LEVEL;
pub const PER_PLAYER_CLOCK_TOP: usize = TOP_CLOCK;
pub const PER_PLAYER_WAITING_TOP: usize = TOP_WAITING_ROOM;
pub const PER_PLAYER_RESOLUTION_TOP: usize = TOP_RESOLUTION;
pub const PER_PLAYER_STOCK_TOP: usize = TOP_STOCK;
pub const PER_PLAYER_HAND: usize = MAX_HAND;
pub const PER_PLAYER_DECK: usize = MAX_DECK;
pub const PER_PLAYER_BLOCK_LEN: usize = PER_PLAYER_COUNTS
    + PER_PLAYER_STAGE
    + PER_PLAYER_CLIMAX_TOP
    + PER_PLAYER_LEVEL
    + PER_PLAYER_CLOCK_TOP
    + PER_PLAYER_WAITING_TOP
    + PER_PLAYER_RESOLUTION_TOP
    + PER_PLAYER_STOCK_TOP
    + PER_PLAYER_HAND
    + PER_PLAYER_DECK;
pub const OBS_REASON_BASE: usize = OBS_HEADER_LEN + 2 * PER_PLAYER_BLOCK_LEN;
pub const OBS_REVEAL_BASE: usize = OBS_REASON_BASE + OBS_REASON_LEN;
pub const OBS_CONTEXT_BASE: usize = OBS_REVEAL_BASE + OBS_REVEAL_LEN;
pub const OBS_LEN: usize = OBS_CONTEXT_BASE + OBS_CONTEXT_LEN;

#[derive(Clone, Debug, Serialize)]
pub struct ObsFieldSpec {
    pub name: &'static str,
    pub index: usize,
    pub visibility: &'static str,
    pub description: &'static str,
}

#[derive(Clone, Debug, Serialize)]
pub struct ObsSliceSpec {
    pub name: &'static str,
    pub start: usize,
    pub len: usize,
    pub visibility: &'static str,
    pub description: &'static str,
}

#[derive(Clone, Debug, Serialize)]
pub struct PlayerBlockSpec {
    pub player_index: u8,
    pub base: usize,
    pub len: usize,
    pub slices: Vec<ObsSliceSpec>,
}

#[derive(Clone, Debug, Serialize)]
pub struct ObservationSpec {
    pub obs_encoding_version: u32,
    pub obs_len: usize,
    pub dtype: &'static str,
    pub self_first: bool,
    pub sentinel_hidden: i32,
    pub sentinel_empty_card: i32,
    pub header_fields: Vec<ObsFieldSpec>,
    pub player_blocks: Vec<PlayerBlockSpec>,
    pub tail_slices: Vec<ObsSliceSpec>,
    pub notes: Vec<&'static str>,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(untagged)]
pub enum ActionParamValue {
    Int(i32),
    Str(&'static str),
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct ActionParam {
    pub name: &'static str,
    pub value: ActionParamValue,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct ActionIdDesc {
    pub family: &'static str,
    pub params: Vec<ActionParam>,
}

#[derive(Clone, Debug, Serialize)]
pub struct ActionFamilySpec {
    pub name: &'static str,
    pub base: usize,
    pub count: usize,
    pub params: Vec<&'static str>,
    pub description: &'static str,
}

#[derive(Clone, Debug, Serialize)]
pub struct ActionSpec {
    pub action_encoding_version: u32,
    pub action_space_size: usize,
    pub pass_action_id: usize,
    pub attack_type_encoding: Vec<(&'static str, i32)>,
    pub constants: Vec<(&'static str, usize)>,
    pub families: Vec<ActionFamilySpec>,
    pub notes: Vec<&'static str>,
}

pub fn observation_spec() -> ObservationSpec {
    let header_fields = vec![
        ObsFieldSpec {
            name: "active_player",
            index: 0,
            visibility: "public",
            description: "active player id",
        },
        ObsFieldSpec {
            name: "phase",
            index: 1,
            visibility: "public",
            description: "phase enum encoding",
        },
        ObsFieldSpec {
            name: "decision_kind",
            index: 2,
            visibility: "public",
            description: "decision kind encoding (or -1 if none)",
        },
        ObsFieldSpec {
            name: "decision_player",
            index: 3,
            visibility: "public",
            description: "player who must act (or -1)",
        },
        ObsFieldSpec {
            name: "terminal",
            index: 4,
            visibility: "public",
            description: "terminal encoding",
        },
        ObsFieldSpec {
            name: "last_action_kind",
            index: 5,
            visibility: "public_or_masked",
            description: "last action kind (masked when opponent action hidden)",
        },
        ObsFieldSpec {
            name: "last_action_param1",
            index: 6,
            visibility: "public_or_masked",
            description: "last action param1 (masked when hidden)",
        },
        ObsFieldSpec {
            name: "last_action_param2",
            index: 7,
            visibility: "public_or_masked",
            description: "last action param2 (masked when hidden)",
        },
        ObsFieldSpec {
            name: "attack_attacker_slot",
            index: 8,
            visibility: "public",
            description: "attacker slot (or -1)",
        },
        ObsFieldSpec {
            name: "attack_defender_slot",
            index: 9,
            visibility: "public",
            description: "defender slot (or -1)",
        },
        ObsFieldSpec {
            name: "attack_type",
            index: 10,
            visibility: "public",
            description: "attack type encoding",
        },
        ObsFieldSpec {
            name: "attack_damage",
            index: 11,
            visibility: "public",
            description: "pending attack damage",
        },
        ObsFieldSpec {
            name: "counter_power",
            index: 12,
            visibility: "public",
            description: "counter power (if any)",
        },
        ObsFieldSpec {
            name: "focus_slot",
            index: 13,
            visibility: "public",
            description: "focus slot (or -1)",
        },
        ObsFieldSpec {
            name: "choice_page_start",
            index: 14,
            visibility: "public",
            description: "choice pagination start (or -1)",
        },
        ObsFieldSpec {
            name: "choice_total_candidates",
            index: 15,
            visibility: "public",
            description: "choice total candidates (or -1)",
        },
    ];

    let mut player_blocks = Vec::new();
    for player_index in 0..2u8 {
        let base = OBS_HEADER_LEN + (player_index as usize) * PER_PLAYER_BLOCK_LEN;
        let mut slices = Vec::new();
        slices.push(ObsSliceSpec {
            name: "counts",
            start: base,
            len: PER_PLAYER_COUNTS,
            visibility: "public",
            description: "zone counts (level, clock, deck, hand, stock, waiting_room, memory, climax, resolution)",
        });
        let stage_start = base + PER_PLAYER_COUNTS;
        slices.push(ObsSliceSpec {
            name: "stage",
            start: stage_start,
            len: PER_PLAYER_STAGE,
            visibility: "public",
            description: "stage slots (card id, status, attacked, power, soul)",
        });
        let mut offset = stage_start + PER_PLAYER_STAGE;
        slices.push(ObsSliceSpec {
            name: "climax_top",
            start: offset,
            len: PER_PLAYER_CLIMAX_TOP,
            visibility: "public",
            description: "top climax card id",
        });
        offset += PER_PLAYER_CLIMAX_TOP;
        slices.push(ObsSliceSpec {
            name: "level",
            start: offset,
            len: PER_PLAYER_LEVEL,
            visibility: "public",
            description: "level zone (top cards)",
        });
        offset += PER_PLAYER_LEVEL;
        slices.push(ObsSliceSpec {
            name: "clock_top",
            start: offset,
            len: PER_PLAYER_CLOCK_TOP,
            visibility: "public",
            description: "clock top cards",
        });
        offset += PER_PLAYER_CLOCK_TOP;
        slices.push(ObsSliceSpec {
            name: "waiting_top",
            start: offset,
            len: PER_PLAYER_WAITING_TOP,
            visibility: "public",
            description: "waiting room top cards",
        });
        offset += PER_PLAYER_WAITING_TOP;
        slices.push(ObsSliceSpec {
            name: "resolution_top",
            start: offset,
            len: PER_PLAYER_RESOLUTION_TOP,
            visibility: "public",
            description: "resolution top cards",
        });
        offset += PER_PLAYER_RESOLUTION_TOP;
        slices.push(ObsSliceSpec {
            name: "stock_top",
            start: offset,
            len: PER_PLAYER_STOCK_TOP,
            visibility: "private_opponent_masked",
            description: "stock top cards (masked for opponent in public mode)",
        });
        offset += PER_PLAYER_STOCK_TOP;
        slices.push(ObsSliceSpec {
            name: "hand",
            start: offset,
            len: PER_PLAYER_HAND,
            visibility: "private_opponent_masked",
            description: "hand cards (masked for opponent in public mode)",
        });
        offset += PER_PLAYER_HAND;
        slices.push(ObsSliceSpec {
            name: "deck",
            start: offset,
            len: PER_PLAYER_DECK,
            visibility: "private_opponent_masked",
            description: "deck cards (masked for opponent in public mode)",
        });
        player_blocks.push(PlayerBlockSpec {
            player_index,
            base,
            len: PER_PLAYER_BLOCK_LEN,
            slices,
        });
    }

    let tail_slices = vec![
        ObsSliceSpec {
            name: "reason_bits",
            start: OBS_REASON_BASE,
            len: OBS_REASON_LEN,
            visibility: "actor_only",
            description: "reason bits (only set for acting player)",
        },
        ObsSliceSpec {
            name: "reveal_history",
            start: OBS_REVEAL_BASE,
            len: OBS_REVEAL_LEN,
            visibility: "viewer_only",
            description: "recent revealed card ids for viewer",
        },
        ObsSliceSpec {
            name: "context_bits",
            start: OBS_CONTEXT_BASE,
            len: OBS_CONTEXT_LEN,
            visibility: "public",
            description: "context bits (priority, choice, stack, encore)",
        },
    ];

    ObservationSpec {
        obs_encoding_version: OBS_ENCODING_VERSION,
        obs_len: OBS_LEN,
        dtype: "int32",
        self_first: true,
        sentinel_hidden: -1,
        sentinel_empty_card: 0,
        header_fields,
        player_blocks,
        tail_slices,
        notes: vec![
            "Observation is encoded from acting player perspective.",
            "Self player block comes first; opponent block comes second.",
            "Hidden zones are masked with sentinel_hidden in public visibility.",
        ],
    }
}

pub fn observation_spec_json() -> String {
    serde_json::to_string_pretty(&observation_spec()).unwrap_or_else(|_| "{}".to_string())
}

pub fn action_spec() -> ActionSpec {
    ActionSpec {
        action_encoding_version: ACTION_ENCODING_VERSION,
        action_space_size: ACTION_SPACE_SIZE,
        pass_action_id: PASS_ACTION_ID,
        attack_type_encoding: vec![("frontal", 0), ("side", 1), ("direct", 2)],
        constants: vec![
            ("max_hand", MAX_HAND),
            ("max_stage", MAX_STAGE),
            ("attack_slot_count", ATTACK_SLOT_COUNT),
            ("choice_page_size", CHOICE_COUNT),
            ("trigger_order_count", TRIGGER_ORDER_COUNT),
            ("level_up_count", LEVEL_UP_COUNT),
        ],
        families: vec![
            ActionFamilySpec {
                name: "mulligan_confirm",
                base: MULLIGAN_CONFIRM_ID,
                count: 1,
                params: vec![],
                description: "confirm mulligan selection",
            },
            ActionFamilySpec {
                name: "mulligan_select",
                base: MULLIGAN_SELECT_BASE,
                count: MULLIGAN_SELECT_COUNT,
                params: vec!["hand_index"],
                description: "toggle mulligan selection",
            },
            ActionFamilySpec {
                name: "pass",
                base: PASS_ACTION_ID,
                count: 1,
                params: vec![],
                description: "contextual pass",
            },
            ActionFamilySpec {
                name: "clock_from_hand",
                base: CLOCK_HAND_BASE,
                count: CLOCK_HAND_COUNT,
                params: vec!["hand_index"],
                description: "clock a card from hand",
            },
            ActionFamilySpec {
                name: "main_play_character",
                base: MAIN_PLAY_CHAR_BASE,
                count: MAIN_PLAY_CHAR_COUNT,
                params: vec!["hand_index", "stage_slot"],
                description: "play character to stage",
            },
            ActionFamilySpec {
                name: "main_play_event",
                base: MAIN_PLAY_EVENT_BASE,
                count: MAIN_PLAY_EVENT_COUNT,
                params: vec!["hand_index"],
                description: "play event",
            },
            ActionFamilySpec {
                name: "main_move",
                base: MAIN_MOVE_BASE,
                count: MAIN_MOVE_COUNT,
                params: vec!["from_slot", "to_slot"],
                description: "move character between stage slots",
            },
            ActionFamilySpec {
                name: "climax_play",
                base: CLIMAX_PLAY_BASE,
                count: CLIMAX_PLAY_COUNT,
                params: vec!["hand_index"],
                description: "play climax",
            },
            ActionFamilySpec {
                name: "attack",
                base: ATTACK_BASE,
                count: ATTACK_COUNT,
                params: vec!["slot", "attack_type"],
                description: "declare attack",
            },
            ActionFamilySpec {
                name: "level_up",
                base: LEVEL_UP_BASE,
                count: LEVEL_UP_COUNT,
                params: vec!["index"],
                description: "select level-up cards",
            },
            ActionFamilySpec {
                name: "encore_pay",
                base: ENCORE_PAY_BASE,
                count: ENCORE_PAY_COUNT,
                params: vec!["slot"],
                description: "pay encore cost for a slot",
            },
            ActionFamilySpec {
                name: "encore_decline",
                base: ENCORE_DECLINE_BASE,
                count: ENCORE_DECLINE_COUNT,
                params: vec!["slot"],
                description: "decline encore for a slot",
            },
            ActionFamilySpec {
                name: "trigger_order",
                base: TRIGGER_ORDER_BASE,
                count: TRIGGER_ORDER_COUNT,
                params: vec!["index"],
                description: "choose trigger order",
            },
            ActionFamilySpec {
                name: "choice_select",
                base: CHOICE_BASE,
                count: CHOICE_COUNT,
                params: vec!["index"],
                description: "select choice option on current page",
            },
            ActionFamilySpec {
                name: "choice_prev_page",
                base: CHOICE_PREV_ID,
                count: 1,
                params: vec![],
                description: "choice pagination previous",
            },
            ActionFamilySpec {
                name: "choice_next_page",
                base: CHOICE_NEXT_ID,
                count: 1,
                params: vec![],
                description: "choice pagination next",
            },
            ActionFamilySpec {
                name: "concede",
                base: CONCEDE_ID,
                count: 1,
                params: vec![],
                description: "concede game (if enabled)",
            },
        ],
        notes: vec![
            "Action ids are stable within ACTION_ENCODING_VERSION.",
            "Use legality masks or legal_action_ids for valid choices.",
        ],
    }
}

pub fn action_spec_json() -> String {
    serde_json::to_string_pretty(&action_spec()).unwrap_or_else(|_| "{}".to_string())
}

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

#[allow(clippy::too_many_arguments)]
pub fn encode_observation(
    state: &GameState,
    db: &CardDb,
    curriculum: &CurriculumConfig,
    perspective: u8,
    decision: Option<&Decision>,
    last_action: Option<&ActionDesc>,
    last_action_player: Option<u8>,
    visibility: ObservationVisibility,
    out: &mut [i32],
) {
    let mut slot_powers = [[0i32; MAX_STAGE]; 2];
    compute_slot_powers_from_state(state, db, &mut slot_powers);
    encode_observation_with_slot_power(
        state,
        db,
        curriculum,
        perspective,
        decision,
        last_action,
        last_action_player,
        visibility,
        &slot_powers,
        out,
    );
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn encode_obs_header(
    state: &GameState,
    perspective: u8,
    decision: Option<&Decision>,
    last_action: Option<&ActionDesc>,
    last_action_player: Option<u8>,
    visibility: ObservationVisibility,
    out: &mut [i32],
) {
    assert!(out.len() >= OBS_HEADER_LEN);
    out[0] = state.turn.active_player as i32;
    out[1] = phase_to_i32(state.turn.phase);
    out[2] = decision_kind_to_i32(decision.map(|d| d.kind));
    out[3] = decision.map(|d| d.player as i32).unwrap_or(-1);
    out[4] = terminal_to_i32(state.terminal);
    let (last_kind, last_p1, last_p2) =
        last_action_to_fields(last_action, last_action_player, perspective, visibility);
    out[5] = last_kind;
    out[6] = last_p1;
    out[7] = last_p2;
    if let Some(ctx) = &state.turn.attack {
        out[8] = ctx.attacker_slot as i32;
        out[9] = ctx.defender_slot.map(|s| s as i32).unwrap_or(-1);
        out[10] = attack_type_to_i32(ctx.attack_type);
        out[11] = ctx.damage;
        out[12] = ctx.counter_power;
    } else {
        out[8] = -1;
        out[9] = -1;
        out[10] = -1;
        out[11] = 0;
        out[12] = 0;
    }
    out[13] = decision
        .and_then(|d| d.focus_slot.map(|s| s as i32))
        .unwrap_or(-1);
    let choice_page = decision
        .filter(|d| d.kind == DecisionKind::Choice)
        .and(state.turn.choice.as_ref())
        .map(|choice| (choice.page_start as i32, choice.total_candidates as i32));
    if let Some((page_start, total)) = choice_page {
        out[14] = page_start;
        out[15] = total;
    } else {
        out[14] = -1;
        out[15] = -1;
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn encode_obs_player_block(
    state: &GameState,
    db: &CardDb,
    curriculum: &CurriculumConfig,
    perspective: u8,
    player_index: u8,
    visibility: ObservationVisibility,
    slot_powers: &[[i32; MAX_STAGE]; 2],
    out: &mut [i32],
) {
    assert!(out.len() >= OBS_HEADER_LEN + 2 * PER_PLAYER_BLOCK_LEN);
    let p = player_index as usize;
    let block_index = if p == perspective as usize { 0 } else { 1 };
    let is_self = block_index == 0;
    let memory_visible =
        visibility == ObservationVisibility::Full || curriculum.memory_is_public || is_self;
    let hand_visible = visibility == ObservationVisibility::Full || is_self;
    let stock_visible = visibility == ObservationVisibility::Full || is_self;
    let deck_visible = visibility == ObservationVisibility::Full || is_self;
    let base = OBS_HEADER_LEN + block_index * PER_PLAYER_BLOCK_LEN;
    let block = &mut out[base..base + PER_PLAYER_BLOCK_LEN];
    encode_obs_player_block_into(
        state,
        db,
        player_index,
        memory_visible,
        hand_visible,
        stock_visible,
        deck_visible,
        slot_powers,
        block,
    );
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn encode_obs_player_block_into(
    state: &GameState,
    db: &CardDb,
    player_index: u8,
    memory_visible: bool,
    hand_visible: bool,
    stock_visible: bool,
    deck_visible: bool,
    slot_powers: &[[i32; MAX_STAGE]; 2],
    out: &mut [i32],
) {
    assert!(out.len() >= PER_PLAYER_BLOCK_LEN);
    let p = player_index as usize;
    let mut offset = 0;
    let player = &state.players[p];
    out[offset] = player.level.len() as i32;
    out[offset + 1] = player.clock.len() as i32;
    out[offset + 2] = player.deck.len() as i32;
    out[offset + 3] = player.hand.len() as i32;
    out[offset + 4] = player.stock.len() as i32;
    out[offset + 5] = player.waiting_room.len() as i32;
    out[offset + 6] = if memory_visible {
        player.memory.len() as i32
    } else {
        0
    };
    out[offset + 7] = player.climax.len() as i32;
    out[offset + 8] = player.resolution.len() as i32;
    offset += PER_PLAYER_COUNTS;

    for (slot, slot_state) in player.stage.iter().enumerate() {
        let card_id = slot_state.card.map(|c| c.id).unwrap_or(0) as i32;
        let status = if slot_state.card.is_some() {
            status_to_i32(slot_state.status)
        } else {
            0
        };
        let has_attacked = if slot_state.has_attacked { 1 } else { 0 };
        let (power, soul) = if let Some(card_inst) = slot_state.card {
            let power = slot_powers[p][slot];
            let soul = db.soul_by_id(card_inst.id) as i32;
            (power, soul)
        } else {
            (0, 0)
        };
        let base = offset + slot * PER_STAGE_SLOT;
        out[base] = card_id;
        out[base + 1] = status;
        out[base + 2] = has_attacked;
        out[base + 3] = power;
        out[base + 4] = soul;
    }
    offset += PER_PLAYER_STAGE;

    out[offset] = player.climax.last().map(|c| c.id).unwrap_or(0) as i32;
    offset += PER_PLAYER_CLIMAX_TOP;

    for i in 0..MAX_LEVEL {
        out[offset + i] = player.level.get(i).map(|c| c.id).unwrap_or(0) as i32;
    }
    offset += PER_PLAYER_LEVEL;

    for i in 0..TOP_CLOCK {
        if i < player.clock.len() {
            let idx = player.clock.len() - 1 - i;
            out[offset + i] = player.clock[idx].id as i32;
        } else {
            out[offset + i] = 0;
        }
    }
    offset += PER_PLAYER_CLOCK_TOP;

    for i in 0..TOP_WAITING_ROOM {
        if i < player.waiting_room.len() {
            let idx = player.waiting_room.len() - 1 - i;
            out[offset + i] = player.waiting_room[idx].id as i32;
        } else {
            out[offset + i] = 0;
        }
    }
    offset += PER_PLAYER_WAITING_TOP;

    for i in 0..TOP_RESOLUTION {
        if i < player.resolution.len() {
            let idx = player.resolution.len() - 1 - i;
            out[offset + i] = player.resolution[idx].id as i32;
        } else {
            out[offset + i] = 0;
        }
    }
    offset += PER_PLAYER_RESOLUTION_TOP;

    if stock_visible {
        for i in 0..TOP_STOCK {
            if i < player.stock.len() {
                let idx = player.stock.len() - 1 - i;
                out[offset + i] = player.stock[idx].id as i32;
            } else {
                out[offset + i] = 0;
            }
        }
    } else {
        out[offset..offset + TOP_STOCK].fill(-1);
    }
    offset += PER_PLAYER_STOCK_TOP;

    if hand_visible {
        for i in 0..MAX_HAND {
            out[offset + i] = player.hand.get(i).map(|c| c.id).unwrap_or(0) as i32;
        }
    } else {
        out[offset..offset + MAX_HAND].fill(-1);
    }
    offset += MAX_HAND;

    if deck_visible {
        for i in 0..MAX_DECK {
            out[offset + i] = if i < player.deck.len() {
                let deck_idx = player.deck.len() - 1 - i;
                player.deck[deck_idx].id as i32
            } else {
                0
            };
        }
    } else {
        out[offset..offset + MAX_DECK].fill(-1);
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn encode_obs_reason(
    state: &GameState,
    db: &CardDb,
    curriculum: &CurriculumConfig,
    perspective: u8,
    decision: Option<&Decision>,
    out: &mut [i32],
) {
    assert!(out.len() >= OBS_REASON_BASE + OBS_REASON_LEN);
    let reason_bits = compute_reason_bits(state, db, curriculum, perspective, decision);
    let reason_base = OBS_REASON_BASE;
    out[reason_base..reason_base + OBS_REASON_LEN].copy_from_slice(&reason_bits);
}

pub(crate) fn encode_obs_reveal(state: &GameState, perspective: u8, out: &mut [i32]) {
    assert!(out.len() >= OBS_REVEAL_BASE + OBS_REVEAL_LEN);
    let reveal_base = OBS_REVEAL_BASE;
    let reveal_slice = &mut out[reveal_base..reveal_base + OBS_REVEAL_LEN];
    state.reveal_history[perspective as usize].write_chronological(reveal_slice);
}

pub(crate) fn encode_obs_context(state: &GameState, out: &mut [i32]) {
    assert!(out.len() >= OBS_CONTEXT_BASE + OBS_CONTEXT_LEN);
    let context_base = OBS_CONTEXT_BASE;
    let context_bits = compute_context_bits(state);
    out[context_base..context_base + OBS_CONTEXT_LEN].copy_from_slice(&context_bits);
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn encode_observation_with_slot_power(
    state: &GameState,
    db: &CardDb,
    curriculum: &CurriculumConfig,
    perspective: u8,
    decision: Option<&Decision>,
    last_action: Option<&ActionDesc>,
    last_action_player: Option<u8>,
    visibility: ObservationVisibility,
    slot_powers: &[[i32; MAX_STAGE]; 2],
    out: &mut [i32],
) {
    assert!(out.len() >= OBS_LEN);
    encode_obs_header(
        state,
        perspective,
        decision,
        last_action,
        last_action_player,
        visibility,
        out,
    );
    encode_obs_player_block(
        state,
        db,
        curriculum,
        perspective,
        perspective,
        visibility,
        slot_powers,
        out,
    );
    let other = 1 - perspective;
    encode_obs_player_block(
        state,
        db,
        curriculum,
        perspective,
        other,
        visibility,
        slot_powers,
        out,
    );
    encode_obs_reason(state, db, curriculum, perspective, decision, out);
    encode_obs_reveal(state, perspective, out);
    encode_obs_context(state, out);
}

fn compute_slot_powers_from_state(state: &GameState, db: &CardDb, out: &mut [[i32; MAX_STAGE]; 2]) {
    let mut has_power_mods = false;
    for modifier in &state.modifiers {
        if modifier.kind == ModifierKind::Power {
            has_power_mods = true;
            break;
        }
    }
    if !has_power_mods {
        for (player, p) in state.players.iter().enumerate() {
            for (slot, slot_state) in p.stage.iter().enumerate() {
                let power = if let Some(card_inst) = slot_state.card {
                    db.power_by_id(card_inst.id)
                        + slot_state.power_mod_turn
                        + slot_state.power_mod_battle
                } else {
                    0
                };
                out[player][slot] = power;
            }
        }
        return;
    }
    let mut slot_card_ids = [[0u32; MAX_STAGE]; 2];
    for (player, p) in state.players.iter().enumerate() {
        for (slot, slot_state) in p.stage.iter().enumerate() {
            slot_card_ids[player][slot] = slot_state.card.map(|c| c.id).unwrap_or(0);
        }
    }
    let mut slot_power_mods = [[0i32; MAX_STAGE]; 2];
    for modifier in &state.modifiers {
        if modifier.kind != ModifierKind::Power {
            continue;
        }
        let p = modifier.target_player as usize;
        let s = modifier.target_slot as usize;
        if p >= 2 || s >= MAX_STAGE {
            continue;
        }
        if slot_card_ids[p][s] != modifier.target_card {
            continue;
        }
        slot_power_mods[p][s] = slot_power_mods[p][s].saturating_add(modifier.magnitude);
    }
    for (player, p) in state.players.iter().enumerate() {
        for (slot, slot_state) in p.stage.iter().enumerate() {
            let power = if let Some(card_inst) = slot_state.card {
                db.power_by_id(card_inst.id)
                    + slot_state.power_mod_turn
                    + slot_state.power_mod_battle
                    + slot_power_mods[player][slot]
            } else {
                0
            };
            out[player][slot] = power;
        }
    }
}

fn compute_reason_bits(
    state: &GameState,
    db: &CardDb,
    curriculum: &CurriculumConfig,
    perspective: u8,
    decision: Option<&Decision>,
) -> [i32; OBS_REASON_LEN] {
    let mut out = [0i32; OBS_REASON_LEN];
    let decision = match decision {
        Some(decision) if decision.player == perspective => decision,
        _ => return out,
    };
    let in_main = decision.kind == DecisionKind::Main;
    let in_climax = decision.kind == DecisionKind::Climax;
    let in_attack = decision.kind == DecisionKind::AttackDeclaration;
    let in_counter_window = state
        .turn
        .priority
        .as_ref()
        .map(|p| p.window == crate::state::TimingWindow::CounterWindow)
        .unwrap_or(false);
    out[OBS_REASON_IN_MAIN] = i32::from(in_main);
    out[OBS_REASON_IN_CLIMAX] = i32::from(in_climax);
    out[OBS_REASON_IN_ATTACK] = i32::from(in_attack);
    out[OBS_REASON_IN_COUNTER_WINDOW] = i32::from(in_counter_window);

    let p = &state.players[perspective as usize];
    let mut any_candidate = false;
    let mut stock_blocked = false;
    let mut color_blocked = false;
    if in_main || in_climax {
        for card_inst in &p.hand {
            let Some(card) = db.get(card_inst.id) else {
                continue;
            };
            if !card_set_allowed(card, curriculum) {
                continue;
            }
            if in_main {
                match card.card_type {
                    crate::db::CardType::Character => {
                        if !curriculum.allow_character {
                            continue;
                        }
                    }
                    crate::db::CardType::Event => {
                        if !curriculum.allow_event {
                            continue;
                        }
                    }
                    _ => continue,
                }
            } else if in_climax {
                if card.card_type != crate::db::CardType::Climax || !curriculum.allow_climax {
                    continue;
                }
                if !curriculum.enable_climax_phase {
                    continue;
                }
            }
            if !meets_level_requirement(card, p.level.len()) {
                continue;
            }
            any_candidate = true;
            if !meets_cost_requirement(card, p, curriculum) {
                stock_blocked = true;
            }
            if !meets_color_requirement(card, p, db, curriculum) {
                color_blocked = true;
            }
        }
    }
    if in_main || in_climax {
        out[OBS_REASON_NO_HAND] = i32::from(!any_candidate);
        out[OBS_REASON_NO_STOCK] = i32::from(stock_blocked);
        out[OBS_REASON_NO_COLOR] = i32::from(color_blocked);
    }

    let no_targets = decision.kind == DecisionKind::Choice
        && state
            .turn
            .choice
            .as_ref()
            .map(|choice| {
                choice
                    .options
                    .iter()
                    .all(|opt| opt.zone == crate::state::ChoiceZone::Skip)
            })
            .unwrap_or(true);
    out[OBS_REASON_NO_TARGETS] = i32::from(no_targets);

    out
}

fn compute_context_bits(state: &GameState) -> [i32; OBS_CONTEXT_LEN] {
    let mut out = [0i32; OBS_CONTEXT_LEN];
    out[OBS_CONTEXT_PRIORITY_WINDOW] = i32::from(state.turn.priority.is_some());
    out[OBS_CONTEXT_CHOICE_ACTIVE] = i32::from(state.turn.choice.is_some());
    out[OBS_CONTEXT_STACK_NONEMPTY] = i32::from(!state.turn.stack.is_empty());
    out[OBS_CONTEXT_ENCORE_PENDING] = i32::from(!state.turn.encore_queue.is_empty());
    out
}

fn card_set_allowed(card: &crate::db::CardStatic, curriculum: &CurriculumConfig) -> bool {
    if let Some(set) = curriculum.allowed_card_sets_cache.as_ref() {
        match &card.card_set {
            Some(set_id) => set.contains(set_id),
            None => false,
        }
    } else if curriculum.allowed_card_sets.is_empty() {
        true
    } else {
        card.card_set
            .as_ref()
            .map(|s| curriculum.allowed_card_sets.iter().any(|a| a == s))
            .unwrap_or(false)
    }
}

fn meets_level_requirement(card: &crate::db::CardStatic, level_count: usize) -> bool {
    card.level as usize <= level_count
}

fn meets_cost_requirement(
    card: &crate::db::CardStatic,
    player: &crate::state::PlayerState,
    curriculum: &CurriculumConfig,
) -> bool {
    if !curriculum.enforce_cost_requirement {
        return true;
    }
    player.stock.len() >= card.cost as usize
}

fn meets_color_requirement(
    card: &crate::db::CardStatic,
    player: &crate::state::PlayerState,
    db: &CardDb,
    curriculum: &CurriculumConfig,
) -> bool {
    if !curriculum.enforce_color_requirement {
        return true;
    }
    if card.level == 0 || card.color == crate::db::CardColor::Colorless {
        return true;
    }
    for card_id in player.level.iter().chain(player.clock.iter()) {
        let id = card_id.id;
        if id != 0 && db.color_by_id(id) == card.color {
            return true;
        }
    }
    false
}

fn phase_to_i32(phase: Phase) -> i32 {
    match phase {
        Phase::Mulligan => 0,
        Phase::Stand => 1,
        Phase::Draw => 2,
        Phase::Clock => 3,
        Phase::Main => 4,
        Phase::Climax => 5,
        Phase::Attack => 6,
        Phase::End => 7,
    }
}

fn decision_kind_to_i32(kind: Option<DecisionKind>) -> i32 {
    match kind {
        Some(DecisionKind::Mulligan) => 0,
        Some(DecisionKind::Clock) => 1,
        Some(DecisionKind::Main) => 2,
        Some(DecisionKind::Climax) => 3,
        Some(DecisionKind::AttackDeclaration) => 4,
        Some(DecisionKind::LevelUp) => 5,
        Some(DecisionKind::Encore) => 6,
        Some(DecisionKind::TriggerOrder) => 7,
        Some(DecisionKind::Choice) => 8,
        None => -1,
    }
}

fn attack_type_to_i32(attack_type: AttackType) -> i32 {
    match attack_type {
        AttackType::Frontal => 0,
        AttackType::Side => 1,
        AttackType::Direct => 2,
    }
}

fn status_to_i32(status: StageStatus) -> i32 {
    match status {
        StageStatus::Stand => 1,
        StageStatus::Rest => 2,
        StageStatus::Reverse => 3,
    }
}

fn terminal_to_i32(term: Option<TerminalResult>) -> i32 {
    match term {
        None => 0,
        Some(TerminalResult::Win { winner }) => {
            if winner == 0 {
                1
            } else {
                2
            }
        }
        Some(TerminalResult::Draw) => 3,
        Some(TerminalResult::Timeout) => 4,
    }
}

fn last_action_to_fields(
    action: Option<&ActionDesc>,
    actor: Option<u8>,
    perspective: u8,
    visibility: ObservationVisibility,
) -> (i32, i32, i32) {
    let mask = visibility == ObservationVisibility::Public
        && actor.map(|p| p != perspective).unwrap_or(false);
    match action {
        None => (0, -1, -1),
        Some(ActionDesc::MulliganConfirm) => (1, -1, -1),
        Some(ActionDesc::MulliganSelect { hand_index }) => {
            let idx = if mask { -1 } else { *hand_index as i32 };
            (2, idx, -1)
        }
        Some(ActionDesc::Pass) => (3, -1, -1),
        Some(ActionDesc::Clock { hand_index }) => {
            let idx = if mask { -1 } else { *hand_index as i32 };
            (4, idx, -1)
        }
        Some(ActionDesc::MainPlayCharacter {
            hand_index,
            stage_slot,
        }) => {
            let idx = if mask { -1 } else { *hand_index as i32 };
            (6, idx, *stage_slot as i32)
        }
        Some(ActionDesc::MainPlayEvent { hand_index }) => {
            let idx = if mask { -1 } else { *hand_index as i32 };
            (7, idx, -1)
        }
        Some(ActionDesc::MainMove { from_slot, to_slot }) => {
            (8, *from_slot as i32, *to_slot as i32)
        }
        Some(ActionDesc::MainActivateAbility {
            slot,
            ability_index,
        }) => (9, *slot as i32, *ability_index as i32),
        Some(ActionDesc::ClimaxPlay { hand_index }) => {
            let idx = if mask { -1 } else { *hand_index as i32 };
            (11, idx, -1)
        }
        Some(ActionDesc::Attack { slot, attack_type }) => {
            (13, *slot as i32, attack_type_to_i32(*attack_type))
        }
        Some(ActionDesc::CounterPlay { hand_index }) => {
            let idx = if mask { -1 } else { *hand_index as i32 };
            (15, idx, -1)
        }
        Some(ActionDesc::LevelUp { index }) => (16, *index as i32, -1),
        Some(ActionDesc::EncorePay { slot }) => (17, *slot as i32, -1),
        Some(ActionDesc::EncoreDecline { slot }) => (22, *slot as i32, -1),
        Some(ActionDesc::TriggerOrder { index }) => (18, *index as i32, -1),
        Some(ActionDesc::ChoiceSelect { index }) => {
            let idx = if mask { -1 } else { *index as i32 };
            (19, idx, -1)
        }
        Some(ActionDesc::ChoicePrevPage) => (20, -1, -1),
        Some(ActionDesc::ChoiceNextPage) => (21, -1, -1),
        Some(ActionDesc::Concede) => (23, -1, -1),
    }
}

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
        ActionDesc::MainActivateAbility {
            slot,
            ability_index,
        } => {
            let _ = (slot, ability_index);
            None
        }
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
        ActionDesc::CounterPlay { hand_index } => {
            let _ = hand_index;
            None
        }
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

pub fn fill_action_mask(
    actions: &[ActionDesc],
    mask: &mut [u8],
    lookup: &mut [Option<ActionDesc>],
) {
    mask.fill(0);
    for slot in lookup.iter_mut() {
        *slot = None;
    }
    for action in actions {
        if let Some(id) = action_id_for(action) {
            if id < ACTION_SPACE_SIZE {
                mask[id] = 1;
                lookup[id] = Some(action.clone());
            }
        }
    }
}

pub fn fill_action_mask_sparse(
    actions: &[ActionDesc],
    mask: &mut [u8],
    last_action_ids: &mut Vec<u16>,
    mask_bits: &mut [u64],
    write_mask: bool,
) {
    for &id_u16 in last_action_ids.iter() {
        let id = id_u16 as usize;
        if id < ACTION_SPACE_SIZE {
            if write_mask {
                mask[id] = 0;
            }
            let word = id / 64;
            let bit = id % 64;
            if word < mask_bits.len() {
                mask_bits[word] &= !(1u64 << bit);
            }
        }
    }
    last_action_ids.clear();
    for (idx, action) in actions.iter().enumerate() {
        if idx > u16::MAX as usize {
            debug_assert!(false, "legal action count exceeds u16::MAX, cannot index");
            break;
        }
        if let Some(id) = action_id_for(action) {
            if id < ACTION_SPACE_SIZE {
                if write_mask {
                    mask[id] = 1;
                }
                if let Ok(id_u16) = u16::try_from(id) {
                    last_action_ids.push(id_u16);
                }
                let word = id / 64;
                let bit = id % 64;
                if word < mask_bits.len() {
                    mask_bits[word] |= 1u64 << bit;
                }
            }
        }
    }
}

pub fn build_action_mask(actions: &[ActionDesc]) -> (Vec<u8>, Vec<Option<ActionDesc>>) {
    let mut mask = vec![0u8; ACTION_SPACE_SIZE];
    let mut lookup = vec![None; ACTION_SPACE_SIZE];
    fill_action_mask(actions, &mut mask, &mut lookup);
    (mask, lookup)
}

#[cfg(test)]
mod tests {
    use super::*;

    const OBS_SPEC_HASH: u64 = 4199711457902658906;
    const ACTION_SPEC_HASH: u64 = 10374799669423425379;

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
                    to_slot: 2,
                },
                ActionIdDesc {
                    family: "main_move",
                    params: vec![
                        param("from_slot", ActionParamValue::Int(0)),
                        param("to_slot", ActionParamValue::Int(2)),
                    ],
                },
            ),
            (
                ActionDesc::ClimaxPlay { hand_index: 0 },
                ActionIdDesc {
                    family: "climax_play",
                    params: vec![param("hand_index", ActionParamValue::Int(0))],
                },
            ),
            (
                ActionDesc::Attack {
                    slot: 1,
                    attack_type: AttackType::Side,
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
                ActionDesc::EncoreDecline { slot: 1 },
                ActionIdDesc {
                    family: "encore_decline",
                    params: vec![param("slot", ActionParamValue::Int(1))],
                },
            ),
            (
                ActionDesc::TriggerOrder { index: 4 },
                ActionIdDesc {
                    family: "trigger_order",
                    params: vec![param("index", ActionParamValue::Int(4))],
                },
            ),
            (
                ActionDesc::ChoiceSelect { index: 5 },
                ActionIdDesc {
                    family: "choice_select",
                    params: vec![param("index", ActionParamValue::Int(5))],
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
            let id = action_id_for(&action).expect("action id");
            let decoded = decode_action_id(id).expect("decoded action id");
            assert_eq!(decoded, expected);
        }
    }
}
