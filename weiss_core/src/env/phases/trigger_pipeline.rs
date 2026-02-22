use super::super::{
    EngineErrorCode, FaultSource, GameEnv, TriggerCompileContext, TRIGGER_EFFECT_BOUNCE,
    TRIGGER_EFFECT_DRAW, TRIGGER_EFFECT_GATE, TRIGGER_EFFECT_POOL_MOVE, TRIGGER_EFFECT_POOL_STOCK,
    TRIGGER_EFFECT_SHOT, TRIGGER_EFFECT_SOUL, TRIGGER_EFFECT_STANDBY, TRIGGER_EFFECT_TREASURE_MOVE,
    TRIGGER_EFFECT_TREASURE_STOCK,
};
use crate::db::*;
use crate::effects::*;
use crate::encode::MAX_STAGE;
use crate::events::*;
use crate::legal::*;
use crate::state::*;
use anyhow::Result;

struct TriggerSeed {
    player: u8,
    source: CardId,
    effect: TriggerEffect,
}

fn trigger_effect_sort_key(effect: TriggerEffect) -> (u8, u64) {
    match effect {
        TriggerEffect::Soul => (0, 0),
        TriggerEffect::Draw => (1, 0),
        TriggerEffect::Shot => (2, 0),
        TriggerEffect::Bounce => (3, 0),
        TriggerEffect::Choice => (4, 0),
        TriggerEffect::Pool => (5, 0),
        TriggerEffect::Treasure => (6, 0),
        TriggerEffect::Gate => (7, 0),
        TriggerEffect::Standby => (8, 0),
        TriggerEffect::AutoAbility { ability_index } => (9, ability_index as u64),
        TriggerEffect::GrantedAutoAbility { grant_id } => (10, grant_id),
    }
}

fn trigger_seed_sort_key(seed: &TriggerSeed) -> (u8, u32, u8, u64) {
    let (kind, sub) = trigger_effect_sort_key(seed.effect);
    (seed.player, seed.source, kind, sub)
}

fn pending_trigger_sort_key(trigger: &PendingTrigger) -> (u32, u8, u32, u8, u64, u32) {
    let (kind, sub) = trigger_effect_sort_key(trigger.effect);
    (
        trigger.group_id,
        trigger.player,
        trigger.source_card,
        kind,
        sub,
        trigger.id,
    )
}

impl GameEnv {
    fn collect_stage_timing_trigger_seeds(
        &self,
        player: u8,
        timing: AbilityTiming,
        pending: &mut Vec<TriggerSeed>,
    ) {
        for slot_idx in 0..self.state.players[player as usize].stage.len() {
            let Some(card_inst) = self.state.players[player as usize].stage[slot_idx].card else {
                continue;
            };
            let card_id = card_inst.id;
            if self.db.get(card_id).is_none() {
                continue;
            }
            let total_abilities = self.live_stage_ability_count(player, slot_idx as u8);
            for ability_index in 0..total_abilities {
                let Some(live) = self.live_stage_ability_at(player, slot_idx as u8, ability_index)
                else {
                    continue;
                };
                let spec = live.spec;
                if spec.kind != AbilityKind::Auto {
                    continue;
                }
                if spec.timing() == Some(timing) {
                    if !self.auto_ability_conditions_met(player, card_id, spec) {
                        continue;
                    }
                    let effect = if let Some(grant_id) = live.grant_id {
                        TriggerEffect::GrantedAutoAbility { grant_id }
                    } else {
                        let Ok(ability_index) = u8::try_from(ability_index) else {
                            debug_assert!(
                                ability_index <= u8::MAX as usize,
                                "ability index out of range"
                            );
                            continue;
                        };
                        TriggerEffect::AutoAbility { ability_index }
                    };
                    pending.push(TriggerSeed {
                        player,
                        source: card_id,
                        effect,
                    });
                }
            }
        }
    }

    fn collect_canonical_card_auto_ability_seeds(
        &self,
        player: u8,
        card_id: CardId,
        timing: AbilityTiming,
        pending: &mut Vec<TriggerSeed>,
    ) {
        if self.db.get(card_id).is_none() {
            return;
        }
        let specs = self.db.iter_card_abilities_in_canonical_order(card_id);
        for (ability_index, spec) in specs.iter().enumerate() {
            if spec.kind != AbilityKind::Auto {
                continue;
            }
            if spec.timing() == Some(timing) {
                if !self.auto_ability_conditions_met(player, card_id, spec) {
                    continue;
                }
                let Ok(ability_index) = u8::try_from(ability_index) else {
                    debug_assert!(
                        ability_index > u8::MAX as usize,
                        "ability index out of range"
                    );
                    continue;
                };
                pending.push(TriggerSeed {
                    player,
                    source: card_id,
                    effect: TriggerEffect::AutoAbility { ability_index },
                });
            }
        }
    }

    fn queue_new_trigger_seed_group(&mut self, pending: Vec<TriggerSeed>) -> bool {
        if pending.is_empty() {
            return false;
        }
        let group_id = self.allocate_trigger_group();
        self.queue_trigger_group_batch(group_id, pending);
        true
    }

    fn queue_new_trigger_seed_group_and_validate(
        &mut self,
        pending: Vec<TriggerSeed>,
        validate_tag: &'static str,
    ) {
        if self.queue_new_trigger_seed_group(pending) {
            let _ = self.maybe_validate_state(validate_tag);
        }
    }

    fn sort_pending_triggers(&mut self) {
        self.state
            .turn
            .pending_triggers
            .sort_by_key(pending_trigger_sort_key);
        self.state.turn.pending_triggers_sorted = true;
    }

    fn ensure_pending_triggers_sorted(&mut self) {
        if !self.state.turn.pending_triggers_sorted {
            self.sort_pending_triggers();
        }
    }

    pub(in crate::env) fn queue_timing_triggers(&mut self, timing: AbilityTiming) {
        if !self.curriculum.enable_triggers {
            return;
        }
        let mut pending: Vec<TriggerSeed> = Vec::new();
        for player in 0..2u8 {
            self.collect_stage_timing_trigger_seeds(player, timing, &mut pending);
            for card_inst in &self.state.players[player as usize].climax {
                self.collect_canonical_card_auto_ability_seeds(
                    player,
                    card_inst.id,
                    timing,
                    &mut pending,
                );
            }
        }
        self.queue_new_trigger_seed_group_and_validate(pending, "check_timing_triggers");
    }

    pub(in crate::env) fn queue_on_reverse_triggers(&mut self, reversed: &[(u8, CardId)]) {
        if !self.curriculum.enable_triggers || !self.curriculum.enable_on_reverse_triggers {
            return;
        }
        let mut pending: Vec<TriggerSeed> = Vec::new();
        for (player, card_id) in reversed {
            self.collect_canonical_card_auto_ability_seeds(
                *player,
                *card_id,
                AbilityTiming::OnReverse,
                &mut pending,
            );
        }
        self.queue_new_trigger_seed_group_and_validate(pending, "on_reverse_triggers");
    }

    pub(in crate::env) fn queue_battle_opponent_reverse_triggers(
        &mut self,
        sources: &[(u8, CardId)],
    ) {
        if !self.curriculum.enable_triggers || !self.curriculum.enable_on_reverse_triggers {
            return;
        }
        let mut pending: Vec<TriggerSeed> = Vec::new();
        for (player, card_id) in sources {
            self.collect_canonical_card_auto_ability_seeds(
                *player,
                *card_id,
                AbilityTiming::BattleOpponentReverse,
                &mut pending,
            );
        }
        self.queue_new_trigger_seed_group_and_validate(pending, "battle_opponent_reverse_triggers");
    }

    pub(in crate::env) fn handle_trigger_pipeline(&mut self) -> bool {
        // Invariants:
        // - Preserve pending trigger stable sort key/order.
        //   See `weiss_core/tests/trigger_order_tests.rs`.
        if let Some(choice) = &self.state.turn.choice {
            self.set_decision(Decision {
                player: choice.player,
                kind: DecisionKind::Choice,
                focus_slot: None,
            });
            if self.maybe_validate_state("choice_decision") {
                return true;
            }
            return true;
        }
        if self.state.turn.pending_triggers.is_empty() {
            self.state.turn.trigger_order = None;
            return false;
        }
        self.ensure_pending_triggers_sorted();

        if let Some(order) = &self.state.turn.trigger_order {
            self.set_decision(Decision {
                player: order.player,
                kind: DecisionKind::TriggerOrder,
                focus_slot: None,
            });
            if self.maybe_validate_state("trigger_order_decision") {
                return true;
            }
            return true;
        }

        let group_id = match self
            .state
            .turn
            .pending_triggers
            .iter()
            .map(|t| t.group_id)
            .min()
        {
            Some(id) => id,
            None => return false,
        };
        let active = self.state.turn.active_player;
        for player in [active, 1 - active] {
            let mut choices: Vec<&PendingTrigger> = self
                .state
                .turn
                .pending_triggers
                .iter()
                .filter(|t| t.group_id == group_id && t.player == player)
                .collect();
            if choices.len() > 1 {
                choices.sort_by_key(|t| pending_trigger_sort_key(t));
                let ids: Vec<u32> = choices.iter().map(|t| t.id).collect();
                self.state.turn.trigger_order = Some(TriggerOrderState {
                    group_id,
                    player,
                    choices: ids,
                });
                self.set_decision(Decision {
                    player,
                    kind: DecisionKind::TriggerOrder,
                    focus_slot: None,
                });
                if self.maybe_validate_state("trigger_order_decision") {
                    return true;
                }
                return true;
            }
            if choices.len() == 1 {
                let trigger_id = choices[0].id;
                if let Some(index) = self
                    .state
                    .turn
                    .pending_triggers
                    .iter()
                    .position(|t| t.id == trigger_id)
                {
                    let trigger = self.state.turn.pending_triggers.remove(index);
                    let processed_any = match self.resolve_trigger(trigger) {
                        Ok(processed) => processed,
                        Err(err) => {
                            self.last_engine_error = true;
                            self.last_engine_error_code = EngineErrorCode::ActionError;
                            eprintln!("Trigger resolve failed: {err}");
                            false
                        }
                    };
                    if processed_any {
                        if self.maybe_validate_state("trigger_choice_pause") {
                            return true;
                        }
                        return true;
                    }
                    if self.maybe_validate_state("trigger_pipeline") {
                        return true;
                    }
                    return true;
                }
                break;
            }
        }
        if self.maybe_validate_state("trigger_pipeline") {
            return true;
        }
        false
    }

    pub(in crate::env) fn queue_trigger_group(
        &mut self,
        player: u8,
        source: CardId,
        effects: Vec<TriggerEffect>,
    ) {
        if effects.is_empty() {
            return;
        }
        let triggers = effects
            .into_iter()
            .map(|effect| TriggerSeed {
                player,
                source,
                effect,
            })
            .collect();
        let _ = self.queue_new_trigger_seed_group(triggers);
    }

    fn queue_trigger_group_batch(&mut self, group_id: u32, mut triggers: Vec<TriggerSeed>) {
        if triggers.is_empty() {
            return;
        }
        let current_id = self.state.turn.next_trigger_id as u64;
        let batch_len = triggers.len() as u64;
        if current_id.saturating_add(batch_len) > u32::MAX as u64 {
            if let Some(first) = triggers.first() {
                self.latch_fault_deferred(
                    EngineErrorCode::InvariantViolation,
                    Some(first.player),
                    FaultSource::Step,
                );
            }
            return;
        }
        triggers.sort_by_key(trigger_seed_sort_key);
        let mut trigger_ids = Vec::with_capacity(triggers.len());
        for trigger in triggers {
            let id = self.state.turn.next_trigger_id;
            self.state.turn.next_trigger_id = id + 1;
            let pending = PendingTrigger {
                id,
                group_id,
                player: trigger.player,
                source_card: trigger.source,
                effect: trigger.effect,
                effect_id: None,
            };
            self.state.turn.pending_triggers.push(pending);
            trigger_ids.push(id);
            self.log_event(Event::TriggerQueued {
                trigger_id: id,
                group_id,
                player: trigger.player,
                source: trigger.source,
                effect: trigger.effect,
            });
        }
        if !trigger_ids.is_empty() {
            self.state.turn.pending_triggers_sorted = false;
        }
        self.sort_pending_triggers();
        if !trigger_ids.is_empty() {
            self.log_event(Event::TriggerGrouped {
                group_id,
                trigger_ids,
            });
        }
    }

    pub(in crate::env) fn trigger_effect_id(
        &self,
        source_card: CardId,
        effect_index: u8,
    ) -> EffectId {
        EffectId::new(EffectSourceKind::Trigger, source_card, 0, effect_index)
    }

    pub(in crate::env) fn compile_trigger_icon_effects(
        &self,
        icon: TriggerIcon,
        ctx: TriggerCompileContext,
    ) -> Vec<EffectSpec> {
        match icon {
            TriggerIcon::Soul => vec![EffectSpec {
                id: self.trigger_effect_id(ctx.source_card, TRIGGER_EFFECT_SOUL),
                kind: EffectKind::ModifyPendingAttackDamage { delta: 1 },
                target: None,
                optional: false,
            }],
            TriggerIcon::Draw => vec![EffectSpec {
                id: self.trigger_effect_id(ctx.source_card, TRIGGER_EFFECT_DRAW),
                kind: EffectKind::Draw { count: 1 },
                target: None,
                optional: false,
            }],
            TriggerIcon::Shot => vec![EffectSpec {
                id: self.trigger_effect_id(ctx.source_card, TRIGGER_EFFECT_SHOT),
                kind: EffectKind::EnableShotDamage { amount: 1 },
                target: None,
                optional: false,
            }],
            TriggerIcon::Choice => Vec::new(),
            TriggerIcon::Pool => vec![
                EffectSpec {
                    id: self.trigger_effect_id(ctx.source_card, TRIGGER_EFFECT_POOL_MOVE),
                    kind: EffectKind::MoveTriggerCardToStock,
                    target: None,
                    optional: false,
                },
                EffectSpec {
                    id: self.trigger_effect_id(ctx.source_card, TRIGGER_EFFECT_POOL_STOCK),
                    kind: EffectKind::StockCharge { count: 1 },
                    target: None,
                    optional: false,
                },
            ],
            TriggerIcon::Gate => vec![EffectSpec {
                id: self.trigger_effect_id(ctx.source_card, TRIGGER_EFFECT_GATE),
                kind: EffectKind::MoveToHand,
                target: Some(TargetSpec {
                    zone: TargetZone::WaitingRoom,
                    side: TargetSide::SelfSide,
                    slot_filter: TargetSlotFilter::Any,
                    card_type: Some(CardType::Climax),
                    card_trait: None,
                    level_max: None,
                    cost_max: None,
                    card_ids: Vec::new(),
                    count: 1,
                    limit: None,
                    source_only: false,
                    reveal_to_controller: false,
                }),
                optional: true,
            }],
            TriggerIcon::Bounce => vec![EffectSpec {
                id: self.trigger_effect_id(ctx.source_card, TRIGGER_EFFECT_BOUNCE),
                kind: EffectKind::MoveToHand,
                target: Some(TargetSpec {
                    zone: TargetZone::Stage,
                    side: TargetSide::Opponent,
                    slot_filter: TargetSlotFilter::Any,
                    card_type: Some(CardType::Character),
                    card_trait: None,
                    level_max: None,
                    cost_max: None,
                    card_ids: Vec::new(),
                    count: 1,
                    limit: None,
                    source_only: false,
                    reveal_to_controller: false,
                }),
                optional: true,
            }],
            TriggerIcon::Standby => {
                let Some(slot) = ctx.standby_slot else {
                    return Vec::new();
                };
                vec![EffectSpec {
                    id: self.trigger_effect_id(ctx.source_card, TRIGGER_EFFECT_STANDBY),
                    kind: EffectKind::Standby { target_slot: slot },
                    target: Some(TargetSpec {
                        zone: TargetZone::WaitingRoom,
                        side: TargetSide::SelfSide,
                        slot_filter: TargetSlotFilter::Any,
                        card_type: Some(CardType::Character),
                        card_trait: None,
                        level_max: None,
                        cost_max: None,
                        card_ids: Vec::new(),
                        count: 1,
                        limit: None,
                        source_only: false,
                        reveal_to_controller: false,
                    }),
                    optional: false,
                }]
            }
            TriggerIcon::Treasure => {
                let Some(take_stock) = ctx.treasure_take_stock else {
                    return Vec::new();
                };
                let mut effects = Vec::new();
                if take_stock {
                    // Stack is LIFO; enqueue move first so stock resolves before hand move.
                    effects.push(EffectSpec {
                        id: self.trigger_effect_id(ctx.source_card, TRIGGER_EFFECT_TREASURE_MOVE),
                        kind: EffectKind::MoveTriggerCardToHand,
                        target: None,
                        optional: false,
                    });
                    effects.push(EffectSpec {
                        id: self.trigger_effect_id(ctx.source_card, TRIGGER_EFFECT_TREASURE_STOCK),
                        kind: EffectKind::TreasureStock { take_stock },
                        target: None,
                        optional: false,
                    });
                    return effects;
                }
                effects.push(EffectSpec {
                    id: self.trigger_effect_id(ctx.source_card, TRIGGER_EFFECT_TREASURE_MOVE),
                    kind: EffectKind::MoveTriggerCardToHand,
                    target: None,
                    optional: false,
                });
                effects
            }
        }
    }

    fn trigger_auto_source_context(
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

    pub(in crate::env) fn resolve_trigger(&mut self, trigger: PendingTrigger) -> Result<bool> {
        if self.db.get(trigger.source_card).is_none() {
            self.log_event(Event::TriggerCanceled {
                trigger_id: trigger.id,
                player: trigger.player,
                reason: TriggerCancelReason::InvalidSource,
            });
            return Ok(false);
        }
        match trigger.effect {
            TriggerEffect::Soul => {
                let ctx = TriggerCompileContext {
                    source_card: trigger.source_card,
                    standby_slot: None,
                    treasure_take_stock: None,
                };
                for spec in self.compile_trigger_icon_effects(TriggerIcon::Soul, ctx) {
                    self.enqueue_effect_spec(trigger.player, trigger.source_card, spec);
                }
            }
            TriggerEffect::Draw => {
                return Ok(self.resolve_trigger_draw(trigger));
            }
            TriggerEffect::Shot => {
                let ctx = TriggerCompileContext {
                    source_card: trigger.source_card,
                    standby_slot: None,
                    treasure_take_stock: None,
                };
                for spec in self.compile_trigger_icon_effects(TriggerIcon::Shot, ctx) {
                    self.enqueue_effect_spec(trigger.player, trigger.source_card, spec);
                }
            }
            TriggerEffect::Gate => {
                let ctx = TriggerCompileContext {
                    source_card: trigger.source_card,
                    standby_slot: None,
                    treasure_take_stock: None,
                };
                for spec in self.compile_trigger_icon_effects(TriggerIcon::Gate, ctx) {
                    self.enqueue_effect_spec(trigger.player, trigger.source_card, spec);
                }
            }
            TriggerEffect::Bounce => {
                let ctx = TriggerCompileContext {
                    source_card: trigger.source_card,
                    standby_slot: None,
                    treasure_take_stock: None,
                };
                for spec in self.compile_trigger_icon_effects(TriggerIcon::Bounce, ctx) {
                    self.enqueue_effect_spec(trigger.player, trigger.source_card, spec);
                }
            }
            TriggerEffect::Choice => {
                return Ok(self.resolve_trigger_choice(trigger));
            }
            TriggerEffect::Pool => {
                let ctx = TriggerCompileContext {
                    source_card: trigger.source_card,
                    standby_slot: None,
                    treasure_take_stock: None,
                };
                for spec in self.compile_trigger_icon_effects(TriggerIcon::Pool, ctx) {
                    self.enqueue_effect_spec(trigger.player, trigger.source_card, spec);
                }
            }
            TriggerEffect::Treasure => {
                return Ok(self.resolve_trigger_treasure(trigger));
            }
            TriggerEffect::Standby => {
                return Ok(self.resolve_trigger_standby(trigger));
            }
            TriggerEffect::AutoAbility { ability_index } => {
                let db = self.db.clone();
                let Some(spec) = db
                    .iter_card_abilities_in_canonical_order(trigger.source_card)
                    .get(ability_index as usize)
                else {
                    self.log_event(Event::TriggerCanceled {
                        trigger_id: trigger.id,
                        player: trigger.player,
                        reason: TriggerCancelReason::InvalidSource,
                    });
                    return Ok(false);
                };
                if !self.auto_ability_conditions_met(trigger.player, trigger.source_card, spec) {
                    self.log_event(Event::TriggerCanceled {
                        trigger_id: trigger.id,
                        player: trigger.player,
                        reason: TriggerCancelReason::Suppressed,
                    });
                    return Ok(false);
                }
                if !self.ability_cost_for_spec(spec).is_empty() {
                    if matches!(spec.template, AbilityTemplate::Bond { .. }) {
                        return Ok(self.present_trigger_auto_cost_choice(trigger, ability_index));
                    }
                    if !self.resolve_trigger_auto_ability_with_cost(
                        trigger.player,
                        trigger.source_card,
                        ability_index,
                    ) {
                        self.log_event(Event::TriggerCanceled {
                            trigger_id: trigger.id,
                            player: trigger.player,
                            reason: TriggerCancelReason::Suppressed,
                        });
                        return Ok(false);
                    }
                    self.log_event(Event::TriggerResolved {
                        trigger_id: trigger.id,
                        player: trigger.player,
                        effect: trigger.effect,
                    });
                    return Ok(true);
                }
                let effects =
                    db.compiled_effects_for_ability(trigger.source_card, ability_index as usize);
                let needs_source_ref = effects.iter().any(|effect| {
                    matches!(
                        effect.kind,
                        EffectKind::MoveToMarker
                            | EffectKind::MoveTopDeckToMarker
                            | EffectKind::MoveWaitingRoomCardToSourceSlot
                            | EffectKind::AddPowerIfOtherAttackerMatches { .. }
                            | EffectKind::HealIfSourcePlayedFromHandThisTurn
                            | EffectKind::FacingOpponentAddModifier { .. }
                            | EffectKind::SelfAddModifierIfFacingOpponent { .. }
                            | EffectKind::BattleOpponentReverseIf { .. }
                            | EffectKind::BattleOpponentMoveToDeckBottomIf { .. }
                            | EffectKind::BattleOpponentMoveToStockThenBottomStockToWaitingRoomIf { .. }
                            | EffectKind::BattleOpponentMoveToClockAfterClockTopToWaitingRoomIf { .. }
                            | EffectKind::BattleOpponentMoveToMemoryIf { .. }
                            | EffectKind::BattleOpponentMoveToClockIf { .. }
                            | EffectKind::BattleOpponentMove { .. }
                    )
                });
                let source_ref = if needs_source_ref {
                    self.trigger_auto_source_context(trigger.player, trigger.source_card)
                        .and_then(|(source_slot, source_inst)| {
                            source_slot.map(|slot| TargetRef {
                                player: trigger.player,
                                zone: TargetZone::Stage,
                                index: slot,
                                card_id: source_inst.id,
                                instance_id: source_inst.instance_id,
                            })
                        })
                } else {
                    None
                };
                for effect in effects {
                    if needs_source_ref {
                        self.enqueue_effect_spec_with_source(
                            trigger.player,
                            trigger.source_card,
                            effect.clone(),
                            source_ref,
                        );
                    } else {
                        self.enqueue_effect_spec(
                            trigger.player,
                            trigger.source_card,
                            effect.clone(),
                        );
                    }
                }
            }
            TriggerEffect::GrantedAutoAbility { grant_id } => {
                let Some(grant) = self
                    .state
                    .turn
                    .granted_abilities
                    .iter()
                    .find(|grant| grant.grant_id == grant_id)
                    .cloned()
                else {
                    self.log_event(Event::TriggerCanceled {
                        trigger_id: trigger.id,
                        player: trigger.player,
                        reason: TriggerCancelReason::InvalidSource,
                    });
                    return Ok(false);
                };
                let source_ref = self.state.players[grant.target_player as usize]
                    .stage
                    .get(grant.target_slot as usize)
                    .and_then(|slot| slot.card)
                    .filter(|card| card.instance_id == grant.target_instance_id)
                    .map(|card| TargetRef {
                        player: grant.target_player,
                        zone: TargetZone::Stage,
                        index: grant.target_slot,
                        card_id: card.id,
                        instance_id: card.instance_id,
                    });
                let spec = grant.spec;
                let effects = grant.compiled_effects;
                if !self.auto_ability_conditions_met(trigger.player, trigger.source_card, &spec) {
                    self.log_event(Event::TriggerCanceled {
                        trigger_id: trigger.id,
                        player: trigger.player,
                        reason: TriggerCancelReason::Suppressed,
                    });
                    return Ok(false);
                }
                if !self.ability_cost_for_spec(&spec).is_empty() {
                    self.log_event(Event::TriggerCanceled {
                        trigger_id: trigger.id,
                        player: trigger.player,
                        reason: TriggerCancelReason::Suppressed,
                    });
                    return Ok(false);
                }
                let needs_source_ref = effects.iter().any(|effect| {
                    matches!(
                        effect.kind,
                        EffectKind::MoveToMarker
                            | EffectKind::MoveTopDeckToMarker
                            | EffectKind::MoveWaitingRoomCardToSourceSlot
                            | EffectKind::AddPowerIfOtherAttackerMatches { .. }
                            | EffectKind::HealIfSourcePlayedFromHandThisTurn
                            | EffectKind::FacingOpponentAddModifier { .. }
                            | EffectKind::SelfAddModifierIfFacingOpponent { .. }
                            | EffectKind::BattleOpponentReverseIf { .. }
                            | EffectKind::BattleOpponentMoveToDeckBottomIf { .. }
                            | EffectKind::BattleOpponentMoveToStockThenBottomStockToWaitingRoomIf { .. }
                            | EffectKind::BattleOpponentMoveToClockAfterClockTopToWaitingRoomIf { .. }
                            | EffectKind::BattleOpponentMoveToMemoryIf { .. }
                            | EffectKind::BattleOpponentMoveToClockIf { .. }
                            | EffectKind::BattleOpponentMove { .. }
                    )
                });
                for effect in effects {
                    if needs_source_ref {
                        self.enqueue_effect_spec_with_source(
                            trigger.player,
                            trigger.source_card,
                            effect,
                            source_ref,
                        );
                    } else {
                        self.enqueue_effect_spec(trigger.player, trigger.source_card, effect);
                    }
                }
            }
        }
        self.log_event(Event::TriggerResolved {
            trigger_id: trigger.id,
            player: trigger.player,
            effect: trigger.effect,
        });
        if self.maybe_validate_state("trigger_resolve") {
            return Ok(true);
        }
        Ok(false)
    }

    pub(in crate::env) fn resolve_trigger_draw(&mut self, trigger: PendingTrigger) -> bool {
        self.scratch.choice_options.clear();
        self.scratch.choice_options.push(ChoiceOptionRef {
            card_id: 0,
            instance_id: 0,
            zone: ChoiceZone::DeckTop,
            index: Some(0),
            target_slot: None,
        });
        self.scratch.choice_options.push(ChoiceOptionRef {
            card_id: 0,
            instance_id: 0,
            zone: ChoiceZone::Skip,
            index: Some(1),
            target_slot: None,
        });
        let options = std::mem::take(&mut self.scratch.choice_options);
        self.start_choice(
            ChoiceReason::TriggerDrawSelect,
            trigger.player,
            options,
            Some(trigger),
        )
    }

    pub(in crate::env) fn resolve_trigger_choice(&mut self, trigger: PendingTrigger) -> bool {
        self.scratch.choice_options.clear();
        for (idx, card_inst) in self.state.players[trigger.player as usize]
            .waiting_room
            .iter()
            .copied()
            .enumerate()
        {
            let Some(card) = self.db.get(card_inst.id) else {
                continue;
            };
            if card.card_type != CardType::Character {
                continue;
            }
            if !card.triggers.contains(&TriggerIcon::Soul) {
                continue;
            }
            let Some(index) = u16::try_from(idx).ok() else {
                self.last_engine_error = true;
                self.last_engine_error_code = EngineErrorCode::ActionError;
                continue;
            };
            self.scratch.choice_options.push(ChoiceOptionRef {
                card_id: card_inst.id,
                instance_id: card_inst.instance_id,
                zone: ChoiceZone::WaitingRoom,
                index: Some(index),
                target_slot: Some(0),
            });
            self.scratch.choice_options.push(ChoiceOptionRef {
                card_id: card_inst.id,
                instance_id: card_inst.instance_id,
                zone: ChoiceZone::WaitingRoom,
                index: Some(index),
                target_slot: Some(1),
            });
        }
        if !self.scratch.choice_options.is_empty() {
            self.scratch.choice_options.push(ChoiceOptionRef {
                card_id: 0,
                instance_id: 0,
                zone: ChoiceZone::Skip,
                index: None,
                target_slot: None,
            });
        }
        let options = std::mem::take(&mut self.scratch.choice_options);
        self.start_choice(
            ChoiceReason::TriggerChoiceSelect,
            trigger.player,
            options,
            Some(trigger),
        )
    }

    pub(in crate::env) fn resolve_trigger_standby(&mut self, trigger: PendingTrigger) -> bool {
        let max_slot = if self.curriculum.reduced_stage_mode {
            1
        } else {
            MAX_STAGE
        };
        let target_slots = (0..max_slot).map(|slot| slot as u8).collect::<Vec<_>>();
        let level_limit = self.state.players[trigger.player as usize]
            .level
            .len()
            .saturating_add(1);
        self.scratch.choice_options.clear();
        // Deterministic ordering: waiting room order, then slot order (ascending).
        for (idx, card_inst) in self.state.players[trigger.player as usize]
            .waiting_room
            .iter()
            .copied()
            .enumerate()
        {
            let Some(card) = self.db.get(card_inst.id) else {
                continue;
            };
            if card.card_type != CardType::Character {
                continue;
            }
            if card.level as usize > level_limit {
                continue;
            }
            let index = if idx <= u16::MAX as usize {
                Some(idx as u16)
            } else {
                self.last_engine_error = true;
                self.last_engine_error_code = EngineErrorCode::ActionError;
                continue;
            };
            for slot in &target_slots {
                self.scratch.choice_options.push(ChoiceOptionRef {
                    card_id: card_inst.id,
                    instance_id: card_inst.instance_id,
                    zone: ChoiceZone::WaitingRoom,
                    index,
                    target_slot: Some(*slot),
                });
            }
        }
        if !self.scratch.choice_options.is_empty() {
            self.scratch.choice_options.push(ChoiceOptionRef {
                card_id: 0,
                instance_id: 0,
                zone: ChoiceZone::Skip,
                index: None,
                target_slot: None,
            });
        }
        let candidates = std::mem::take(&mut self.scratch.choice_options);
        self.start_choice(
            ChoiceReason::TriggerStandbySelect,
            trigger.player,
            candidates,
            Some(trigger),
        )
    }

    pub(in crate::env) fn resolve_trigger_treasure(&mut self, trigger: PendingTrigger) -> bool {
        self.scratch.choice_options.clear();
        if self.treasure_stock_available(trigger.player) {
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
            zone: ChoiceZone::DeckTop,
            index: Some(1),
            target_slot: None,
        });
        let options = std::mem::take(&mut self.scratch.choice_options);
        self.start_choice(
            ChoiceReason::TriggerTreasureSelect,
            trigger.player,
            options,
            Some(trigger),
        )
    }
}
