use super::GameEnv;
use crate::db::*;
use crate::effects::*;
use crate::events::*;
use crate::state::*;

impl GameEnv {
    pub(in crate::env) fn bump_modifiers_version(&mut self) {
        self.modifiers_version = self.modifiers_version.wrapping_add(1);
    }

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
        self.bump_modifiers_version();
        match kind {
            ModifierKind::Power => self.mark_slot_power_dirty(target_player, target_slot),
            ModifierKind::Soul => self.touch_player_obs(target_player),
            ModifierKind::Level => self.touch_player_obs(target_player),
            ModifierKind::AttackCost
            | ModifierKind::EncoreStockCost
            | ModifierKind::CannotAttack
            | ModifierKind::CannotSideAttack
            | ModifierKind::CannotFrontalAttack => {
                self.state.turn.derived_attack = None;
                self.touch_player_obs(target_player);
            }
            ModifierKind::CannotBecomeReverse
            | ModifierKind::CannotBeChosenByOpponentEffects
            | ModifierKind::CannotMoveStagePosition
            | ModifierKind::CannotPlayEventsFromHand
            | ModifierKind::CannotPlayBackupFromHand
            | ModifierKind::CannotStandDuringStandPhase
            | ModifierKind::BattleOpponentMoveToMemoryOnReverse => {
                self.touch_player_obs(target_player);
            }
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

    pub(in crate::env) fn slot_has_active_modifier_kind(
        &self,
        player: u8,
        slot: u8,
        kind: ModifierKind,
    ) -> bool {
        let p = player as usize;
        let s = slot as usize;
        if s >= self.state.players[p].stage.len() {
            return false;
        }
        let Some(card_inst) = self.state.players[p].stage[s].card else {
            return false;
        };
        self.state.modifiers.iter().any(|modifier| {
            modifier.target_player == player
                && modifier.target_slot == slot
                && modifier.target_card == card_inst.id
                && modifier.kind == kind
                && modifier.magnitude != 0
        })
    }

    pub(super) fn remove_modifiers_for_slot(&mut self, player: u8, slot: u8) {
        let p = player;
        let s = slot;
        let target_instance_id = self.state.players[p as usize]
            .stage
            .get(s as usize)
            .and_then(|stage_slot| stage_slot.card)
            .map(|card| card.instance_id);
        let mut removed: Vec<u32> = Vec::new();
        self.state.modifiers.retain(|m| {
            if m.target_player != p || m.target_slot != s {
                return true;
            }
            removed.push(m.id);
            false
        });
        if !removed.is_empty() {
            self.bump_modifiers_version();
            self.mark_slot_power_dirty(player, slot);
        }
        if let Some(instance_id) = target_instance_id {
            self.remove_granted_abilities_for_stage_instance(player, slot, instance_id);
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
        let mut visit_live = false;
        if let Some(source) = source_ref {
            if source.zone == TargetZone::Stage {
                let total = self.live_stage_ability_count(source.player, source.index);
                for ability_index in 0..total {
                    let Some(live) =
                        self.live_stage_ability_at(source.player, source.index, ability_index)
                    else {
                        continue;
                    };
                    let spec = live.spec.clone();
                    let live_effects: Vec<_> = live.effects.to_vec();
                    let live_grant_id = live.grant_id;
                    if spec.kind != AbilityKind::Auto {
                        continue;
                    }
                    if spec.timing() != Some(crate::db::AbilityTiming::OnPlay) {
                        continue;
                    }
                    if !self.auto_ability_conditions_met(player, source_id, &spec) {
                        continue;
                    }
                    let cost = self.ability_cost_for_spec(&spec);
                    if !cost.is_empty() {
                        if live_grant_id.is_none()
                            && matches!(spec.template, crate::db::AbilityTemplate::Bond { .. })
                        {
                            let Ok(ability_index_u8) = u8::try_from(ability_index) else {
                                continue;
                            };
                            let trigger_id = self.state.turn.next_trigger_id;
                            self.state.turn.next_trigger_id =
                                self.state.turn.next_trigger_id.wrapping_add(1);
                            let trigger = PendingTrigger {
                                id: trigger_id,
                                group_id: 0,
                                player,
                                source_card: source_id,
                                effect: TriggerEffect::AutoAbility {
                                    ability_index: ability_index_u8,
                                },
                                effect_id: None,
                            };
                            if self.present_trigger_auto_cost_choice(trigger, ability_index_u8) {
                                break;
                            }
                        } else if live_grant_id.is_none() {
                            let Ok(ability_index_u8) = u8::try_from(ability_index) else {
                                continue;
                            };
                            let _ = self.resolve_trigger_auto_ability_with_cost(
                                player,
                                source_id,
                                ability_index_u8,
                            );
                            if self.state.turn.choice.is_some()
                                || self.state.turn.pending_cost.is_some()
                            {
                                break;
                            }
                        }
                        continue;
                    }
                    for effect in live_effects {
                        self.enqueue_effect_spec_with_source(
                            player,
                            source_id,
                            effect,
                            Some(source),
                        );
                    }
                }
                visit_live = true;
            }
        }
        if visit_live {
            return;
        }

        let db = self.db.clone();
        let specs = db.iter_card_abilities_in_canonical_order(source_id);
        for (ability_index, spec) in specs.iter().enumerate() {
            if spec.kind != AbilityKind::Auto {
                continue;
            }
            if spec.timing() == Some(crate::db::AbilityTiming::OnPlay) {
                if !self.auto_ability_conditions_met(player, source_id, spec) {
                    continue;
                }
                let Ok(ability_index_u8) = u8::try_from(ability_index) else {
                    continue;
                };
                let cost = self.ability_cost_for_spec(spec);
                if !cost.is_empty() {
                    if matches!(spec.template, crate::db::AbilityTemplate::Bond { .. }) {
                        let trigger_id = self.state.turn.next_trigger_id;
                        self.state.turn.next_trigger_id =
                            self.state.turn.next_trigger_id.wrapping_add(1);
                        let trigger = PendingTrigger {
                            id: trigger_id,
                            group_id: 0,
                            player,
                            source_card: source_id,
                            effect: TriggerEffect::AutoAbility {
                                ability_index: ability_index_u8,
                            },
                            effect_id: None,
                        };
                        if self.present_trigger_auto_cost_choice(trigger, ability_index_u8) {
                            break;
                        }
                        continue;
                    }
                    let _ = self.resolve_trigger_auto_ability_with_cost(
                        player,
                        source_id,
                        ability_index_u8,
                    );
                    if self.state.turn.choice.is_some() || self.state.turn.pending_cost.is_some() {
                        break;
                    }
                    continue;
                }
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
            ModifierKind::Soul => 1,
            ModifierKind::Level => 2,
            ModifierKind::AttackCost => 3,
            ModifierKind::CannotAttack => 4,
            ModifierKind::CannotSideAttack => 5,
            ModifierKind::CannotFrontalAttack => 6,
            ModifierKind::CannotBecomeReverse => 7,
            ModifierKind::CannotBeChosenByOpponentEffects => 8,
            ModifierKind::CannotMoveStagePosition => 9,
            ModifierKind::CannotPlayEventsFromHand => 10,
            ModifierKind::CannotPlayBackupFromHand => 11,
            ModifierKind::CannotStandDuringStandPhase => 12,
            ModifierKind::BattleOpponentMoveToMemoryOnReverse => 13,
            ModifierKind::EncoreStockCost => 14,
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

    fn continuous_source_marker_count(&self, source_ref: &TargetRef) -> usize {
        if source_ref.zone != TargetZone::Stage {
            return 0;
        }
        let p = source_ref.player as usize;
        let s = source_ref.index as usize;
        if s >= self.state.players[p].stage.len() {
            return 0;
        }
        let slot = &self.state.players[p].stage[s];
        if slot.card.map(|card| card.instance_id) != Some(source_ref.instance_id) {
            return 0;
        }
        slot.markers.len()
    }

    fn continuous_zone_count_value(&self, controller: u8, condition: &ZoneCountCondition) -> usize {
        self.zone_count_value_for_condition(controller, condition)
    }

    fn continuous_modifier_context_satisfied(
        &self,
        controller: u8,
        source_ref: &TargetRef,
        turn: Option<ConditionTurn>,
        zone_count: Option<&ZoneCountCondition>,
        require_source_marker: bool,
    ) -> bool {
        if let Some(turn_condition) = turn {
            let is_self_turn = self.state.turn.active_player == controller;
            match turn_condition {
                ConditionTurn::SelfTurn if !is_self_turn => return false,
                ConditionTurn::OpponentTurn if is_self_turn => return false,
                _ => {}
            }
        }
        if let Some(zone_condition) = zone_count {
            if !self.zone_count_condition_met(controller, zone_condition) {
                return false;
            }
        }
        if require_source_marker && self.continuous_source_marker_count(source_ref) == 0 {
            return false;
        }
        true
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
                    let total_abilities = self.live_stage_ability_count(player, slot as u8);
                    for ability_index in 0..total_abilities {
                        let Some(live) =
                            self.live_stage_ability_at(player, slot as u8, ability_index)
                        else {
                            continue;
                        };
                        let spec = live.spec.clone();
                        let live_effects: Vec<_> = live.effects.to_vec();
                        if spec.kind != AbilityKind::Continuous {
                            continue;
                        }
                        if !self.auto_ability_conditions_met(player, card_id, &spec) {
                            continue;
                        }
                        let source_ref = TargetRef {
                            player,
                            zone: TargetZone::Stage,
                            index: slot as u8,
                            card_id,
                            instance_id: card_inst.instance_id,
                        };
                        for effect in &live_effects {
                            if let EffectKind::FacingOpponentAddSoul { amount } = &effect.kind {
                                if source_ref.index >= 3 {
                                    continue;
                                }
                                let opponent_player = 1 - player;
                                let opponent_slot = source_ref.index as usize;
                                if opponent_slot
                                    >= self.state.players[opponent_player as usize].stage.len()
                                {
                                    continue;
                                }
                                let Some(opponent_card) = self.state.players
                                    [opponent_player as usize]
                                    .stage[opponent_slot]
                                    .card
                                else {
                                    continue;
                                };
                                new_continuous.push(ModifierInstance {
                                    id: 0,
                                    source: card_id,
                                    source_slot: Some(slot as u8),
                                    target_player: opponent_player,
                                    target_slot: source_ref.index,
                                    target_card: opponent_card.id,
                                    kind: ModifierKind::Soul,
                                    magnitude: *amount,
                                    duration: ModifierDuration::WhileOnStage,
                                    layer: ModifierLayer::Continuous,
                                    insertion: 0,
                                });
                                continue;
                            }
                            if let EffectKind::FacingOpponentAddModifier {
                                kind,
                                magnitude,
                                duration,
                            } = &effect.kind
                            {
                                if source_ref.index >= 3 {
                                    continue;
                                }
                                let opponent_player = 1 - player;
                                let opponent_slot = source_ref.index as usize;
                                if opponent_slot
                                    >= self.state.players[opponent_player as usize].stage.len()
                                {
                                    continue;
                                }
                                let Some(opponent_card) = self.state.players
                                    [opponent_player as usize]
                                    .stage[opponent_slot]
                                    .card
                                else {
                                    continue;
                                };
                                if *magnitude == 0 {
                                    continue;
                                }
                                new_continuous.push(ModifierInstance {
                                    id: 0,
                                    source: card_id,
                                    source_slot: Some(slot as u8),
                                    target_player: opponent_player,
                                    target_slot: source_ref.index,
                                    target_card: opponent_card.id,
                                    kind: *kind,
                                    magnitude: *magnitude,
                                    duration: *duration,
                                    layer: ModifierLayer::Continuous,
                                    insertion: 0,
                                });
                                continue;
                            }
                            if let EffectKind::SelfAddModifierIfFacingOpponent {
                                kind,
                                magnitude,
                                duration,
                                max_level,
                                max_cost,
                                level_gt_source_level,
                            } = &effect.kind
                            {
                                if source_ref.index >= 3 {
                                    continue;
                                }
                                let opponent_player = 1 - player;
                                let opponent_slot = source_ref.index as usize;
                                if opponent_slot
                                    >= self.state.players[opponent_player as usize].stage.len()
                                {
                                    continue;
                                }
                                let Some(opponent_card_inst) = self.state.players
                                    [opponent_player as usize]
                                    .stage[opponent_slot]
                                    .card
                                else {
                                    continue;
                                };
                                let Some(opponent_card) = self.db.get(opponent_card_inst.id) else {
                                    continue;
                                };
                                let opponent_level = self
                                    .compute_slot_level(opponent_player as usize, opponent_slot);
                                if let Some(level_cap) = max_level {
                                    if opponent_level > i32::from(*level_cap) {
                                        continue;
                                    }
                                }
                                if let Some(cost_cap) = max_cost {
                                    if opponent_card.cost > *cost_cap {
                                        continue;
                                    }
                                }
                                if *level_gt_source_level {
                                    let source_level =
                                        self.compute_slot_level(player as usize, slot);
                                    if opponent_level <= source_level {
                                        continue;
                                    }
                                }
                                if *magnitude == 0 {
                                    continue;
                                }
                                new_continuous.push(ModifierInstance {
                                    id: 0,
                                    source: card_id,
                                    source_slot: Some(slot as u8),
                                    target_player: player,
                                    target_slot: slot as u8,
                                    target_card: card_id,
                                    kind: *kind,
                                    magnitude: *magnitude,
                                    duration: *duration,
                                    layer: ModifierLayer::Continuous,
                                    insertion: 0,
                                });
                                continue;
                            }
                            if let EffectKind::AddSoulIfMiddleCenter { amount } = &effect.kind {
                                let middle_slot = if self.curriculum.reduced_stage_mode {
                                    0
                                } else {
                                    1
                                };
                                if slot != middle_slot {
                                    continue;
                                }
                                if *amount == 0 {
                                    continue;
                                }
                                new_continuous.push(ModifierInstance {
                                    id: 0,
                                    source: card_id,
                                    source_slot: Some(slot as u8),
                                    target_player: player,
                                    target_slot: slot as u8,
                                    target_card: card_id,
                                    kind: ModifierKind::Soul,
                                    magnitude: *amount,
                                    duration: ModifierDuration::WhileOnStage,
                                    layer: ModifierLayer::Continuous,
                                    insertion: 0,
                                });
                                continue;
                            }
                            let (
                                kind,
                                mut magnitude,
                                duration,
                                exclude_source,
                                turn,
                                zone_count,
                                require_source_marker,
                                per_source_marker,
                                per_zone_count,
                            ) = match &effect.kind {
                                EffectKind::AddModifier {
                                    kind,
                                    magnitude,
                                    duration,
                                } => (
                                    *kind, *magnitude, *duration, false, None, None, false, false,
                                    false,
                                ),
                                EffectKind::ConditionalAddModifier {
                                    kind,
                                    magnitude,
                                    duration,
                                    turn,
                                    zone_count,
                                    require_source_marker,
                                    per_source_marker,
                                    per_zone_count,
                                    exclude_source,
                                } => (
                                    *kind,
                                    *magnitude,
                                    *duration,
                                    *exclude_source,
                                    *turn,
                                    zone_count.clone(),
                                    *require_source_marker,
                                    *per_source_marker,
                                    *per_zone_count,
                                ),
                                _ => continue,
                            };
                            if !self.continuous_modifier_context_satisfied(
                                player,
                                &source_ref,
                                turn,
                                zone_count.as_ref(),
                                require_source_marker,
                            ) {
                                continue;
                            }
                            if per_source_marker {
                                let marker_count = self.continuous_source_marker_count(&source_ref);
                                magnitude = magnitude.saturating_mul(marker_count as i32);
                            }
                            if per_zone_count {
                                let Some(zone_condition) = zone_count.as_ref() else {
                                    continue;
                                };
                                let zone_count_value =
                                    self.continuous_zone_count_value(player, zone_condition);
                                magnitude = magnitude.saturating_mul(zone_count_value as i32);
                            }
                            if magnitude == 0 {
                                continue;
                            }
                            let Some(target_spec) = effect.target.as_ref() else {
                                continue;
                            };
                            if target_spec.source_only {
                                if exclude_source {
                                    continue;
                                }
                                new_continuous.push(ModifierInstance {
                                    id: 0,
                                    source: card_id,
                                    source_slot: Some(slot as u8),
                                    target_player: player,
                                    target_slot: slot as u8,
                                    target_card: card_id,
                                    kind,
                                    magnitude,
                                    duration,
                                    layer: ModifierLayer::Continuous,
                                    insertion: 0,
                                });
                            } else {
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
                                    if exclude_source
                                        && target.player == source_ref.player
                                        && target.zone == source_ref.zone
                                        && target.index == source_ref.index
                                        && target.instance_id == source_ref.instance_id
                                    {
                                        continue;
                                    }
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
            self.bump_modifiers_version();
            self.mark_all_slot_power_dirty();
            self.state.turn.derived_attack = None;
        }
    }
}
