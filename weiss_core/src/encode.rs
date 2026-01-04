use crate::config::{CurriculumConfig, ObservationVisibility};
use crate::db::CardDb;
use crate::legal::{ActionDesc, Decision, DecisionKind};
use crate::state::{
    AttackType, GameState, ModifierKind, Phase, StageStatus, TerminalResult, REVEAL_HISTORY_LEN,
};

pub const OBS_ENCODING_VERSION: u32 = 1;
pub const ACTION_ENCODING_VERSION: u32 = 1;
pub const POLICY_VERSION: u32 = 1;
pub const SPEC_HASH: u64 = ((OBS_ENCODING_VERSION as u64) << 32)
    | ((ACTION_ENCODING_VERSION as u64) << 16)
    | (POLICY_VERSION as u64);

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
    out.fill(0);
    let p0 = perspective as usize;
    let p1 = 1 - p0;
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

    let mut offset = OBS_HEADER_LEN;
    for (idx, player_index) in [p0, p1].iter().enumerate() {
        let p = &state.players[*player_index];
        out[offset] = p.level.len() as i32;
        out[offset + 1] = p.clock.len() as i32;
        out[offset + 2] = p.deck.len() as i32;
        out[offset + 3] = p.hand.len() as i32;
        out[offset + 4] = p.stock.len() as i32;
        out[offset + 5] = p.waiting_room.len() as i32;
        let memory_visible =
            if visibility == ObservationVisibility::Public && !curriculum.memory_is_public {
                *player_index == perspective as usize
            } else {
                true
            };
        out[offset + 6] = if memory_visible {
            p.memory.len() as i32
        } else {
            0
        };
        out[offset + 7] = p.climax.len() as i32;
        out[offset + 8] = p.resolution.len() as i32;
        offset += PER_PLAYER_COUNTS;

        for (slot, slot_state) in p.stage.iter().enumerate() {
            let card_id = slot_state.card.map(|c| c.id).unwrap_or(0) as i32;
            let status = if slot_state.card.is_some() {
                status_to_i32(slot_state.status)
            } else {
                0
            };
            let has_attacked = if slot_state.has_attacked { 1 } else { 0 };
            let (power, soul) = if let Some(card) = slot_state.card.and_then(|inst| db.get(inst.id))
            {
                let power = slot_powers[*player_index][slot];
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
            let value = if idx < p.clock.len() {
                p.clock[idx].id as i32
            } else {
                0
            };
            out[offset + i] = value;
        }
        offset += PER_PLAYER_CLOCK_TOP;

        for i in 0..TOP_WAITING_ROOM {
            let idx = p.waiting_room.len().saturating_sub(1 + i);
            let value = if idx < p.waiting_room.len() {
                p.waiting_room[idx].id as i32
            } else {
                0
            };
            out[offset + i] = value;
        }
        offset += PER_PLAYER_WAITING_TOP;

        for i in 0..TOP_RESOLUTION {
            let idx = p.resolution.len().saturating_sub(1 + i);
            let value = if idx < p.resolution.len() {
                p.resolution[idx].id as i32
            } else {
                0
            };
            out[offset + i] = value;
        }
        offset += PER_PLAYER_RESOLUTION_TOP;

        for i in 0..TOP_STOCK {
            let value = if visibility == ObservationVisibility::Full {
                let idx = p.stock.len().saturating_sub(1 + i);
                if idx < p.stock.len() {
                    p.stock[idx].id as i32
                } else {
                    0
                }
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

    let reason_bits = compute_reason_bits(state, db, curriculum, perspective, decision);
    let reason_base = OBS_REASON_BASE;
    out[reason_base..reason_base + OBS_REASON_LEN].copy_from_slice(&reason_bits);

    let reveal_base = OBS_REVEAL_BASE;
    let reveal_slice = &mut out[reveal_base..reveal_base + OBS_REVEAL_LEN];
    state.reveal_history[p0].write_chronological(reveal_slice);

    let context_base = OBS_CONTEXT_BASE;
    let context_bits = compute_context_bits(state);
    out[context_base..context_base + OBS_CONTEXT_LEN].copy_from_slice(&context_bits);
}

fn compute_slot_powers_from_state(state: &GameState, db: &CardDb, out: &mut [[i32; MAX_STAGE]; 2]) {
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
            let power = if let Some(card) = slot_state.card.and_then(|inst| db.get(inst.id)) {
                card.power
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
        if let Some(c) = db.get(card_id.id) {
            if c.color == card.color {
                return true;
            }
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

pub fn build_action_mask(actions: &[ActionDesc]) -> (Vec<u8>, Vec<Option<ActionDesc>>) {
    let mut mask = vec![0u8; ACTION_SPACE_SIZE];
    let mut lookup = vec![None; ACTION_SPACE_SIZE];
    fill_action_mask(actions, &mut mask, &mut lookup);
    (mask, lookup)
}
