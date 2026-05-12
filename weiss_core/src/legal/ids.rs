use std::collections::HashSet;

use crate::config::CurriculumConfig;
use crate::db::CardDb;
use crate::encode::{
    ATTACK_BASE, CHOICE_BASE, CHOICE_COUNT, CHOICE_NEXT_ID, CHOICE_PREV_ID, CLIMAX_PLAY_BASE,
    CLOCK_HAND_BASE, CONCEDE_ID, ENCORE_DECLINE_BASE, ENCORE_PAY_BASE, LEVEL_UP_BASE,
    MAIN_MOVE_BASE, MAIN_PLAY_CHAR_BASE, MAIN_PLAY_EVENT_BASE, MULLIGAN_CONFIRM_ID,
    MULLIGAN_SELECT_BASE, PASS_ACTION_ID, TRIGGER_ORDER_BASE,
};
use crate::state::{AttackType, GameState, StageStatus};

use super::attack::can_declare_attack;
use super::hand_play_requirements::card_set_allowed;
use super::helpers::{
    attack_type_to_index, can_pay_encore_for_slot, for_each_playable_hand_card, is_character_slot,
    push_id, starting_player_first_turn_attack_used, HandScanMode, PlayableHandCard,
    StageModifierCache,
};
use super::types::{Decision, DecisionKind, LegalActionIds};
use super::{MAX_HAND, MAX_STAGE};

#[inline(always)]
fn append_mulligan_action_ids(state: &GameState, player: usize, out: &mut LegalActionIds) {
    push_id(out, MULLIGAN_CONFIRM_ID);
    let p = &state.players[player];
    for (hand_index, _) in p.hand.iter().enumerate() {
        if hand_index >= MAX_HAND || hand_index > u8::MAX as usize {
            break;
        }
        push_id(out, MULLIGAN_SELECT_BASE + hand_index);
    }
}

#[inline(always)]
fn append_clock_action_ids(
    state: &GameState,
    player: usize,
    db: &CardDb,
    curriculum: &CurriculumConfig,
    allowed_card_sets: Option<&HashSet<String>>,
    out: &mut LegalActionIds,
) {
    push_id(out, PASS_ACTION_ID);
    let p = &state.players[player];
    for (hand_index, card_inst) in p.hand.iter().enumerate() {
        if hand_index >= MAX_HAND || hand_index > u8::MAX as usize {
            break;
        }
        if let Some(card) = db.get(card_inst.id) {
            if !card_set_allowed(card, curriculum, allowed_card_sets) {
                continue;
            }
            push_id(out, CLOCK_HAND_BASE + hand_index);
        }
    }
}

#[inline(always)]
fn append_main_action_ids(
    state: &GameState,
    player: usize,
    db: &CardDb,
    curriculum: &CurriculumConfig,
    allowed_card_sets: Option<&HashSet<String>>,
    out: &mut LegalActionIds,
) {
    let p = &state.players[player];
    let modifier_cache = StageModifierCache::build(state, player);
    let max_slot = if curriculum.reduced_stage_mode {
        1
    } else {
        MAX_STAGE
    };
    let events_locked = modifier_cache.cannot_play_events_from_hand;
    push_id(out, PASS_ACTION_ID);
    for_each_playable_hand_card(
        p,
        db,
        curriculum,
        allowed_card_sets,
        HandScanMode::Main,
        events_locked,
        |playable| match playable {
            PlayableHandCard::MainCharacter { hand_index } => {
                for slot in 0..max_slot {
                    let id = MAIN_PLAY_CHAR_BASE + hand_index * MAX_STAGE + slot;
                    push_id(out, id);
                }
            }
            PlayableHandCard::MainEvent { hand_index } => {
                push_id(out, MAIN_PLAY_EVENT_BASE + hand_index);
            }
            PlayableHandCard::Climax { .. } => {}
        },
    );
    if !state.turn.main_move_used {
        for from in 0..max_slot {
            for to in 0..max_slot {
                if from == to {
                    continue;
                }
                let from_slot = &p.stage[from];
                let to_slot = &p.stage[to];
                if from_slot.card.is_some()
                    && is_character_slot(from_slot, db)
                    && (to_slot.card.is_none() || is_character_slot(to_slot, db))
                    && !modifier_cache.cannot_move_stage_position[from]
                    && (to_slot.card.is_none() || !modifier_cache.cannot_move_stage_position[to])
                {
                    let to_index = if to < from { to } else { to - 1 };
                    let id = MAIN_MOVE_BASE + from * (MAX_STAGE - 1) + to_index;
                    push_id(out, id);
                }
            }
        }
    }
}

#[inline(always)]
fn append_climax_action_ids(
    state: &GameState,
    player: usize,
    db: &CardDb,
    curriculum: &CurriculumConfig,
    allowed_card_sets: Option<&HashSet<String>>,
    out: &mut LegalActionIds,
) {
    let p = &state.players[player];
    push_id(out, PASS_ACTION_ID);
    for_each_playable_hand_card(
        p,
        db,
        curriculum,
        allowed_card_sets,
        HandScanMode::Climax,
        false,
        |playable| {
            if let PlayableHandCard::Climax { hand_index } = playable {
                push_id(out, CLIMAX_PLAY_BASE + hand_index);
            }
        },
    );
}

#[inline(always)]
fn append_attack_declaration_action_ids(
    state: &GameState,
    player: u8,
    curriculum: &CurriculumConfig,
    out: &mut LegalActionIds,
) {
    push_id(out, PASS_ACTION_ID);
    if starting_player_first_turn_attack_used(state, player) {
        return;
    }
    let max_slot = if curriculum.reduced_stage_mode { 1 } else { 3 };
    for slot in 0..max_slot {
        let slot_u8 = slot as u8;
        for attack_type in [AttackType::Frontal, AttackType::Side, AttackType::Direct] {
            if can_declare_attack(state, player, slot_u8, attack_type, curriculum).is_ok() {
                let id = ATTACK_BASE + slot * 3 + attack_type_to_index(attack_type);
                push_id(out, id);
            }
        }
    }
}

#[inline(always)]
fn append_level_up_action_ids(state: &GameState, player: usize, out: &mut LegalActionIds) {
    if state.players[player].clock.len() >= 7 {
        for idx in 0..7 {
            push_id(out, LEVEL_UP_BASE + idx);
        }
    }
}

#[inline(always)]
fn append_encore_action_ids(
    state: &GameState,
    player: usize,
    db: &CardDb,
    curriculum: &CurriculumConfig,
    out: &mut LegalActionIds,
) {
    let p = &state.players[player];
    let modifier_cache = StageModifierCache::build(state, player);
    for slot in 0..p.stage.len() {
        if p.stage[slot].card.is_some()
            && p.stage[slot].status == StageStatus::Reverse
            && can_pay_encore_for_slot(state, db, curriculum, player, slot, Some(&modifier_cache))
        {
            push_id(out, ENCORE_PAY_BASE + slot);
        }
    }
    for slot in 0..p.stage.len() {
        if p.stage[slot].card.is_some() && p.stage[slot].status == StageStatus::Reverse {
            push_id(out, ENCORE_DECLINE_BASE + slot);
        }
    }
}

#[inline(always)]
fn append_trigger_order_action_ids(state: &GameState, out: &mut LegalActionIds) {
    let choices = state
        .turn
        .trigger_order
        .as_ref()
        .map(|o| o.choices.len())
        .unwrap_or(0);
    let max = choices.min(10);
    for idx in 0..max {
        push_id(out, TRIGGER_ORDER_BASE + idx);
    }
}

#[inline(always)]
fn append_choice_action_ids(state: &GameState, out: &mut LegalActionIds) {
    if let Some(choice) = state.turn.choice.as_ref() {
        let total = choice.total_candidates as usize;
        let page_start = choice.page_start as usize;
        let safe_start = page_start.min(total);
        let page_end = total.min(safe_start + CHOICE_COUNT);
        for idx in 0..(page_end - safe_start) {
            push_id(out, CHOICE_BASE + idx);
        }
        if page_start >= CHOICE_COUNT {
            push_id(out, CHOICE_PREV_ID);
        }
        if page_start + CHOICE_COUNT < total {
            push_id(out, CHOICE_NEXT_ID);
        }
    }
}

/// Compute legal action ids for a decision into a reusable buffer.
#[inline(always)]
pub fn legal_action_ids_cached_into(
    state: &GameState,
    decision: &Decision,
    db: &CardDb,
    curriculum: &CurriculumConfig,
    allowed_card_sets: Option<&HashSet<String>>,
    out: &mut LegalActionIds,
) {
    // Invariants:
    // - Preserve canonical legal action ordering and action-id packing per decision.
    // - Keep descriptor/id parity covered by `weiss_core/tests/legal_cache_parity_tests.rs`.
    let player = decision.player as usize;
    out.clear();
    match decision.kind {
        DecisionKind::Mulligan => append_mulligan_action_ids(state, player, out),
        DecisionKind::Clock => {
            append_clock_action_ids(state, player, db, curriculum, allowed_card_sets, out)
        }
        DecisionKind::Main => {
            append_main_action_ids(state, player, db, curriculum, allowed_card_sets, out)
        }
        DecisionKind::Climax => {
            append_climax_action_ids(state, player, db, curriculum, allowed_card_sets, out)
        }
        DecisionKind::AttackDeclaration => {
            append_attack_declaration_action_ids(state, decision.player, curriculum, out)
        }
        DecisionKind::LevelUp => append_level_up_action_ids(state, player, out),
        DecisionKind::Encore => append_encore_action_ids(state, player, db, curriculum, out),
        DecisionKind::TriggerOrder => append_trigger_order_action_ids(state, out),
        DecisionKind::Choice => append_choice_action_ids(state, out),
    }
    if curriculum.allow_concede {
        push_id(out, CONCEDE_ID);
    }
}
