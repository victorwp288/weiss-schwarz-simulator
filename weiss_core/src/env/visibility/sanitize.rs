use super::super::{GameEnv, VisibilityContext};
use crate::effects::*;
use crate::events::*;
use crate::legal::*;
use crate::replay::*;
use crate::state::*;
use crate::visibility_policy::{hide_target_zone_for_viewer, hide_zone_for_viewer};

impl GameEnv {
    pub(in crate::env) fn sanitize_action_for_viewer(
        &self,
        action: &ActionDesc,
        actor: u8,
        ctx: VisibilityContext,
    ) -> ActionDesc {
        const UNKNOWN_INDEX: u8 = u8::MAX;
        if !ctx.is_public() {
            return action.clone();
        }
        let hide_for_viewer = match ctx.viewer {
            Some(viewer) => viewer != actor,
            None => true,
        };
        if !hide_for_viewer {
            return action.clone();
        }
        match action {
            ActionDesc::MulliganSelect { .. } => ActionDesc::MulliganSelect {
                hand_index: UNKNOWN_INDEX,
            },
            ActionDesc::Clock { .. } => ActionDesc::Clock {
                hand_index: UNKNOWN_INDEX,
            },
            ActionDesc::MainPlayCharacter { stage_slot, .. } => ActionDesc::MainPlayCharacter {
                hand_index: UNKNOWN_INDEX,
                stage_slot: *stage_slot,
            },
            ActionDesc::MainPlayEvent { .. } => ActionDesc::MainPlayEvent {
                hand_index: UNKNOWN_INDEX,
            },
            ActionDesc::ClimaxPlay { .. } => ActionDesc::ClimaxPlay {
                hand_index: UNKNOWN_INDEX,
            },
            ActionDesc::CounterPlay { .. } => ActionDesc::CounterPlay {
                hand_index: UNKNOWN_INDEX,
            },
            ActionDesc::ChoiceSelect { .. } => ActionDesc::ChoiceSelect {
                index: UNKNOWN_INDEX,
            },
            _ => action.clone(),
        }
    }

    pub(in crate::env) fn replay_visibility_context(&self) -> VisibilityContext {
        let policies_enabled = self.curriculum.enable_visibility_policies;
        let mode = self.config.observation_visibility;
        let viewer = None;
        VisibilityContext {
            viewer,
            mode,
            policies_enabled,
        }
    }

    pub(in crate::env) fn debug_visibility_context(&self, viewer: u8) -> VisibilityContext {
        VisibilityContext {
            viewer: Some(viewer),
            mode: self.config.observation_visibility,
            policies_enabled: true,
        }
    }

    pub(in crate::env) fn zone_hidden_for_viewer(
        &self,
        ctx: VisibilityContext,
        owner: u8,
        zone: Zone,
    ) -> bool {
        if !ctx.is_public() {
            return false;
        }
        hide_zone_for_viewer(ctx.mode, ctx.viewer, owner, zone, &self.curriculum)
    }

    pub(in crate::env) fn instance_revealed_to_viewer(
        &self,
        ctx: VisibilityContext,
        instance_id: CardInstanceId,
    ) -> bool {
        if instance_id == 0 {
            return false;
        }
        match ctx.viewer {
            Some(viewer) => self.revealed_to_viewer[viewer as usize].contains(&instance_id),
            None => {
                self.revealed_to_viewer[0].contains(&instance_id)
                    && self.revealed_to_viewer[1].contains(&instance_id)
            }
        }
    }

    pub(in crate::env) fn target_hidden_for_viewer(
        &self,
        ctx: VisibilityContext,
        owner: u8,
        zone: TargetZone,
    ) -> bool {
        if !ctx.is_public() {
            return false;
        }
        hide_target_zone_for_viewer(ctx.mode, ctx.viewer, owner, zone, &self.curriculum)
    }

    pub(in crate::env) fn reveal_visible_to_viewer(
        &self,
        ctx: VisibilityContext,
        owner: u8,
        audience: RevealAudience,
    ) -> bool {
        if !ctx.is_public() {
            return true;
        }
        match audience {
            RevealAudience::Public | RevealAudience::BothPlayers => true,
            RevealAudience::OwnerOnly | RevealAudience::ControllerOnly => {
                ctx.viewer.map(|viewer| viewer == owner).unwrap_or(false)
            }
            RevealAudience::ReplayOnly => false,
        }
    }

    pub(in crate::env) fn sanitize_target_ref(
        &self,
        ctx: VisibilityContext,
        target: TargetRef,
    ) -> TargetRef {
        if !self.target_hidden_for_viewer(ctx, target.player, target.zone) {
            return target;
        }
        TargetRef {
            player: target.player,
            zone: target.zone,
            index: 0,
            card_id: 0,
            instance_id: 0,
        }
    }

    pub(in crate::env) fn sanitize_stack_item(
        &self,
        ctx: VisibilityContext,
        item: &StackItem,
    ) -> StackItem {
        if !ctx.is_public() {
            return item.clone();
        }
        let hide_source = match ctx.viewer {
            Some(viewer) => viewer != item.controller,
            None => true,
        };
        let source_id = if hide_source { 0 } else { item.source_id };
        let targets = item
            .payload
            .targets
            .iter()
            .copied()
            .map(|t| self.sanitize_target_ref(ctx, t))
            .collect();
        StackItem {
            id: item.id,
            controller: item.controller,
            source_id,
            effect_id: item.effect_id,
            payload: EffectPayload {
                spec: item.payload.spec.clone(),
                targets,
                source_ref: item
                    .payload
                    .source_ref
                    .map(|target| self.sanitize_target_ref(ctx, target)),
            },
        }
    }

    pub(in crate::env) fn sanitize_event_for_viewer(
        &self,
        event: &Event,
        ctx: VisibilityContext,
    ) -> ReplayEvent {
        match event {
            Event::Draw { player, card } => {
                let hide = self.zone_hidden_for_viewer(ctx, *player, Zone::Deck)
                    || self.zone_hidden_for_viewer(ctx, *player, Zone::Hand);
                let card = if hide { 0 } else { *card };
                ReplayEvent::Draw {
                    player: *player,
                    card,
                }
            }
            Event::Damage { player, card } => ReplayEvent::Damage {
                player: *player,
                card: *card,
            },
            Event::DamageCancel { player } => ReplayEvent::DamageCancel { player: *player },
            Event::DamageIntent {
                event_id,
                source_player,
                source_slot,
                target,
                amount,
                damage_type,
                cancelable,
            } => ReplayEvent::DamageIntent {
                event_id: *event_id,
                source_player: *source_player,
                source_slot: *source_slot,
                target: *target,
                amount: *amount,
                damage_type: *damage_type,
                cancelable: *cancelable,
            },
            Event::DamageModifierApplied {
                event_id,
                modifier,
                before_amount,
                after_amount,
                before_cancelable,
                after_cancelable,
                before_canceled,
                after_canceled,
            } => ReplayEvent::DamageModifierApplied {
                event_id: *event_id,
                modifier: *modifier,
                before_amount: *before_amount,
                after_amount: *after_amount,
                before_cancelable: *before_cancelable,
                after_cancelable: *after_cancelable,
                before_canceled: *before_canceled,
                after_canceled: *after_canceled,
            },
            Event::DamageModified {
                event_id,
                target,
                original,
                modified,
                canceled,
                damage_type,
            } => ReplayEvent::DamageModified {
                event_id: *event_id,
                target: *target,
                original: *original,
                modified: *modified,
                canceled: *canceled,
                damage_type: *damage_type,
            },
            Event::DamageCommitted {
                event_id,
                target,
                card,
                damage_type,
            } => ReplayEvent::DamageCommitted {
                event_id: *event_id,
                target: *target,
                card: *card,
                damage_type: *damage_type,
            },
            Event::ReversalCommitted {
                player,
                slot,
                cause_damage_event,
            } => ReplayEvent::ReversalCommitted {
                player: *player,
                slot: *slot,
                cause_damage_event: *cause_damage_event,
            },
            Event::Reveal {
                player,
                card,
                reason,
                audience,
            } => {
                let visible = self.reveal_visible_to_viewer(ctx, *player, *audience);
                ReplayEvent::Reveal {
                    player: *player,
                    card: if visible { *card } else { 0 },
                    reason: *reason,
                    audience: *audience,
                }
            }
            Event::TriggerQueued {
                trigger_id,
                group_id,
                player,
                source,
                effect,
            } => ReplayEvent::TriggerQueued {
                trigger_id: *trigger_id,
                group_id: *group_id,
                player: *player,
                source: *source,
                effect: *effect,
            },
            Event::TriggerGrouped {
                group_id,
                trigger_ids,
            } => ReplayEvent::TriggerGrouped {
                group_id: *group_id,
                trigger_ids: trigger_ids.clone(),
            },
            Event::TriggerResolved {
                trigger_id,
                player,
                effect,
            } => ReplayEvent::TriggerResolved {
                trigger_id: *trigger_id,
                player: *player,
                effect: *effect,
            },
            Event::TriggerCanceled {
                trigger_id,
                player,
                reason,
            } => ReplayEvent::TriggerCanceled {
                trigger_id: *trigger_id,
                player: *player,
                reason: *reason,
            },
            Event::TimingWindowEntered { window, player } => ReplayEvent::TimingWindowEntered {
                window: *window,
                player: *player,
            },
            Event::PriorityGranted { window, player } => ReplayEvent::PriorityGranted {
                window: *window,
                player: *player,
            },
            Event::PriorityPassed {
                player,
                window,
                pass_count,
            } => ReplayEvent::PriorityPassed {
                player: *player,
                window: *window,
                pass_count: *pass_count,
            },
            Event::StackGroupPresented {
                group_id,
                controller,
                items,
            } => ReplayEvent::StackGroupPresented {
                group_id: *group_id,
                controller: *controller,
                items: items
                    .iter()
                    .map(|item| self.sanitize_stack_item(ctx, item))
                    .collect(),
            },
            Event::StackOrderChosen {
                group_id,
                controller,
                stack_id,
            } => ReplayEvent::StackOrderChosen {
                group_id: *group_id,
                controller: *controller,
                stack_id: *stack_id,
            },
            Event::StackPushed { item } => ReplayEvent::StackPushed {
                item: self.sanitize_stack_item(ctx, item),
            },
            Event::StackResolved { item } => ReplayEvent::StackResolved {
                item: self.sanitize_stack_item(ctx, item),
            },
            Event::AutoResolveCapExceeded {
                cap,
                stack_len,
                window,
            } => ReplayEvent::AutoResolveCapExceeded {
                cap: *cap,
                stack_len: *stack_len,
                window: *window,
            },
            Event::WindowAdvanced { from, to } => ReplayEvent::WindowAdvanced {
                from: *from,
                to: *to,
            },
            Event::ChoicePresented {
                choice_id,
                player,
                reason,
                options,
                total_candidates,
                page_start,
            } => {
                let summaries = self.summarize_choice_options_for_event(
                    *reason,
                    *player,
                    options,
                    *page_start,
                    *choice_id,
                    ctx,
                );
                ReplayEvent::ChoicePresented {
                    choice_id: *choice_id,
                    player: *player,
                    reason: *reason,
                    options: summaries,
                    total_candidates: *total_candidates,
                    page_start: *page_start,
                }
            }
            Event::ChoicePageChanged {
                choice_id,
                player,
                from_start,
                to_start,
            } => ReplayEvent::ChoicePageChanged {
                choice_id: *choice_id,
                player: *player,
                from_start: *from_start,
                to_start: *to_start,
            },
            Event::ChoiceMade {
                choice_id,
                player,
                reason,
                option,
            } => {
                let sanitized =
                    self.sanitize_choice_option_for_event(*reason, *player, ctx, option);
                ReplayEvent::ChoiceMade {
                    choice_id: *choice_id,
                    player: *player,
                    reason: *reason,
                    option: sanitized,
                }
            }
            Event::ChoiceAutopicked {
                choice_id,
                player,
                reason,
                option,
            } => {
                let sanitized =
                    self.sanitize_choice_option_for_event(*reason, *player, ctx, option);
                ReplayEvent::ChoiceAutopicked {
                    choice_id: *choice_id,
                    player: *player,
                    reason: *reason,
                    option: sanitized,
                }
            }
            Event::ChoiceSkipped {
                choice_id,
                player,
                reason,
                skip_reason,
            } => ReplayEvent::ChoiceSkipped {
                choice_id: *choice_id,
                player: *player,
                reason: *reason,
                skip_reason: *skip_reason,
            },
            Event::ZoneMove {
                player,
                card,
                from,
                to,
                from_slot,
                to_slot,
            } => {
                let hide_from = self.zone_hidden_for_viewer(ctx, *player, *from);
                let hide_to = self.zone_hidden_for_viewer(ctx, *player, *to);
                ReplayEvent::ZoneMove {
                    player: *player,
                    card: if hide_from && hide_to { 0 } else { *card },
                    from: *from,
                    to: *to,
                    from_slot: if hide_from { None } else { *from_slot },
                    to_slot: if hide_to { None } else { *to_slot },
                }
            }
            Event::ControlChanged {
                card,
                owner,
                from_controller,
                to_controller,
                from_slot,
                to_slot,
            } => ReplayEvent::ControlChanged {
                card: *card,
                owner: *owner,
                from_controller: *from_controller,
                to_controller: *to_controller,
                from_slot: *from_slot,
                to_slot: *to_slot,
            },
            Event::ModifierAdded {
                id,
                source,
                target_player,
                target_slot,
                target_card,
                kind,
                magnitude,
                duration,
            } => ReplayEvent::ModifierAdded {
                id: *id,
                source: *source,
                target_player: *target_player,
                target_slot: *target_slot,
                target_card: *target_card,
                kind: *kind,
                magnitude: *magnitude,
                duration: *duration,
            },
            Event::ModifierRemoved { id, reason } => ReplayEvent::ModifierRemoved {
                id: *id,
                reason: *reason,
            },
            Event::Concede { player } => ReplayEvent::Concede { player: *player },
            Event::Play { player, card, slot } => ReplayEvent::Play {
                player: *player,
                card: *card,
                slot: *slot,
            },
            Event::PlayEvent { player, card } => ReplayEvent::PlayEvent {
                player: *player,
                card: *card,
            },
            Event::PlayClimax { player, card } => ReplayEvent::PlayClimax {
                player: *player,
                card: *card,
            },
            Event::Trigger { player, icon, card } => {
                let reveal = if self.replay_config.include_trigger_card_id {
                    *card
                } else {
                    None
                };
                if ctx.is_public() && reveal.is_some() {
                    // Trigger checks are public, so no additional masking.
                }
                ReplayEvent::Trigger {
                    player: *player,
                    icon: *icon,
                    card: reveal,
                }
            }
            Event::Attack { player, slot } => ReplayEvent::Attack {
                player: *player,
                slot: *slot,
            },
            Event::AttackType {
                player,
                attacker_slot,
                attack_type,
            } => ReplayEvent::AttackType {
                player: *player,
                attacker_slot: *attacker_slot,
                attack_type: *attack_type,
            },
            Event::Counter {
                player,
                card,
                power,
            } => ReplayEvent::Counter {
                player: *player,
                card: *card,
                power: *power,
            },
            Event::Clock { player, card } => ReplayEvent::Clock {
                player: *player,
                card: *card,
            },
            Event::Shuffle { player, zone } => ReplayEvent::Shuffle {
                player: *player,
                zone: *zone,
            },
            Event::Refresh { player } => ReplayEvent::Refresh { player: *player },
            Event::RefreshPenalty { player, card } => ReplayEvent::RefreshPenalty {
                player: *player,
                card: *card,
            },
            Event::LevelUpChoice { player, card } => ReplayEvent::LevelUpChoice {
                player: *player,
                card: *card,
            },
            Event::Encore { player, slot, kept } => ReplayEvent::Encore {
                player: *player,
                slot: *slot,
                kept: *kept,
            },
            Event::Stand { player } => ReplayEvent::Stand { player: *player },
            Event::EndTurn { player } => ReplayEvent::EndTurn { player: *player },
            Event::Terminal { winner } => ReplayEvent::Terminal { winner: *winner },
        }
    }
}
