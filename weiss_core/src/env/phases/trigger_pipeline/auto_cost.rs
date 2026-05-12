use super::*;

impl GameEnv {
    pub(super) fn trigger_auto_source_context(
        &self,
        player: u8,
        source_card: CardId,
    ) -> Option<(Option<u8>, CardInstance)> {
        let p = player as usize;
        if p < self.state.players.len() {
            for (slot_idx, slot_state) in self.state.players[p].stage.iter().enumerate() {
                let Some(card_inst) = slot_state.card else {
                    continue;
                };
                if card_inst.id == source_card {
                    let slot = if slot_idx <= u8::MAX as usize {
                        Some(slot_idx as u8)
                    } else {
                        None
                    };
                    return Some((slot, card_inst));
                }
            }
            for card_inst in &self.state.players[p].climax {
                if card_inst.id == source_card {
                    return Some((None, *card_inst));
                }
            }
        }
        None
    }

    pub(in crate::env) fn resolve_trigger_auto_ability_with_cost(
        &mut self,
        player: u8,
        source_card: CardId,
        ability_index: u8,
    ) -> bool {
        let db = self.db.clone();
        let Some(spec) = db
            .iter_card_abilities_in_canonical_order(source_card)
            .get(ability_index as usize)
        else {
            return false;
        };
        if !self.auto_ability_conditions_met(player, source_card, spec) {
            return false;
        }
        let effects = db.compiled_effects_for_ability(source_card, ability_index as usize);
        let Some((source_slot, source_inst)) =
            self.trigger_auto_source_context(player, source_card)
        else {
            return false;
        };
        let mut cost = self.ability_cost_for_spec(spec);

        if !cost.is_empty() {
            if !self.can_pay_ability_cost(player, source_slot, source_inst, &cost) {
                return false;
            }
            if self
                .pay_ability_cost_immediate(player, source_slot, source_inst, &mut cost)
                .is_err()
            {
                return false;
            }
            if Self::next_cost_step(&cost).is_some() {
                self.state.turn.cost_payment_depth =
                    self.state.turn.cost_payment_depth.saturating_add(1);
                self.state.turn.pending_cost = Some(CostPaymentState {
                    controller: player,
                    source_id: source_card,
                    source_instance_id: source_inst.instance_id,
                    source_slot,
                    ability_index,
                    remaining: cost,
                    current_step: None,
                    outcome: CostPaymentOutcome::ResolveAbility,
                });
                self.start_cost_choice();
                return true;
            }
        }

        let source_ref = source_slot.map(|slot| TargetRef {
            player,
            zone: TargetZone::Stage,
            index: slot,
            card_id: source_inst.id,
            instance_id: source_inst.instance_id,
        });
        for effect in effects {
            self.enqueue_effect_spec_with_source(player, source_card, effect.clone(), source_ref);
        }
        !effects.is_empty()
    }

    pub(in crate::env) fn present_trigger_auto_cost_choice(
        &mut self,
        trigger: PendingTrigger,
        ability_index: u8,
    ) -> bool {
        let Some(spec) = self
            .db
            .iter_card_abilities_in_canonical_order(trigger.source_card)
            .get(ability_index as usize)
        else {
            return false;
        };
        if !self.auto_ability_conditions_met(trigger.player, trigger.source_card, spec) {
            return false;
        }
        self.scratch.choice_options.clear();
        let cost = self.ability_cost_for_spec(spec);
        let can_pay_now = if cost.is_empty() {
            true
        } else {
            match self.trigger_auto_source_context(trigger.player, trigger.source_card) {
                Some((source_slot, source_inst)) => {
                    self.can_pay_ability_cost(trigger.player, source_slot, source_inst, &cost)
                }
                None => false,
            }
        };
        if can_pay_now {
            self.scratch.choice_options.push(ChoiceOptionRef {
                card_id: 0,
                instance_id: 0,
                zone: ChoiceZone::DeckTop,
                index: Some(0),
                target_slot: None,
            });
        }
        self.scratch.choice_options.push(ChoiceOptionRef {
            card_id: 0,
            instance_id: 0,
            zone: ChoiceZone::Skip,
            index: Some(1),
            target_slot: None,
        });
        let options = std::mem::take(&mut self.scratch.choice_options);
        self.start_choice(
            ChoiceReason::TriggerAutoCostSelect,
            trigger.player,
            options,
            Some(trigger),
        )
    }
}
