use super::super::GameEnv;
use crate::db::{CardColor, CardStatic, CardType};
use crate::events::Zone;
use anyhow::{anyhow, Result};

impl GameEnv {
    pub(in crate::env) fn meets_level_requirement(&self, player: u8, card: &CardStatic) -> bool {
        card.level as usize <= self.state.players[player as usize].level.len()
    }

    pub(in crate::env) fn meets_cost_requirement(&self, player: u8, card: &CardStatic) -> bool {
        if !self.curriculum.enforce_cost_requirement {
            return true;
        }
        self.state.players[player as usize].stock.len() >= card.cost as usize
    }

    pub(in crate::env) fn meets_color_requirement(&self, player: u8, card: &CardStatic) -> bool {
        if !self.curriculum.enforce_color_requirement {
            return true;
        }
        if card.level == 0 || card.color == CardColor::Colorless {
            return true;
        }
        let p = &self.state.players[player as usize];
        for card_id in p.level.iter().chain(p.clock.iter()) {
            if let Some(c) = self.db.get(card_id.id) {
                if c.color == card.color {
                    return true;
                }
            }
        }
        false
    }

    pub(in crate::env) fn pay_cost(&mut self, player: u8, cost: usize) -> Result<()> {
        if cost == 0 {
            return Ok(());
        }
        let p = player as usize;
        if self.state.players[p].stock.len() < cost {
            return Err(anyhow!("Insufficient stock"));
        }
        self.state.turn.cost_payment_depth = self.state.turn.cost_payment_depth.saturating_add(1);
        let result = (|| {
            for _ in 0..cost {
                let card = self.state.players[p]
                    .stock
                    .pop()
                    .ok_or_else(|| anyhow!("Insufficient stock"))?;
                self.move_card_between_zones(
                    player,
                    card,
                    Zone::Stock,
                    Zone::WaitingRoom,
                    None,
                    None,
                );
            }
            Ok(())
        })();
        self.state.turn.cost_payment_depth = self.state.turn.cost_payment_depth.saturating_sub(1);
        result
    }

    pub(in crate::env) fn looks_like_event(&self, card: &CardStatic) -> bool {
        matches!(card.card_type, CardType::Event)
    }
}
