use super::GameEnv;
use crate::db::*;
use crate::effects::*;
use crate::events::*;
use crate::state::*;

impl GameEnv {
    pub(super) fn push_attack_damage_modifier(
        ctx: &mut AttackContext,
        kind: DamageModifierKind,
        source_id: u32,
    ) {
        let insertion = ctx.next_modifier_id;
        ctx.next_modifier_id = ctx.next_modifier_id.wrapping_add(1);
        let priority = match kind {
            DamageModifierKind::CancelNext => 0,
            DamageModifierKind::SetCancelable { .. } => 1,
            DamageModifierKind::SetAmount { .. } => 2,
            DamageModifierKind::AddAmount { .. } => 3,
        };
        let remaining = match kind {
            DamageModifierKind::AddAmount { delta } if delta < 0 => -delta,
            _ => 0,
        };
        ctx.damage_modifiers.push(DamageModifier {
            kind,
            priority,
            insertion,
            source_id,
            remaining,
            used: false,
        });
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn add_modifier_instance(
        &mut self,
        source: CardId,
        source_slot: Option<u8>,
        target_player: u8,
        target_slot: u8,
        kind: ModifierKind,
        magnitude: i32,
        duration: ModifierDuration,
        layer: ModifierLayer,
    ) -> Option<u32> {
        let p = target_player as usize;
        let s = target_slot as usize;
        if s >= self.state.players[p].stage.len() {
            return None;
        }
        let target_card = self.state.players[p].stage[s].card?.id;
        let id = self.state.next_modifier_id;
        self.state.next_modifier_id = self.state.next_modifier_id.wrapping_add(1);
        self.state.modifiers.push(crate::state::ModifierInstance {
            id,
            source,
            source_slot,
            target_player,
            target_slot,
            target_card,
            kind,
            magnitude,
            duration,
            layer,
            insertion: id,
        });
        if kind == ModifierKind::Power {
            self.mark_slot_power_dirty(target_player, target_slot);
        }
        self.log_event(Event::ModifierAdded {
            id,
            source,
            target_player,
            target_slot,
            target_card,
            kind,
            magnitude,
            duration,
        });
        Some(id)
    }

    pub(super) fn remove_modifiers_for_slot(&mut self, player: u8, slot: u8) {
        let p = player;
        let s = slot;
        let mut removed: Vec<u32> = Vec::new();
        self.state.modifiers.retain(|m| {
            if m.target_player != p || m.target_slot != s {
                return true;
            }
            removed.push(m.id);
            false
        });
        if !removed.is_empty() {
            self.mark_slot_power_dirty(player, slot);
        }
        for id in removed {
            self.log_event(Event::ModifierRemoved {
                id,
                reason: ModifierRemoveReason::TargetLeftStage,
            });
        }
    }

    pub(super) fn resolve_on_play_abilities(
        &mut self,
        player: u8,
        source_id: CardId,
        source_ref: Option<TargetRef>,
    ) {
        let db = self.db.clone();
        let specs = db.iter_card_abilities_in_canonical_order(source_id);
        for (ability_index, spec) in specs.iter().enumerate() {
            if spec.kind != AbilityKind::Auto {
                continue;
            }
            if spec.timing() == Some(crate::db::AbilityTiming::OnPlay) {
                let effects = db.compiled_effects_for_ability(source_id, ability_index);
                for effect in effects {
                    self.enqueue_effect_spec_with_source(
                        player,
                        source_id,
                        effect.clone(),
                        source_ref,
                    );
                }
            }
        }
    }

    pub(super) fn apply_continuous_modifiers_for_slot(
        &mut self,
        player: u8,
        slot: u8,
        card_id: CardId,
    ) {
        let _ = (player, slot, card_id);
        self.mark_continuous_modifiers_dirty();
    }

    pub(super) fn refresh_continuous_modifiers_if_needed(&mut self) {
        if !self.continuous_modifiers_dirty {
            return;
        }
        self.continuous_modifiers_dirty = false;
        self.recompute_continuous_modifiers();
    }

    fn continuous_modifier_key(modifier: &ModifierInstance) -> (u32, u8, u8, u32, u8, i32, u8, u8) {
        let kind = match modifier.kind {
            ModifierKind::Power => 0,
            ModifierKind::AttackCost => 1,
            ModifierKind::CannotAttack => 2,
        };
        let duration = match modifier.duration {
            ModifierDuration::UntilEndOfTurn => 0,
            ModifierDuration::WhileOnStage => 1,
        };
        let source_slot = modifier.source_slot.unwrap_or(u8::MAX);
        (
            modifier.source,
            source_slot,
            modifier.target_player,
            modifier.target_card,
            modifier.target_slot,
            modifier.magnitude,
            kind,
            duration,
        )
    }

    fn recompute_continuous_modifiers(&mut self) {
        let mut existing_continuous: Vec<ModifierInstance> = Vec::new();
        let mut non_continuous: Vec<ModifierInstance> = Vec::new();
        for modifier in self.state.modifiers.drain(..) {
            if modifier.layer == ModifierLayer::Continuous {
                existing_continuous.push(modifier);
            } else {
                non_continuous.push(modifier);
            }
        }

        let mut next_id = self.state.next_modifier_id;
        let mut new_continuous: Vec<ModifierInstance> = Vec::new();
        if self.curriculum.enable_continuous_modifiers {
            for player in 0..2u8 {
                let max_slot = if self.curriculum.reduced_stage_mode {
                    1
                } else {
                    crate::encode::MAX_STAGE
                };
                for slot in 0..max_slot {
                    let slot_state = &self.state.players[player as usize].stage[slot];
                    let Some(card_inst) = slot_state.card else {
                        continue;
                    };
                    let card_id = card_inst.id;
                    let specs = self.db.iter_card_abilities_in_canonical_order(card_id);
                    for (ability_index, spec) in specs.iter().enumerate() {
                        if spec.kind != AbilityKind::Continuous {
                            continue;
                        }
                        let effects = self.db.compiled_effects_for_ability(card_id, ability_index);
                        for effect in effects {
                            if let EffectKind::AddModifier {
                                kind,
                                magnitude,
                                duration,
                            } = effect.kind
                            {
                                let Some(target_spec) = effect.target.as_ref() else {
                                    continue;
                                };
                                self.scratch.targets.clear();
                                Self::enumerate_target_candidates_into(
                                    &self.state,
                                    &self.db,
                                    &self.curriculum,
                                    player,
                                    target_spec,
                                    &[],
                                    &mut self.scratch.targets,
                                );
                                let limit = if target_spec.count == 0 {
                                    self.scratch.targets.len()
                                } else {
                                    target_spec.count as usize
                                };
                                for target in self.scratch.targets.iter().take(limit) {
                                    new_continuous.push(ModifierInstance {
                                        id: 0,
                                        source: card_id,
                                        source_slot: Some(slot as u8),
                                        target_player: target.player,
                                        target_slot: target.index,
                                        target_card: target.card_id,
                                        kind,
                                        magnitude,
                                        duration,
                                        layer: ModifierLayer::Continuous,
                                        insertion: 0,
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }

        existing_continuous.sort_by_key(Self::continuous_modifier_key);
        new_continuous.sort_by_key(Self::continuous_modifier_key);

        let mut changed = false;
        let mut used_existing = vec![false; existing_continuous.len()];
        let mut final_continuous: Vec<ModifierInstance> = Vec::with_capacity(new_continuous.len());
        let mut existing_idx = 0usize;
        for mut modifier in new_continuous {
            let key = Self::continuous_modifier_key(&modifier);
            while existing_idx < existing_continuous.len()
                && Self::continuous_modifier_key(&existing_continuous[existing_idx]) < key
            {
                existing_idx += 1;
            }
            if existing_idx < existing_continuous.len()
                && Self::continuous_modifier_key(&existing_continuous[existing_idx]) == key
                && !used_existing[existing_idx]
            {
                let existing = &existing_continuous[existing_idx];
                modifier.id = existing.id;
                modifier.insertion = existing.insertion;
                used_existing[existing_idx] = true;
                existing_idx += 1;
            } else {
                modifier.id = next_id;
                modifier.insertion = next_id;
                next_id = next_id.wrapping_add(1);
                changed = true;
                self.log_event(Event::ModifierAdded {
                    id: modifier.id,
                    source: modifier.source,
                    target_player: modifier.target_player,
                    target_slot: modifier.target_slot,
                    target_card: modifier.target_card,
                    kind: modifier.kind,
                    magnitude: modifier.magnitude,
                    duration: modifier.duration,
                });
            }
            final_continuous.push(modifier);
        }

        for (idx, modifier) in existing_continuous.iter().enumerate() {
            if used_existing.get(idx).copied().unwrap_or(false) {
                continue;
            }
            changed = true;
            self.log_event(Event::ModifierRemoved {
                id: modifier.id,
                reason: ModifierRemoveReason::ContinuousRefresh,
            });
        }

        if next_id != self.state.next_modifier_id {
            self.state.next_modifier_id = next_id;
        }
        self.state.modifiers = non_continuous;
        self.state.modifiers.extend(final_continuous);
        if changed {
            self.mark_all_slot_power_dirty();
            self.state.turn.derived_attack = None;
        }
    }
}
