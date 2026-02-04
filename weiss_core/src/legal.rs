use serde::{Deserialize, Serialize};
use smallvec::SmallVec;
use std::collections::HashSet;

use crate::config::CurriculumConfig;
use crate::db::{CardColor, CardDb, CardStatic, CardType};
use crate::encode::{
    ACTION_SPACE_SIZE, ATTACK_BASE, CHOICE_BASE, CHOICE_COUNT, CHOICE_NEXT_ID, CHOICE_PREV_ID,
    CLIMAX_PLAY_BASE, CLOCK_HAND_BASE, CONCEDE_ID, ENCORE_DECLINE_BASE, ENCORE_PAY_BASE,
    LEVEL_UP_BASE, MAIN_MOVE_BASE, MAIN_PLAY_CHAR_BASE, MAIN_PLAY_EVENT_BASE, MULLIGAN_CONFIRM_ID,
    MULLIGAN_SELECT_BASE, PASS_ACTION_ID, TRIGGER_ORDER_BASE,
};
use crate::state::{AttackType, GameState, StageSlot, StageStatus};

const MAX_HAND: usize = crate::encode::MAX_HAND;
const MAX_STAGE: usize = 5;

/// Player decision kinds exposed to callers.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum DecisionKind {
    Mulligan,
    Clock,
    Main,
    Climax,
    AttackDeclaration,
    LevelUp,
    Encore,
    TriggerOrder,
    Choice,
}

/// A pending decision describing which player must act next.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Decision {
    pub player: u8,
    pub kind: DecisionKind,
    pub focus_slot: Option<u8>,
}

/// Canonical action descriptor used as the truth representation of legal actions.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum ActionDesc {
    MulliganConfirm,
    MulliganSelect { hand_index: u8 },
    Pass,
    Clock { hand_index: u8 },
    MainPlayCharacter { hand_index: u8, stage_slot: u8 },
    MainPlayEvent { hand_index: u8 },
    MainMove { from_slot: u8, to_slot: u8 },
    MainActivateAbility { slot: u8, ability_index: u8 },
    ClimaxPlay { hand_index: u8 },
    Attack { slot: u8, attack_type: AttackType },
    CounterPlay { hand_index: u8 },
    LevelUp { index: u8 },
    EncorePay { slot: u8 },
    EncoreDecline { slot: u8 },
    TriggerOrder { index: u8 },
    ChoiceSelect { index: u8 },
    ChoicePrevPage,
    ChoiceNextPage,
    Concede,
}

pub type LegalActions = SmallVec<[ActionDesc; 64]>;
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
pub fn legal_action_ids_cached_into(
    state: &GameState,
    decision: &Decision,
    db: &CardDb,
    curriculum: &CurriculumConfig,
    allowed_card_sets: Option<&HashSet<String>>,
    out: &mut LegalActionIds,
) {
    let player = decision.player as usize;
    out.clear();
    match decision.kind {
        DecisionKind::Mulligan => {
            push_id(out, MULLIGAN_CONFIRM_ID);
            let p = &state.players[player];
            for (hand_index, _) in p.hand.iter().enumerate() {
                if hand_index >= MAX_HAND || hand_index > u8::MAX as usize {
                    break;
                }
                push_id(out, MULLIGAN_SELECT_BASE + hand_index);
            }
        }
        DecisionKind::Clock => {
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
        DecisionKind::Main => {
            let p = &state.players[player];
            let max_slot = if curriculum.reduced_stage_mode {
                1
            } else {
                MAX_STAGE
            };
            for (hand_index, card_inst) in p.hand.iter().enumerate() {
                if hand_index >= MAX_HAND || hand_index > u8::MAX as usize {
                    break;
                }
                if let Some(card) = db.get(card_inst.id) {
                    if !card_set_allowed(card, curriculum, allowed_card_sets) {
                        continue;
                    }
                    match card.card_type {
                        CardType::Character => {
                            if curriculum.allow_character
                                && meets_level_requirement(card, p.level.len())
                                && meets_color_requirement(card, p, db, curriculum)
                                && meets_cost_requirement(card, p, curriculum)
                            {
                                for slot in 0..max_slot {
                                    let id = MAIN_PLAY_CHAR_BASE + hand_index * MAX_STAGE + slot;
                                    push_id(out, id);
                                }
                            }
                        }
                        CardType::Event => {
                            if curriculum.allow_event
                                && meets_level_requirement(card, p.level.len())
                                && meets_color_requirement(card, p, db, curriculum)
                                && meets_cost_requirement(card, p, curriculum)
                            {
                                push_id(out, MAIN_PLAY_EVENT_BASE + hand_index);
                            }
                        }
                        CardType::Climax => {}
                    }
                }
            }
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
                    {
                        let to_index = if to < from { to } else { to - 1 };
                        let id = MAIN_MOVE_BASE + from * (MAX_STAGE - 1) + to_index;
                        push_id(out, id);
                    }
                }
            }
            push_id(out, PASS_ACTION_ID);
        }
        DecisionKind::Climax => {
            let p = &state.players[player];
            if curriculum.enable_climax_phase {
                for (hand_index, card_inst) in p.hand.iter().enumerate() {
                    if hand_index >= MAX_HAND || hand_index > u8::MAX as usize {
                        break;
                    }
                    if let Some(card) = db.get(card_inst.id) {
                        if !card_set_allowed(card, curriculum, allowed_card_sets) {
                            continue;
                        }
                        if card.card_type == CardType::Climax
                            && curriculum.allow_climax
                            && p.climax.is_empty()
                            && meets_level_requirement(card, p.level.len())
                            && meets_color_requirement(card, p, db, curriculum)
                            && meets_cost_requirement(card, p, curriculum)
                        {
                            push_id(out, CLIMAX_PLAY_BASE + hand_index);
                        }
                    }
                }
            }
            push_id(out, PASS_ACTION_ID);
        }
        DecisionKind::AttackDeclaration => {
            if state.turn.turn_number == 0 && decision.player == state.turn.starting_player {
                push_id(out, PASS_ACTION_ID);
            } else {
                let max_slot = if curriculum.reduced_stage_mode { 1 } else { 3 };
                for slot in 0..max_slot {
                    let slot_u8 = slot as u8;
                    for attack_type in [AttackType::Frontal, AttackType::Side, AttackType::Direct] {
                        if can_declare_attack(
                            state,
                            decision.player,
                            slot_u8,
                            attack_type,
                            curriculum,
                        )
                        .is_ok()
                        {
                            let id = ATTACK_BASE + slot * 3 + attack_type_to_index(attack_type);
                            push_id(out, id);
                        }
                    }
                }
                push_id(out, PASS_ACTION_ID);
            }
        }
        DecisionKind::LevelUp => {
            if state.players[player].clock.len() >= 7 {
                for idx in 0..7 {
                    push_id(out, LEVEL_UP_BASE + idx);
                }
            }
        }
        DecisionKind::Encore => {
            let p = &state.players[player];
            let can_pay = p.stock.len() >= 3;
            for slot in 0..p.stage.len() {
                if p.stage[slot].card.is_some() && p.stage[slot].status == StageStatus::Reverse {
                    if can_pay {
                        push_id(out, ENCORE_PAY_BASE + slot);
                    }
                    push_id(out, ENCORE_DECLINE_BASE + slot);
                }
            }
        }
        DecisionKind::TriggerOrder => {
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
        DecisionKind::Choice => {
            if let Some(choice) = state.turn.choice.as_ref() {
                let total = choice.total_candidates as usize;
                let page_size = CHOICE_COUNT;
                let page_start = choice.page_start as usize;
                let safe_start = page_start.min(total);
                let page_end = total.min(safe_start + page_size);
                for idx in 0..(page_end - safe_start) {
                    push_id(out, CHOICE_BASE + idx);
                }
                if page_start >= page_size {
                    push_id(out, CHOICE_PREV_ID);
                }
                if page_start + page_size < total {
                    push_id(out, CHOICE_NEXT_ID);
                }
            }
        }
    }
    if curriculum.allow_concede {
        push_id(out, CONCEDE_ID);
    }
}

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
    let (cannot_attack, attack_cost) = if let Some(derived) = state.turn.derived_attack.as_ref() {
        let entry = derived.per_player[p][s];
        (entry.cannot_attack, entry.attack_cost)
    } else {
        (attacker_slot.cannot_attack, attacker_slot.attack_cost)
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

#[inline(always)]
pub fn legal_attack_actions_into(
    state: &GameState,
    player: u8,
    curriculum: &CurriculumConfig,
    actions: &mut LegalActions,
) {
    if state.turn.turn_number == 0 && player == state.turn.starting_player {
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

#[inline(always)]
pub fn legal_actions(
    state: &GameState,
    decision: &Decision,
    db: &CardDb,
    curriculum: &CurriculumConfig,
) -> LegalActions {
    legal_actions_cached(state, decision, db, curriculum, None)
}

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

#[inline(always)]
pub fn legal_actions_cached_into(
    state: &GameState,
    decision: &Decision,
    db: &CardDb,
    curriculum: &CurriculumConfig,
    allowed_card_sets: Option<&HashSet<String>>,
    actions: &mut LegalActions,
) {
    let player = decision.player as usize;
    actions.clear();
    match decision.kind {
        DecisionKind::Mulligan => {
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
        DecisionKind::Clock => {
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
        DecisionKind::Main => {
            let p = &state.players[player];
            let max_slot = if curriculum.reduced_stage_mode {
                1
            } else {
                MAX_STAGE
            };
            for (hand_index, card_inst) in p.hand.iter().enumerate() {
                if hand_index >= MAX_HAND || hand_index > u8::MAX as usize {
                    break;
                }
                if let Some(card) = db.get(card_inst.id) {
                    if !card_set_allowed(card, curriculum, allowed_card_sets) {
                        continue;
                    }
                    match card.card_type {
                        CardType::Character => {
                            if curriculum.allow_character
                                && meets_level_requirement(card, p.level.len())
                                && meets_color_requirement(card, p, db, curriculum)
                                && meets_cost_requirement(card, p, curriculum)
                            {
                                for slot in 0..max_slot {
                                    actions.push(ActionDesc::MainPlayCharacter {
                                        hand_index: hand_index as u8,
                                        stage_slot: slot as u8,
                                    });
                                }
                            }
                        }
                        CardType::Event => {
                            if curriculum.allow_event
                                && meets_level_requirement(card, p.level.len())
                                && meets_color_requirement(card, p, db, curriculum)
                                && meets_cost_requirement(card, p, curriculum)
                            {
                                actions.push(ActionDesc::MainPlayEvent {
                                    hand_index: hand_index as u8,
                                });
                            }
                        }
                        CardType::Climax => {
                            // Climax cards are played in the Climax phase.
                        }
                    }
                }
            }
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
                    {
                        actions.push(ActionDesc::MainMove {
                            from_slot: from as u8,
                            to_slot: to as u8,
                        });
                    }
                }
            }
            actions.push(ActionDesc::Pass);
        }
        DecisionKind::Climax => {
            let p = &state.players[player];
            if curriculum.enable_climax_phase {
                for (hand_index, card_inst) in p.hand.iter().enumerate() {
                    if hand_index >= MAX_HAND || hand_index > u8::MAX as usize {
                        break;
                    }
                    if let Some(card) = db.get(card_inst.id) {
                        if !card_set_allowed(card, curriculum, allowed_card_sets) {
                            continue;
                        }
                        if card.card_type == CardType::Climax
                            && curriculum.allow_climax
                            && p.climax.is_empty()
                            && meets_level_requirement(card, p.level.len())
                            && meets_color_requirement(card, p, db, curriculum)
                            && meets_cost_requirement(card, p, curriculum)
                        {
                            actions.push(ActionDesc::ClimaxPlay {
                                hand_index: hand_index as u8,
                            });
                        }
                    }
                }
            }
            actions.push(ActionDesc::Pass);
        }
        DecisionKind::AttackDeclaration => {
            legal_attack_actions_into(state, decision.player, curriculum, actions);
            actions.push(ActionDesc::Pass);
        }
        DecisionKind::LevelUp => {
            if state.players[player].clock.len() >= 7 {
                actions.extend((0..7).map(|idx| ActionDesc::LevelUp { index: idx }));
            }
        }
        DecisionKind::Encore => {
            let p = &state.players[player];
            let can_pay = p.stock.len() >= 3;
            for slot in 0..p.stage.len() {
                if p.stage[slot].card.is_some() && p.stage[slot].status == StageStatus::Reverse {
                    if can_pay {
                        actions.push(ActionDesc::EncorePay { slot: slot as u8 });
                    }
                    actions.push(ActionDesc::EncoreDecline { slot: slot as u8 });
                }
            }
        }
        DecisionKind::TriggerOrder => {
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
        DecisionKind::Choice => {
            if let Some(choice) = state.turn.choice.as_ref() {
                let total = choice.total_candidates as usize;
                let page_size = crate::encode::CHOICE_COUNT;
                let page_start = choice.page_start as usize;
                let safe_start = page_start.min(total);
                let page_end = total.min(safe_start + page_size);
                for idx in 0..(page_end - safe_start) {
                    actions.push(ActionDesc::ChoiceSelect { index: idx as u8 });
                }
                if page_start >= page_size {
                    actions.push(ActionDesc::ChoicePrevPage);
                }
                if page_start + page_size < total {
                    actions.push(ActionDesc::ChoiceNextPage);
                }
            }
        }
    }
    if curriculum.allow_concede {
        actions.push(ActionDesc::Concede);
    }
}

fn card_set_allowed(
    card: &CardStatic,
    curriculum: &CurriculumConfig,
    allowed_card_sets: Option<&HashSet<String>>,
) -> bool {
    match (allowed_card_sets, &card.card_set) {
        (Some(set), Some(set_id)) => set.contains(set_id),
        (Some(_), None) => false,
        (None, _) => {
            if curriculum.allowed_card_sets.is_empty() {
                true
            } else {
                card.card_set
                    .as_ref()
                    .map(|s| curriculum.allowed_card_sets.iter().any(|a| a == s))
                    .unwrap_or(false)
            }
        }
    }
}

fn meets_level_requirement(card: &CardStatic, level_count: usize) -> bool {
    card.level as usize <= level_count
}

fn meets_cost_requirement(
    card: &CardStatic,
    player: &crate::state::PlayerState,
    curriculum: &CurriculumConfig,
) -> bool {
    if !curriculum.enforce_cost_requirement {
        return true;
    }
    player.stock.len() >= card.cost as usize
}

fn meets_color_requirement(
    card: &CardStatic,
    player: &crate::state::PlayerState,
    db: &CardDb,
    curriculum: &CurriculumConfig,
) -> bool {
    if !curriculum.enforce_color_requirement {
        return true;
    }
    if card.level == 0 || card.color == CardColor::Colorless {
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

fn is_character_slot(slot: &StageSlot, db: &CardDb) -> bool {
    slot.card
        .and_then(|inst| db.get(inst.id))
        .map(|c| c.card_type == CardType::Character)
        .unwrap_or(false)
}
