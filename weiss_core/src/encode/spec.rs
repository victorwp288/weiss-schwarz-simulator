use serde::Serialize;

use super::constants::*;

/// Single observation field specification.
#[derive(Clone, Debug, Serialize)]
pub struct ObsFieldSpec {
    /// Field name.
    pub name: &'static str,
    /// Index into the observation vector.
    pub index: usize,
    /// Visibility label (public/private).
    pub visibility: &'static str,
    /// Human-readable description.
    pub description: &'static str,
}

/// Slice specification for contiguous observation segments.
#[derive(Clone, Debug, Serialize)]
pub struct ObsSliceSpec {
    /// Slice name.
    pub name: &'static str,
    /// Start index in the observation vector.
    pub start: usize,
    /// Slice length.
    pub len: usize,
    /// Visibility label (public/private).
    pub visibility: &'static str,
    /// Human-readable description.
    pub description: &'static str,
}

/// Per-player observation block specification.
#[derive(Clone, Debug, Serialize)]
pub struct PlayerBlockSpec {
    /// Player index (0 or 1).
    pub player_index: u8,
    /// Base index for the block.
    pub base: usize,
    /// Block length.
    pub len: usize,
    /// Slices inside the block.
    pub slices: Vec<ObsSliceSpec>,
}

/// Full observation specification.
#[derive(Clone, Debug, Serialize)]
pub struct ObservationSpec {
    /// Observation encoding version.
    pub obs_encoding_version: u32,
    /// Observation vector length.
    pub obs_len: usize,
    /// Data type for observation values.
    pub dtype: &'static str,
    /// Whether the current player is encoded first.
    pub self_first: bool,
    /// Sentinel value for hidden cards.
    pub sentinel_hidden: i32,
    /// Sentinel value for empty slots.
    pub sentinel_empty_card: i32,
    /// Header fields.
    pub header_fields: Vec<ObsFieldSpec>,
    /// Per-player blocks.
    pub player_blocks: Vec<PlayerBlockSpec>,
    /// Tail slices after player blocks.
    pub tail_slices: Vec<ObsSliceSpec>,
    /// Additional notes.
    pub notes: Vec<&'static str>,
}

/// Action family specification.
#[derive(Clone, Debug, Serialize)]
pub struct ActionFamilySpec {
    /// Family name.
    pub name: &'static str,
    /// Base id for this family.
    pub base: usize,
    /// Number of actions in this family.
    pub count: usize,
    /// Parameter names used by the family.
    pub params: Vec<&'static str>,
    /// Human-readable description.
    pub description: &'static str,
}

/// Full action specification.
#[derive(Clone, Debug, Serialize)]
pub struct ActionSpec {
    /// Action encoding version.
    pub action_encoding_version: u32,
    /// Total action space size.
    pub action_space_size: usize,
    /// Id of the pass action.
    pub pass_action_id: usize,
    /// Encoding of attack types.
    pub attack_type_encoding: Vec<(&'static str, i32)>,
    /// Named constants included in the spec.
    pub constants: Vec<(&'static str, usize)>,
    /// Action families.
    pub families: Vec<ActionFamilySpec>,
    /// Additional notes.
    pub notes: Vec<&'static str>,
}

/// Build the observation specification.
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
            description: "terminal status encoding",
        },
        ObsFieldSpec {
            name: "last_action_kind",
            index: 5,
            visibility: "public",
            description: "last action encoding",
        },
        ObsFieldSpec {
            name: "last_action_arg0",
            index: 6,
            visibility: "public",
            description: "last action arg0",
        },
        ObsFieldSpec {
            name: "last_action_arg1",
            index: 7,
            visibility: "public",
            description: "last action arg1",
        },
        ObsFieldSpec {
            name: "attack_slot",
            index: 8,
            visibility: "public",
            description: "attacker slot if in attack",
        },
        ObsFieldSpec {
            name: "defender_slot",
            index: 9,
            visibility: "public",
            description: "defender slot if in attack",
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
            description: "current attack damage",
        },
        ObsFieldSpec {
            name: "attack_counter_power",
            index: 12,
            visibility: "public",
            description: "current counter power",
        },
        ObsFieldSpec {
            name: "focus_slot",
            index: 13,
            visibility: "public",
            description: "focused slot for some decisions",
        },
        ObsFieldSpec {
            name: "choice_page_start",
            index: 14,
            visibility: "public",
            description: "choice page start index",
        },
        ObsFieldSpec {
            name: "choice_total",
            index: 15,
            visibility: "public",
            description: "choice total candidates",
        },
    ];

    let counts = vec![
        ObsSliceSpec {
            name: "level_count",
            start: 0,
            len: 1,
            visibility: "public",
            description: "level count",
        },
        ObsSliceSpec {
            name: "clock_count",
            start: 1,
            len: 1,
            visibility: "public",
            description: "clock count",
        },
        ObsSliceSpec {
            name: "deck_count",
            start: 2,
            len: 1,
            visibility: "public",
            description: "deck count",
        },
        ObsSliceSpec {
            name: "hand_count",
            start: 3,
            len: 1,
            visibility: "private",
            description:
                "hand count (private by default; visible in full mode or when reveal_opponent_hand_stock_counts is enabled)",
        },
        ObsSliceSpec {
            name: "stock_count",
            start: 4,
            len: 1,
            visibility: "private",
            description:
                "stock count (private by default; visible in full mode or when reveal_opponent_hand_stock_counts is enabled)",
        },
        ObsSliceSpec {
            name: "waiting_room_count",
            start: 5,
            len: 1,
            visibility: "public",
            description: "waiting room count",
        },
        ObsSliceSpec {
            name: "memory_count",
            start: 6,
            len: 1,
            visibility: "private",
            description: "memory count (private unless full visibility)",
        },
        ObsSliceSpec {
            name: "climax_count",
            start: 7,
            len: 1,
            visibility: "public",
            description: "climax count",
        },
        ObsSliceSpec {
            name: "resolution_count",
            start: 8,
            len: 1,
            visibility: "public",
            description: "resolution count",
        },
    ];

    let stage = ObsSliceSpec {
        name: "stage",
        start: PER_PLAYER_COUNTS,
        len: PER_PLAYER_STAGE,
        visibility: "public",
        description:
            "stage slots (card id, status, has_attacked, power, base soul, effective soul, side-attack-allowed)",
    };

    let climax = ObsSliceSpec {
        name: "climax_top",
        start: PER_PLAYER_COUNTS + PER_PLAYER_STAGE,
        len: PER_PLAYER_CLIMAX_TOP,
        visibility: "public",
        description: "top climax card id",
    };

    let level = ObsSliceSpec {
        name: "level_top",
        start: PER_PLAYER_COUNTS + PER_PLAYER_STAGE + PER_PLAYER_CLIMAX_TOP,
        len: PER_PLAYER_LEVEL,
        visibility: "public",
        description: "top level cards (chronological)",
    };

    let clock = ObsSliceSpec {
        name: "clock_top",
        start: PER_PLAYER_COUNTS + PER_PLAYER_STAGE + PER_PLAYER_CLIMAX_TOP + PER_PLAYER_LEVEL,
        len: PER_PLAYER_CLOCK_TOP,
        visibility: "public",
        description: "top clock cards",
    };

    let waiting_room = ObsSliceSpec {
        name: "waiting_room_top",
        start: PER_PLAYER_COUNTS
            + PER_PLAYER_STAGE
            + PER_PLAYER_CLIMAX_TOP
            + PER_PLAYER_LEVEL
            + PER_PLAYER_CLOCK_TOP,
        len: PER_PLAYER_WAITING_TOP,
        visibility: "public",
        description: "top waiting room cards",
    };

    let resolution = ObsSliceSpec {
        name: "resolution_top",
        start: PER_PLAYER_COUNTS
            + PER_PLAYER_STAGE
            + PER_PLAYER_CLIMAX_TOP
            + PER_PLAYER_LEVEL
            + PER_PLAYER_CLOCK_TOP
            + PER_PLAYER_WAITING_TOP,
        len: PER_PLAYER_RESOLUTION_TOP,
        visibility: "public",
        description: "top resolution cards",
    };

    let stock = ObsSliceSpec {
        name: "stock_top",
        start: PER_PLAYER_COUNTS
            + PER_PLAYER_STAGE
            + PER_PLAYER_CLIMAX_TOP
            + PER_PLAYER_LEVEL
            + PER_PLAYER_CLOCK_TOP
            + PER_PLAYER_WAITING_TOP
            + PER_PLAYER_RESOLUTION_TOP,
        len: PER_PLAYER_STOCK_TOP,
        visibility: "private",
        description: "top stock cards",
    };

    let hand = ObsSliceSpec {
        name: "hand",
        start: PER_PLAYER_COUNTS
            + PER_PLAYER_STAGE
            + PER_PLAYER_CLIMAX_TOP
            + PER_PLAYER_LEVEL
            + PER_PLAYER_CLOCK_TOP
            + PER_PLAYER_WAITING_TOP
            + PER_PLAYER_RESOLUTION_TOP
            + PER_PLAYER_STOCK_TOP,
        len: PER_PLAYER_HAND,
        visibility: "private",
        description: "hand cards",
    };

    let deck = ObsSliceSpec {
        name: "deck",
        start: PER_PLAYER_COUNTS
            + PER_PLAYER_STAGE
            + PER_PLAYER_CLIMAX_TOP
            + PER_PLAYER_LEVEL
            + PER_PLAYER_CLOCK_TOP
            + PER_PLAYER_WAITING_TOP
            + PER_PLAYER_RESOLUTION_TOP
            + PER_PLAYER_STOCK_TOP
            + PER_PLAYER_HAND,
        len: PER_PLAYER_DECK,
        visibility: "private",
        description: "deck cards",
    };

    let mut self_slices = counts.clone();
    self_slices.push(stage.clone());
    self_slices.push(climax.clone());
    self_slices.push(level.clone());
    self_slices.push(clock.clone());
    self_slices.push(waiting_room.clone());
    self_slices.push(resolution.clone());
    self_slices.push(stock.clone());
    self_slices.push(hand.clone());
    self_slices.push(deck.clone());

    let player_blocks = vec![
        PlayerBlockSpec {
            player_index: 0,
            base: OBS_HEADER_LEN,
            len: PER_PLAYER_BLOCK_LEN,
            slices: self_slices.clone(),
        },
        PlayerBlockSpec {
            player_index: 1,
            base: OBS_HEADER_LEN + PER_PLAYER_BLOCK_LEN,
            len: PER_PLAYER_BLOCK_LEN,
            slices: self_slices,
        },
    ];

    let tail_slices = vec![
        ObsSliceSpec {
            name: "reason",
            start: OBS_REASON_BASE,
            len: OBS_REASON_LEN,
            visibility: "public",
            description: "reason bits",
        },
        ObsSliceSpec {
            name: "reveal",
            start: OBS_REVEAL_BASE,
            len: OBS_REVEAL_LEN,
            visibility: "public",
            description: "recent reveal history",
        },
        ObsSliceSpec {
            name: "context",
            start: OBS_CONTEXT_BASE,
            len: OBS_CONTEXT_LEN,
            visibility: "public",
            description: "context bits",
        },
    ];

    ObservationSpec {
        obs_encoding_version: OBS_ENCODING_VERSION,
        obs_len: OBS_LEN,
        dtype: "i32",
        self_first: true,
        sentinel_hidden: -1,
        sentinel_empty_card: 0,
        header_fields,
        player_blocks,
        tail_slices,
        notes: vec![
            "Player blocks are ordered perspective, opponent.",
            "Hidden zones are masked by sentinel_hidden.",
        ],
    }
}

/// Serialize the observation specification as JSON.
pub fn observation_spec_json() -> String {
    serde_json::to_string_pretty(&observation_spec()).unwrap_or_else(|_| "{}".to_string())
}

/// Build the action specification.
pub fn action_spec() -> ActionSpec {
    ActionSpec {
        action_encoding_version: ACTION_ENCODING_VERSION,
        action_space_size: ACTION_SPACE_SIZE,
        pass_action_id: PASS_ACTION_ID,
        attack_type_encoding: vec![("frontal", 0), ("side", 1), ("direct", 2)],
        constants: vec![
            ("MAX_HAND", MAX_HAND),
            ("MAX_STAGE", MAX_STAGE),
            ("MAX_LEVEL", MAX_LEVEL),
            ("ATTACK_SLOT_COUNT", ATTACK_SLOT_COUNT),
            ("MAX_ABILITIES_PER_CARD", MAX_ABILITIES_PER_CARD),
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
                description: "select card in hand for mulligan",
            },
            ActionFamilySpec {
                name: "pass",
                base: PASS_ACTION_ID,
                count: 1,
                params: vec![],
                description: "pass action",
            },
            ActionFamilySpec {
                name: "clock_from_hand",
                base: CLOCK_HAND_BASE,
                count: CLOCK_HAND_COUNT,
                params: vec!["hand_index"],
                description: "clock card from hand",
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
                description: "play event from hand",
            },
            ActionFamilySpec {
                name: "main_move",
                base: MAIN_MOVE_BASE,
                count: MAIN_MOVE_COUNT,
                params: vec!["from_slot", "to_slot"],
                description: "move stage slot",
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
                description: "choose card for level up",
            },
            ActionFamilySpec {
                name: "encore_pay",
                base: ENCORE_PAY_BASE,
                count: ENCORE_PAY_COUNT,
                params: vec!["slot"],
                description: "pay encore for a slot",
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

/// Serialize the action specification as JSON.
pub fn action_spec_json() -> String {
    serde_json::to_string_pretty(&action_spec()).unwrap_or_else(|_| "{}".to_string())
}
