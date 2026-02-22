use serde::{Deserialize, Serialize};
use smallvec::SmallVec;
use std::collections::HashSet;

use crate::config::CurriculumConfig;
use crate::db::{AbilityCost, CardDb, CardType};
use crate::encode::{
    ACTION_SPACE_SIZE, ATTACK_BASE, CHOICE_BASE, CHOICE_COUNT, CHOICE_NEXT_ID, CHOICE_PREV_ID,
    CLIMAX_PLAY_BASE, CLOCK_HAND_BASE, CONCEDE_ID, ENCORE_DECLINE_BASE, ENCORE_PAY_BASE,
    LEVEL_UP_BASE, MAIN_MOVE_BASE, MAIN_PLAY_CHAR_BASE, MAIN_PLAY_EVENT_BASE, MULLIGAN_CONFIRM_ID,
    MULLIGAN_SELECT_BASE, PASS_ACTION_ID, TRIGGER_ORDER_BASE,
};
use crate::modifier_queries::{collect_attack_slot_state, modifier_targets_slot_card};
use crate::state::{AttackType, CardInstance, GameState, ModifierKind, StageSlot, StageStatus};

use self::hand_play_requirements::{card_set_allowed, meets_play_requirements};

/// Shared play-requirement predicates used by legality and observation reasons.
pub(crate) mod hand_play_requirements;

const MAX_HAND: usize = crate::encode::MAX_HAND;
const MAX_STAGE: usize = 5;

#[derive(Clone, Copy)]
struct StageModifierCache {
    cannot_play_events_from_hand: bool,
    cannot_move_stage_position: [bool; MAX_STAGE],
    encore_stock_cost_min: [Option<usize>; MAX_STAGE],
}

impl StageModifierCache {
    #[inline(always)]
    fn build(state: &GameState, player: usize) -> Self {
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

/// Player decision kinds exposed to callers.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum DecisionKind {
    /// Mulligan decision (select cards to discard).
    Mulligan,
    /// Clock phase decision.
    Clock,
    /// Main phase decision.
    Main,
    /// Climax phase decision.
    Climax,
    /// Attack declaration decision.
    AttackDeclaration,
    /// Level-up choice decision.
    LevelUp,
    /// Encore step decision.
    Encore,
    /// Trigger order decision.
    TriggerOrder,
    /// Choice selection decision.
    Choice,
}

/// A pending decision describing which player must act next.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Decision {
    /// Player index who must act.
    pub player: u8,
    /// Decision kind.
    pub kind: DecisionKind,
    /// Optional focus slot for contextual decisions.
    pub focus_slot: Option<u8>,
}

/// Canonical action descriptor used as the truth representation of legal actions.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum ActionDesc {
    /// Confirm mulligan without selecting additional cards.
    MulliganConfirm,
    /// Select a hand card for mulligan.
    MulliganSelect {
        /// Zero-based hand index parameter.
        hand_index: u8,
    },
    /// Pass the current decision.
    Pass,
    /// Clock a hand card.
    Clock {
        /// Zero-based hand index parameter.
        hand_index: u8,
    },
    /// Play a character from hand to a stage slot.
    MainPlayCharacter {
        /// Zero-based hand index parameter.
        hand_index: u8,
        /// Zero-based stage slot parameter.
        stage_slot: u8,
    },
    /// Play an event from hand.
    MainPlayEvent {
        /// Zero-based hand index parameter.
        hand_index: u8,
    },
    /// Move a character between stage slots.
    MainMove {
        /// Zero-based source stage slot parameter.
        from_slot: u8,
        /// Zero-based destination stage slot parameter.
        to_slot: u8,
    },
    /// Activate a character ability from a stage slot.
    MainActivateAbility {
        /// Zero-based stage slot parameter.
        slot: u8,
        /// Zero-based ability index on the source card.
        ability_index: u8,
    },
    /// Play a climax from hand.
    ClimaxPlay {
        /// Zero-based hand index parameter.
        hand_index: u8,
    },
    /// Declare an attack from a stage slot.
    Attack {
        /// Zero-based stage slot parameter.
        slot: u8,
        /// Attack type parameter.
        attack_type: AttackType,
    },
    /// Play a counter from hand.
    CounterPlay {
        /// Zero-based hand index parameter.
        hand_index: u8,
    },
    /// Select a card for level up.
    LevelUp {
        /// Zero-based selection index parameter.
        index: u8,
    },
    /// Pay encore for a character.
    EncorePay {
        /// Zero-based stage slot parameter.
        slot: u8,
    },
    /// Decline encore for a character.
    EncoreDecline {
        /// Zero-based stage slot parameter.
        slot: u8,
    },
    /// Select trigger order index.
    TriggerOrder {
        /// Zero-based selection index parameter.
        index: u8,
    },
    /// Select a choice option by index.
    ChoiceSelect {
        /// Zero-based selection index parameter.
        index: u8,
    },
    /// Page to previous choice options.
    ChoicePrevPage,
    /// Page to next choice options.
    ChoiceNextPage,
    /// Concede the game.
    Concede,
}

/// Compact list of canonical legal actions.
pub type LegalActions = SmallVec<[ActionDesc; 64]>;
/// Compact list of legal action ids.
pub type LegalActionIds = SmallVec<[u16; 64]>;

#[inline(always)]
fn push_id(out: &mut LegalActionIds, id: usize) {
    debug_assert!(ACTION_SPACE_SIZE <= u16::MAX as usize);
    debug_assert!(id < ACTION_SPACE_SIZE);
    out.push(id as u16);
}

#[inline(always)]
fn attack_type_to_index(attack_type: AttackType) -> usize {
    match attack_type {
        AttackType::Frontal => 0,
        AttackType::Side => 1,
        AttackType::Direct => 2,
    }
}

#[inline(always)]
fn starting_player_first_turn(state: &GameState, player: u8) -> bool {
    state.turn.turn_number == 0 && player == state.turn.starting_player
}

#[inline(always)]
fn starting_player_first_turn_attack_used(state: &GameState, player: u8) -> bool {
    if !starting_player_first_turn(state, player) {
        return false;
    }
    state.turn.attack_subphase_count > 0
}

#[inline(always)]
fn can_pay_cost_from_state(
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
fn can_pay_encore_for_slot(
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
enum PlayableHandCard {
    MainCharacter { hand_index: usize },
    MainEvent { hand_index: usize },
    Climax { hand_index: usize },
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum HandScanMode {
    Main,
    Climax,
}

#[inline(always)]
fn for_each_playable_hand_card<F>(
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

/// Validate whether an attack can be declared from a slot.
pub fn can_declare_attack(
    state: &GameState,
    player: u8,
    slot: u8,
    attack_type: AttackType,
    curriculum: &CurriculumConfig,
) -> Result<(), &'static str> {
    let p = player as usize;
    let s = slot as usize;
    if s >= MAX_STAGE || (curriculum.reduced_stage_mode && s > 0) {
        return Err("Attack slot out of range");
    }
    if s >= 3 {
        return Err("Attack must be from center stage");
    }
    let attacker_slot = &state.players[p].stage[s];
    if attacker_slot.card.is_none() {
        return Err("No attacker in slot");
    }
    if attacker_slot.status != StageStatus::Stand {
        return Err("Attacker is rested");
    }
    if attacker_slot.has_attacked {
        return Err("Attacker already attacked");
    }
    if starting_player_first_turn_attack_used(state, player) {
        return Err("Starting player can only attack once on first turn");
    }
    let (cannot_attack, cannot_side_attack, cannot_frontal_attack, attack_cost) =
        if let Some(derived) = state.turn.derived_attack.as_ref() {
            let entry = derived.per_player[p][s];
            (
                entry.cannot_attack,
                entry.cannot_side_attack,
                entry.cannot_frontal_attack,
                entry.attack_cost,
            )
        } else if let Some(card_inst) = attacker_slot.card {
            collect_attack_slot_state(
                state,
                p,
                s,
                card_inst.id,
                attacker_slot.cannot_attack,
                attacker_slot.attack_cost,
            )
        } else {
            (
                attacker_slot.cannot_attack,
                false,
                false,
                attacker_slot.attack_cost,
            )
        };
    if cannot_attack {
        return Err("Attacker cannot attack");
    }
    if attack_cost as usize > state.players[p].stock.len() {
        return Err("Attack cost not payable");
    }
    let defender_player = 1 - p;
    let defender_present = state.players[defender_player].stage[s].card.is_some();
    match attack_type {
        AttackType::Frontal | AttackType::Side if !defender_present => {
            return Err("No defender for frontal/side attack");
        }
        AttackType::Frontal if cannot_frontal_attack => {
            return Err("Attacker cannot frontal attack");
        }
        AttackType::Side if cannot_side_attack => {
            return Err("Attacker cannot side attack");
        }
        AttackType::Direct if defender_present => {
            return Err("Direct attack requires empty opposing slot");
        }
        AttackType::Side if !curriculum.enable_side_attacks => {
            return Err("Side attacks disabled");
        }
        AttackType::Direct if !curriculum.enable_direct_attacks => {
            return Err("Direct attacks disabled");
        }
        _ => {}
    }
    Ok(())
}

/// Compute legal attack actions into a reusable buffer.
#[inline(always)]
pub fn legal_attack_actions_into(
    state: &GameState,
    player: u8,
    curriculum: &CurriculumConfig,
    actions: &mut LegalActions,
) {
    if starting_player_first_turn_attack_used(state, player) {
        return;
    }
    let max_slot = if curriculum.reduced_stage_mode { 1 } else { 3 };
    for slot in 0..max_slot {
        let slot_u8 = slot as u8;
        for attack_type in [AttackType::Frontal, AttackType::Side, AttackType::Direct] {
            if can_declare_attack(state, player, slot_u8, attack_type, curriculum).is_ok() {
                actions.push(ActionDesc::Attack {
                    slot: slot_u8,
                    attack_type,
                });
            }
        }
    }
}

/// Compute legal attack actions for a player.
#[inline(always)]
pub fn legal_attack_actions(
    state: &GameState,
    player: u8,
    curriculum: &CurriculumConfig,
) -> LegalActions {
    let mut actions = LegalActions::new();
    legal_attack_actions_into(state, player, curriculum, &mut actions);
    actions
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

fn is_character_slot(slot: &StageSlot, db: &CardDb) -> bool {
    slot.card
        .and_then(|inst| db.get(inst.id))
        .map(|c| c.card_type == CardType::Character)
        .unwrap_or(false)
}
