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

    pub(super) fn add_modifier_instance(
        &mut self,
        source: CardId,
        target_player: u8,
        target_slot: u8,
        kind: ModifierKind,
        magnitude: i32,
        duration: ModifierDuration,
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
            target_player,
            target_slot,
            target_card,
            kind,
            magnitude,
            duration,
            insertion: id,
        });
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
        for id in removed {
            self.log_event(Event::ModifierRemoved {
                id,
                reason: ModifierRemoveReason::TargetLeftStage,
            });
        }
    }

    pub(super) fn resolve_on_play_abilities(&mut self, player: u8, source_id: CardId) {
        let db = self.db.clone();
        let specs = db.iter_card_abilities_in_canonical_order(source_id);
        for (ability_index, spec) in specs.iter().enumerate() {
            if spec.kind != AbilityKind::Auto {
                continue;
            }
            let timing = match &spec.template {
                AbilityTemplate::AutoOnPlayDraw { .. } => Some(crate::db::AbilityTiming::OnPlay),
                AbilityTemplate::AbilityDef(def) => def.timing,
                _ => None,
            };
            if timing == Some(crate::db::AbilityTiming::OnPlay) {
                let effects = db.compiled_effects_for_ability(source_id, ability_index);
                for effect in effects {
                    self.enqueue_effect_spec(player, source_id, effect.clone());
                }
            }
        }
    }

    pub(super) fn apply_continuous_modifiers_for_slot(&mut self, player: u8, slot: u8, card_id: CardId) {
        if !self.curriculum.enable_continuous_modifiers {
            return;
        }
        let db = self.db.clone();
        let specs = db.iter_card_abilities_in_canonical_order(card_id);
        for (idx, spec) in specs.iter().enumerate() {
            if spec.kind != AbilityKind::Continuous {
                continue;
            }
            let effects = db.compiled_effects_for_ability(card_id, idx);
            if effects.is_empty() {
                continue;
            }
            for effect in effects {
                let instance_id = self.state.players[player as usize]
                    .stage
                    .get(slot as usize)
                    .and_then(|s| s.card)
                    .map(|c| c.instance_id)
                    .unwrap_or(0);
                let targets = vec![TargetRef {
                    player,
                    zone: TargetZone::Stage,
                    index: slot,
                    card_id,
                    instance_id,
                }];
                let payload = EffectPayload {
                    spec: effect.clone(),
                    targets,
                };
                self.resolve_effect_payload(player, card_id, &payload);
            }
        }
    }
}
