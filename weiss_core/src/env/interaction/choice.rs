use super::super::{
    GameEnv, TriggerCompileContext, VisibilityContext, HAND_LIMIT, MAX_CHOICE_OPTIONS,
};
use crate::db::TriggerIcon;
use crate::events::{ChoiceOptionSnapshot, ChoiceSkipReason, Event, Zone};
use crate::state::{
    ChoiceOptionRef, ChoiceReason, ChoiceState, ChoiceZone, PendingTrigger, TargetRef, TargetSide,
    TargetZone, TriggerEffect,
};

impl GameEnv {
    pub(in crate::env) fn allocate_choice_id(&mut self) -> u32 {
        let choice_id = self.state.turn.next_choice_id;
        self.state.turn.next_choice_id = self.state.turn.next_choice_id.wrapping_add(1);
        choice_id
    }

    pub(in crate::env) fn choice_option_id(
        &self,
        option: &ChoiceOptionRef,
        choice_id: u32,
        global_index: usize,
    ) -> u64 {
        let zone_id = match option.zone {
            ChoiceZone::WaitingRoom => 1u64,
            ChoiceZone::Stage => 2u64,
            ChoiceZone::DeckTop => 3u64,
            ChoiceZone::Hand => 4u64,
            ChoiceZone::Clock => 5u64,
            ChoiceZone::Level => 6u64,
            ChoiceZone::Stock => 7u64,
            ChoiceZone::Memory => 8u64,
            ChoiceZone::Climax => 9u64,
            ChoiceZone::Resolution => 10u64,
            ChoiceZone::Stack => 11u64,
            ChoiceZone::PriorityCounter => 12u64,
            ChoiceZone::PriorityAct => 13u64,
            ChoiceZone::PriorityPass => 14u64,
            ChoiceZone::Skip => 15u64,
        };
        let index = option.index.unwrap_or(0) as u64;
        let target = option.target_slot.unwrap_or(0) as u64;
        let hidden_zone = matches!(
            option.zone,
            ChoiceZone::Hand
                | ChoiceZone::DeckTop
                | ChoiceZone::Stock
                | ChoiceZone::PriorityCounter
        ) || (option.zone == ChoiceZone::Memory
            && !self.curriculum.memory_is_public);
        if option.instance_id != 0 {
            (option.instance_id as u64) << 32 | (zone_id << 24) | (index << 8) | target
        } else if option.card_id != 0 && !hidden_zone {
            (option.card_id as u64) << 32 | (zone_id << 24) | (index << 8) | target
        } else {
            let choice_tag = (choice_id as u64) << 32;
            let global_tag = (global_index as u64 & 0xFFFF) << 8;
            choice_tag | (zone_id << 24) | global_tag | target
        }
    }

    pub(in crate::env) fn summarize_choice_options_for_event(
        &self,
        reason: ChoiceReason,
        player: u8,
        options: &[ChoiceOptionSnapshot],
        page_start: u16,
        choice_id: u32,
        ctx: VisibilityContext,
    ) -> Vec<ChoiceOptionSnapshot> {
        options
            .iter()
            .enumerate()
            .map(|(idx, opt)| {
                let global_index = page_start as usize + idx;
                let sanitized =
                    self.sanitize_choice_option_for_event(reason, player, ctx, &opt.reference);
                ChoiceOptionSnapshot {
                    option_id: self.choice_option_id(&sanitized, choice_id, global_index),
                    reference: sanitized,
                }
            })
            .collect()
    }

    pub(in crate::env) fn sanitize_choice_option_for_event(
        &self,
        reason: ChoiceReason,
        player: u8,
        ctx: VisibilityContext,
        option: &ChoiceOptionRef,
    ) -> ChoiceOptionRef {
        if !ctx.is_public() {
            return *option;
        }
        let sanitize_instance = ctx.viewer.is_none();
        let option_player = if reason == ChoiceReason::TargetSelect {
            self.state
                .turn
                .target_selection
                .as_ref()
                .map(|selection| match selection.spec.side {
                    TargetSide::SelfSide => selection.controller,
                    TargetSide::Opponent => {
                        debug_assert!(
                            selection.controller <= 1,
                            "invalid target selection controller {}",
                            selection.controller
                        );
                        match selection.controller {
                            0 => 1,
                            1 => 0,
                            _ => 0,
                        }
                    }
                })
                .unwrap_or(player)
        } else {
            player
        };
        let hide_for_viewer = match ctx.viewer {
            Some(viewer) => viewer != option_player,
            None => true,
        };
        if !hide_for_viewer {
            if sanitize_instance && option.instance_id != 0 {
                let mut sanitized = *option;
                sanitized.instance_id = 0;
                return sanitized;
            }
            return *option;
        }
        let hide_zone = matches!(
            option.zone,
            ChoiceZone::Hand
                | ChoiceZone::DeckTop
                | ChoiceZone::Stock
                | ChoiceZone::PriorityCounter
        ) || (option.zone == ChoiceZone::Memory
            && !self.curriculum.memory_is_public);
        if !hide_zone {
            if sanitize_instance && option.instance_id != 0 {
                let mut sanitized = *option;
                sanitized.instance_id = 0;
                return sanitized;
            }
            return *option;
        }
        let revealed = self.instance_revealed_to_viewer(ctx, option.instance_id);
        ChoiceOptionRef {
            card_id: if revealed { option.card_id } else { 0 },
            instance_id: 0,
            zone: option.zone,
            index: None,
            target_slot: option.target_slot,
        }
    }

    pub(in crate::env) fn choice_page_bounds(
        &self,
        total: usize,
        page_start: usize,
    ) -> (usize, usize) {
        let start = page_start.min(total);
        let end = total.min(start + MAX_CHOICE_OPTIONS);
        (start, end)
    }

    pub(in crate::env) fn recycle_choice_options(&mut self, options: Vec<ChoiceOptionRef>) {
        self.scratch.choice_options = options;
    }

    pub(in crate::env) fn start_choice(
        &mut self,
        reason: ChoiceReason,
        player: u8,
        candidates: Vec<ChoiceOptionRef>,
        pending_trigger: Option<PendingTrigger>,
    ) -> bool {
        let total = candidates.len();
        let choice_id = self.allocate_choice_id();
        if total == 0 {
            if self.recording {
                self.log_event(Event::ChoiceSkipped {
                    choice_id,
                    player,
                    reason,
                    skip_reason: ChoiceSkipReason::NoCandidates,
                });
            }
            if self.recording {
                if let Some(trigger) = pending_trigger {
                    self.log_event(Event::TriggerResolved {
                        trigger_id: trigger.id,
                        player: trigger.player,
                        effect: trigger.effect,
                    });
                }
            }
            self.recycle_choice_options(candidates);
            return false;
        }
        if total == 1 {
            let option = candidates[0];
            if self.recording {
                self.log_event(Event::ChoiceAutopicked {
                    choice_id,
                    player,
                    reason,
                    option,
                });
            }
            self.recycle_choice_options(candidates);
            self.apply_choice_effect(reason, player, option, pending_trigger);
            return false;
        }
        let page_start = 0u16;
        let (page_start_idx, page_end_idx) = self.choice_page_bounds(total, 0);
        let page_slice = &candidates[page_start_idx..page_end_idx];
        let total_candidates = total.min(u16::MAX as usize) as u16;
        if self.recording {
            let mut options = Vec::with_capacity(page_slice.len());
            for (idx, opt) in page_slice.iter().enumerate() {
                options.push(ChoiceOptionSnapshot {
                    option_id: self.choice_option_id(opt, choice_id, page_start as usize + idx),
                    reference: *opt,
                });
            }
            self.log_event(Event::ChoicePresented {
                choice_id,
                player,
                reason,
                options,
                total_candidates,
                page_start,
            });
        }
        self.state.turn.choice = Some(ChoiceState {
            id: choice_id,
            reason,
            player,
            options: candidates,
            total_candidates,
            page_start,
            pending_trigger,
        });
        true
    }

    pub(in crate::env) fn apply_choice_effect(
        &mut self,
        reason: ChoiceReason,
        player: u8,
        option: ChoiceOptionRef,
        pending_trigger: Option<PendingTrigger>,
    ) {
        let resolve_pending_trigger = |env: &mut Self| {
            if let Some(trigger) = pending_trigger.as_ref() {
                env.log_event(Event::TriggerResolved {
                    trigger_id: trigger.id,
                    player: trigger.player,
                    effect: trigger.effect,
                });
            }
        };
        match reason {
            ChoiceReason::TriggerStandbySelect => {
                if option.zone != ChoiceZone::Skip {
                    let Some(target_slot) = option.target_slot else {
                        resolve_pending_trigger(self);
                        return;
                    };
                    let ctx = TriggerCompileContext {
                        source_card: pending_trigger
                            .as_ref()
                            .map(|t| t.source_card)
                            .unwrap_or(option.card_id),
                        standby_slot: Some(target_slot),
                        treasure_take_stock: None,
                    };
                    let effects = self.compile_trigger_icon_effects(TriggerIcon::Standby, ctx);
                    if effects.is_empty() {
                        resolve_pending_trigger(self);
                        return;
                    }
                    let Some(index) = option.index else {
                        resolve_pending_trigger(self);
                        return;
                    };
                    let Ok(index) = u8::try_from(index) else {
                        resolve_pending_trigger(self);
                        return;
                    };
                    let targets = vec![TargetRef {
                        player,
                        zone: TargetZone::WaitingRoom,
                        index,
                        card_id: option.card_id,
                        instance_id: option.instance_id,
                    }];
                    for effect in effects {
                        self.enqueue_effect_with_targets(
                            player,
                            ctx.source_card,
                            effect,
                            targets.clone(),
                        );
                    }
                }
            }
            ChoiceReason::TriggerTreasureSelect => {
                let take_stock = option.index.unwrap_or(1) == 0;
                let ctx = TriggerCompileContext {
                    source_card: pending_trigger.as_ref().map(|t| t.source_card).unwrap_or(0),
                    standby_slot: None,
                    treasure_take_stock: Some(take_stock),
                };
                let effects = self.compile_trigger_icon_effects(TriggerIcon::Treasure, ctx);
                for effect in effects {
                    self.enqueue_effect_spec(player, ctx.source_card, effect);
                }
            }
            ChoiceReason::TriggerDrawSelect => {
                if option.zone == ChoiceZone::Skip {
                    resolve_pending_trigger(self);
                    return;
                }
                let ctx = TriggerCompileContext {
                    source_card: pending_trigger.as_ref().map(|t| t.source_card).unwrap_or(0),
                    standby_slot: None,
                    treasure_take_stock: None,
                };
                let effects = self.compile_trigger_icon_effects(TriggerIcon::Draw, ctx);
                for effect in effects {
                    self.enqueue_effect_spec(player, ctx.source_card, effect);
                }
            }
            ChoiceReason::BrainstormDrawSelect => {
                if option.zone == ChoiceZone::Skip {
                    resolve_pending_trigger(self);
                    return;
                }
                self.draw_to_hand(player, 1);
            }
            ChoiceReason::TriggerChoiceSelect => {
                if option.zone == ChoiceZone::Skip {
                    resolve_pending_trigger(self);
                    return;
                }
                if option.zone != ChoiceZone::WaitingRoom {
                    resolve_pending_trigger(self);
                    return;
                }
                let Some(index) = option.index else {
                    resolve_pending_trigger(self);
                    return;
                };
                let idx = index as usize;
                let p = player as usize;
                if idx >= self.state.players[p].waiting_room.len() {
                    resolve_pending_trigger(self);
                    return;
                }
                let Some(card) = self.state.players[p].waiting_room.get(idx).copied() else {
                    resolve_pending_trigger(self);
                    return;
                };
                if card.instance_id != option.instance_id {
                    resolve_pending_trigger(self);
                    return;
                }
                let target_slot = option.target_slot.unwrap_or(0);
                if target_slot != 0 && target_slot != 1 {
                    resolve_pending_trigger(self);
                    return;
                }
                let card = self.state.players[p].waiting_room.remove(idx);
                if target_slot == 0 {
                    self.move_card_between_zones(
                        player,
                        card,
                        crate::events::Zone::WaitingRoom,
                        crate::events::Zone::Hand,
                        None,
                        None,
                    )
                } else {
                    self.move_card_between_zones(
                        player,
                        card,
                        crate::events::Zone::WaitingRoom,
                        crate::events::Zone::Stock,
                        None,
                        None,
                    )
                }
            }
            ChoiceReason::TriggerAutoCostSelect => {
                if option.zone == ChoiceZone::Skip {
                    resolve_pending_trigger(self);
                    return;
                }
                let Some(trigger) = pending_trigger.as_ref() else {
                    resolve_pending_trigger(self);
                    return;
                };
                let TriggerEffect::AutoAbility { ability_index } = trigger.effect else {
                    resolve_pending_trigger(self);
                    return;
                };
                if !self.resolve_trigger_auto_ability_with_cost(
                    player,
                    trigger.source_card,
                    ability_index,
                ) {
                    resolve_pending_trigger(self);
                    return;
                }
            }
            ChoiceReason::StackOrderSelect => {
                self.apply_stack_order_choice(player, option);
            }
            ChoiceReason::PriorityActionSelect => {
                self.apply_priority_action_choice(player, option);
            }
            ChoiceReason::CostPayment => {
                self.apply_cost_payment_choice(player, option);
            }
            ChoiceReason::TargetSelect => {
                self.apply_target_choice(player, option);
            }
            ChoiceReason::EndPhaseDiscard => {
                if option.zone != ChoiceZone::Hand {
                    resolve_pending_trigger(self);
                    return;
                }
                let Some(index) = option.index else {
                    resolve_pending_trigger(self);
                    return;
                };
                let Ok(index_u8) = u8::try_from(index) else {
                    resolve_pending_trigger(self);
                    return;
                };
                let p = player as usize;
                let idx = index as usize;
                if idx >= self.state.players[p].hand.len() {
                    resolve_pending_trigger(self);
                    return;
                }
                let card = self.state.players[p].hand[idx];
                if card.instance_id != option.instance_id {
                    resolve_pending_trigger(self);
                    return;
                }
                let card = self.state.players[p].hand.remove(idx);
                self.move_card_between_zones(
                    player,
                    card,
                    Zone::Hand,
                    Zone::WaitingRoom,
                    Some(index_u8),
                    None,
                );
                if self.state.players[p].hand.len() > HAND_LIMIT {
                    let _ = self.start_end_phase_discard_choice(player);
                } else {
                    self.state.turn.end_phase_discard_done = true;
                }
            }
        }
        resolve_pending_trigger(self);
    }
}
