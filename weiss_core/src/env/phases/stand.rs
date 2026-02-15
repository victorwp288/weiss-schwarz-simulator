use super::super::GameEnv;
use crate::events::Event;
use crate::state::StageStatus;

impl GameEnv {
    pub(in crate::env) fn resolve_stand_phase(&mut self, player: u8) {
        let p = player as usize;
        for slot_idx in 0..self.state.players[p].stage.len() {
            let has_card = self.state.players[p].stage[slot_idx].card.is_some();
            let blocked = has_card
                && self.slot_has_active_modifier_kind(
                    player,
                    slot_idx as u8,
                    crate::state::ModifierKind::CannotStandDuringStandPhase,
                );
            let slot = &mut self.state.players[p].stage[slot_idx];
            if has_card && !blocked {
                slot.status = StageStatus::Stand;
            }
            if has_card {
                slot.has_attacked = false;
            }
            slot.power_mod_battle = 0;
        }
        self.mark_player_slot_power_dirty(player);
        self.mark_continuous_modifiers_dirty();
        self.log_event(Event::Stand { player });
    }
}
