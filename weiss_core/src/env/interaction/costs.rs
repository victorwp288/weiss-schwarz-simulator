use super::super::GameEnv;
use crate::db::{AbilityCost, AbilitySpec};
use crate::events::{Event, RevealAudience, RevealReason, Zone};
use crate::state::{
    CardInstance, ChoiceOptionRef, ChoiceReason, ChoiceZone, CostPaymentState, CostStepKind,
    StageStatus, TargetRef, TargetZone,
};
use anyhow::{anyhow, Result};

impl GameEnv {
    pub(in crate::env) fn ability_cost_for_spec(&self, spec: &AbilitySpec) -> AbilityCost {
        spec.template.activation_cost_spec()
    }

    pub(in crate::env) fn can_pay_ability_cost(
        &self,
        player: u8,
        slot: u8,
        source: CardInstance,
        cost: AbilityCost,
    ) -> bool {
        let p = player as usize;
        if cost.rest_self {
            let slot_idx = slot as usize;
            if slot_idx >= self.state.players[p].stage.len() {
                return false;
            }
            let slot_state = &self.state.players[p].stage[slot_idx];
            if slot_state.card.map(|c| c.instance_id) != Some(source.instance_id) {
                return false;
            }
            if slot_state.status != StageStatus::Stand {
                return false;
            }
        }
        if cost.rest_other > 0 {
            let mut available = 0usize;
            for (idx, slot_state) in self.state.players[p].stage.iter().enumerate() {
                if idx == slot as usize {
                    continue;
                }
                if slot_state.card.is_some() && slot_state.status == StageStatus::Stand {
                    available += 1;
                }
            }
            if available < cost.rest_other as usize {
                return false;
            }
        }
        if cost.stock > 0
            && self.curriculum.enforce_cost_requirement
            && self.state.players[p].stock.len() < cost.stock as usize
        {
            return false;
        }
        let required_hand = cost.discard_from_hand as usize
            + cost.clock_from_hand as usize
            + cost.reveal_from_hand as usize;
        if required_hand > self.state.players[p].hand.len() {
            return false;
        }
        if cost.clock_from_deck_top > 0
            && self.state.players[p].deck.len() < cost.clock_from_deck_top as usize
        {
            return false;
        }
        true
    }

    pub(in crate::env) fn pay_ability_cost_immediate(
        &mut self,
        player: u8,
        slot: u8,
        source: CardInstance,
        cost: &mut AbilityCost,
    ) -> Result<()> {
        let p = player as usize;
        if cost.rest_self {
            let slot_idx = slot as usize;
            if slot_idx >= self.state.players[p].stage.len() {
                return Err(anyhow!("Cost rest slot out of range"));
            }
            let slot_state = &mut self.state.players[p].stage[slot_idx];
            if slot_state.card.map(|c| c.instance_id) != Some(source.instance_id) {
                return Err(anyhow!("Cost rest target mismatch"));
            }
            if slot_state.status != StageStatus::Stand {
                return Err(anyhow!("Cost rest requires stand"));
            }
            slot_state.status = StageStatus::Rest;
            self.mark_slot_power_dirty(player, slot);
            self.mark_continuous_modifiers_dirty();
            cost.rest_self = false;
        }
        if cost.stock > 0 && self.curriculum.enforce_cost_requirement {
            self.pay_cost(player, cost.stock as usize)?;
            cost.stock = 0;
        } else {
            cost.stock = 0;
        }
        Ok(())
    }

    pub(in crate::env) fn next_cost_step(cost: &AbilityCost) -> Option<CostStepKind> {
        if cost.rest_other > 0 {
            Some(CostStepKind::RestOther)
        } else if cost.discard_from_hand > 0 {
            Some(CostStepKind::DiscardFromHand)
        } else if cost.clock_from_hand > 0 {
            Some(CostStepKind::ClockFromHand)
        } else if cost.clock_from_deck_top > 0 {
            Some(CostStepKind::ClockFromDeckTop)
        } else if cost.reveal_from_hand > 0 {
            Some(CostStepKind::RevealFromHand)
        } else {
            None
        }
    }

    pub(in crate::env) fn start_cost_choice(&mut self) {
        let Some(cost_state) = self.state.turn.pending_cost.as_mut() else {
            return;
        };
        let Some(step) = Self::next_cost_step(&cost_state.remaining) else {
            let cost_state = self.state.turn.pending_cost.take();
            if let Some(cost_state) = cost_state {
                self.finish_cost_payment(cost_state);
            }
            return;
        };
        cost_state.current_step = Some(step);
        let player = cost_state.controller;
        match step {
            CostStepKind::RestOther => {
                self.scratch.choice_options.clear();
                let source_slot = cost_state.source_slot;
                let p = player as usize;
                for (idx, slot_state) in self.state.players[p].stage.iter().enumerate() {
                    if Some(idx as u8) == source_slot {
                        continue;
                    }
                    let Some(card_inst) = slot_state.card else {
                        continue;
                    };
                    if slot_state.status != StageStatus::Stand {
                        continue;
                    }
                    if idx > u8::MAX as usize {
                        break;
                    }
                    self.scratch.choice_options.push(ChoiceOptionRef {
                        card_id: card_inst.id,
                        instance_id: card_inst.instance_id,
                        zone: ChoiceZone::Stage,
                        index: Some(idx as u16),
                        target_slot: None,
                    });
                }
                let options = std::mem::take(&mut self.scratch.choice_options);
                let _ = self.start_choice(ChoiceReason::CostPayment, player, options, None);
            }
            CostStepKind::ClockFromDeckTop => {
                if let Some(card) = self.draw_from_deck(player) {
                    let card_id = card.id;
                    self.move_card_between_zones(player, card, Zone::Deck, Zone::Clock, None, None);
                    self.log_event(Event::Clock {
                        player,
                        card: Some(card_id),
                    });
                }
                if let Some(cost_state) = self.state.turn.pending_cost.as_mut() {
                    cost_state.remaining.clock_from_deck_top =
                        cost_state.remaining.clock_from_deck_top.saturating_sub(1);
                    cost_state.current_step = None;
                }
                self.start_cost_choice();
            }
            _ => {
                self.scratch.choice_options.clear();
                for (idx, card_inst) in self.state.players[player as usize].hand.iter().enumerate()
                {
                    if idx > u8::MAX as usize {
                        break;
                    }
                    self.scratch.choice_options.push(ChoiceOptionRef {
                        card_id: card_inst.id,
                        instance_id: card_inst.instance_id,
                        zone: ChoiceZone::Hand,
                        index: Some(idx as u16),
                        target_slot: None,
                    });
                }
                let options = std::mem::take(&mut self.scratch.choice_options);
                let _ = self.start_choice(ChoiceReason::CostPayment, player, options, None);
            }
        }
    }

    pub(in crate::env) fn apply_cost_payment_choice(
        &mut self,
        player: u8,
        option: ChoiceOptionRef,
    ) {
        let Some(mut cost_state) = self.state.turn.pending_cost.take() else {
            return;
        };
        if cost_state.controller != player {
            self.state.turn.pending_cost = Some(cost_state);
            return;
        }
        let Some(step) = cost_state.current_step else {
            self.state.turn.pending_cost = Some(cost_state);
            return;
        };
        let p = player as usize;
        match step {
            CostStepKind::RestOther => {
                if option.zone != ChoiceZone::Stage {
                    self.state.turn.pending_cost = Some(cost_state);
                    return;
                }
                let Some(index) = option.index else {
                    self.state.turn.pending_cost = Some(cost_state);
                    return;
                };
                let idx = index as usize;
                let Ok(index_u8) = u8::try_from(index) else {
                    self.state.turn.pending_cost = Some(cost_state);
                    return;
                };
                if idx >= self.state.players[p].stage.len() {
                    self.state.turn.pending_cost = Some(cost_state);
                    return;
                }
                let Some(card_inst) = self.state.players[p].stage[idx].card else {
                    self.state.turn.pending_cost = Some(cost_state);
                    return;
                };
                if card_inst.instance_id != option.instance_id {
                    self.state.turn.pending_cost = Some(cost_state);
                    return;
                }
                if self.state.players[p].stage[idx].status != StageStatus::Stand {
                    self.state.turn.pending_cost = Some(cost_state);
                    return;
                }
                self.state.players[p].stage[idx].status = StageStatus::Rest;
                self.mark_slot_power_dirty(player, index_u8);
                self.mark_continuous_modifiers_dirty();
                cost_state.remaining.rest_other = cost_state.remaining.rest_other.saturating_sub(1);
            }
            CostStepKind::DiscardFromHand => {
                if option.zone != ChoiceZone::Hand {
                    self.state.turn.pending_cost = Some(cost_state);
                    return;
                }
                let Some(index) = option.index else {
                    self.state.turn.pending_cost = Some(cost_state);
                    return;
                };
                let idx = index as usize;
                let Ok(index_u8) = u8::try_from(index) else {
                    self.state.turn.pending_cost = Some(cost_state);
                    return;
                };
                if idx >= self.state.players[p].hand.len() {
                    self.state.turn.pending_cost = Some(cost_state);
                    return;
                }
                let card_inst = self.state.players[p].hand[idx];
                if card_inst.instance_id != option.instance_id {
                    self.state.turn.pending_cost = Some(cost_state);
                    return;
                }
                let card = self.state.players[p].hand.remove(idx);
                self.move_card_between_zones(
                    player,
                    card,
                    Zone::Hand,
                    Zone::WaitingRoom,
                    Some(index_u8),
                    None,
                );
                cost_state.remaining.discard_from_hand =
                    cost_state.remaining.discard_from_hand.saturating_sub(1);
            }
            CostStepKind::ClockFromHand => {
                if option.zone != ChoiceZone::Hand {
                    self.state.turn.pending_cost = Some(cost_state);
                    return;
                }
                let Some(index) = option.index else {
                    self.state.turn.pending_cost = Some(cost_state);
                    return;
                };
                let idx = index as usize;
                let Ok(index_u8) = u8::try_from(index) else {
                    self.state.turn.pending_cost = Some(cost_state);
                    return;
                };
                if idx >= self.state.players[p].hand.len() {
                    self.state.turn.pending_cost = Some(cost_state);
                    return;
                }
                let card_inst = self.state.players[p].hand[idx];
                if card_inst.instance_id != option.instance_id {
                    self.state.turn.pending_cost = Some(cost_state);
                    return;
                }
                let card = self.state.players[p].hand.remove(idx);
                self.move_card_between_zones(
                    player,
                    card,
                    Zone::Hand,
                    Zone::Clock,
                    Some(index_u8),
                    None,
                );
                cost_state.remaining.clock_from_hand =
                    cost_state.remaining.clock_from_hand.saturating_sub(1);
            }
            CostStepKind::RevealFromHand => {
                if option.zone != ChoiceZone::Hand {
                    self.state.turn.pending_cost = Some(cost_state);
                    return;
                }
                let Some(index) = option.index else {
                    self.state.turn.pending_cost = Some(cost_state);
                    return;
                };
                let idx = index as usize;
                if idx >= self.state.players[p].hand.len() {
                    self.state.turn.pending_cost = Some(cost_state);
                    return;
                }
                let card_inst = self.state.players[p].hand[idx];
                if card_inst.instance_id != option.instance_id {
                    self.state.turn.pending_cost = Some(cost_state);
                    return;
                }
                let card = self.state.players[p].hand[idx];
                self.reveal_card(
                    player,
                    &card,
                    RevealReason::AbilityEffect,
                    RevealAudience::Public,
                );
                cost_state.remaining.reveal_from_hand =
                    cost_state.remaining.reveal_from_hand.saturating_sub(1);
            }
            CostStepKind::ClockFromDeckTop => {
                self.state.turn.pending_cost = Some(cost_state);
                return;
            }
        }
        cost_state.current_step = None;
        self.state.turn.pending_cost = Some(cost_state);
        self.start_cost_choice();
    }

    pub(in crate::env) fn finish_cost_payment(&mut self, cost_state: CostPaymentState) {
        self.state.turn.cost_payment_depth = self.state.turn.cost_payment_depth.saturating_sub(1);
        let effects: Vec<_> = self
            .db
            .compiled_effects_for_ability(cost_state.source_id, cost_state.ability_index as usize)
            .to_vec();
        let source_ref = cost_state.source_slot.and_then(|slot| {
            let p = cost_state.controller as usize;
            let slot_state = self.state.players[p].stage.get(slot as usize)?;
            let card_inst = slot_state.card?;
            if card_inst.instance_id != cost_state.source_instance_id {
                return None;
            }
            Some(TargetRef {
                player: cost_state.controller,
                zone: TargetZone::Stage,
                index: slot,
                card_id: card_inst.id,
                instance_id: card_inst.instance_id,
            })
        });
        for effect in effects {
            self.enqueue_effect_spec_with_source(
                cost_state.controller,
                cost_state.source_id,
                effect.clone(),
                source_ref,
            );
        }
        let mut grant_priority = None;
        if let Some(priority) = &mut self.state.turn.priority {
            if priority.holder == cost_state.controller {
                priority.holder = 1 - cost_state.controller;
                priority.passes = 0;
                grant_priority = Some((priority.window, priority.holder));
            }
        }
        if let Some((window, player)) = grant_priority {
            self.log_event(Event::PriorityGranted { window, player });
        }
    }
}
