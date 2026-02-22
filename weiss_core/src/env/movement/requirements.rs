use super::super::GameEnv;
use crate::db::{AbilityDefConditions, AbilityKind, CardStatic, CardType, ConditionTurn};
use crate::events::Zone;
use crate::legal::hand_play_requirements::{
    meets_color_requirement as shared_meets_color_requirement,
    meets_cost_requirement as shared_meets_cost_requirement,
    meets_level_requirement as shared_meets_level_requirement,
};
use anyhow::{anyhow, Result};

impl GameEnv {
    fn hand_play_conditions_met(&self, player: u8, conditions: &AbilityDefConditions) -> bool {
        if conditions.requires_approx_effects && !self.curriculum.enable_approx_effects {
            return false;
        }
        if let Some(turn) = conditions.turn {
            let is_self_turn = self.state.turn.active_player == player;
            match turn {
                ConditionTurn::SelfTurn if !is_self_turn => return false,
                ConditionTurn::OpponentTurn if is_self_turn => return false,
                _ => {}
            }
        }
        if let Some(max_climax) = conditions.self_waiting_room_climax_at_most {
            let climax_count = self.state.players[player as usize]
                .waiting_room
                .iter()
                .filter(|card_inst| {
                    self.db
                        .get(card_inst.id)
                        .map(|card| card.card_type == CardType::Climax)
                        .unwrap_or(false)
                })
                .count();
            if climax_count > max_climax as usize {
                return false;
            }
        }
        if !conditions.self_clock_card_ids_any.is_empty() {
            let has_match = self.state.players[player as usize]
                .clock
                .iter()
                .any(|card_inst| conditions.self_clock_card_ids_any.contains(&card_inst.id));
            if !has_match {
                return false;
            }
        }
        if let Some(max_memory) = conditions.self_memory_at_most {
            if self.state.players[player as usize].memory.len() > max_memory as usize {
                return false;
            }
        }
        if let Some(min_level) = conditions.opponent_stage_has_level_at_least {
            let player_idx = player as usize;
            if player_idx >= self.state.players.len() {
                return false;
            }
            let opponent = player_idx ^ 1;
            if opponent >= self.state.players.len() {
                return false;
            }
            let has_level =
                self.state.players[opponent]
                    .stage
                    .iter()
                    .enumerate()
                    .any(|(slot_idx, slot)| {
                        slot.card.is_some()
                            && self.compute_slot_level(opponent, slot_idx) >= i32::from(min_level)
                    });
            if !has_level {
                return false;
            }
        }
        true
    }

    fn hand_play_level_delta(&self, player: u8, card: &CardStatic) -> i32 {
        let mut delta = 0i32;
        for def in &card.ability_defs {
            if def.kind != AbilityKind::Continuous {
                continue;
            }
            if def.conditions.hand_level_delta == 0 {
                continue;
            }
            if !self.hand_play_conditions_met(player, &def.conditions) {
                continue;
            }
            delta = delta.saturating_add(def.conditions.hand_level_delta as i32);
        }
        delta
    }

    fn hand_play_ignores_color_requirement(&self, player: u8, card: &CardStatic) -> bool {
        card.ability_defs.iter().any(|def| {
            def.kind == AbilityKind::Continuous
                && def.conditions.ignore_color_requirement
                && self.hand_play_conditions_met(player, &def.conditions)
        })
    }

    pub(in crate::env) fn meets_level_requirement(&self, player: u8, card: &CardStatic) -> bool {
        shared_meets_level_requirement(
            card,
            self.state.players[player as usize].level.len(),
            self.hand_play_level_delta(player, card),
        )
    }

    pub(in crate::env) fn meets_cost_requirement(&self, player: u8, card: &CardStatic) -> bool {
        shared_meets_cost_requirement(
            card,
            self.state.players[player as usize].stock.len(),
            self.curriculum.enforce_cost_requirement,
        )
    }

    pub(in crate::env) fn meets_color_requirement(&self, player: u8, card: &CardStatic) -> bool {
        shared_meets_color_requirement(
            card,
            &self.state.players[player as usize],
            &self.db,
            self.curriculum.enforce_color_requirement,
            self.hand_play_ignores_color_requirement(player, card),
        )
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
