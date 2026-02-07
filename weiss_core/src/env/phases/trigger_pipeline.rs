use super::super::{
    EngineErrorCode, GameEnv, TriggerCompileContext, TRIGGER_EFFECT_BOUNCE, TRIGGER_EFFECT_DRAW,
    TRIGGER_EFFECT_GATE, TRIGGER_EFFECT_SHOT, TRIGGER_EFFECT_SOUL, TRIGGER_EFFECT_STANDBY,
    TRIGGER_EFFECT_TREASURE_MOVE, TRIGGER_EFFECT_TREASURE_STOCK,
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

fn trigger_effect_sort_key(effect: TriggerEffect) -> (u8, u8) {
    match effect {
        TriggerEffect::Soul => (0, 0),
        TriggerEffect::Draw => (1, 0),
        TriggerEffect::Shot => (2, 0),
        TriggerEffect::Bounce => (3, 0),
        TriggerEffect::Treasure => (4, 0),
        TriggerEffect::Gate => (5, 0),
        TriggerEffect::Standby => (6, 0),
        TriggerEffect::AutoAbility { ability_index } => (7, ability_index),
    }
}

fn trigger_seed_sort_key(seed: &TriggerSeed) -> (u8, u32, u8, u8) {
    let (kind, sub) = trigger_effect_sort_key(seed.effect);
    (seed.player, seed.source, kind, sub)
}

fn pending_trigger_sort_key(trigger: &PendingTrigger) -> (u32, u8, u32, u8, u8, u32) {
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
    pub(in crate::env) fn queue_timing_triggers(&mut self, timing: AbilityTiming) {
        if !self.curriculum.enable_triggers {
            return;
        }
        let mut pending: Vec<TriggerSeed> = Vec::new();
        for player in 0..2u8 {
            for slot in &self.state.players[player as usize].stage {
                let Some(card_inst) = slot.card else {
                    continue;
                };
                let card_id = card_inst.id;
                if self.db.get(card_id).is_none() {
                    continue;
                }
                let specs = self.db.iter_card_abilities_in_canonical_order(card_id);
                for (ability_index, spec) in specs.iter().enumerate() {
                    if spec.kind != AbilityKind::Auto {
                        continue;
                    }
                    if spec.timing() == Some(timing) {
                        let Ok(ability_index) = u8::try_from(ability_index) else {
                            debug_assert!(
                                ability_index <= u8::MAX as usize,
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
            for card_inst in &self.state.players[player as usize].climax {
                let card_id = card_inst.id;
                if self.db.get(card_id).is_none() {
                    continue;
                }
                let specs = self.db.iter_card_abilities_in_canonical_order(card_id);
                for (ability_index, spec) in specs.iter().enumerate() {
                    if spec.kind != AbilityKind::Auto {
                        continue;
                    }
                    if spec.timing() == Some(timing) {
                        let Ok(ability_index) = u8::try_from(ability_index) else {
                            debug_assert!(
                                ability_index <= u8::MAX as usize,
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
        }
        if pending.is_empty() {
            return;
        }
        let group_id = self.allocate_trigger_group();
        self.queue_trigger_group_batch(group_id, pending);
        self.maybe_validate_state("check_timing_triggers");
    }

    pub(in crate::env) fn queue_on_reverse_triggers(&mut self, reversed: &[(u8, CardId)]) {
        if !self.curriculum.enable_triggers || !self.curriculum.enable_on_reverse_triggers {
            return;
        }
        let mut pending: Vec<TriggerSeed> = Vec::new();
        for (player, card_id) in reversed {
            if self.db.get(*card_id).is_none() {
                continue;
            }
            let specs = self.db.iter_card_abilities_in_canonical_order(*card_id);
            for (ability_index, spec) in specs.iter().enumerate() {
                if spec.kind != AbilityKind::Auto {
                    continue;
                }
                if spec.timing() == Some(AbilityTiming::OnReverse) {
                    let Ok(ability_index) = u8::try_from(ability_index) else {
                        debug_assert!(
                            ability_index <= u8::MAX as usize,
                            "ability index out of range"
                        );
                        continue;
                    };
                    pending.push(TriggerSeed {
                        player: *player,
                        source: *card_id,
                        effect: TriggerEffect::AutoAbility { ability_index },
                    });
                }
            }
        }
        if pending.is_empty() {
            return;
        }
        let group_id = self.allocate_trigger_group();
        self.queue_trigger_group_batch(group_id, pending);
        self.maybe_validate_state("on_reverse_triggers");
    }

    pub(in crate::env) fn handle_trigger_pipeline(&mut self) -> bool {
        if let Some(choice) = &self.state.turn.choice {
            self.set_decision(Decision {
                player: choice.player,
                kind: DecisionKind::Choice,
                focus_slot: None,
            });
            self.maybe_validate_state("choice_decision");
            return true;
        }
        if self.state.turn.pending_triggers.is_empty() {
            self.state.turn.trigger_order = None;
            return false;
        }
        if !self.state.turn.pending_triggers_sorted {
            self.state
                .turn
                .pending_triggers
                .sort_by_key(pending_trigger_sort_key);
            self.state.turn.pending_triggers_sorted = true;
        }

        if let Some(order) = &self.state.turn.trigger_order {
            self.set_decision(Decision {
                player: order.player,
                kind: DecisionKind::TriggerOrder,
                focus_slot: None,
            });
            self.maybe_validate_state("trigger_order_decision");
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
                self.maybe_validate_state("trigger_order_decision");
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
                        self.maybe_validate_state("trigger_choice_pause");
                        return true;
                    }
                    self.maybe_validate_state("trigger_pipeline");
                    return true;
                }
                break;
            }
        }
        self.maybe_validate_state("trigger_pipeline");
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
        let group_id = self.allocate_trigger_group();
        let triggers = effects
            .into_iter()
            .map(|effect| TriggerSeed {
                player,
                source,
                effect,
            })
            .collect();
        self.queue_trigger_group_batch(group_id, triggers);
    }

    fn queue_trigger_group_batch(&mut self, group_id: u32, mut triggers: Vec<TriggerSeed>) {
        triggers.sort_by_key(trigger_seed_sort_key);
        let mut trigger_ids = Vec::with_capacity(triggers.len());
        for trigger in triggers {
            let id = self.state.turn.next_trigger_id;
            self.state.turn.next_trigger_id = self
                .state
                .turn
                .next_trigger_id
                .checked_add(1)
                .expect("trigger id overflow");
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
        self.state
            .turn
            .pending_triggers
            .sort_by_key(pending_trigger_sort_key);
        self.state.turn.pending_triggers_sorted = true;
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
                kind: EffectKind::Damage {
                    amount: 1,
                    cancelable: true,
                    damage_type: DamageType::Effect,
                },
                target: None,
                optional: false,
            }],
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
                    side: TargetSide::SelfSide,
                    slot_filter: TargetSlotFilter::Any,
                    card_type: Some(CardType::Character),
                    card_trait: None,
                    level_max: None,
                    cost_max: None,
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
                    effects.push(EffectSpec {
                        id: self.trigger_effect_id(ctx.source_card, TRIGGER_EFFECT_TREASURE_STOCK),
                        kind: EffectKind::TreasureStock { take_stock },
                        target: None,
                        optional: false,
                    });
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
                let ctx = TriggerCompileContext {
                    source_card: trigger.source_card,
                    standby_slot: None,
                    treasure_take_stock: None,
                };
                for spec in self.compile_trigger_icon_effects(TriggerIcon::Draw, ctx) {
                    self.enqueue_effect_spec(trigger.player, trigger.source_card, spec);
                }
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
            TriggerEffect::Treasure => {
                return Ok(self.resolve_trigger_treasure(trigger));
            }
            TriggerEffect::Standby => {
                return Ok(self.resolve_trigger_standby(trigger));
            }
            TriggerEffect::AutoAbility { ability_index } => {
                let db = self.db.clone();
                let effects =
                    db.compiled_effects_for_ability(trigger.source_card, ability_index as usize);
                for effect in effects {
                    self.enqueue_effect_spec(trigger.player, trigger.source_card, effect.clone());
                }
            }
        }
        self.log_event(Event::TriggerResolved {
            trigger_id: trigger.id,
            player: trigger.player,
            effect: trigger.effect,
        });
        self.maybe_validate_state("trigger_resolve");
        Ok(false)
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
