use super::*;

impl GameEnv {
    #[inline]
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
            let static_specs = self.db.iter_card_abilities_in_canonical_order(card_id);
            for (ability_index, spec) in static_specs.iter().enumerate() {
                if spec.kind != AbilityKind::Auto {
                    continue;
                }
                if spec.timing() == Some(timing) {
                    if !self.auto_ability_conditions_met(player, card_id, spec) {
                        continue;
                    }
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
            for grant in &self.state.turn.granted_abilities {
                if grant.target_player != player
                    || grant.target_slot as usize != slot_idx
                    || grant.target_instance_id != card_inst.instance_id
                {
                    continue;
                }
                let spec = &grant.spec;
                if spec.kind != AbilityKind::Auto || spec.timing() != Some(timing) {
                    continue;
                }
                if !self.auto_ability_conditions_met(player, card_id, spec) {
                    continue;
                }
                pending.push(TriggerSeed {
                    player,
                    source: card_id,
                    effect: TriggerEffect::GrantedAutoAbility {
                        grant_id: grant.grant_id,
                    },
                });
            }
        }
    }

    #[inline]
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

    #[inline]
    fn queue_new_trigger_seed_group(&mut self, pending: Vec<TriggerSeed>) -> bool {
        if pending.is_empty() {
            return false;
        }
        let group_id = self.allocate_trigger_group();
        self.queue_trigger_group_batch(group_id, pending);
        true
    }

    #[inline]
    fn queue_new_trigger_seed_group_and_validate(
        &mut self,
        pending: Vec<TriggerSeed>,
        validate_tag: &'static str,
    ) {
        if self.queue_new_trigger_seed_group(pending) {
            let _ = self.maybe_validate_state(validate_tag);
        }
    }

    #[inline]
    fn sort_pending_triggers(&mut self) {
        self.state
            .turn
            .pending_triggers
            .sort_by_key(pending_trigger_sort_key);
        self.state.turn.pending_triggers_sorted = true;
    }

    #[inline]
    fn ensure_pending_triggers_sorted(&mut self) {
        if !self.state.turn.pending_triggers_sorted {
            self.sort_pending_triggers();
        }
    }

    #[inline]
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

    #[inline]
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

    #[inline]
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

    #[inline]
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

    #[inline]
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

    #[inline]
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
        if triggers.len() > 1 {
            triggers.sort_by_key(trigger_seed_sort_key);
        }
        let mut trigger_ids = Vec::with_capacity(triggers.len());
        let append_preserves_order = self.state.turn.pending_triggers.is_empty()
            || (self.state.turn.pending_triggers_sorted
                && self
                    .state
                    .turn
                    .pending_triggers
                    .last()
                    .map(|last| group_id >= last.group_id)
                    .unwrap_or(true));
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
            if append_preserves_order {
                self.state.turn.pending_triggers_sorted = true;
            } else {
                self.state.turn.pending_triggers_sorted = false;
                self.sort_pending_triggers();
            }
            self.log_event(Event::TriggerGrouped {
                group_id,
                trigger_ids,
            });
        }
    }
}
