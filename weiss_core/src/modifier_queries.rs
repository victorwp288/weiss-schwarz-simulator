use crate::db::CardId;
use crate::state::{GameState, ModifierInstance, ModifierKind};

#[inline(always)]
/// Return whether `modifier` still targets the exact staged card identity.
pub(crate) fn modifier_targets_slot_card(
    modifier: &ModifierInstance,
    player: usize,
    slot: usize,
    card_id: CardId,
) -> bool {
    modifier.target_player as usize == player
        && modifier.target_slot as usize == slot
        && modifier.target_card == card_id
}

#[inline(always)]
/// Collect attack permission/cost state for one attack slot from active modifiers.
pub(crate) fn collect_attack_slot_state(
    state: &GameState,
    player: usize,
    slot: usize,
    card_id: CardId,
    base_cannot_attack: bool,
    base_attack_cost: u8,
) -> (bool, bool, bool, u8) {
    let mut cannot_attack = base_cannot_attack;
    let mut cannot_side_attack = false;
    let mut cannot_frontal_attack = false;
    let mut attack_cost = base_attack_cost;
    for modifier in &state.modifiers {
        if !modifier_targets_slot_card(modifier, player, slot, card_id) {
            continue;
        }
        match modifier.kind {
            ModifierKind::AttackCost if modifier.magnitude > 0 => {
                let cost_delta = (modifier.magnitude as u16).min(u8::MAX as u16) as u8;
                attack_cost = attack_cost.saturating_add(cost_delta);
            }
            ModifierKind::CannotAttack if modifier.magnitude != 0 => {
                cannot_attack = true;
            }
            ModifierKind::CannotSideAttack if modifier.magnitude != 0 => {
                cannot_side_attack = true;
            }
            ModifierKind::CannotFrontalAttack if modifier.magnitude != 0 => {
                cannot_frontal_attack = true;
            }
            _ => {}
        }
    }
    (
        cannot_attack,
        cannot_side_attack,
        cannot_frontal_attack,
        attack_cost,
    )
}
