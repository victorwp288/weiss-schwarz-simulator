use super::*;

impl GameEnv {
    #[inline]
    pub(in crate::env) fn recompute_derived_attack(&mut self) {
        let mut derived = crate::state::DerivedAttackState::new();
        for player in 0..2usize {
            let max_slot = if self.curriculum.reduced_stage_mode {
                1
            } else {
                MAX_STAGE
            };
            for slot in 0..max_slot {
                let slot_state = &self.state.players[player].stage[slot];
                let mut entry = crate::state::DerivedAttackSlot::empty();
                entry.cannot_attack = slot_state.cannot_attack;
                entry.attack_cost = slot_state.attack_cost;
                if let Some(card_inst) = slot_state.card {
                    let card_id = card_inst.id;
                    if self.db.get(card_id).is_none() {
                        derived.per_player[player][slot] = entry;
                        continue;
                    }
                    let (cannot_attack, cannot_side_attack, cannot_frontal_attack, attack_cost) =
                        collect_attack_slot_state(
                            &self.state,
                            player,
                            slot,
                            card_id,
                            entry.cannot_attack,
                            entry.attack_cost,
                        );
                    entry.cannot_attack = cannot_attack;
                    entry.cannot_side_attack = cannot_side_attack;
                    entry.cannot_frontal_attack = cannot_frontal_attack;
                    entry.attack_cost = attack_cost;
                }
                derived.per_player[player][slot] = entry;
            }
        }
        self.state.turn.derived_attack = Some(derived);
        if self.maybe_validate_state("derived_attack_recompute") {
            debug_assert!(
                self.is_fault_latched(),
                "validation failure should latch a deferred fault"
            );
        }
    }
}
