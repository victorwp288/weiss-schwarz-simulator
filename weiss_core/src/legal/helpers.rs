use std::collections::HashSet;

use crate::config::CurriculumConfig;
use crate::db::{AbilityCost, CardDb, CardType};
use crate::encode::ACTION_SPACE_SIZE;
use crate::modifier_queries::modifier_targets_slot_card;
use crate::state::{AttackType, CardInstance, GameState, ModifierKind, StageSlot, StageStatus};

use super::hand_play_requirements::{card_set_allowed, meets_play_requirements};
use super::types::LegalActionIds;
use super::{MAX_HAND, MAX_STAGE};

#[derive(Clone, Copy)]
pub(super) struct StageModifierCache {
    pub(super) cannot_play_events_from_hand: bool,
    pub(super) cannot_move_stage_position: [bool; MAX_STAGE],
    pub(super) encore_stock_cost_min: [Option<usize>; MAX_STAGE],
}

impl StageModifierCache {
    #[inline(always)]
    pub(super) fn build(state: &GameState, player: usize) -> Self {
        let mut cache = Self {
            cannot_play_events_from_hand: false,
            cannot_move_stage_position: [false; MAX_STAGE],
            encore_stock_cost_min: [None; MAX_STAGE],
        };
        if state.modifiers.is_empty() {
            return cache;
        }
        let stage = &state.players[player].stage;
        let stage_len = stage.len().min(MAX_STAGE);
        let mut slot_card_ids = [0u32; MAX_STAGE];
        for (slot, slot_state) in stage.iter().take(stage_len).enumerate() {
            slot_card_ids[slot] = slot_state.card.map(|c| c.id).unwrap_or(0);
        }
        for modifier in &state.modifiers {
            if modifier.magnitude == 0 {
                continue;
            }
            let slot = modifier.target_slot as usize;
            if slot >= stage_len {
                continue;
            }
            let card_id = slot_card_ids[slot];
            if card_id == 0 || !modifier_targets_slot_card(modifier, player, slot, card_id) {
                continue;
            }
            match modifier.kind {
                ModifierKind::CannotPlayEventsFromHand => {
                    cache.cannot_play_events_from_hand = true;
                }
                ModifierKind::CannotMoveStagePosition => {
                    cache.cannot_move_stage_position[slot] = true;
                }
                ModifierKind::EncoreStockCost if modifier.magnitude > 0 => {
                    let cost = modifier.magnitude as usize;
                    let entry = &mut cache.encore_stock_cost_min[slot];
                    *entry = Some(match *entry {
                        Some(existing) => existing.min(cost),
                        None => cost,
                    });
                }
                _ => {}
            }
        }
        cache
    }
}

#[inline(always)]
pub(super) fn push_id(out: &mut LegalActionIds, id: usize) {
    debug_assert!(ACTION_SPACE_SIZE <= u16::MAX as usize);
    debug_assert!(id < ACTION_SPACE_SIZE);
    out.push(id as u16);
}

#[inline(always)]
pub(super) fn attack_type_to_index(attack_type: AttackType) -> usize {
    match attack_type {
        AttackType::Frontal => 0,
        AttackType::Side => 1,
        AttackType::Direct => 2,
    }
}

#[inline(always)]
pub(super) fn starting_player_first_turn(state: &GameState, player: u8) -> bool {
    state.turn.turn_number == 0 && player == state.turn.starting_player
}

#[inline(always)]
pub(super) fn starting_player_first_turn_attack_used(state: &GameState, player: u8) -> bool {
    if !starting_player_first_turn(state, player) {
        return false;
    }
    state.turn.attack_subphase_count > 0
}

#[inline(always)]
pub(super) fn can_pay_cost_from_state(
    state: &GameState,
    player: usize,
    slot: usize,
    source: CardInstance,
    cost: AbilityCost,
    enforce_cost_requirement: bool,
) -> bool {
    if cost.rest_self {
        if slot >= state.players[player].stage.len() {
            return false;
        }
        let slot_state = &state.players[player].stage[slot];
        if slot_state.card.map(|c| c.instance_id) != Some(source.instance_id) {
            return false;
        }
        if slot_state.status != StageStatus::Stand {
            return false;
        }
    }
    if cost.rest_other > 0 {
        let mut available = 0usize;
        for (idx, slot_state) in state.players[player].stage.iter().enumerate() {
            if idx == slot {
                continue;
            }
            if slot_state.card.is_some() && slot_state.status == StageStatus::Stand {
                available += 1;
            }
        }
        if available < cost.rest_other as usize {
            return false;
        }
    }
    if cost.stock > 0
        && enforce_cost_requirement
        && state.players[player].stock.len() < cost.stock as usize
    {
        return false;
    }
    let required_hand = cost.discard_from_hand as usize
        + cost.clock_from_hand as usize
        + cost.reveal_from_hand as usize;
    if required_hand > state.players[player].hand.len() {
        return false;
    }
    if cost.clock_from_deck_top > 0
        && state.players[player].deck.len() < cost.clock_from_deck_top as usize
    {
        return false;
    }
    true
}

#[inline(always)]
pub(super) fn can_pay_encore_for_slot(
    state: &GameState,
    db: &CardDb,
    curriculum: &CurriculumConfig,
    player: usize,
    slot: usize,
    modifier_cache: Option<&StageModifierCache>,
) -> bool {
    if state.turn.cannot_use_auto_encore[player] {
        return false;
    }
    if slot >= state.players[player].stage.len() {
        return false;
    }
    let Some(card_inst) = state.players[player].stage[slot].card else {
        return false;
    };
    let stock_len = state.players[player].stock.len();
    let mut min_stock_cost = if stock_len >= 3 { Some(3usize) } else { None };
    if let Some(cache) = modifier_cache {
        if let Some(cost) = cache
            .encore_stock_cost_min
            .get(slot)
            .and_then(|entry| *entry)
        {
            min_stock_cost = Some(match min_stock_cost {
                Some(existing) => existing.min(cost),
                None => cost,
            });
        }
    } else {
        for modifier in &state.modifiers {
            if modifier.kind != ModifierKind::EncoreStockCost || modifier.magnitude <= 0 {
                continue;
            }
            if !modifier_targets_slot_card(modifier, player, slot, card_inst.id) {
                continue;
            }
            let cost = modifier.magnitude as usize;
            min_stock_cost = Some(match min_stock_cost {
                Some(existing) => existing.min(cost),
                None => cost,
            });
        }
    }
    if let Some(cost) = min_stock_cost {
        if stock_len >= cost {
            return true;
        }
    }
    db.iter_card_abilities_in_canonical_order(card_inst.id)
        .iter()
        .filter_map(|spec| spec.template.encore_variant_cost())
        .any(|cost| {
            can_pay_cost_from_state(
                state,
                player,
                slot,
                card_inst,
                cost,
                curriculum.enforce_cost_requirement,
            )
        })
}

#[derive(Clone, Copy)]
pub(super) enum PlayableHandCard {
    MainCharacter { hand_index: usize },
    MainEvent { hand_index: usize },
    Climax { hand_index: usize },
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum HandScanMode {
    Main,
    Climax,
}

#[inline(always)]
pub(super) fn for_each_playable_hand_card<F>(
    player: &crate::state::PlayerState,
    db: &CardDb,
    curriculum: &CurriculumConfig,
    allowed_card_sets: Option<&HashSet<String>>,
    mode: HandScanMode,
    events_locked: bool,
    mut visit: F,
) where
    F: FnMut(PlayableHandCard),
{
    let can_play_climax =
        curriculum.enable_climax_phase && curriculum.allow_climax && player.climax.is_empty();
    if mode == HandScanMode::Climax && !can_play_climax {
        return;
    }

    for (hand_index, card_inst) in player.hand.iter().enumerate() {
        if hand_index >= MAX_HAND || hand_index > u8::MAX as usize {
            break;
        }
        let Some(card) = db.get(card_inst.id) else {
            continue;
        };
        if !card_set_allowed(card, curriculum, allowed_card_sets) {
            continue;
        }
        match mode {
            HandScanMode::Main => match card.card_type {
                CardType::Character => {
                    if curriculum.allow_character
                        && meets_play_requirements(card, player, db, curriculum, 0, false)
                    {
                        visit(PlayableHandCard::MainCharacter { hand_index });
                    }
                }
                CardType::Event => {
                    if !events_locked
                        && curriculum.allow_event
                        && meets_play_requirements(card, player, db, curriculum, 0, false)
                    {
                        visit(PlayableHandCard::MainEvent { hand_index });
                    }
                }
                CardType::Climax => {}
            },
            HandScanMode::Climax => {
                if card.card_type == CardType::Climax
                    && meets_play_requirements(card, player, db, curriculum, 0, false)
                {
                    visit(PlayableHandCard::Climax { hand_index });
                }
            }
        }
    }
}

pub(super) fn is_character_slot(slot: &StageSlot, db: &CardDb) -> bool {
    slot.card
        .and_then(|inst| db.get(inst.id))
        .map(|c| c.card_type == CardType::Character)
        .unwrap_or(false)
}
