use super::*;

impl GameEnv {
    pub(in crate::env) fn queue_encore_requests(&mut self) {
        let mut queue = Vec::new();
        for player in 0..2 {
            for slot in 0..self.state.players[player].stage.len() {
                let slot_state = &self.state.players[player].stage[slot];
                if slot_state.card.is_some() && slot_state.status == StageStatus::Reverse {
                    queue.push(EncoreRequest {
                        player: player as u8,
                        slot: slot as u8,
                    });
                }
            }
        }
        self.state.turn.encore_queue = queue;
        self.state.turn.encore_window_done = false;
        self.state.turn.encore_begin_done = false;
        self.state.turn.encore_step_player = if self.state.turn.encore_queue.is_empty() {
            None
        } else {
            Some(self.state.turn.active_player)
        };
    }

    pub(in crate::env) fn cleanup_reversed_to_waiting_room(&mut self) {
        for player in 0..2 {
            for slot in 0..self.state.players[player].stage.len() {
                if self.state.players[player].stage[slot].status == StageStatus::Reverse {
                    self.send_stage_to_waiting_room(player as u8, slot as u8);
                }
            }
        }
    }

    pub(in crate::env) fn clear_battle_mods(&mut self) {
        for player in 0..2 {
            for slot in &mut self.state.players[player].stage {
                slot.power_mod_battle = 0;
            }
        }
        self.mark_all_slot_power_dirty();
    }
}
