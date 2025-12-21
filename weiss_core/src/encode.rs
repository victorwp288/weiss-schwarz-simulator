use crate::config::{CurriculumConfig, ObservationVisibility};
use crate::db::CardDb;
use crate::legal::{ActionDesc, Decision, DecisionKind};
use crate::state::{AttackType, GameState, ModifierKind, Phase, StageStatus, TerminalResult};

pub const OBS_ENCODING_VERSION: u32 = 3;
pub const ACTION_ENCODING_VERSION: u32 = 3;

pub const MAX_HAND: usize = 10;
pub const MAX_DECK: usize = 60;
pub const MAX_STAGE: usize = 5;
pub const MAX_ABILITIES_PER_CARD: usize = 4;
pub const MAX_LEVEL: usize = 4;
pub const TOP_CLOCK: usize = 7;
pub const TOP_WAITING_ROOM: usize = 5;
pub const TOP_STOCK: usize = 5;

pub const CLOCK_PASS_ID: usize = 2;
pub const CLOCK_HAND_BASE: usize = CLOCK_PASS_ID + 1;
pub const CLOCK_HAND_COUNT: usize = MAX_HAND;

pub const MAIN_PASS_ID: usize = CLOCK_HAND_BASE + CLOCK_HAND_COUNT;
pub const MAIN_PLAY_CHAR_BASE: usize = MAIN_PASS_ID + 1;
pub const MAIN_PLAY_CHAR_COUNT: usize = MAX_HAND * MAX_STAGE;
pub const MAIN_PLAY_EVENT_BASE: usize = MAIN_PLAY_CHAR_BASE + MAIN_PLAY_CHAR_COUNT;
pub const MAIN_PLAY_EVENT_COUNT: usize = MAX_HAND;
pub const MAIN_MOVE_BASE: usize = MAIN_PLAY_EVENT_BASE + MAIN_PLAY_EVENT_COUNT;
pub const MAIN_MOVE_COUNT: usize = MAX_STAGE * MAX_STAGE;
pub const MAIN_ACTIVATE_BASE: usize = MAIN_MOVE_BASE + MAIN_MOVE_COUNT;
pub const MAIN_ACTIVATE_COUNT: usize = MAX_STAGE * MAX_ABILITIES_PER_CARD;

pub const CLIMAX_PASS_ID: usize = MAIN_ACTIVATE_BASE + MAIN_ACTIVATE_COUNT;
pub const CLIMAX_PLAY_BASE: usize = CLIMAX_PASS_ID + 1;
pub const CLIMAX_PLAY_COUNT: usize = MAX_HAND;

pub const ATTACK_PASS_ID: usize = CLIMAX_PLAY_BASE + CLIMAX_PLAY_COUNT;
pub const ATTACK_BASE: usize = ATTACK_PASS_ID + 1;
pub const ATTACK_COUNT: usize = MAX_STAGE * 3;

pub const COUNTER_PASS_ID: usize = ATTACK_BASE + ATTACK_COUNT;
pub const COUNTER_PLAY_BASE: usize = COUNTER_PASS_ID + 1;
pub const COUNTER_PLAY_COUNT: usize = MAX_HAND;

pub const LEVEL_UP_BASE: usize = COUNTER_PLAY_BASE + COUNTER_PLAY_COUNT;
pub const LEVEL_UP_COUNT: usize = 7;

pub const ENCORE_YES_ID: usize = LEVEL_UP_BASE + LEVEL_UP_COUNT;
pub const ENCORE_NO_ID: usize = ENCORE_YES_ID + 1;

pub const TRIGGER_ORDER_BASE: usize = ENCORE_NO_ID + 1;
pub const TRIGGER_ORDER_COUNT: usize = 10;

pub const CHOICE_BASE: usize = TRIGGER_ORDER_BASE + TRIGGER_ORDER_COUNT;
pub const CHOICE_COUNT: usize = 16;

pub const ACTION_SPACE_SIZE: usize = CHOICE_BASE + CHOICE_COUNT;

pub const OBS_HEADER_LEN: usize = 14;
pub const PER_PLAYER_COUNTS: usize = 8;
pub const PER_STAGE_SLOT: usize = 5;
pub const PER_PLAYER_STAGE: usize = MAX_STAGE * PER_STAGE_SLOT;
pub const PER_PLAYER_CLIMAX_TOP: usize = 1;
pub const PER_PLAYER_LEVEL: usize = MAX_LEVEL;
pub const PER_PLAYER_CLOCK_TOP: usize = TOP_CLOCK;
pub const PER_PLAYER_WAITING_TOP: usize = TOP_WAITING_ROOM;
pub const PER_PLAYER_STOCK_TOP: usize = TOP_STOCK;
pub const PER_PLAYER_HAND: usize = MAX_HAND;
pub const PER_PLAYER_DECK: usize = MAX_DECK;
pub const PER_PLAYER_BLOCK_LEN: usize = PER_PLAYER_COUNTS
    + PER_PLAYER_STAGE
    + PER_PLAYER_CLIMAX_TOP
    + PER_PLAYER_LEVEL
    + PER_PLAYER_CLOCK_TOP
    + PER_PLAYER_WAITING_TOP
    + PER_PLAYER_STOCK_TOP
    + PER_PLAYER_HAND
    + PER_PLAYER_DECK;
pub const OBS_LEN: usize = OBS_HEADER_LEN + 2 * PER_PLAYER_BLOCK_LEN;

#[allow(clippy::too_many_arguments)]
pub fn encode_observation(
    state: &GameState,
    db: &CardDb,
    _curriculum: &CurriculumConfig,
    perspective: u8,
    decision: Option<&Decision>,
    last_action: Option<&ActionDesc>,
    visibility: ObservationVisibility,
    out: &mut [i32],
) {
    assert!(out.len() >= OBS_LEN);
    out.fill(0);
    let p0 = perspective as usize;
    let p1 = 1 - p0;
    out[0] = state.turn.active_player as i32;
    out[1] = phase_to_i32(state.turn.phase);
    out[2] = decision_kind_to_i32(decision.map(|d| d.kind));
    out[3] = decision.map(|d| d.player as i32).unwrap_or(-1);
    out[4] = terminal_to_i32(state.terminal);
    let (last_kind, last_p1, last_p2) = last_action_to_fields(last_action);
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
    out[13] = decision.and_then(|d| d.focus_slot.map(|s| s as i32)).unwrap_or(-1);

    let mut offset = OBS_HEADER_LEN;
    for (idx, player_index) in [p0, p1].iter().enumerate() {
        let p = &state.players[*player_index];
        out[offset] = p.level.len() as i32;
        out[offset + 1] = p.clock.len() as i32;
        out[offset + 2] = p.deck.len() as i32;
        out[offset + 3] = p.hand.len() as i32;
        out[offset + 4] = p.stock.len() as i32;
        out[offset + 5] = p.waiting_room.len() as i32;
        out[offset + 6] = p.memory.len() as i32;
        out[offset + 7] = p.climax.len() as i32;
        offset += PER_PLAYER_COUNTS;

        for slot in 0..MAX_STAGE {
            let slot_state = &p.stage[slot];
            let card_id = slot_state.card.map(|c| c.id).unwrap_or(0) as i32;
            let status = if slot_state.card.is_some() {
                status_to_i32(slot_state.status)
            } else {
                0
            };
            let has_attacked = if slot_state.has_attacked { 1 } else { 0 };
            let (power, soul) = if let Some(card) = slot_state.card.and_then(|inst| db.get(inst.id)) {
                let mut power = card.power + slot_state.power_mod_turn + slot_state.power_mod_battle;
                for modifier in &state.modifiers {
                    if modifier.kind != ModifierKind::Power {
                        continue;
                    }
                    if modifier.target_player as usize != *player_index || modifier.target_slot as usize != slot {
                        continue;
                    }
                    if modifier.target_card != slot_state.card.map(|c| c.id).unwrap_or(0) {
                        continue;
                    }
                    power += modifier.magnitude;
                }
                let soul = card.soul as i32;
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

        out[offset] = p.climax.last().map(|c| c.id).unwrap_or(0) as i32;
        offset += PER_PLAYER_CLIMAX_TOP;

        for i in 0..MAX_LEVEL {
            out[offset + i] = p.level.get(i).map(|c| c.id).unwrap_or(0) as i32;
        }
        offset += PER_PLAYER_LEVEL;

        for i in 0..TOP_CLOCK {
            let idx = p.clock.len().saturating_sub(1 + i);
            let value = if idx < p.clock.len() { p.clock[idx].id as i32 } else { 0 };
            out[offset + i] = value;
        }
        offset += PER_PLAYER_CLOCK_TOP;

        for i in 0..TOP_WAITING_ROOM {
            let idx = p.waiting_room.len().saturating_sub(1 + i);
            let value = if idx < p.waiting_room.len() { p.waiting_room[idx].id as i32 } else { 0 };
            out[offset + i] = value;
        }
        offset += PER_PLAYER_WAITING_TOP;

        for i in 0..TOP_STOCK {
            let value = if visibility == ObservationVisibility::Full {
                let idx = p.stock.len().saturating_sub(1 + i);
                if idx < p.stock.len() { p.stock[idx].id as i32 } else { 0 }
            } else {
                -1
            };
            out[offset + i] = value;
        }
        offset += PER_PLAYER_STOCK_TOP;

        for i in 0..MAX_HAND {
            let value = if visibility == ObservationVisibility::Full || idx == 0 {
                p.hand.get(i).map(|c| c.id).unwrap_or(0) as i32
            } else {
                -1
            };
            out[offset + i] = value;
        }
        offset += MAX_HAND;

        for i in 0..MAX_DECK {
            let value = if visibility == ObservationVisibility::Full {
                if i < p.deck.len() {
                    let deck_idx = p.deck.len() - 1 - i;
                    p.deck[deck_idx].id as i32
                } else {
                    0
                }
            } else {
                -1
            };
            out[offset + i] = value;
        }
        offset += MAX_DECK;
    }
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
        Some(DecisionKind::Counter) => 5,
        Some(DecisionKind::LevelUp) => 6,
        Some(DecisionKind::Encore) => 7,
        Some(DecisionKind::TriggerOrder) => 8,
        Some(DecisionKind::Choice) => 9,
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
        Some(TerminalResult::Win { winner }) => if winner == 0 { 1 } else { 2 },
        Some(TerminalResult::Draw) => 3,
        Some(TerminalResult::Timeout) => 4,
    }
}

fn last_action_to_fields(action: Option<&ActionDesc>) -> (i32, i32, i32) {
    match action {
        None => (0, -1, -1),
        Some(ActionDesc::MulliganKeep) => (1, -1, -1),
        Some(ActionDesc::MulliganAll) => (2, -1, -1),
        Some(ActionDesc::ClockPass) => (3, -1, -1),
        Some(ActionDesc::Clock { hand_index }) => (4, *hand_index as i32, -1),
        Some(ActionDesc::MainPass) => (5, -1, -1),
        Some(ActionDesc::MainPlayCharacter { hand_index, stage_slot }) => (6, *hand_index as i32, *stage_slot as i32),
        Some(ActionDesc::MainPlayEvent { hand_index }) => (7, *hand_index as i32, -1),
        Some(ActionDesc::MainMove { from_slot, to_slot }) => (8, *from_slot as i32, *to_slot as i32),
        Some(ActionDesc::MainActivateAbility { slot, ability_index }) => (9, *slot as i32, *ability_index as i32),
        Some(ActionDesc::ClimaxPass) => (10, -1, -1),
        Some(ActionDesc::ClimaxPlay { hand_index }) => (11, *hand_index as i32, -1),
        Some(ActionDesc::AttackPass) => (12, -1, -1),
        Some(ActionDesc::Attack { slot, attack_type }) => (13, *slot as i32, attack_type_to_i32(*attack_type)),
        Some(ActionDesc::CounterPass) => (14, -1, -1),
        Some(ActionDesc::CounterPlay { hand_index }) => (15, *hand_index as i32, -1),
        Some(ActionDesc::LevelUp { index }) => (16, *index as i32, -1),
        Some(ActionDesc::EncoreYes) => (17, -1, -1),
        Some(ActionDesc::EncoreNo) => (18, -1, -1),
        Some(ActionDesc::TriggerOrder { index }) => (19, *index as i32, -1),
        Some(ActionDesc::ChoiceSelect { index }) => (20, *index as i32, -1),
    }
}

pub fn action_id_for(action: &ActionDesc) -> Option<usize> {
    match action {
        ActionDesc::MulliganKeep => Some(0),
        ActionDesc::MulliganAll => Some(1),
        ActionDesc::ClockPass => Some(CLOCK_PASS_ID),
        ActionDesc::Clock { hand_index } => {
            let hi = *hand_index as usize;
            if hi < MAX_HAND {
                Some(CLOCK_HAND_BASE + hi)
            } else {
                None
            }
        }
        ActionDesc::MainPass => Some(MAIN_PASS_ID),
        ActionDesc::MainPlayCharacter { hand_index, stage_slot } => {
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
            if fs < MAX_STAGE && ts < MAX_STAGE {
                Some(MAIN_MOVE_BASE + fs * MAX_STAGE + ts)
            } else {
                None
            }
        }
        ActionDesc::MainActivateAbility { slot, ability_index } => {
            let s = *slot as usize;
            let a = *ability_index as usize;
            if s < MAX_STAGE && a < MAX_ABILITIES_PER_CARD {
                Some(MAIN_ACTIVATE_BASE + s * MAX_ABILITIES_PER_CARD + a)
            } else {
                None
            }
        }
        ActionDesc::ClimaxPass => Some(CLIMAX_PASS_ID),
        ActionDesc::ClimaxPlay { hand_index } => {
            let hi = *hand_index as usize;
            if hi < MAX_HAND {
                Some(CLIMAX_PLAY_BASE + hi)
            } else {
                None
            }
        }
        ActionDesc::AttackPass => Some(ATTACK_PASS_ID),
        ActionDesc::Attack { slot, attack_type } => {
            let s = *slot as usize;
            let t = attack_type_to_i32(*attack_type) as usize;
            if s < MAX_STAGE && t < 3 {
                Some(ATTACK_BASE + s * 3 + t)
            } else {
                None
            }
        }
        ActionDesc::CounterPass => Some(COUNTER_PASS_ID),
        ActionDesc::CounterPlay { hand_index } => {
            let hi = *hand_index as usize;
            if hi < MAX_HAND {
                Some(COUNTER_PLAY_BASE + hi)
            } else {
                None
            }
        }
        ActionDesc::LevelUp { index } => {
            let idx = *index as usize;
            if idx < LEVEL_UP_COUNT {
                Some(LEVEL_UP_BASE + idx)
            } else {
                None
            }
        }
        ActionDesc::EncoreYes => Some(ENCORE_YES_ID),
        ActionDesc::EncoreNo => Some(ENCORE_NO_ID),
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
    }
}

pub fn fill_action_mask(actions: &[ActionDesc], mask: &mut [u8], lookup: &mut [Option<ActionDesc>]) {
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

pub fn build_action_mask(actions: &[ActionDesc]) -> (Vec<u8>, Vec<Option<ActionDesc>>) {
    let mut mask = vec![0u8; ACTION_SPACE_SIZE];
    let mut lookup = vec![None; ACTION_SPACE_SIZE];
    fill_action_mask(actions, &mut mask, &mut lookup);
    (mask, lookup)
}
