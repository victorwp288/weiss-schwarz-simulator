use super::super::GameEnv;
use crate::events::Event;
use crate::state::StageStatus;

impl GameEnv {
    pub(in crate::env) fn resolve_stand_phase(&mut self, player: u8) {
        let p = player as usize;
        for slot in &mut self.state.players[p].stage {
            if slot.card.is_some() {
                slot.status = StageStatus::Stand;
                slot.has_attacked = false;
            }
            slot.power_mod_battle = 0;
        }
        self.mark_player_slot_power_dirty(player);
        self.mark_continuous_modifiers_dirty();
        self.log_event(Event::Stand { player });
    }
}
