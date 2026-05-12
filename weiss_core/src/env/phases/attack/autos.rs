use super::*;

impl GameEnv {
    pub(super) fn enqueue_attack_auto_effects(
        &mut self,
        ctx: &AttackContext,
        attacker: u8,
        phase: AttackAutoResolvePhase,
    ) {
        let attacker_slot = ctx.attacker_slot as usize;
        if let Some(card_inst) = self.state.players[attacker as usize].stage[attacker_slot].card {
            let card_id = card_inst.id;
            let source_ref = Some(TargetRef {
                player: attacker,
                zone: TargetZone::Stage,
                index: ctx.attacker_slot,
                card_id,
                instance_id: card_inst.instance_id,
            });
            let db = self.db.clone();
            if db.get(card_id).is_none() {
                return;
            }
            let total_abilities = self.live_stage_ability_count(attacker, ctx.attacker_slot);
            for ability_index in 0..total_abilities {
                let Some(live) =
                    self.live_stage_ability_at(attacker, ctx.attacker_slot, ability_index)
                else {
                    continue;
                };
                let spec = live.spec.clone();
                let live_effects: Vec<_> = live.effects.to_vec();
                let live_grant_id = live.grant_id;
                if spec.kind != AbilityKind::Auto {
                    continue;
                }
                if spec.timing() == Some(crate::db::AbilityTiming::AttackDeclaration) {
                    if !self.auto_ability_conditions_met(attacker, card_id, &spec) {
                        continue;
                    }
                    let has_trigger_step_effect = live_effects.iter().any(|effect| {
                        matches!(effect.kind, EffectKind::SetTriggerCheckCount { .. })
                    });
                    let should_resolve = match phase {
                        AttackAutoResolvePhase::TriggerStep => has_trigger_step_effect,
                        AttackAutoResolvePhase::DamageStep => !has_trigger_step_effect,
                    };
                    if !should_resolve {
                        continue;
                    }
                    match phase {
                        AttackAutoResolvePhase::TriggerStep => {
                            let cost = self.ability_cost_for_spec(&spec);
                            if !cost.is_empty() && live_grant_id.is_none() {
                                let Ok(ability_index_u8) = u8::try_from(ability_index) else {
                                    continue;
                                };
                                let _ = self.resolve_trigger_auto_ability_with_cost(
                                    attacker,
                                    card_id,
                                    ability_index_u8,
                                );
                            } else if cost.is_empty() {
                                for effect in &live_effects {
                                    self.enqueue_effect_spec_with_source(
                                        attacker,
                                        card_id,
                                        effect.clone(),
                                        source_ref,
                                    );
                                }
                            }
                        }
                        AttackAutoResolvePhase::DamageStep => {
                            for effect in &live_effects {
                                self.enqueue_effect_spec_with_source(
                                    attacker,
                                    card_id,
                                    effect.clone(),
                                    source_ref,
                                );
                            }
                        }
                    }
                }
            }
        }
    }

    pub(super) fn enqueue_stage_auto_effects_for_timing(
        &mut self,
        player: u8,
        slot: u8,
        timing: crate::db::AbilityTiming,
    ) {
        let slot_idx = slot as usize;
        let Some(card_inst) = self.state.players[player as usize].stage[slot_idx].card else {
            return;
        };
        let card_id = card_inst.id;
        let source_ref = Some(TargetRef {
            player,
            zone: TargetZone::Stage,
            index: slot,
            card_id,
            instance_id: card_inst.instance_id,
        });
        let db = self.db.clone();
        if db.get(card_id).is_none() {
            return;
        }
        let total_abilities = self.live_stage_ability_count(player, slot);
        for ability_index in 0..total_abilities {
            let Some(live) = self.live_stage_ability_at(player, slot, ability_index) else {
                continue;
            };
            let spec = live.spec.clone();
            let live_effects: Vec<_> = live.effects.to_vec();
            if spec.kind != AbilityKind::Auto || spec.timing() != Some(timing) {
                continue;
            }
            if !self.auto_ability_conditions_met(player, card_id, &spec) {
                continue;
            }
            let AbilityTemplate::AbilityDef(def) = &spec.template else {
                continue;
            };
            if !def.cost.is_empty() {
                continue;
            }
            for effect in &live_effects {
                self.enqueue_effect_spec_with_source(player, card_id, effect.clone(), source_ref);
            }
        }
    }

    pub(super) fn enqueue_other_attack_declaration_auto_effects(
        &mut self,
        ctx: &AttackContext,
        attacker: u8,
    ) {
        let max_slot = if self.curriculum.reduced_stage_mode {
            1
        } else {
            crate::encode::MAX_STAGE
        };
        for slot in 0..max_slot {
            if slot == ctx.attacker_slot as usize {
                continue;
            }
            if slot > u8::MAX as usize {
                break;
            }
            self.enqueue_stage_auto_effects_for_timing(
                attacker,
                slot as u8,
                crate::db::AbilityTiming::OtherAttackDeclaration,
            );
        }
    }

    pub(super) fn enqueue_damage_canceled_auto_effects(
        &mut self,
        ctx: &AttackContext,
        attacker: u8,
        defender: u8,
    ) {
        self.enqueue_stage_auto_effects_for_timing(
            attacker,
            ctx.attacker_slot,
            crate::db::AbilityTiming::DamageDealtCanceled,
        );
        if let Some(def_slot) = ctx.defender_slot {
            self.enqueue_stage_auto_effects_for_timing(
                defender,
                def_slot,
                crate::db::AbilityTiming::DamageReceivedCanceled,
            );
        }
    }

    pub(super) fn enqueue_damage_not_canceled_auto_effects(
        &mut self,
        ctx: &AttackContext,
        attacker: u8,
        defender: u8,
    ) {
        self.enqueue_stage_auto_effects_for_timing(
            attacker,
            ctx.attacker_slot,
            crate::db::AbilityTiming::DamageDealtNotCanceled,
        );
        if let Some(def_slot) = ctx.defender_slot {
            self.enqueue_stage_auto_effects_for_timing(
                defender,
                def_slot,
                crate::db::AbilityTiming::DamageReceivedNotCanceled,
            );
        }
    }
}
