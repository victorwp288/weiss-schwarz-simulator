//! Encoding constants for observations and action ids.
//!
//! These values are part of the stable RL contract. See docs/rl_contract.md.

use crate::state::REVEAL_HISTORY_LEN;

/// Observation encoding version.
/// Contract: <https://github.com/victorwp288/weiss-schwarz-simulator/blob/main/docs/rl_contract.md>
pub const OBS_ENCODING_VERSION: u32 = 2;
/// Action encoding version.
/// Contract: <https://github.com/victorwp288/weiss-schwarz-simulator/blob/main/docs/rl_contract.md>
pub const ACTION_ENCODING_VERSION: u32 = 1;
/// Policy version used in spec hash composition.
pub const POLICY_VERSION: u32 = 2;
/// Combined encoding spec hash.
pub const SPEC_HASH: u64 = ((OBS_ENCODING_VERSION as u64) << 32)
    | ((ACTION_ENCODING_VERSION as u64) << 16)
    | (POLICY_VERSION as u64);

/// Sentinel for missing actor.
pub const ACTOR_NONE: i8 = -1;
/// Sentinel for no decision.
pub const DECISION_KIND_NONE: i8 = -1;

/// Maximum hand size encoded in observations.
pub const MAX_HAND: usize = 50;
/// Deck size per player.
pub const MAX_DECK: usize = 50;
/// Number of stage slots per player.
pub const MAX_STAGE: usize = 5;
/// Maximum number of abilities encoded per card (padding beyond this is ignored).
pub const MAX_ABILITIES_PER_CARD: usize = 4;
/// Number of attack slots encoded/considered (front row).
pub const ATTACK_SLOT_COUNT: usize = 3;
/// Maximum number of cards encoded in the level zone.
pub const MAX_LEVEL: usize = 4;
/// Number of top cards encoded from the clock zone.
pub const TOP_CLOCK: usize = 7;
/// Number of top cards encoded from the waiting room zone.
pub const TOP_WAITING_ROOM: usize = 5;
/// Number of top cards encoded from the stock zone.
pub const TOP_STOCK: usize = 5;
/// Number of top cards encoded from the resolution zone.
pub const TOP_RESOLUTION: usize = 5;

/// Action id for "confirm mulligan".
pub const MULLIGAN_CONFIRM_ID: usize = 0;
/// Base action id for "mulligan select" (hand index offset).
pub const MULLIGAN_SELECT_BASE: usize = MULLIGAN_CONFIRM_ID + 1;
/// Number of mulligan select actions (one per possible hand index).
pub const MULLIGAN_SELECT_COUNT: usize = MAX_HAND;

/// Action id for "pass".
pub const PASS_ACTION_ID: usize = MULLIGAN_SELECT_BASE + MULLIGAN_SELECT_COUNT;
/// Base action id for "clock from hand" (hand index offset).
pub const CLOCK_HAND_BASE: usize = PASS_ACTION_ID + 1;
/// Number of clock-from-hand actions (one per possible hand index).
pub const CLOCK_HAND_COUNT: usize = MAX_HAND;

/// Base action id for "main: play character" (hand index and slot offsets).
pub const MAIN_PLAY_CHAR_BASE: usize = CLOCK_HAND_BASE + CLOCK_HAND_COUNT;
/// Number of main play character actions.
pub const MAIN_PLAY_CHAR_COUNT: usize = MAX_HAND * MAX_STAGE;
/// Base action id for "main: play event" (hand index offset).
pub const MAIN_PLAY_EVENT_BASE: usize = MAIN_PLAY_CHAR_BASE + MAIN_PLAY_CHAR_COUNT;
/// Number of main play event actions.
pub const MAIN_PLAY_EVENT_COUNT: usize = MAX_HAND;
/// Base action id for "main: move" (from/to slot offsets).
pub const MAIN_MOVE_BASE: usize = MAIN_PLAY_EVENT_BASE + MAIN_PLAY_EVENT_COUNT;
/// Number of main move actions.
pub const MAIN_MOVE_COUNT: usize = MAX_STAGE * (MAX_STAGE - 1);

/// Base action id for "climax: play climax" (hand index offset).
pub const CLIMAX_PLAY_BASE: usize = MAIN_MOVE_BASE + MAIN_MOVE_COUNT;
/// Number of climax play actions.
pub const CLIMAX_PLAY_COUNT: usize = MAX_HAND;

/// Base action id for "attack" (slot and attack-type offsets).
pub const ATTACK_BASE: usize = CLIMAX_PLAY_BASE + CLIMAX_PLAY_COUNT;
/// Number of attack actions.
pub const ATTACK_COUNT: usize = ATTACK_SLOT_COUNT * 3;

/// Base action id for "level up" (index offset).
pub const LEVEL_UP_BASE: usize = ATTACK_BASE + ATTACK_COUNT;
/// Number of level-up actions.
pub const LEVEL_UP_COUNT: usize = 7;

/// Base action id for "encore: pay" (slot offset).
pub const ENCORE_PAY_BASE: usize = LEVEL_UP_BASE + LEVEL_UP_COUNT;
/// Number of encore pay actions.
pub const ENCORE_PAY_COUNT: usize = MAX_STAGE;
/// Base action id for "encore: decline" (slot offset).
pub const ENCORE_DECLINE_BASE: usize = ENCORE_PAY_BASE + ENCORE_PAY_COUNT;
/// Number of encore decline actions.
pub const ENCORE_DECLINE_COUNT: usize = MAX_STAGE;

/// Base action id for "trigger order" (index offset).
pub const TRIGGER_ORDER_BASE: usize = ENCORE_DECLINE_BASE + ENCORE_DECLINE_COUNT;
/// Number of trigger-order actions.
pub const TRIGGER_ORDER_COUNT: usize = 10;

/// Base action id for "choice select" (index offset).
pub const CHOICE_BASE: usize = TRIGGER_ORDER_BASE + TRIGGER_ORDER_COUNT;
/// Number of choice select actions per page.
pub const CHOICE_COUNT: usize = 16;
/// Action id for "previous choice page".
pub const CHOICE_PREV_ID: usize = CHOICE_BASE + CHOICE_COUNT;
/// Action id for "next choice page".
pub const CHOICE_NEXT_ID: usize = CHOICE_PREV_ID + 1;

/// Action id for "concede".
pub const CONCEDE_ID: usize = CHOICE_NEXT_ID + 1;
/// Total action space size.
pub const ACTION_SPACE_SIZE: usize = CONCEDE_ID + 1;
// Action ids are tracked as u16 in sparse mask caches.
const _: [(); 1] = [(); (ACTION_SPACE_SIZE <= u16::MAX as usize) as usize];
/// Number of u64 words required to represent the action mask.
pub const ACTION_SPACE_WORDS: usize = ACTION_SPACE_SIZE.div_ceil(64);

/// Observation header length.
pub const OBS_HEADER_LEN: usize = 16;
/// Length of the per-observation "reason" slice.
pub const OBS_REASON_LEN: usize = 8;
/// Reason bit/index: in main phase.
pub const OBS_REASON_IN_MAIN: usize = 0;
/// Reason bit/index: in climax phase.
pub const OBS_REASON_IN_CLIMAX: usize = 1;
/// Reason bit/index: in attack phase.
pub const OBS_REASON_IN_ATTACK: usize = 2;
/// Reason bit/index: in counter window.
pub const OBS_REASON_IN_COUNTER_WINDOW: usize = 3;
/// Reason bit/index: no stock available.
pub const OBS_REASON_NO_STOCK: usize = 4;
/// Reason bit/index: missing color requirement.
pub const OBS_REASON_NO_COLOR: usize = 5;
/// Reason bit/index: no cards in hand.
pub const OBS_REASON_NO_HAND: usize = 6;
/// Reason bit/index: no valid targets.
pub const OBS_REASON_NO_TARGETS: usize = 7;
/// Length of the "reveal history" slice.
pub const OBS_REVEAL_LEN: usize = REVEAL_HISTORY_LEN;
/// Length of the "context" slice.
pub const OBS_CONTEXT_LEN: usize = 4;
/// Context bit/index: priority window active.
pub const OBS_CONTEXT_PRIORITY_WINDOW: usize = 0;
/// Context bit/index: choice selection active.
pub const OBS_CONTEXT_CHOICE_ACTIVE: usize = 1;
/// Context bit/index: stack is non-empty.
pub const OBS_CONTEXT_STACK_NONEMPTY: usize = 2;
/// Context bit/index: encore is pending.
pub const OBS_CONTEXT_ENCORE_PENDING: usize = 3;
/// Number of scalar count slots per player block.
pub const PER_PLAYER_COUNTS: usize = 9;
/// Scalars encoded per stage slot.
pub const PER_STAGE_SLOT: usize = 7;
/// Total per-player stage slice length.
pub const PER_PLAYER_STAGE: usize = MAX_STAGE * PER_STAGE_SLOT;
/// Cards encoded from the climax zone top.
pub const PER_PLAYER_CLIMAX_TOP: usize = 1;
/// Cards encoded from the level zone.
pub const PER_PLAYER_LEVEL: usize = MAX_LEVEL;
/// Cards encoded from the clock zone top.
pub const PER_PLAYER_CLOCK_TOP: usize = TOP_CLOCK;
/// Cards encoded from the waiting room top.
pub const PER_PLAYER_WAITING_TOP: usize = TOP_WAITING_ROOM;
/// Cards encoded from the resolution top.
pub const PER_PLAYER_RESOLUTION_TOP: usize = TOP_RESOLUTION;
/// Cards encoded from the stock top.
pub const PER_PLAYER_STOCK_TOP: usize = TOP_STOCK;
/// Cards encoded from the hand.
pub const PER_PLAYER_HAND: usize = MAX_HAND;
/// Cards encoded from the deck.
pub const PER_PLAYER_DECK: usize = MAX_DECK;
/// Per-player observation block length.
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
/// Base index of the observation reason slice.
pub const OBS_REASON_BASE: usize = OBS_HEADER_LEN + 2 * PER_PLAYER_BLOCK_LEN;
/// Base index of the reveal history slice.
pub const OBS_REVEAL_BASE: usize = OBS_REASON_BASE + OBS_REASON_LEN;
/// Base index of the context slice.
pub const OBS_CONTEXT_BASE: usize = OBS_REVEAL_BASE + OBS_REVEAL_LEN;
/// Total observation vector length.
pub const OBS_LEN: usize = OBS_CONTEXT_BASE + OBS_CONTEXT_LEN;
