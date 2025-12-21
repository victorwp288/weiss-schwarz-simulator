use serde::{Deserialize, Serialize};
use std::collections::HashSet;

use crate::config::CurriculumConfig;
use crate::db::{AbilityTemplate, CardColor, CardDb, CardStatic, CardType};
use crate::state::{AttackType, GameState, StageSlot, StageStatus};

const MAX_ABILITIES_PER_CARD: usize = 4;
const MAX_HAND: usize = 10;
const MAX_STAGE: usize = 5;

/// Player decision kinds exposed to callers.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum DecisionKind {
    Mulligan,
    Clock,
    Main,
    Climax,
    AttackDeclaration,
    Counter,
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
    MulliganKeep,
    MulliganAll,
    ClockPass,
    Clock { hand_index: u8 },
    MainPass,
    MainPlayCharacter { hand_index: u8, stage_slot: u8 },
    MainPlayEvent { hand_index: u8 },
    MainMove { from_slot: u8, to_slot: u8 },
    MainActivateAbility { slot: u8, ability_index: u8 },
    ClimaxPass,
    ClimaxPlay { hand_index: u8 },
    AttackPass,
    Attack { slot: u8, attack_type: AttackType },
    CounterPass,
    CounterPlay { hand_index: u8 },
    LevelUp { index: u8 },
    EncoreYes,
    EncoreNo,
    TriggerOrder { index: u8 },
    ChoiceSelect { index: u8 },
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

pub fn legal_attack_actions(
    state: &GameState,
    player: u8,
    curriculum: &CurriculumConfig,
) -> Vec<ActionDesc> {
    let mut actions = Vec::new();
    let max_slot = if curriculum.reduced_stage_mode { 1 } else { MAX_STAGE };
    for slot in 0..max_slot {
        let slot_u8 = slot as u8;
        for attack_type in [AttackType::Frontal, AttackType::Side, AttackType::Direct] {
            if can_declare_attack(state, player, slot_u8, attack_type, curriculum).is_ok() {
                actions.push(ActionDesc::Attack { slot: slot_u8, attack_type });
            }
        }
    }
    actions
}

pub fn legal_actions(state: &GameState, decision: &Decision, db: &CardDb, curriculum: &CurriculumConfig) -> Vec<ActionDesc> {
    legal_actions_cached(state, decision, db, curriculum, None)
}

pub fn legal_actions_cached(
    state: &GameState,
    decision: &Decision,
    db: &CardDb,
    curriculum: &CurriculumConfig,
    allowed_card_sets: Option<&HashSet<String>>,
) -> Vec<ActionDesc> {
    let player = decision.player as usize;
    match decision.kind {
        DecisionKind::Mulligan => vec![ActionDesc::MulliganKeep, ActionDesc::MulliganAll],
        DecisionKind::Clock => {
            let mut actions = Vec::new();
            actions.push(ActionDesc::ClockPass);
            let p = &state.players[player];
            for (hand_index, card_id) in p.hand.iter().enumerate() {
                if hand_index >= MAX_HAND || hand_index > u8::MAX as usize {
                    break;
                }
                if let Some(card) = db.get(*card_id) {
                    if !card_set_allowed(card, curriculum, allowed_card_sets) {
                        continue;
                    }
                    actions.push(ActionDesc::Clock { hand_index: hand_index as u8 });
                }
            }
            actions
        }
        DecisionKind::Main => {
            let mut actions = Vec::new();
            let p = &state.players[player];
            let max_slot = if curriculum.reduced_stage_mode { 1 } else { MAX_STAGE };
            for (hand_index, card_id) in p.hand.iter().enumerate() {
                if hand_index >= MAX_HAND || hand_index > u8::MAX as usize {
                    break;
                }
                if let Some(card) = db.get(*card_id) {
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
                                    if p.stage[slot].card.is_none() {
                                        actions.push(ActionDesc::MainPlayCharacter {
                                            hand_index: hand_index as u8,
                                            stage_slot: slot as u8,
                                        });
                                    }
                                }
                            }
                        }
                        CardType::Event => {
                            if curriculum.allow_event
                                && meets_level_requirement(card, p.level.len())
                                && meets_color_requirement(card, p, db, curriculum)
                                && meets_cost_requirement(card, p, curriculum)
                            {
                                actions.push(ActionDesc::MainPlayEvent { hand_index: hand_index as u8 });
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
                        && to_slot.card.is_some()
                        && is_character_slot(from_slot, db)
                        && is_character_slot(to_slot, db)
                    {
                        actions.push(ActionDesc::MainMove { from_slot: from as u8, to_slot: to as u8 });
                    }
                }
            }
            if curriculum.enable_activated_abilities {
                for slot in 0..max_slot {
                    let slot_state = &p.stage[slot];
                    if let Some(card_id) = slot_state.card {
                        if let Some(card) = db.get(card_id) {
                            for (idx, ability) in card.abilities.iter().enumerate() {
                                if idx >= MAX_ABILITIES_PER_CARD || idx > u8::MAX as usize {
                                    break;
                                }
                                if matches!(ability, AbilityTemplate::ActivatedPlaceholder) {
                                    actions.push(ActionDesc::MainActivateAbility { slot: slot as u8, ability_index: idx as u8 });
                                }
                            }
                        }
                    }
                }
            }
            actions.push(ActionDesc::MainPass);
            actions
        }
        DecisionKind::Climax => {
            let mut actions = Vec::new();
            let p = &state.players[player];
            if curriculum.enable_climax_phase {
                for (hand_index, card_id) in p.hand.iter().enumerate() {
                    if hand_index >= MAX_HAND || hand_index > u8::MAX as usize {
                        break;
                    }
                    if let Some(card) = db.get(*card_id) {
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
                            actions.push(ActionDesc::ClimaxPlay { hand_index: hand_index as u8 });
                        }
                    }
                }
            }
            actions.push(ActionDesc::ClimaxPass);
            actions
        }
        DecisionKind::AttackDeclaration => {
            let mut actions = Vec::new();
            let attacks = legal_attack_actions(state, decision.player, curriculum);
            actions.extend(attacks);
            if actions.is_empty() {
                actions.push(ActionDesc::AttackPass);
            }
            actions
        }
        DecisionKind::Counter => {
            let mut actions = Vec::new();
            let p = &state.players[player];
            actions.push(ActionDesc::CounterPass);
            if curriculum.enable_counters {
                for (hand_index, card_id) in p.hand.iter().enumerate() {
                    if hand_index >= MAX_HAND || hand_index > u8::MAX as usize {
                        break;
                    }
                    if let Some(card) = db.get(*card_id) {
                        if !card_set_allowed(card, curriculum, allowed_card_sets) {
                            continue;
                        }
                        if is_counter_card(card)
                            && meets_level_requirement(card, p.level.len())
                            && meets_color_requirement(card, p, db, curriculum)
                            && meets_cost_requirement(card, p, curriculum)
                        {
                            actions.push(ActionDesc::CounterPlay { hand_index: hand_index as u8 });
                        }
                    }
                }
            }
            actions
        }
        DecisionKind::LevelUp => {
            if state.players[player].clock.len() >= 7 {
                (0..7).map(|idx| ActionDesc::LevelUp { index: idx }).collect()
            } else {
                Vec::new()
            }
        }
        DecisionKind::Encore => {
            let mut actions = Vec::new();
            let slot = decision.focus_slot.unwrap_or(0) as usize;
            let p = &state.players[player];
            if slot < p.stage.len() && p.stage[slot].card.is_some() {
                if p.stock.len() >= 3 {
                    actions.push(ActionDesc::EncoreYes);
                }
                actions.push(ActionDesc::EncoreNo);
            }
            actions
        }
        DecisionKind::TriggerOrder => {
            let mut actions = Vec::new();
            let choices = state.turn.trigger_order.as_ref().map(|o| o.choices.len()).unwrap_or(0);
            let max = choices.min(10);
            for idx in 0..max {
                actions.push(ActionDesc::TriggerOrder { index: idx as u8 });
            }
            actions
        }
        DecisionKind::Choice => {
            let mut actions = Vec::new();
            let choices = state.turn.choice.as_ref().map(|c| c.options.len()).unwrap_or(0);
            let max = choices.min(16);
            for idx in 0..max {
                actions.push(ActionDesc::ChoiceSelect { index: idx as u8 });
            }
            actions
        }
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

fn meets_cost_requirement(card: &CardStatic, player: &crate::state::PlayerState, curriculum: &CurriculumConfig) -> bool {
    if !curriculum.enforce_cost_requirement {
        return true;
    }
    player.stock.len() >= card.cost as usize
}

fn meets_color_requirement(card: &CardStatic, player: &crate::state::PlayerState, db: &CardDb, curriculum: &CurriculumConfig) -> bool {
    if !curriculum.enforce_color_requirement {
        return true;
    }
    if card.level == 0 || card.color == CardColor::Colorless {
        return true;
    }
    for card_id in player.level.iter().chain(player.clock.iter()) {
        if let Some(c) = db.get(*card_id) {
            if c.color == card.color {
                return true;
            }
        }
    }
    false
}

fn is_character_slot(slot: &StageSlot, db: &CardDb) -> bool {
    slot.card.and_then(|id| db.get(id)).map(|c| c.card_type == CardType::Character).unwrap_or(false)
}

fn is_counter_card(card: &CardStatic) -> bool {
    if !card.counter_timing {
        return false;
    }
    card.abilities.iter().any(|a| matches!(a, AbilityTemplate::CounterBackup { .. } | AbilityTemplate::CounterDamageReduce { .. } | AbilityTemplate::CounterDamageCancel))
}
