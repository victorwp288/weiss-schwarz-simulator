use super::super::GameEnv;
use crate::events::*;
use crate::state::*;
use anyhow::{anyhow, Result};

impl GameEnv {
    pub(in crate::env) fn resolve_level_up(&mut self, player: u8, index: u8) -> Result<()> {
        let p = player as usize;
        if self.state.players[p].clock.len() < 7 {
            return Err(anyhow!("Clock has fewer than 7 cards"));
        }
        let idx = index as usize;
        if idx >= 7 {
            return Err(anyhow!("Level up index out of range"));
        }
        let bottom: Vec<CardInstance> = self.state.players[p].clock.drain(0..7).collect();
        if bottom.len() != 7 {
            return Err(anyhow!("Clock underflow on level up"));
        }
        let chosen_id = bottom[idx].id;
        for (i, card) in bottom.into_iter().enumerate() {
            if i == idx {
                self.move_card_between_zones(player, card, Zone::Clock, Zone::Level, None, None);
            } else {
                self.move_card_between_zones(
                    player,
                    card,
                    Zone::Clock,
                    Zone::WaitingRoom,
                    None,
                    None,
                );
            }
        }
        self.log_event(Event::LevelUpChoice {
            player,
            card: chosen_id,
        });
        self.state.turn.pending_level_up = None;
        if self.state.players[p].level.len() >= 4 {
            self.register_loss(player);
        }
        self.check_level_up(player);
        Ok(())
    }

    pub(in crate::env) fn check_level_up(&mut self, player: u8) {
        let p = player as usize;
        if self.state.players[p].clock.len() < 7 {
            return;
        }
        if self.curriculum.enable_level_up_choice {
            if self.state.turn.pending_level_up.is_none() {
                self.state.turn.pending_level_up = Some(player);
            }
        } else {
            self.resolve_level_up(player, 0)
                .expect("resolve_level_up failed when clock >=7");
        }
    }
}
