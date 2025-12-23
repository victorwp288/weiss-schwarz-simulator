use super::{
    DamageIntentLocal, GameEnv, TriggerCompileContext, TRIGGER_EFFECT_BOUNCE, TRIGGER_EFFECT_DRAW,
    TRIGGER_EFFECT_GATE, TRIGGER_EFFECT_SHOT, TRIGGER_EFFECT_SOUL, TRIGGER_EFFECT_STANDBY,
    TRIGGER_EFFECT_TREASURE_MOVE, TRIGGER_EFFECT_TREASURE_STOCK,
};
use crate::config::*;
use crate::db::*;
use crate::encode::MAX_STAGE;
use crate::effects::*;
use crate::events::*;
use crate::legal::*;
use crate::state::*;

impl GameEnv {
    pub(super) fn handle_trigger_pipeline(&mut self) -> bool {
        if let Some(choice) = &self.state.turn.choice {
            self.decision = Some(Decision {
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

        if let Some(order) = &self.state.turn.trigger_order {
            self.decision = Some(Decision {
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
            let mut choices: Vec<u32> = self
                .state
                .turn
                .pending_triggers
                .iter()
                .filter(|t| t.group_id == group_id && t.player == player)
                .map(|t| t.id)
                .collect();
            if choices.len() > 1 {
                choices.sort_by_key(|id| *id);
                self.state.turn.trigger_order = Some(TriggerOrderState {
                    group_id,
                    player,
                    choices,
                });
                self.decision = Some(Decision {
                    player,
                    kind: DecisionKind::TriggerOrder,
                    focus_slot: None,
                });
                self.maybe_validate_state("trigger_order_decision");
                return true;
            }
            if choices.len() == 1 {
                let trigger_id = choices[0];
                if let Some(index) = self
                    .state
                    .turn
                    .pending_triggers
                    .iter()
                    .position(|t| t.id == trigger_id)
                {
                    let trigger = self.state.turn.pending_triggers.remove(index);
                    if self.resolve_trigger(trigger) {
                        self.maybe_validate_state("trigger_choice_pause");
                        return true;
                    }
                }
                self.maybe_validate_state("trigger_pipeline");
                return true;
            }
        }
        self.maybe_validate_state("trigger_pipeline");
        true
    }

    pub(super) fn handle_priority_window(&mut self) -> bool {
        let Some(priority) = self.state.turn.priority.clone() else {
            return false;
        };
        if self.decision.is_some() {
            return true;
        }
        self.collect_priority_actions(priority.holder);
        if self.scratch.priority_actions.is_empty() {
            self.priority_pass(priority.holder);
            return true;
        }
        if self.scratch.priority_actions.len() == 1
            && self.curriculum.priority_autopick_single_action
        {
            let action = self.scratch.priority_actions[0].clone();
            let _ = self.apply_priority_action(priority.holder, action);
            return true;
        }
        self.start_priority_choice(priority.holder);
        true
    }

    pub(super) fn queue_trigger_group(&mut self, player: u8, source: CardId, effects: Vec<TriggerEffect>) {
        if effects.is_empty() {
            return;
        }
        let group_id = self.allocate_trigger_group();
        self.queue_trigger_group_with_group(group_id, player, source, effects);
    }

    pub(super) fn queue_trigger_group_with_group(
        &mut self,
        group_id: u32,
        player: u8,
        source: CardId,
        effects: Vec<TriggerEffect>,
    ) {
        for effect in effects {
            let id = self.state.turn.next_trigger_id;
            self.state.turn.next_trigger_id = self.state.turn.next_trigger_id.wrapping_add(1);
            let pending = PendingTrigger {
                id,
                group_id,
                player,
                source_card: source,
                effect,
                effect_id: None,
            };
            self.state.turn.pending_triggers.push(pending);
            self.log_event(Event::TriggerQueued {
                trigger_id: id,
                group_id,
                player,
                source,
                effect,
            });
        }
    }

    pub(super) fn trigger_effect_id(&self, source_card: CardId, effect_index: u8) -> EffectId {
        EffectId::new(EffectSourceKind::Trigger, source_card, 0, effect_index)
    }

    pub(super) fn compile_trigger_icon_effects(
        &self,
        icon: TriggerIcon,
        ctx: TriggerCompileContext,
    ) -> Vec<EffectSpec> {
        match icon {
            TriggerIcon::Soul => vec![EffectSpec {
                id: self.trigger_effect_id(ctx.source_card, TRIGGER_EFFECT_SOUL),
                kind: EffectKind::ModifyPendingAttackDamage { delta: 1 },
                target: None,
            }],
            TriggerIcon::Draw => vec![EffectSpec {
                id: self.trigger_effect_id(ctx.source_card, TRIGGER_EFFECT_DRAW),
                kind: EffectKind::Draw { count: 1 },
                target: None,
            }],
            TriggerIcon::Shot => vec![EffectSpec {
                id: self.trigger_effect_id(ctx.source_card, TRIGGER_EFFECT_SHOT),
                kind: EffectKind::Damage {
                    amount: 1,
                    cancelable: true,
                    damage_type: DamageType::Effect,
                },
                target: None,
            }],
            TriggerIcon::Gate => vec![EffectSpec {
                id: self.trigger_effect_id(ctx.source_card, TRIGGER_EFFECT_GATE),
                kind: EffectKind::MoveToHand,
                target: Some(TargetSpec {
                    zone: TargetZone::WaitingRoom,
                    side: TargetSide::SelfSide,
                    slot_filter: TargetSlotFilter::Any,
                    card_type: Some(CardType::Character),
                    count: 1,
                }),
            }],
            TriggerIcon::Bounce => vec![EffectSpec {
                id: self.trigger_effect_id(ctx.source_card, TRIGGER_EFFECT_BOUNCE),
                kind: EffectKind::MoveToHand,
                target: Some(TargetSpec {
                    zone: TargetZone::Stage,
                    side: TargetSide::SelfSide,
                    slot_filter: TargetSlotFilter::Any,
                    card_type: Some(CardType::Character),
                    count: 1,
                }),
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
                        count: 1,
                    }),
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
                    });
                }
                effects.push(EffectSpec {
                    id: self.trigger_effect_id(ctx.source_card, TRIGGER_EFFECT_TREASURE_MOVE),
                    kind: EffectKind::MoveTriggerCardToHand,
                    target: None,
                });
                effects
            }
        }
    }

    pub(super) fn resolve_trigger(&mut self, trigger: PendingTrigger) -> bool {
        if self.db.get(trigger.source_card).is_none() {
            self.log_event(Event::TriggerCanceled {
                trigger_id: trigger.id,
                player: trigger.player,
                reason: TriggerCancelReason::InvalidSource,
            });
            return false;
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
                return self.resolve_trigger_treasure(trigger);
            }
            TriggerEffect::Standby => {
                return self.resolve_trigger_standby(trigger);
            }
            TriggerEffect::EndPhaseDraw { ability_index, .. } => {
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
        false
    }

    pub(super) fn resolve_trigger_standby(&mut self, trigger: PendingTrigger) -> bool {
        let open_slots = self.enumerate_open_stage_slots(trigger.player);
        let target_slots = if open_slots.is_empty() {
            let max_slot = if self.curriculum.reduced_stage_mode {
                1
            } else {
                MAX_STAGE
            };
            (0..max_slot).map(|slot| slot as u8).collect::<Vec<_>>()
        } else {
            open_slots
        };
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
            let index = if idx <= u8::MAX as usize {
                Some(idx as u8)
            } else {
                None
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
        let candidates = std::mem::take(&mut self.scratch.choice_options);
        self.start_choice(
            ChoiceReason::TriggerStandbySelect,
            trigger.player,
            candidates,
            Some(trigger),
        )
    }

    pub(super) fn resolve_trigger_treasure(&mut self, trigger: PendingTrigger) -> bool {
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

    pub(super) fn resolve_stand_phase(&mut self, player: u8) {
        let p = player as usize;
        for slot in &mut self.state.players[p].stage {
            if slot.card.is_some() {
                slot.status = StageStatus::Stand;
                slot.has_attacked = false;
            }
            slot.power_mod_battle = 0;
        }
        self.log_event(Event::Stand { player });
    }

    pub(super) fn resolve_end_phase(&mut self, player: u8) -> bool {
        if !self.state.turn.end_phase_pending {
            self.expire_end_of_turn_effects();
            self.queue_end_phase_triggers();
            self.state.turn.end_phase_pending = true;
            self.state.turn.end_phase_window_done = false;
        }
        if !self.state.turn.pending_triggers.is_empty() {
            return false;
        }
        if self.curriculum.enable_priority_windows && !self.state.turn.end_phase_window_done {
            self.state.turn.end_phase_window_done = true;
            if self.state.turn.priority.is_none() {
                self.enter_timing_window(TimingWindow::EndPhaseWindow, player);
            }
            return false;
        }
        self.finish_end_phase(player);
        self.state.turn.end_phase_pending = false;
        true
    }

    pub(super) fn expire_end_of_turn_effects(&mut self) {
        for pid in 0..2 {
            for slot in &mut self.state.players[pid].stage {
                slot.power_mod_battle = 0;
                slot.power_mod_turn = 0;
                slot.cannot_attack = false;
                slot.attack_cost = 0;
            }
        }
        let mut removed: Vec<u32> = Vec::new();
        self.state.modifiers.retain(|m| {
            if m.duration == ModifierDuration::UntilEndOfTurn {
                removed.push(m.id);
                false
            } else {
                true
            }
        });
        for id in removed {
            self.log_event(Event::ModifierRemoved {
                id,
                reason: ModifierRemoveReason::EndOfTurn,
            });
        }
        self.state.turn.derived_attack = None;
        self.maybe_validate_state("end_phase_expire");
    }

    pub(super) fn recompute_derived_attack(&mut self) {
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
                            _ => {}
                        }
                    }
                }
                derived.per_player[player][slot] = entry;
            }
        }
        self.state.turn.derived_attack = Some(derived);
        self.maybe_validate_state("derived_attack_recompute");
    }

    pub(super) fn queue_end_phase_triggers(&mut self) {
        if !self.curriculum.enable_triggers {
            return;
        }
        let mut pending: Vec<(u8, CardId, TriggerEffect)> = Vec::new();
        for player in 0..2 {
            for slot in &self.state.players[player].stage {
                let Some(card_inst) = slot.card else {
                    continue;
                };
                let card_id = card_inst.id;
                if self.db.get(card_id).is_none() {
                    continue;
                }
                let specs = self.db.iter_card_abilities_in_canonical_order(card_id);
                for (ability_index, spec) in specs.iter().enumerate() {
                    match &spec.template {
                        AbilityTemplate::AutoEndPhaseDraw { count } => {
                            pending.push((
                                player as u8,
                                card_id,
                                TriggerEffect::EndPhaseDraw {
                                    count: *count,
                                    ability_index: ability_index as u8,
                                },
                            ));
                        }
                        AbilityTemplate::AbilityDef(def) => {
                            if def.kind != AbilityKind::Auto {
                                continue;
                            }
                            if def.timing != Some(crate::db::AbilityTiming::EndPhase) {
                                continue;
                            }
                            for effect in &def.effects {
                                if let crate::db::EffectTemplate::Draw { count } = effect {
                                    pending.push((
                                        player as u8,
                                        card_id,
                                        TriggerEffect::EndPhaseDraw {
                                            count: *count,
                                            ability_index: ability_index as u8,
                                        },
                                    ));
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
        if pending.is_empty() {
            return;
        }
        let group_id = self.allocate_trigger_group();
        for (player, source, effect) in pending {
            self.queue_trigger_group_with_group(group_id, player, source, vec![effect]);
        }
        self.maybe_validate_state("end_phase_triggers");
    }

    pub(super) fn finish_end_phase(&mut self, player: u8) {
        let p = player as usize;
        if let Some(card) = self.state.players[p].climax.pop() {
            self.move_card_between_zones(player, card, Zone::Climax, Zone::WaitingRoom, None, None);
        }
        self.state.turn.pending_triggers.clear();
        self.state.turn.trigger_order = None;
        self.state.turn.choice = None;
        self.state.turn.priority = None;
        self.state.turn.stack.clear();
        self.state.turn.pending_stack_groups.clear();
        self.state.turn.stack_order = None;
        self.state.turn.derived_attack = None;
        self.state.turn.attack = None;
        self.state.turn.encore_queue.clear();
        self.state.turn.pending_level_up = None;
        self.state.turn.main_passed = false;
        self.state.turn.active_window = None;
        self.state.turn.end_phase_window_done = false;
        self.state.turn.encore_window_done = false;
        self.state.turn.pending_losses = [false; 2];
        self.log_event(Event::EndTurn { player });
        self.maybe_validate_state("end_phase_finish");
    }

    pub(super) fn has_attackers(&self, player: u8) -> bool {
        !crate::legal::legal_attack_actions(&self.state, player, &self.curriculum).is_empty()
    }

    pub(super) fn resolve_attack_pipeline(&mut self) {
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
                    self.resolve_trigger_step(&mut ctx);
                    if ctx.counter_allowed && self.curriculum.enable_counters {
                        ctx.step = AttackStep::Counter;
                    } else {
                        ctx.step = AttackStep::Damage;
                    }
                    if self.state.turn.pending_level_up.is_some()
                        || !self.state.turn.pending_triggers.is_empty()
                    {
                        self.state.turn.attack = Some(ctx);
                        self.maybe_validate_state("attack_trigger_pause");
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
                    self.maybe_validate_state("attack_counter_window");
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
                    let pause = self.resolve_damage_step(&mut ctx);
                    if pause {
                        self.state.turn.attack = Some(ctx);
                        self.maybe_validate_state("attack_damage_pause");
                        break;
                    }
                    if ctx.attack_type == AttackType::Direct {
                        self.clear_battle_mods();
                        self.state.turn.attack = None;
                        self.maybe_validate_state("attack_direct_done");
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
                    self.maybe_validate_state("attack_battle_done");
                    break;
                }
                AttackStep::Encore => {
                    self.state.turn.attack = Some(ctx);
                    self.maybe_validate_state("attack_encore_hold");
                    break;
                }
            }
            self.maybe_validate_state("attack_pipeline");
        }
    }

    pub(super) fn resolve_trigger_step(&mut self, ctx: &mut AttackContext) {
        let active = self.state.turn.active_player as usize;
        let card = self.draw_from_deck(active as u8);
        if let Some(card_inst) = card {
            let card_id = card_inst.id;
            ctx.trigger_card = Some(card_id);
            let _ = self.reveal_cards(
                active as u8,
                &[card_inst],
                RevealReason::TriggerCheck,
                RevealAudience::Public,
            );
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
                    self.queue_trigger_group(active as u8, card_id, effects);
                }
            }
            self.move_card_between_zones(
                active as u8,
                card_inst,
                Zone::Deck,
                Zone::Stock,
                None,
                None,
            );
        }
    }

    pub(super) fn resolve_damage_step(&mut self, ctx: &mut AttackContext) -> bool {
        let attacker = self.state.turn.active_player;
        let defender = 1 - attacker;
        if !ctx.auto_damage_enqueued {
            self.enqueue_attack_auto_effects(ctx, attacker);
            ctx.auto_damage_enqueued = true;
            if !self.state.turn.stack.is_empty() {
                return true;
            }
        }
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
            let event_id = self.resolve_damage_intent(intent, &mut ctx.damage_modifiers);
            ctx.last_damage_event_id = Some(event_id);
            ctx.battle_damage_applied = true;
        }
        self.state.turn.pending_level_up.is_some()
    }

    pub(super) fn enqueue_attack_auto_effects(&mut self, ctx: &AttackContext, attacker: u8) {
        let attacker_slot = ctx.attacker_slot as usize;
        if let Some(card_inst) = self.state.players[attacker as usize].stage[attacker_slot].card {
            let card_id = card_inst.id;
            let db = self.db.clone();
            if db.get(card_id).is_none() {
                return;
            }
            let specs = db.iter_card_abilities_in_canonical_order(card_id);
            for (ability_index, spec) in specs.iter().enumerate() {
                if spec.kind != AbilityKind::Auto {
                    continue;
                }
                let timing = match &spec.template {
                    AbilityTemplate::AutoOnAttackDealDamage { .. } => {
                        Some(crate::db::AbilityTiming::AttackDeclaration)
                    }
                    AbilityTemplate::AbilityDef(def) => def.timing,
                    _ => None,
                };
                if timing == Some(crate::db::AbilityTiming::AttackDeclaration) {
                    let effects = db.compiled_effects_for_ability(card_id, ability_index);
                    for effect in effects {
                        self.enqueue_effect_spec(attacker, card_id, effect.clone());
                    }
                }
            }
        }
    }

    pub(super) fn resolve_effect_damage(
        &mut self,
        source_player: u8,
        target: u8,
        amount: i32,
        cancelable: bool,
        refresh_penalty: bool,
        _source_card: Option<CardId>,
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
        let _ = self.resolve_damage_intent(intent, &mut modifiers);
        if let Some(ctx) = &mut self.state.turn.attack {
            ctx.damage_modifiers = modifiers;
        }
        self.state.turn.pending_level_up.is_some()
    }

    pub(super) fn resolve_damage_intent(
        &mut self,
        intent: DamageIntentLocal,
        modifiers: &mut [DamageModifier],
    ) -> u32 {
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
        if cancelable && !canceled && amount > 0 {
            for _ in 0..amount {
                if let Some(card) = self.draw_from_deck(intent.target) {
                    self.reveal_card(
                        intent.target,
                        &card,
                        RevealReason::DamageCheck,
                        RevealAudience::Public,
                    );
                    revealed.push(card);
                    if let Some(static_card) = self.db.get(card.id) {
                        if static_card.card_type == CardType::Climax {
                            canceled = true;
                            break;
                        }
                    }
                } else {
                    break;
                }
            }
        }

        let committed = if canceled {
            0
        } else if cancelable {
            revealed.len() as i32
        } else {
            amount
        };
        self.log_event(Event::DamageModified {
            event_id,
            target: intent.target,
            original: intent.amount,
            modified: committed,
            canceled,
            damage_type: intent.damage_type,
        });

        let target = intent.target as usize;
        if canceled {
            self.log_event(Event::DamageCancel {
                player: intent.target,
            });
            for card in revealed {
                self.move_card_between_zones(
                    intent.target,
                    card,
                    Zone::Deck,
                    Zone::WaitingRoom,
                    None,
                    None,
                );
            }
            return event_id;
        }

        if cancelable {
            for card in revealed {
                let card_id = card.id;
                self.move_card_between_zones(
                    intent.target,
                    card,
                    Zone::Deck,
                    Zone::Clock,
                    None,
                    None,
                );
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
        } else {
            let count = amount as usize;
            for _ in 0..count {
                if let Some(card) = self.draw_from_deck(intent.target) {
                    let card_id = card.id;
                    if intent.refresh_penalty {
                        self.reveal_card(
                            intent.target,
                            &card,
                            RevealReason::RefreshPenalty,
                            RevealAudience::Public,
                        );
                    }
                    self.move_card_between_zones(
                        intent.target,
                        card,
                        Zone::Deck,
                        Zone::Clock,
                        None,
                        None,
                    );
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
                    if intent.refresh_penalty {
                        self.log_event(Event::RefreshPenalty {
                            player: intent.target,
                            card: card_id,
                        });
                    }
                    self.pending_damage_delta[target] += 1;
                }
            }
        }
        self.check_level_up(intent.target);
        event_id
    }

    pub(super) fn resolve_battle_step(&mut self, ctx: &AttackContext) {
        let attacker = self.state.turn.active_player as usize;
        let defender = 1 - attacker;
        let atk_slot = ctx.attacker_slot as usize;
        let def_slot = match ctx.defender_slot {
            Some(s) => s as usize,
            None => return,
        };
        let atk_power = self.compute_slot_power(attacker, atk_slot);
        let def_power = self.compute_slot_power(defender, def_slot);
        if atk_power > def_power {
            self.state.players[defender].stage[def_slot].status = StageStatus::Reverse;
            self.log_event(Event::ReversalCommitted {
                player: defender as u8,
                slot: def_slot as u8,
                cause_damage_event: ctx.last_damage_event_id,
            });
        } else if atk_power < def_power {
            self.state.players[attacker].stage[atk_slot].status = StageStatus::Reverse;
            self.log_event(Event::ReversalCommitted {
                player: attacker as u8,
                slot: atk_slot as u8,
                cause_damage_event: ctx.last_damage_event_id,
            });
        } else {
            self.state.players[defender].stage[def_slot].status = StageStatus::Reverse;
            self.state.players[attacker].stage[atk_slot].status = StageStatus::Reverse;
            self.log_event(Event::ReversalCommitted {
                player: defender as u8,
                slot: def_slot as u8,
                cause_damage_event: ctx.last_damage_event_id,
            });
            self.log_event(Event::ReversalCommitted {
                player: attacker as u8,
                slot: atk_slot as u8,
                cause_damage_event: ctx.last_damage_event_id,
            });
        }
    }

    pub(super) fn queue_encore_requests(&mut self) {
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
    }

    pub(super) fn cleanup_reversed_to_waiting_room(&mut self) {
        for player in 0..2 {
            for slot in 0..self.state.players[player].stage.len() {
                if self.state.players[player].stage[slot].status == StageStatus::Reverse {
                    self.send_stage_to_waiting_room(player as u8, slot as u8);
                }
            }
        }
    }

    pub(super) fn clear_battle_mods(&mut self) {
        for player in 0..2 {
            for slot in &mut self.state.players[player].stage {
                slot.power_mod_battle = 0;
            }
        }
    }

    pub(super) fn register_loss(&mut self, player: u8) {
        if !self.curriculum.use_alternate_end_conditions {
            self.state.terminal = Some(TerminalResult::Win { winner: 1 - player });
            return;
        }
        self.state.turn.pending_losses[player as usize] = true;
    }

    pub(super) fn resolve_pending_losses(&mut self) {
        if !self.curriculum.use_alternate_end_conditions {
            return;
        }
        if self.state.terminal.is_some() {
            return;
        }
        let p0 = self.state.turn.pending_losses[0];
        let p1 = self.state.turn.pending_losses[1];
        if !(p0 || p1) {
            return;
        }
        let result = if p0 && p1 {
            match self.config.end_condition_policy.simultaneous_loss {
                SimultaneousLossPolicy::Draw => {
                    if self
                        .config
                        .end_condition_policy
                        .allow_draw_on_simultaneous_loss
                    {
                        TerminalResult::Draw
                    } else {
                        TerminalResult::Win {
                            winner: self.state.turn.active_player,
                        }
                    }
                }
                SimultaneousLossPolicy::ActivePlayerWins => TerminalResult::Win {
                    winner: self.state.turn.active_player,
                },
                SimultaneousLossPolicy::NonActivePlayerWins => TerminalResult::Win {
                    winner: 1 - self.state.turn.active_player,
                },
            }
        } else if p0 {
            TerminalResult::Win { winner: 1 }
        } else {
            TerminalResult::Win { winner: 0 }
        };
        self.state.terminal = Some(result);
    }
}
