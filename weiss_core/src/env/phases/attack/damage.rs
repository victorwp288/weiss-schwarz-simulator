use super::*;

impl GameEnv {
    #[inline]
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

    #[allow(clippy::too_many_arguments)]
    #[inline]
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

    #[inline]
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

    #[inline]
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

    #[inline]
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
}
