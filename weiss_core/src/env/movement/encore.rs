use super::super::GameEnv;
use crate::events::*;
use crate::state::*;
use anyhow::{anyhow, Result};

impl GameEnv {
    pub(in crate::env) fn resolve_encore(&mut self, player: u8, slot: u8, pay: bool) -> Result<()> {
        let p = player as usize;
        if p >= self.state.players.len() {
            return Err(anyhow!("Encore player out of range"));
        }
        let s = slot as usize;
        if s >= self.state.players[p].stage.len() {
            return Err(anyhow!("Encore slot out of range"));
        }
        if self.state.players[p].stage[s].card.is_none() {
            return Err(anyhow!("Encore slot empty"));
        }
        if self.state.players[p].stage[s].status != StageStatus::Reverse {
            return Err(anyhow!("Encore slot not reversed"));
        }
        let Some(pos) = self
            .state
            .turn
            .encore_queue
            .iter()
            .position(|r| r.player == player && r.slot == slot)
        else {
            return Err(anyhow!("Encore slot not pending"));
        };
        if pay {
            if self.state.players[p].stock.len() < 3 {
                return Err(anyhow!("Encore cost unpaid"));
            }
            for _ in 0..3 {
                if let Some(card) = self.state.players[p].stock.pop() {
                    self.move_card_between_zones(
                        player,
                        card,
                        Zone::Stock,
                        Zone::WaitingRoom,
                        None,
                        None,
                    );
                }
            }
            if let Some(slot_state) = self.state.players[p].stage.get_mut(s) {
                slot_state.status = StageStatus::Rest;
                self.touch_player_obs(player);
            }
            self.log_event(Event::Encore {
                player,
                slot,
                kept: true,
            });
        } else {
            self.send_stage_to_waiting_room(player, slot);
            self.log_event(Event::Encore {
                player,
                slot,
                kept: false,
            });
        }
        self.state.turn.encore_queue.remove(pos);
        Ok(())
    }
}
