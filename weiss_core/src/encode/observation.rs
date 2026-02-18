use crate::config::{CurriculumConfig, ObservationVisibility};
use crate::db::CardDb;
use crate::legal::{ActionDesc, Decision, DecisionKind};
use crate::state::{AttackType, GameState, ModifierKind, Phase, StageStatus, TerminalResult};

use super::constants::*;

/// Encode a full observation into a fixed-length buffer.
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
    let hand_count_visible = private_count_visible(visibility, curriculum, is_self);
    let stock_count_visible = private_count_visible(visibility, curriculum, is_self);
    let base = OBS_HEADER_LEN + block_index * PER_PLAYER_BLOCK_LEN;
    let block = &mut out[base..base + PER_PLAYER_BLOCK_LEN];
    encode_obs_player_block_into(
        state,
        db,
        player_index,
        memory_visible,
        hand_count_visible,
        hand_visible,
        stock_count_visible,
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
    hand_count_visible: bool,
    hand_visible: bool,
    stock_count_visible: bool,
    stock_visible: bool,
    deck_visible: bool,
    slot_powers: &[[i32; MAX_STAGE]; 2],
    out: &mut [i32],
) {
    assert!(out.len() >= PER_PLAYER_BLOCK_LEN);
    let p = player_index as usize;
    let mut offset = 0;
    let player = &state.players[p];
    let mut slot_card_ids = [0u32; MAX_STAGE];
    let mut slot_soul_mods = [0i32; MAX_STAGE];
    let mut slot_side_attack_allowed = [0i32; MAX_STAGE];
    for (slot, slot_state) in player.stage.iter().enumerate() {
        let card_id = slot_state.card.map(|c| c.id).unwrap_or(0);
        slot_card_ids[slot] = card_id;
        slot_side_attack_allowed[slot] = i32::from(card_id != 0);
    }
    let use_derived_attack = state.turn.derived_attack.is_some();
    if !state.modifiers.is_empty() {
        for modifier in &state.modifiers {
            if modifier.target_player as usize != p {
                continue;
            }
            let slot = modifier.target_slot as usize;
            if slot >= MAX_STAGE {
                continue;
            }
            let card_id = slot_card_ids[slot];
            if card_id == 0 || modifier.target_card != card_id {
                continue;
            }
            match modifier.kind {
                ModifierKind::Soul => {
                    slot_soul_mods[slot] = slot_soul_mods[slot].saturating_add(modifier.magnitude);
                }
                ModifierKind::CannotSideAttack
                    if !use_derived_attack && modifier.magnitude != 0 =>
                {
                    slot_side_attack_allowed[slot] = 0;
                }
                _ => {}
            }
        }
    }
    if let Some(derived) = state.turn.derived_attack.as_ref() {
        for (slot, card_id) in slot_card_ids.iter().enumerate() {
            if *card_id == 0 {
                continue;
            }
            slot_side_attack_allowed[slot] =
                i32::from(!derived.per_player[p][slot].cannot_side_attack);
        }
    }
    out[offset] = player.level.len() as i32;
    out[offset + 1] = player.clock.len() as i32;
    out[offset + 2] = player.deck.len() as i32;
    out[offset + 3] = if hand_count_visible {
        player.hand.len() as i32
    } else {
        0
    };
    out[offset + 4] = if stock_count_visible {
        player.stock.len() as i32
    } else {
        0
    };
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
        let (power, soul, effective_soul, side_attack_allowed) =
            if let Some(card_inst) = slot_state.card {
                let power = slot_powers[p][slot];
                let soul = db.soul_by_id(card_inst.id) as i32;
                let effective_soul = soul.saturating_add(slot_soul_mods[slot]).max(0);
                let side_attack_allowed = slot_side_attack_allowed[slot];
                (power, soul, effective_soul, side_attack_allowed)
            } else {
                (0, 0, 0, 0)
            };
        let base = offset + slot * PER_STAGE_SLOT;
        out[base] = card_id;
        out[base + 1] = status;
        out[base + 2] = has_attacked;
        out[base + 3] = power;
        out[base + 4] = soul;
        out[base + 5] = effective_soul;
        out[base + 6] = side_attack_allowed;
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

fn private_count_visible(
    visibility: ObservationVisibility,
    curriculum: &CurriculumConfig,
    is_self: bool,
) -> bool {
    visibility == ObservationVisibility::Full
        || is_self
        || curriculum.reveal_opponent_hand_stock_counts
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
    debug_assert!(perspective <= 1, "invalid perspective");
    let other = match perspective {
        0 => 1,
        1 => 0,
        _ => 0,
    };
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
