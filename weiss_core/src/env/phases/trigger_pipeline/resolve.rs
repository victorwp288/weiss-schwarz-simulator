use super::*;

impl GameEnv {
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
