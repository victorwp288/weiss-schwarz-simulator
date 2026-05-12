use crate::config::CurriculumConfig;
use crate::modifier_queries::collect_attack_slot_state;
use crate::state::{AttackType, GameState, StageStatus};

use super::helpers::starting_player_first_turn_attack_used;
use super::types::{ActionDesc, LegalActions};
use super::MAX_STAGE;

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
