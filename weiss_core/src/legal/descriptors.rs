use std::collections::HashSet;

use crate::config::CurriculumConfig;
use crate::db::CardDb;
use crate::encode::CHOICE_COUNT;
use crate::state::{GameState, StageStatus};

use super::attack::legal_attack_actions_into;
use super::hand_play_requirements::card_set_allowed;
use super::helpers::{
    can_pay_encore_for_slot, for_each_playable_hand_card, is_character_slot, HandScanMode,
    PlayableHandCard, StageModifierCache,
};
use super::types::{ActionDesc, Decision, DecisionKind, LegalActions};
use super::{MAX_HAND, MAX_STAGE};

#[inline(always)]
fn append_mulligan_actions(state: &GameState, player: usize, actions: &mut LegalActions) {
    let p = &state.players[player];
    actions.push(ActionDesc::MulliganConfirm);
    for (hand_index, _) in p.hand.iter().enumerate() {
        if hand_index >= MAX_HAND || hand_index > u8::MAX as usize {
            break;
        }
        actions.push(ActionDesc::MulliganSelect {
            hand_index: hand_index as u8,
        });
    }
}

#[inline(always)]
fn append_clock_actions(
    state: &GameState,
    player: usize,
    db: &CardDb,
    curriculum: &CurriculumConfig,
    allowed_card_sets: Option<&HashSet<String>>,
    actions: &mut LegalActions,
) {
    actions.push(ActionDesc::Pass);
    let p = &state.players[player];
    for (hand_index, card_inst) in p.hand.iter().enumerate() {
        if hand_index >= MAX_HAND || hand_index > u8::MAX as usize {
            break;
        }
        if let Some(card) = db.get(card_inst.id) {
            if !card_set_allowed(card, curriculum, allowed_card_sets) {
                continue;
            }
            actions.push(ActionDesc::Clock {
                hand_index: hand_index as u8,
            });
        }
    }
}

#[inline(always)]
fn append_main_actions(
    state: &GameState,
    player: usize,
    db: &CardDb,
    curriculum: &CurriculumConfig,
    allowed_card_sets: Option<&HashSet<String>>,
    actions: &mut LegalActions,
) {
    let p = &state.players[player];
    let modifier_cache = StageModifierCache::build(state, player);
    let max_slot = if curriculum.reduced_stage_mode {
        1
    } else {
        MAX_STAGE
    };
    let events_locked = modifier_cache.cannot_play_events_from_hand;
    actions.push(ActionDesc::Pass);
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
                    actions.push(ActionDesc::MainPlayCharacter {
                        hand_index: hand_index as u8,
                        stage_slot: slot as u8,
                    });
                }
            }
            PlayableHandCard::MainEvent { hand_index } => {
                actions.push(ActionDesc::MainPlayEvent {
                    hand_index: hand_index as u8,
                });
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
                    actions.push(ActionDesc::MainMove {
                        from_slot: from as u8,
                        to_slot: to as u8,
                    });
                }
            }
        }
    }
}

#[inline(always)]
fn append_climax_actions(
    state: &GameState,
    player: usize,
    db: &CardDb,
    curriculum: &CurriculumConfig,
    allowed_card_sets: Option<&HashSet<String>>,
    actions: &mut LegalActions,
) {
    let p = &state.players[player];
    actions.push(ActionDesc::Pass);
    for_each_playable_hand_card(
        p,
        db,
        curriculum,
        allowed_card_sets,
        HandScanMode::Climax,
        false,
        |playable| {
            if let PlayableHandCard::Climax { hand_index } = playable {
                actions.push(ActionDesc::ClimaxPlay {
                    hand_index: hand_index as u8,
                });
            }
        },
    );
}

#[inline(always)]
fn append_attack_declaration_actions(
    state: &GameState,
    player: u8,
    curriculum: &CurriculumConfig,
    actions: &mut LegalActions,
) {
    actions.push(ActionDesc::Pass);
    legal_attack_actions_into(state, player, curriculum, actions);
}

#[inline(always)]
fn append_level_up_actions(state: &GameState, player: usize, actions: &mut LegalActions) {
    if state.players[player].clock.len() >= 7 {
        for idx in 0..7 {
            actions.push(ActionDesc::LevelUp { index: idx as u8 });
        }
    }
}

#[inline(always)]
fn append_encore_actions(
    state: &GameState,
    player: usize,
    db: &CardDb,
    curriculum: &CurriculumConfig,
    actions: &mut LegalActions,
) {
    let p = &state.players[player];
    let modifier_cache = StageModifierCache::build(state, player);
    for slot in 0..p.stage.len() {
        if p.stage[slot].card.is_some()
            && p.stage[slot].status == StageStatus::Reverse
            && can_pay_encore_for_slot(state, db, curriculum, player, slot, Some(&modifier_cache))
        {
            actions.push(ActionDesc::EncorePay { slot: slot as u8 });
        }
    }
    for slot in 0..p.stage.len() {
        if p.stage[slot].card.is_some() && p.stage[slot].status == StageStatus::Reverse {
            actions.push(ActionDesc::EncoreDecline { slot: slot as u8 });
        }
    }
}

#[inline(always)]
fn append_trigger_order_actions(state: &GameState, actions: &mut LegalActions) {
    let choices = state
        .turn
        .trigger_order
        .as_ref()
        .map(|o| o.choices.len())
        .unwrap_or(0);
    let max = choices.min(10);
    for idx in 0..max {
        actions.push(ActionDesc::TriggerOrder { index: idx as u8 });
    }
}

#[inline(always)]
fn append_choice_actions(state: &GameState, actions: &mut LegalActions) {
    if let Some(choice) = state.turn.choice.as_ref() {
        let total = choice.total_candidates as usize;
        let page_start = choice.page_start as usize;
        let safe_start = page_start.min(total);
        let page_end = total.min(safe_start + CHOICE_COUNT);
        for idx in 0..(page_end - safe_start) {
            actions.push(ActionDesc::ChoiceSelect { index: idx as u8 });
        }
        if page_start >= CHOICE_COUNT {
            actions.push(ActionDesc::ChoicePrevPage);
        }
        if page_start + CHOICE_COUNT < total {
            actions.push(ActionDesc::ChoiceNextPage);
        }
    }
}

/// Compute legal actions for a decision.
#[inline(always)]
pub fn legal_actions(
    state: &GameState,
    decision: &Decision,
    db: &CardDb,
    curriculum: &CurriculumConfig,
) -> LegalActions {
    legal_actions_cached(state, decision, db, curriculum, None)
}

/// Compute legal actions using cached data structures where possible.
#[inline(always)]
pub fn legal_actions_cached(
    state: &GameState,
    decision: &Decision,
    db: &CardDb,
    curriculum: &CurriculumConfig,
    allowed_card_sets: Option<&HashSet<String>>,
) -> LegalActions {
    let mut actions = LegalActions::new();
    legal_actions_cached_into(
        state,
        decision,
        db,
        curriculum,
        allowed_card_sets,
        &mut actions,
    );
    actions
}

/// Compute legal actions into a reusable buffer using cached data.
#[inline(always)]
pub fn legal_actions_cached_into(
    state: &GameState,
    decision: &Decision,
    db: &CardDb,
    curriculum: &CurriculumConfig,
    allowed_card_sets: Option<&HashSet<String>>,
    actions: &mut LegalActions,
) {
    // Invariants:
    // - Preserve canonical legal action ordering so `action_id_for` stays parity-aligned.
    // - Ordering/id parity is covered by `weiss_core/tests/legal_cache_parity_tests.rs`.
    let player = decision.player as usize;
    actions.clear();
    match decision.kind {
        DecisionKind::Mulligan => append_mulligan_actions(state, player, actions),
        DecisionKind::Clock => {
            append_clock_actions(state, player, db, curriculum, allowed_card_sets, actions)
        }
        DecisionKind::Main => {
            append_main_actions(state, player, db, curriculum, allowed_card_sets, actions)
        }
        DecisionKind::Climax => {
            append_climax_actions(state, player, db, curriculum, allowed_card_sets, actions)
        }
        DecisionKind::AttackDeclaration => {
            append_attack_declaration_actions(state, decision.player, curriculum, actions)
        }
        DecisionKind::LevelUp => append_level_up_actions(state, player, actions),
        DecisionKind::Encore => append_encore_actions(state, player, db, curriculum, actions),
        DecisionKind::TriggerOrder => append_trigger_order_actions(state, actions),
        DecisionKind::Choice => append_choice_actions(state, actions),
    }
    if curriculum.allow_concede {
        actions.push(ActionDesc::Concede);
    }
}
