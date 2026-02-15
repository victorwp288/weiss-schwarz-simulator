use super::super::{EngineErrorCode, GameEnv};
use crate::db::AbilityTiming;
use crate::db::CardType;
use crate::events::Zone;

impl GameEnv {
    pub(in crate::env) fn resolve_rule_actions_until_stable(&mut self) {
        loop {
            if self.state.terminal.is_some() {
                return;
            }
            if self.state.turn.pending_level_up.is_some() {
                return;
            }
            let mut progressed = false;
            for player in 0..2u8 {
                let p = player as usize;
                if self.state.players[p].deck.is_empty() && self.state.turn.cost_payment_depth == 0
                {
                    if self.state.players[p].waiting_room.is_empty() {
                        self.register_loss(player);
                        progressed = true;
                    } else {
                        self.refresh_deck(player);
                        progressed = true;
                    }
                }
                if self.state.players[p].clock.len() >= 7 {
                    if self.curriculum.enable_level_up_choice {
                        if self.state.turn.pending_level_up.is_none() {
                            self.state.turn.pending_level_up = Some(player);
                            return;
                        }
                    } else {
                        match self.resolve_level_up(player, 0) {
                            Ok(()) => {
                                progressed = true;
                            }
                            Err(err) => {
                                self.last_engine_error = true;
                                self.last_engine_error_code = EngineErrorCode::ActionError;
                                eprintln!("resolve_level_up failed during rule actions: {err}");
                                return;
                            }
                        }
                    }
                }

                let mut slot_idx = 0usize;
                while slot_idx < self.state.players[p].stage.len() {
                    let card = self.state.players[p].stage[slot_idx].card;
                    if let Some(card) = card {
                        let is_character = self
                            .db
                            .get(card.id)
                            .map(|c| c.card_type == CardType::Character)
                            .unwrap_or(false);
                        if !is_character {
                            self.send_stage_to_waiting_room(player, slot_idx as u8);
                            progressed = true;
                        }
                    }
                    slot_idx += 1;
                }
                let mut slot_idx = 0usize;
                while slot_idx < self.state.players[p].stage.len() {
                    if self.state.players[p].stage[slot_idx].card.is_some() {
                        let power = self.compute_slot_power(p, slot_idx);
                        if power <= 0 {
                            self.send_stage_to_waiting_room(player, slot_idx as u8);
                            progressed = true;
                        }
                    }
                    slot_idx += 1;
                }

                let mut idx = 0usize;
                while idx < self.state.players[p].climax.len() {
                    let card = self.state.players[p].climax[idx];
                    let is_climax = self
                        .db
                        .get(card.id)
                        .map(|c| c.card_type == CardType::Climax)
                        .unwrap_or(false);
                    if !is_climax {
                        let card = self.state.players[p].climax.remove(idx);
                        self.move_card_between_zones(
                            player,
                            card,
                            Zone::Climax,
                            Zone::WaitingRoom,
                            None,
                            None,
                        );
                        progressed = true;
                    } else {
                        idx += 1;
                    }
                }
                if self.state.players[p].climax.len() > 1 {
                    let Some(last) = self.state.players[p].climax.pop() else {
                        continue;
                    };
                    let extra = std::mem::take(&mut self.state.players[p].climax);
                    for card in extra {
                        self.move_card_between_zones(
                            player,
                            card,
                            Zone::Climax,
                            Zone::WaitingRoom,
                            None,
                            None,
                        );
                    }
                    self.state.players[p].climax.push(last);
                    progressed = true;
                }
            }
            if !progressed {
                break;
            }
        }
    }

    pub(in crate::env) fn run_check_timing(&mut self, timing: AbilityTiming) {
        self.resolve_rule_actions_until_stable();
        if self.state.turn.pending_level_up.is_some() {
            return;
        }
        self.queue_timing_triggers(timing);
        self.resolve_quiescence_until_decision();
    }
}
