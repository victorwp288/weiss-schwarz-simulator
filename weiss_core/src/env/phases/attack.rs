use super::super::{DamageIntentLocal, DamageResolveResult, GameEnv};
use crate::db::*;
use crate::effects::EffectKind;
use crate::encode::MAX_STAGE;
use crate::events::*;
use crate::state::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum AttackAutoResolvePhase {
    TriggerStep,
    DamageStep,
}

impl GameEnv {
    pub(in crate::env) fn recompute_derived_attack(&mut self) {
        let mut derived = crate::state::DerivedAttackState::new();
        for player in 0..2usize {
            let max_slot = if self.curriculum.reduced_stage_mode {
                1
            } else {
                MAX_STAGE
            };
            for slot in 0..max_slot {
                let slot_state = &self.state.players[player].stage[slot];
                let mut entry = crate::state::DerivedAttackSlot::empty();
                entry.cannot_attack = slot_state.cannot_attack;
                entry.attack_cost = slot_state.attack_cost;
                if let Some(card_inst) = slot_state.card {
                    let card_id = card_inst.id;
                    if self.db.get(card_id).is_none() {
                        derived.per_player[player][slot] = entry;
                        continue;
                    }
                    for modifier in &self.state.modifiers {
                        if modifier.target_player as usize != player
                            || modifier.target_slot as usize != slot
                        {
                            continue;
                        }
                        if modifier.target_card != card_id {
                            continue;
                        }
                        match modifier.kind {
                            ModifierKind::AttackCost => {
                                if modifier.magnitude > 0 {
                                    entry.attack_cost =
                                        entry.attack_cost.saturating_add(modifier.magnitude as u8);
                                }
                            }
                            ModifierKind::CannotAttack => {
                                if modifier.magnitude != 0 {
                                    entry.cannot_attack = true;
                                }
                            }
                            ModifierKind::CannotSideAttack => {
                                if modifier.magnitude != 0 {
                                    entry.cannot_side_attack = true;
                                }
                            }
                            ModifierKind::CannotFrontalAttack => {
                                if modifier.magnitude != 0 {
                                    entry.cannot_frontal_attack = true;
                                }
                            }
                            _ => {}
                        }
                    }
                }
                derived.per_player[player][slot] = entry;
            }
        }
        self.state.turn.derived_attack = Some(derived);
        if self.maybe_validate_state("derived_attack_recompute") {
            debug_assert!(
                self.is_fault_latched(),
                "validation failure should latch a deferred fault"
            );
        }
    }

    pub(in crate::env) fn resolve_attack_pipeline(&mut self) {
        loop {
            let Some(mut ctx) = self.state.turn.attack.take() else {
                return;
            };
            match ctx.step {
                AttackStep::Trigger => {
                    if self.curriculum.enable_priority_windows && !ctx.decl_window_done {
                        ctx.decl_window_done = true;
                        self.state.turn.attack = Some(ctx);
                        if self.state.turn.priority.is_none() {
                            self.enter_timing_window(
                                TimingWindow::AttackDeclarationWindow,
                                self.state.turn.active_player,
                            );
                        }
                        break;
                    }
                    if !ctx.auto_trigger_enqueued {
                        self.enqueue_other_attack_declaration_auto_effects(
                            &ctx,
                            self.state.turn.active_player,
                        );
                        self.enqueue_attack_auto_effects(
                            &ctx,
                            self.state.turn.active_player,
                            AttackAutoResolvePhase::TriggerStep,
                        );
                        ctx.auto_trigger_enqueued = true;
                        if !self.state.turn.stack.is_empty()
                            || self.state.turn.pending_level_up.is_some()
                            || !self.state.turn.pending_triggers.is_empty()
                            || self.state.turn.pending_cost.is_some()
                            || self.state.turn.choice.is_some()
                        {
                            self.state.turn.attack = Some(ctx);
                            if self.maybe_validate_state("attack_decl_auto_pause") {
                                return;
                            }
                            break;
                        }
                    }
                    self.resolve_trigger_step(&mut ctx);
                    ctx.trigger_checks_resolved = ctx.trigger_checks_resolved.saturating_add(1);
                    let trigger_checks_total = ctx.trigger_checks_total.max(1);
                    if ctx.trigger_checks_resolved >= trigger_checks_total {
                        if ctx.counter_allowed && self.curriculum.enable_counters {
                            ctx.step = AttackStep::Counter;
                        } else {
                            ctx.step = AttackStep::Damage;
                        }
                    } else {
                        // Re-enter trigger step for additional trigger checks granted this attack.
                        ctx.step = AttackStep::Trigger;
                        ctx.trigger_window_done = false;
                    }
                    if self.state.turn.pending_level_up.is_some()
                        || !self.state.turn.pending_triggers.is_empty()
                    {
                        self.state.turn.attack = Some(ctx);
                        if self.maybe_validate_state("attack_trigger_pause") {
                            return;
                        }
                        break;
                    }
                    if self.curriculum.enable_priority_windows && !ctx.trigger_window_done {
                        ctx.trigger_window_done = true;
                        self.state.turn.attack = Some(ctx);
                        if self.state.turn.priority.is_none() {
                            self.enter_timing_window(
                                TimingWindow::TriggerResolutionWindow,
                                self.state.turn.active_player,
                            );
                        }
                        break;
                    }
                    self.state.turn.attack = Some(ctx);
                }
                AttackStep::Counter => {
                    if self.curriculum.enable_priority_windows && !ctx.trigger_window_done {
                        ctx.trigger_window_done = true;
                        self.state.turn.attack = Some(ctx);
                        if self.state.turn.priority.is_none() {
                            self.enter_timing_window(
                                TimingWindow::TriggerResolutionWindow,
                                self.state.turn.active_player,
                            );
                        }
                        break;
                    }
                    let defender = 1 - self.state.turn.active_player;
                    self.state.turn.attack = Some(ctx);
                    if self.state.turn.priority.is_none() {
                        self.enter_timing_window(TimingWindow::CounterWindow, defender);
                    }
                    if self.maybe_validate_state("attack_counter_window") {
                        return;
                    }
                    break;
                }
                AttackStep::Damage => {
                    if self.curriculum.enable_priority_windows && !ctx.trigger_window_done {
                        ctx.trigger_window_done = true;
                        self.state.turn.attack = Some(ctx);
                        if self.state.turn.priority.is_none() {
                            self.enter_timing_window(
                                TimingWindow::TriggerResolutionWindow,
                                self.state.turn.active_player,
                            );
                        }
                        break;
                    }
                    if !ctx.auto_damage_enqueued {
                        self.enqueue_attack_auto_effects(
                            &ctx,
                            self.state.turn.active_player,
                            AttackAutoResolvePhase::DamageStep,
                        );
                        ctx.auto_damage_enqueued = true;
                        if !self.state.turn.stack.is_empty()
                            || self.state.turn.pending_level_up.is_some()
                            || !self.state.turn.pending_triggers.is_empty()
                            || self.state.turn.pending_cost.is_some()
                            || self.state.turn.choice.is_some()
                        {
                            self.state.turn.attack = Some(ctx);
                            if self.maybe_validate_state("attack_damage_auto_pause") {
                                return;
                            }
                            break;
                        }
                    }
                    let pause = self.resolve_damage_step(&mut ctx);
                    if pause {
                        self.state.turn.attack = Some(ctx);
                        if self.maybe_validate_state("attack_damage_pause") {
                            return;
                        }
                        break;
                    }
                    if ctx.attack_type == AttackType::Direct {
                        self.clear_battle_mods();
                        self.state.turn.attack = None;
                        self.state.turn.attack_decl_check_done = false;
                        self.run_check_timing(crate::db::AbilityTiming::EndOfAttack);
                        if self.state.turn.pending_level_up.is_some()
                            || !self.state.turn.pending_triggers.is_empty()
                        {
                            break;
                        }
                        if self.maybe_validate_state("attack_direct_done") {
                            return;
                        }
                        break;
                    }
                    ctx.step = AttackStep::Battle;
                    if self.curriculum.enable_priority_windows && !ctx.damage_window_done {
                        ctx.damage_window_done = true;
                        self.state.turn.attack = Some(ctx);
                        if self.state.turn.priority.is_none() {
                            self.enter_timing_window(
                                TimingWindow::DamageResolutionWindow,
                                self.state.turn.active_player,
                            );
                        }
                        break;
                    }
                    self.state.turn.attack = Some(ctx);
                }
                AttackStep::Battle => {
                    if self.curriculum.enable_priority_windows && !ctx.damage_window_done {
                        ctx.damage_window_done = true;
                        self.state.turn.attack = Some(ctx);
                        if self.state.turn.priority.is_none() {
                            self.enter_timing_window(
                                TimingWindow::DamageResolutionWindow,
                                self.state.turn.active_player,
                            );
                        }
                        break;
                    }
                    self.resolve_battle_step(&ctx);
                    self.clear_battle_mods();
                    self.state.turn.attack = None;
                    self.state.turn.attack_decl_check_done = false;
                    self.run_check_timing(crate::db::AbilityTiming::EndOfAttack);
                    if self.state.turn.pending_level_up.is_some()
                        || !self.state.turn.pending_triggers.is_empty()
                    {
                        break;
                    }
                    if self.maybe_validate_state("attack_battle_done") {
                        return;
                    }
                    break;
                }
                AttackStep::Encore => {
                    self.state.turn.attack = Some(ctx);
                    if self.maybe_validate_state("attack_encore_hold") {
                        return;
                    }
                    break;
                }
            }
            if self.maybe_validate_state("attack_pipeline") {
                return;
            }
        }
    }

    pub(in crate::env) fn resolve_trigger_step(&mut self, ctx: &mut AttackContext) {
        let active = self.state.turn.active_player as usize;
        let card = self.draw_from_deck(active as u8);
        if let Some(card_inst) = card {
            let card_id = card_inst.id;
            let instance_id = card_inst.instance_id;
            ctx.trigger_card = Some(card_id);
            ctx.trigger_instance_id = Some(instance_id);
            let _ = self.reveal_cards(
                active as u8,
                &[card_inst],
                RevealReason::TriggerCheck,
                RevealAudience::Public,
            );
            self.move_card_between_zones(
                active as u8,
                card_inst,
                Zone::Deck,
                Zone::Resolution,
                None,
                None,
            );
            self.state.turn.attack = Some(ctx.clone());
            self.queue_timing_triggers(crate::db::AbilityTiming::TriggerResolution);
            if self.curriculum.enable_triggers {
                if let Some(static_card) = self.db.get(card_id) {
                    let triggers = static_card.triggers.clone();
                    let mut effects = Vec::new();
                    for icon in triggers {
                        self.log_event(Event::Trigger {
                            player: active as u8,
                            icon,
                            card: Some(card_id),
                        });
                        match icon {
                            TriggerIcon::Soul if self.curriculum.enable_trigger_soul => {
                                effects.push(TriggerEffect::Soul)
                            }
                            TriggerIcon::Draw if self.curriculum.enable_trigger_draw => {
                                effects.push(TriggerEffect::Draw)
                            }
                            TriggerIcon::Shot if self.curriculum.enable_trigger_shot => {
                                effects.push(TriggerEffect::Shot)
                            }
                            TriggerIcon::Bounce if self.curriculum.enable_trigger_bounce => {
                                effects.push(TriggerEffect::Bounce)
                            }
                            TriggerIcon::Choice => effects.push(TriggerEffect::Choice),
                            TriggerIcon::Pool => effects.push(TriggerEffect::Pool),
                            TriggerIcon::Treasure if self.curriculum.enable_trigger_treasure => {
                                effects.push(TriggerEffect::Treasure)
                            }
                            TriggerIcon::Gate if self.curriculum.enable_trigger_gate => {
                                effects.push(TriggerEffect::Gate)
                            }
                            TriggerIcon::Standby if self.curriculum.enable_trigger_standby => {
                                effects.push(TriggerEffect::Standby)
                            }
                            _ => {}
                        }
                    }
                    let has_delayed_trigger_card_move = effects
                        .iter()
                        .any(|e| matches!(e, TriggerEffect::Treasure | TriggerEffect::Pool));
                    self.queue_trigger_group(active as u8, card_id, effects);
                    if has_delayed_trigger_card_move {
                        return;
                    }
                }
            }
            if let Some(resolved) = self.take_resolution_card(active as u8, instance_id) {
                self.move_card_between_zones(
                    active as u8,
                    resolved,
                    Zone::Resolution,
                    Zone::Stock,
                    None,
                    None,
                );
            }
        }
    }

    pub(in crate::env) fn resolve_damage_step(&mut self, ctx: &mut AttackContext) -> bool {
        let attacker = self.state.turn.active_player;
        let defender = 1 - attacker;
        if !ctx.battle_damage_applied {
            let intent = DamageIntentLocal {
                source_player: attacker,
                source_slot: Some(ctx.attacker_slot),
                target: defender,
                amount: ctx.damage,
                damage_type: DamageType::Battle,
                cancelable: true,
                refresh_penalty: false,
            };
            let result = self.resolve_damage_intent(intent, &mut ctx.damage_modifiers);
            ctx.last_damage_event_id = Some(result.event_id);
            if result.canceled && ctx.pending_shot_damage > 0 {
                let pending_shot_damage = std::mem::take(&mut ctx.pending_shot_damage);
                for _ in 0..pending_shot_damage {
                    let _ =
                        self.resolve_effect_damage(attacker, defender, 1, true, false, None, None);
                }
            }
            if result.canceled {
                self.enqueue_damage_canceled_auto_effects(ctx, attacker, defender);
            } else {
                self.enqueue_damage_not_canceled_auto_effects(ctx, attacker, defender);
            }
            ctx.battle_damage_applied = true;
        }
        self.state.turn.pending_level_up.is_some()
            || !self.state.turn.stack.is_empty()
            || !self.state.turn.pending_triggers.is_empty()
            || self.state.turn.pending_cost.is_some()
            || self.state.turn.choice.is_some()
    }

    fn enqueue_attack_auto_effects(
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

    fn enqueue_stage_auto_effects_for_timing(
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

    fn enqueue_other_attack_declaration_auto_effects(&mut self, ctx: &AttackContext, attacker: u8) {
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

    fn enqueue_damage_canceled_auto_effects(
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

    fn enqueue_damage_not_canceled_auto_effects(
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

    #[allow(clippy::too_many_arguments)]
    pub(in crate::env) fn resolve_effect_damage(
        &mut self,
        source_player: u8,
        target: u8,
        amount: i32,
        cancelable: bool,
        refresh_penalty: bool,
        source_card: Option<CardId>,
        source_ref: Option<TargetRef>,
    ) -> bool {
        let intent = DamageIntentLocal {
            source_player,
            source_slot: None,
            target,
            amount,
            damage_type: DamageType::Effect,
            cancelable,
            refresh_penalty,
        };
        let mut modifiers = if let Some(ctx) = &mut self.state.turn.attack {
            std::mem::take(&mut ctx.damage_modifiers)
        } else {
            Vec::new()
        };
        let result = self.resolve_damage_intent(intent, &mut modifiers);
        if let Some(ctx) = &mut self.state.turn.attack {
            ctx.damage_modifiers = modifiers;
        }
        if result.canceled
            && self.should_consume_pending_shot_on_canceled_effect_damage(
                source_player,
                source_card,
                source_ref.as_ref(),
            )
        {
            self.resolve_pending_shot_damage(source_player, target);
        }
        self.state.turn.pending_level_up.is_some()
    }

    fn should_consume_pending_shot_on_canceled_effect_damage(
        &self,
        source_player: u8,
        source_card: Option<CardId>,
        source_ref: Option<&TargetRef>,
    ) -> bool {
        if self.curriculum.enable_legacy_shot_damage_step_only {
            return false;
        }
        let Some(ctx) = self.state.turn.attack.as_ref() else {
            return false;
        };
        if ctx.pending_shot_damage == 0 || source_player != self.state.turn.active_player {
            return false;
        }
        let p = source_player as usize;
        let attacker_slot = ctx.attacker_slot as usize;
        if attacker_slot >= self.state.players[p].stage.len() {
            return false;
        }
        let Some(attacker_card) = self.state.players[p].stage[attacker_slot].card else {
            return false;
        };
        if let Some(source_ref) = source_ref {
            return source_ref.player == source_player
                && source_ref.zone == TargetZone::Stage
                && source_ref.index == ctx.attacker_slot
                && source_ref.instance_id == attacker_card.instance_id;
        }
        if source_card != Some(attacker_card.id) {
            return false;
        }

        let same_id_count = self.state.players[p]
            .stage
            .iter()
            .filter_map(|slot| slot.card)
            .filter(|card| card.id == attacker_card.id)
            .count();
        same_id_count == 1
    }

    fn resolve_pending_shot_damage(&mut self, source_player: u8, target: u8) {
        let pending_shot_damage = self
            .state
            .turn
            .attack
            .as_mut()
            .map(|ctx| std::mem::take(&mut ctx.pending_shot_damage))
            .unwrap_or(0);
        for _ in 0..pending_shot_damage {
            let _ = self.resolve_effect_damage(source_player, target, 1, true, false, None, None);
        }
    }

    pub(in crate::env) fn resolve_damage_intent(
        &mut self,
        intent: DamageIntentLocal,
        modifiers: &mut [DamageModifier],
    ) -> DamageResolveResult {
        let event_id = self.state.turn.next_damage_event_id;
        self.state.turn.next_damage_event_id = self.state.turn.next_damage_event_id.wrapping_add(1);
        self.log_event(Event::DamageIntent {
            event_id,
            source_player: intent.source_player,
            source_slot: intent.source_slot,
            target: intent.target,
            amount: intent.amount,
            damage_type: intent.damage_type,
            cancelable: intent.cancelable,
        });

        let prev_damage_target = self.state.turn.damage_resolution_target;
        self.state.turn.damage_resolution_target = Some(intent.target);

        let mut amount = intent.amount.max(0);
        let mut cancelable = intent.cancelable;
        let mut canceled = false;

        let mut order: Vec<usize> = (0..modifiers.len()).collect();
        order.sort_by_key(|idx| {
            let m = &modifiers[*idx];
            (m.priority, m.insertion, m.source_id)
        });
        for idx in order {
            let modifier = &mut modifiers[idx];
            let before_amount = amount;
            let before_cancelable = cancelable;
            let before_canceled = canceled;
            match modifier.kind {
                DamageModifierKind::AddAmount { delta } => {
                    if delta >= 0 {
                        amount = amount.saturating_add(delta);
                    } else if modifier.remaining > 0 {
                        let reduce = amount.min(modifier.remaining);
                        amount -= reduce;
                        modifier.remaining -= reduce;
                    }
                }
                DamageModifierKind::SetCancelable { cancelable: set } => {
                    cancelable = set;
                }
                DamageModifierKind::CancelNext => {
                    if !modifier.used && cancelable {
                        canceled = true;
                        modifier.used = true;
                    }
                }
                DamageModifierKind::SetAmount { amount: set_amount } => {
                    amount = set_amount;
                }
            }
            self.log_event(Event::DamageModifierApplied {
                event_id,
                modifier: modifier.kind,
                before_amount,
                after_amount: amount,
                before_cancelable,
                after_cancelable: cancelable,
                before_canceled,
                after_canceled: canceled,
            });
        }

        let mut revealed: Vec<CardInstance> = Vec::new();
        if amount > 0 && !canceled {
            for _ in 0..amount {
                if let Some(card) = self.draw_from_deck(intent.target) {
                    let reason = if intent.refresh_penalty {
                        RevealReason::RefreshPenalty
                    } else {
                        RevealReason::DamageCheck
                    };
                    self.reveal_card(intent.target, &card, reason, RevealAudience::Public);
                    self.move_card_between_zones(
                        intent.target,
                        card,
                        Zone::Deck,
                        Zone::Resolution,
                        None,
                        None,
                    );
                    revealed.push(card);
                    if cancelable {
                        if let Some(static_card) = self.db.get(card.id) {
                            if static_card.card_type == CardType::Climax {
                                canceled = true;
                                break;
                            }
                        }
                    }
                } else {
                    break;
                }
            }
        }

        let committed = if canceled { 0 } else { revealed.len() as i32 };
        self.log_event(Event::DamageModified {
            event_id,
            target: intent.target,
            original: intent.amount,
            modified: committed,
            canceled,
            damage_type: intent.damage_type,
        });

        let target = intent.target as usize;
        let mut check_level = false;
        if canceled {
            self.log_event(Event::DamageCancel {
                player: intent.target,
            });
            for card in revealed {
                if let Some(resolved) = self.take_resolution_card(intent.target, card.instance_id) {
                    self.move_card_between_zones(
                        intent.target,
                        resolved,
                        Zone::Resolution,
                        Zone::WaitingRoom,
                        None,
                        None,
                    );
                }
            }
        } else {
            for card in revealed {
                let card_id = card.id;
                if let Some(resolved) = self.take_resolution_card(intent.target, card.instance_id) {
                    self.move_card_between_zones(
                        intent.target,
                        resolved,
                        Zone::Resolution,
                        Zone::Clock,
                        None,
                        None,
                    );
                }
                self.log_event(Event::DamageCommitted {
                    event_id,
                    target: intent.target,
                    card: card_id,
                    damage_type: intent.damage_type,
                });
                self.log_event(Event::Damage {
                    player: intent.target,
                    card: card_id,
                });
                self.pending_damage_delta[target] += 1;
            }
            check_level = true;
        }
        if check_level {
            self.check_level_up(intent.target);
        }
        self.state.turn.damage_resolution_target = prev_damage_target;
        DamageResolveResult { event_id, canceled }
    }

    pub(in crate::env) fn resolve_battle_step(&mut self, ctx: &AttackContext) {
        let attacker = self.state.turn.active_player as usize;
        let defender = 1 - attacker;
        let atk_slot = ctx.attacker_slot as usize;
        let def_slot = match ctx.defender_slot {
            Some(s) => s as usize,
            None => return,
        };
        let mut reversed: Vec<(u8, CardId)> = Vec::new();
        let mut battle_opponent_reversed_sources: Vec<(u8, CardId)> = Vec::new();
        let atk_power = self.compute_slot_power(attacker, atk_slot);
        let def_power = self.compute_slot_power(defender, def_slot);

        let can_become_reverse = |player: usize, slot: usize| -> bool {
            if slot >= self.state.players[player].stage.len() {
                return false;
            }
            let Some(card_inst) = self.state.players[player].stage[slot].card else {
                return false;
            };
            !self.state.modifiers.iter().any(|modifier| {
                modifier.target_player as usize == player
                    && modifier.target_slot as usize == slot
                    && modifier.target_card == card_inst.id
                    && modifier.kind == ModifierKind::CannotBecomeReverse
                    && modifier.magnitude != 0
            })
        };

        let defender_can_reverse = can_become_reverse(defender, def_slot);
        let attacker_can_reverse = can_become_reverse(attacker, atk_slot);

        let maybe_move_reversed_to_memory =
            |source_player: usize,
             source_slot: usize,
             target_player: usize,
             target_slot: usize,
             this: &mut GameEnv| {
                if !this.slot_has_active_modifier_kind(
                    source_player as u8,
                    source_slot as u8,
                    ModifierKind::BattleOpponentMoveToMemoryOnReverse,
                ) {
                    return;
                }
                if target_slot >= this.state.players[target_player].stage.len() {
                    return;
                }
                let Some(target_card_inst) =
                    this.state.players[target_player].stage[target_slot].card
                else {
                    return;
                };
                let target_ref = TargetRef {
                    player: target_player as u8,
                    zone: TargetZone::Stage,
                    index: target_slot as u8,
                    card_id: target_card_inst.id,
                    instance_id: target_card_inst.instance_id,
                };
                let _ = this.move_stage_target_to_memory(target_ref);
            };

        if atk_power > def_power {
            if defender_can_reverse {
                self.state.players[defender].stage[def_slot].status = StageStatus::Reverse;
                self.log_event(Event::ReversalCommitted {
                    player: defender as u8,
                    slot: def_slot as u8,
                    cause_damage_event: ctx.last_damage_event_id,
                });
                if let Some(card_inst) = self.state.players[defender].stage[def_slot].card {
                    reversed.push((defender as u8, card_inst.id));
                }
                if let Some(card_inst) = self.state.players[attacker].stage[atk_slot].card {
                    battle_opponent_reversed_sources.push((attacker as u8, card_inst.id));
                }
                maybe_move_reversed_to_memory(attacker, atk_slot, defender, def_slot, self);
            }
        } else if atk_power < def_power {
            if attacker_can_reverse {
                self.state.players[attacker].stage[atk_slot].status = StageStatus::Reverse;
                self.log_event(Event::ReversalCommitted {
                    player: attacker as u8,
                    slot: atk_slot as u8,
                    cause_damage_event: ctx.last_damage_event_id,
                });
                if let Some(card_inst) = self.state.players[attacker].stage[atk_slot].card {
                    reversed.push((attacker as u8, card_inst.id));
                }
                if let Some(card_inst) = self.state.players[defender].stage[def_slot].card {
                    battle_opponent_reversed_sources.push((defender as u8, card_inst.id));
                }
                maybe_move_reversed_to_memory(defender, def_slot, attacker, atk_slot, self);
            }
        } else {
            if defender_can_reverse {
                self.state.players[defender].stage[def_slot].status = StageStatus::Reverse;
                self.log_event(Event::ReversalCommitted {
                    player: defender as u8,
                    slot: def_slot as u8,
                    cause_damage_event: ctx.last_damage_event_id,
                });
                if let Some(card_inst) = self.state.players[defender].stage[def_slot].card {
                    reversed.push((defender as u8, card_inst.id));
                }
                if let Some(card_inst) = self.state.players[attacker].stage[atk_slot].card {
                    battle_opponent_reversed_sources.push((attacker as u8, card_inst.id));
                }
                maybe_move_reversed_to_memory(attacker, atk_slot, defender, def_slot, self);
            }
            if attacker_can_reverse {
                self.state.players[attacker].stage[atk_slot].status = StageStatus::Reverse;
                self.log_event(Event::ReversalCommitted {
                    player: attacker as u8,
                    slot: atk_slot as u8,
                    cause_damage_event: ctx.last_damage_event_id,
                });
                if let Some(card_inst) = self.state.players[attacker].stage[atk_slot].card {
                    reversed.push((attacker as u8, card_inst.id));
                }
                if let Some(card_inst) = self.state.players[defender].stage[def_slot].card {
                    battle_opponent_reversed_sources.push((defender as u8, card_inst.id));
                }
                maybe_move_reversed_to_memory(defender, def_slot, attacker, atk_slot, self);
            }
        }
        if !reversed.is_empty() {
            self.queue_on_reverse_triggers(&reversed);
        }
        if !battle_opponent_reversed_sources.is_empty() {
            self.queue_battle_opponent_reverse_triggers(&battle_opponent_reversed_sources);
        }
    }

    pub(in crate::env) fn queue_encore_requests(&mut self) {
        let mut queue = Vec::new();
        for player in 0..2 {
            for slot in 0..self.state.players[player].stage.len() {
                let slot_state = &self.state.players[player].stage[slot];
                if slot_state.card.is_some() && slot_state.status == StageStatus::Reverse {
                    queue.push(EncoreRequest {
                        player: player as u8,
                        slot: slot as u8,
                    });
                }
            }
        }
        self.state.turn.encore_queue = queue;
        self.state.turn.encore_window_done = false;
        self.state.turn.encore_begin_done = false;
        self.state.turn.encore_step_player = if self.state.turn.encore_queue.is_empty() {
            None
        } else {
            Some(self.state.turn.active_player)
        };
    }

    pub(in crate::env) fn cleanup_reversed_to_waiting_room(&mut self) {
        for player in 0..2 {
            for slot in 0..self.state.players[player].stage.len() {
                if self.state.players[player].stage[slot].status == StageStatus::Reverse {
                    self.send_stage_to_waiting_room(player as u8, slot as u8);
                }
            }
        }
    }

    pub(in crate::env) fn clear_battle_mods(&mut self) {
        for player in 0..2 {
            for slot in &mut self.state.players[player].stage {
                slot.power_mod_battle = 0;
            }
        }
        self.mark_all_slot_power_dirty();
    }
}
