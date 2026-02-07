use super::super::GameEnv;
use crate::db::AbilityKind;
use crate::effects::{EffectId, EffectKind, EffectSourceKind, EffectSpec};
use crate::encode::MAX_ABILITIES_PER_CARD;
use crate::events::{RevealAudience, RevealReason, Zone};
use crate::state::{AttackType, CostPaymentState, TargetRef, TargetZone};
use anyhow::{anyhow, Result};

impl GameEnv {
    pub(in crate::env) fn allocate_trigger_group(&mut self) -> u32 {
        let group_id = self.state.turn.next_trigger_group_id;
        self.state.turn.next_trigger_group_id =
            self.state.turn.next_trigger_group_id.wrapping_add(1);
        group_id
    }

    pub(in crate::env) fn queue_activated_ability_stack_item(
        &mut self,
        player: u8,
        slot: u8,
        ability_index: u8,
    ) -> Result<bool> {
        if !self.curriculum.enable_activated_abilities {
            return Err(anyhow!("Activated abilities disabled"));
        }
        let p = player as usize;
        let s = slot as usize;
        if s >= self.state.players[p].stage.len() {
            return Err(anyhow!("Ability slot out of range"));
        }
        let card_inst = self.state.players[p].stage[s]
            .card
            .ok_or_else(|| anyhow!("No card in ability slot"))?;
        let card_id = card_inst.id;
        let db = self.db.clone();
        if db.get(card_id).is_none() {
            return Err(anyhow!("Card missing in db"));
        }
        let idx = ability_index as usize;
        let spec_kind = db
            .iter_card_abilities_in_canonical_order(card_id)
            .get(idx)
            .map(|spec| spec.kind);
        if idx >= MAX_ABILITIES_PER_CARD {
            return Err(anyhow!("Ability index out of range"));
        }
        let Some(spec_kind) = spec_kind else {
            return Err(anyhow!("Ability index out of range"));
        };
        if spec_kind != AbilityKind::Activated {
            return Err(anyhow!("Ability is not activated"));
        }
        let spec = db
            .iter_card_abilities_in_canonical_order(card_id)
            .get(idx)
            .ok_or_else(|| anyhow!("Ability index out of range"))?;
        let mut cost = self.ability_cost_for_spec(spec);
        if !self.can_pay_ability_cost(player, slot, card_inst, cost) {
            return Err(anyhow!("Activated ability cost not payable"));
        }
        let effects = db.compiled_effects_for_ability(card_id, idx);
        if effects.is_empty() {
            return Err(anyhow!("Activated ability has no effects"));
        }
        self.pay_ability_cost_immediate(player, slot, card_inst, &mut cost)?;
        if Self::next_cost_step(&cost).is_some() {
            self.state.turn.cost_payment_depth =
                self.state.turn.cost_payment_depth.saturating_add(1);
            self.state.turn.pending_cost = Some(CostPaymentState {
                controller: player,
                source_id: card_id,
                source_instance_id: card_inst.instance_id,
                source_slot: Some(slot),
                ability_index,
                remaining: cost,
                current_step: None,
            });
            self.start_cost_choice();
            return Ok(true);
        }
        let source_ref = Some(TargetRef {
            player,
            zone: TargetZone::Stage,
            index: slot,
            card_id,
            instance_id: card_inst.instance_id,
        });
        for effect in effects {
            self.enqueue_effect_spec_with_source(player, card_id, effect.clone(), source_ref);
        }
        Ok(false)
    }

    pub(in crate::env) fn queue_counter_stack_item(
        &mut self,
        player: u8,
        hand_index: u8,
    ) -> Result<()> {
        if !self.curriculum.enable_counters {
            return Err(anyhow!("Counters disabled"));
        }
        let Some(ctx) = &self.state.turn.attack else {
            return Err(anyhow!("No attack context for counter"));
        };
        if ctx.attack_type != AttackType::Frontal
            || ctx.defender_slot.is_none()
            || ctx.counter_played
        {
            return Err(anyhow!("Counter not allowed for this attack"));
        }
        let p = player as usize;
        let hi = hand_index as usize;
        if hi >= self.state.players[p].hand.len() {
            return Err(anyhow!("Counter hand index out of range"));
        }
        let card_inst = self.state.players[p].hand[hi];
        let card_id = card_inst.id;
        let card = self
            .db
            .get(card_id)
            .ok_or_else(|| anyhow!("Card missing in db"))?;
        if !self.card_set_allowed(card) {
            return Err(anyhow!("Card set not allowed"));
        }
        if !self.is_counter_card(card) {
            return Err(anyhow!("Card is not a counter"));
        }
        if !self.meets_level_requirement(player, card)
            || !self.meets_color_requirement(player, card)
            || !self.meets_cost_requirement(player, card)
        {
            return Err(anyhow!("Counter requirements not met"));
        }
        let power = self.counter_power(card);
        let damage_reductions = self.counter_damage_reductions(card);
        let damage_cancel = self.counter_damage_cancel(card);
        self.pay_cost(player, card.cost as usize)?;
        let card_inst = self.state.players[p].hand.remove(hi);
        let card_id = card_inst.id;
        self.reveal_card(
            player,
            &card_inst,
            RevealReason::Play,
            RevealAudience::Public,
        );
        self.move_card_between_zones(
            player,
            card_inst,
            Zone::Hand,
            Zone::Resolution,
            Some(hand_index),
            None,
        );
        self.state
            .turn
            .pending_resolution_cleanup
            .push((player, card_inst.instance_id));
        if let Some(ctx) = &mut self.state.turn.attack {
            ctx.counter_played = true;
        }
        let mut next_effect_index: u8 = 0;
        let mut alloc_effect_index = || -> Result<u8> {
            let current = next_effect_index;
            next_effect_index = next_effect_index
                .checked_add(1)
                .ok_or_else(|| anyhow!("Too many counter effects for card {}", card_id))?;
            Ok(current)
        };
        if power != 0 {
            let effect_index = alloc_effect_index()?;
            let spec = EffectSpec {
                id: EffectId::new(EffectSourceKind::Counter, card_id, 0, effect_index),
                kind: EffectKind::CounterBackup { power },
                target: None,
                optional: false,
            };
            self.enqueue_effect_spec(player, card_id, spec);
        }
        for reduce in damage_reductions {
            if reduce > 0 {
                let effect_index = alloc_effect_index()?;
                let amount = u8::try_from(reduce)
                    .map_err(|_| anyhow!("Counter damage reduction out of range"))?;
                let spec = EffectSpec {
                    id: EffectId::new(EffectSourceKind::Counter, card_id, 0, effect_index),
                    kind: EffectKind::CounterDamageReduce { amount },
                    target: None,
                    optional: false,
                };
                self.enqueue_effect_spec(player, card_id, spec);
            }
        }
        if damage_cancel {
            let effect_index = alloc_effect_index()?;
            let spec = EffectSpec {
                id: EffectId::new(EffectSourceKind::Counter, card_id, 0, effect_index),
                kind: EffectKind::CounterDamageCancel,
                target: None,
                optional: false,
            };
            self.enqueue_effect_spec(player, card_id, spec);
        }
        Ok(())
    }
}
