use super::super::GameEnv;
use crate::db::{AbilityCost, AbilityTemplate, AbilityTiming};
use crate::events::*;
use crate::state::*;
use anyhow::{anyhow, Result};

impl GameEnv {
    fn encore_cost_sort_key(cost: &AbilityCost) -> (u8, u8, u8, u8, u8, u8, u8, u8, u8, u8) {
        (
            cost.stock,
            u8::from(cost.rest_self),
            cost.rest_other,
            cost.sacrifice_from_stage,
            cost.discard_from_hand,
            cost.clock_from_hand,
            cost.clock_from_deck_top,
            cost.reveal_from_hand,
            u8::from(cost.move_self_to_waiting_room),
            u8::from(cost.return_self_to_hand),
        )
    }

    fn encore_variant_costs(&self, player: u8, slot: u8) -> Vec<(u8, AbilityCost)> {
        let mut costs: Vec<(u8, AbilityCost)> = Vec::new();
        let count = self.live_stage_ability_count(player, slot);
        for ability_index in 0..count {
            let Some(live) = self.live_stage_ability_at(player, slot, ability_index) else {
                continue;
            };
            if live.spec.timing() != Some(AbilityTiming::Encore) {
                continue;
            }
            let cost = match &live.spec.template {
                AbilityTemplate::EncoreVariant { cost } => Some(*cost),
                AbilityTemplate::AbilityDef(def) => Some(def.cost),
                _ => None,
            };
            if let Some(cost) = cost {
                if !cost.is_empty() {
                    let Ok(ability_index_u8) = u8::try_from(ability_index) else {
                        continue;
                    };
                    costs.push((ability_index_u8, cost));
                }
            }
        }
        costs.sort_by_key(|(ability_index, cost)| {
            (Self::encore_cost_sort_key(cost), *ability_index)
        });
        costs.dedup_by_key(|(_, cost)| Self::encore_cost_sort_key(cost));
        costs
    }

    fn encore_stock_cost_override(&self, player: u8, slot: u8, card_id: u32) -> Option<u8> {
        let mut min_cost: Option<u8> = None;
        for modifier in &self.state.modifiers {
            if modifier.target_player != player || modifier.target_slot != slot {
                continue;
            }
            if modifier.target_card != card_id {
                continue;
            }
            if modifier.kind != ModifierKind::EncoreStockCost || modifier.magnitude <= 0 {
                continue;
            }
            let cost = modifier.magnitude as u8;
            min_cost = Some(match min_cost {
                Some(existing) => existing.min(cost),
                None => cost,
            });
        }
        min_cost
    }

    pub(in crate::env) fn complete_encore_keep(&mut self, player: u8, slot: u8) {
        let p = player as usize;
        let s = slot as usize;
        if p >= self.state.players.len() {
            return;
        }
        if s >= self.state.players[p].stage.len() {
            return;
        }
        let kept = {
            let slot_state = &mut self.state.players[p].stage[s];
            if slot_state.card.is_some() {
                slot_state.status = StageStatus::Rest;
                self.touch_player_obs(player);
                true
            } else {
                false
            }
        };
        if let Some(pos) = self
            .state
            .turn
            .encore_queue
            .iter()
            .position(|r| r.player == player && r.slot == slot)
        {
            self.state.turn.encore_queue.remove(pos);
        }
        if kept {
            self.log_event(Event::Encore {
                player,
                slot,
                kept: true,
            });
        }
    }

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
            if self.state.turn.cannot_use_auto_encore[p] {
                return Err(anyhow!("AUTO Encore is unavailable this turn"));
            }
            let card_inst = self.state.players[p].stage[s]
                .card
                .ok_or_else(|| anyhow!("Encore slot empty"))?;
            let stock_len = self.state.players[p].stock.len();
            let mut stock_cost = if stock_len >= 3 { Some(3u8) } else { None };
            if let Some(override_cost) = self.encore_stock_cost_override(player, slot, card_inst.id)
            {
                stock_cost = Some(match stock_cost {
                    Some(existing) => existing.min(override_cost),
                    None => override_cost,
                });
            }
            if let Some(cost) = stock_cost {
                if self.state.players[p].stock.len() >= cost as usize {
                    for _ in 0..cost {
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
                    self.complete_encore_keep(player, slot);
                    return Ok(());
                }
            }
            let Some((ability_index, mut variant_cost)) = self
                .encore_variant_costs(player, slot)
                .into_iter()
                .find(|(_, cost)| self.can_pay_ability_cost(player, slot, card_inst, *cost))
            else {
                return Err(anyhow!("Encore cost unpaid"));
            };
            if self.state.players[p].stage[s].card.is_none() {
                return Err(anyhow!("Encore slot empty"));
            }

            self.pay_ability_cost_immediate(player, slot, card_inst, &mut variant_cost)?;
            if Self::next_cost_step(&variant_cost).is_some() {
                self.state.turn.cost_payment_depth =
                    self.state.turn.cost_payment_depth.saturating_add(1);
                self.state.turn.pending_cost = Some(CostPaymentState {
                    controller: player,
                    source_id: card_inst.id,
                    source_instance_id: card_inst.instance_id,
                    source_slot: Some(slot),
                    ability_index,
                    remaining: variant_cost,
                    current_step: None,
                    outcome: CostPaymentOutcome::EncoreKeep { slot },
                });
                self.start_cost_choice();
                return Ok(());
            } else {
                if let Some(slot_state) = self.state.players[p].stage.get_mut(s) {
                    if slot_state.card.is_none() {
                        return Err(anyhow!("Encore slot empty"));
                    }
                }
                self.complete_encore_keep(player, slot);
                return Ok(());
            }
        } else {
            self.send_stage_to_waiting_room(player, slot);
            self.log_event(Event::Encore {
                player,
                slot,
                kept: false,
            });
            self.state.turn.encore_queue.remove(pos);
        }
        Ok(())
    }
}
