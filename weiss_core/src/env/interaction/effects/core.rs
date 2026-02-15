use crate::db::{CardId, CardType};
use crate::effects::*;
use crate::env::GameEnv;
use crate::events::{Event, RevealAudience, RevealReason, Zone};
use crate::state::*;

impl GameEnv {
    pub(in crate::env) fn allocate_effect_instance_id(&mut self) -> u32 {
        let id = self.state.turn.next_effect_instance_id;
        self.state.turn.next_effect_instance_id =
            self.state.turn.next_effect_instance_id.wrapping_add(1);
        id
    }

    pub(in crate::env) fn enqueue_effect_spec(
        &mut self,
        controller: u8,
        source_id: CardId,
        spec: EffectSpec,
    ) {
        self.enqueue_effect_spec_with_source(controller, source_id, spec, None);
    }

    pub(in crate::env) fn enqueue_effect_spec_with_source(
        &mut self,
        controller: u8,
        source_id: CardId,
        spec: EffectSpec,
        source: Option<TargetRef>,
    ) {
        let instance_id = self.allocate_effect_instance_id();
        if spec.kind.expects_target() {
            if let Some(target_spec) = spec.target.clone() {
                if target_spec.source_only {
                    if let Some(source_ref) = source {
                        if self.source_ref_matches_spec(controller, &target_spec, &source_ref) {
                            self.enqueue_effect_with_targets(
                                controller,
                                source_id,
                                spec,
                                vec![source_ref],
                            );
                        }
                    }
                    return;
                }
                let allow_skip = spec.optional;
                // Target selection snapshots candidates at enqueue time so follow-up resolution is
                // deterministic even if board state mutates before the choice is answered.
                self.start_target_selection(
                    controller,
                    source_id,
                    target_spec,
                    PendingTargetEffect::EffectPending {
                        instance_id,
                        payload: EffectPayload {
                            spec,
                            targets: Vec::new(),
                            source_ref: source,
                        },
                    },
                    allow_skip,
                );
                return;
            }
        }
        let item = StackItem {
            id: instance_id,
            controller,
            source_id,
            effect_id: spec.id,
            payload: EffectPayload {
                spec,
                targets: Vec::new(),
                source_ref: source,
            },
        };
        self.enqueue_stack_items(vec![item]);
    }

    pub(in crate::env) fn enqueue_effect_with_targets(
        &mut self,
        controller: u8,
        source_id: CardId,
        spec: EffectSpec,
        targets: Vec<TargetRef>,
    ) {
        let instance_id = self.allocate_effect_instance_id();
        let item = StackItem {
            id: instance_id,
            controller,
            source_id,
            effect_id: spec.id,
            payload: EffectPayload {
                spec,
                targets,
                source_ref: None,
            },
        };
        self.enqueue_stack_items(vec![item]);
    }

    pub(super) fn battle_opponent_target_from_source(
        &self,
        source_ref: &TargetRef,
    ) -> Option<TargetRef> {
        if source_ref.zone != TargetZone::Stage {
            return None;
        }
        let (opponent_player, opponent_slot) = if let Some(ctx) = self.state.turn.attack.as_ref() {
            let defender_slot = ctx.defender_slot?;
            let active = self.state.turn.active_player;
            if source_ref.player == active && source_ref.index == ctx.attacker_slot {
                (1 - source_ref.player, defender_slot)
            } else if source_ref.player == (1 - active) && source_ref.index == defender_slot {
                (1 - source_ref.player, ctx.attacker_slot)
            } else {
                return None;
            }
        } else {
            // On-reverse auto abilities resolve after battle when attack context may already be
            // cleared. For battle reversals, opponent lane is the mirrored slot index.
            (1 - source_ref.player, source_ref.index)
        };
        let p = opponent_player as usize;
        let s = opponent_slot as usize;
        if s >= self.state.players[p].stage.len() {
            return None;
        }
        let card = self.state.players[p].stage[s].card?;
        Some(TargetRef {
            player: opponent_player,
            zone: TargetZone::Stage,
            index: opponent_slot,
            card_id: card.id,
            instance_id: card.instance_id,
        })
    }

    pub(super) fn battle_opponent_condition_met(
        &self,
        target: &TargetRef,
        max_level: Option<u8>,
        max_cost: Option<u8>,
        level_gt_opponent_level: bool,
    ) -> bool {
        if target.zone != TargetZone::Stage {
            return false;
        }
        let Some(card) = self.db.get(target.card_id) else {
            return false;
        };
        let level = self.compute_slot_level(target.player as usize, target.index as usize);
        if let Some(level_cap) = max_level {
            if level > i32::from(level_cap) {
                return false;
            }
        }
        if let Some(cost_cap) = max_cost {
            if card.cost > cost_cap {
                return false;
            }
        }
        if level_gt_opponent_level {
            let opponent_level = self.state.players[target.player as usize].level.len();
            let Ok(level_usize) = usize::try_from(level) else {
                return false;
            };
            if level_usize <= opponent_level {
                return false;
            }
        }
        true
    }

    pub(super) fn move_target_to_deck_bottom(&mut self, target: TargetRef) {
        match target.zone {
            TargetZone::Stage => {
                self.send_stage_to_deck_bottom(target.player, target.index);
            }
            TargetZone::DeckTop => {
                let p = target.player as usize;
                let offset = target.index as usize;
                if offset >= self.state.players[p].deck.len() {
                    return;
                }
                let deck_idx = self.state.players[p].deck.len().saturating_sub(1 + offset);
                if deck_idx >= self.state.players[p].deck.len() {
                    return;
                }
                if self.state.players[p].deck[deck_idx].instance_id != target.instance_id {
                    return;
                }
                let card = self.state.players[p].deck.remove(deck_idx);
                self.touch_player_obs(target.player);
                self.state.players[p].deck.insert(0, card);
                self.mark_rule_actions_dirty();
                self.mark_continuous_modifiers_dirty();
                self.log_event(Event::ZoneMove {
                    player: target.player,
                    card: card.id,
                    from: Zone::Deck,
                    to: Zone::Deck,
                    from_slot: None,
                    to_slot: None,
                });
            }
            _ => {}
        }
    }

    pub(super) fn move_stage_target_to_stock(&mut self, target: TargetRef) -> bool {
        if target.zone != TargetZone::Stage {
            return false;
        }
        let p = target.player as usize;
        let slot = target.index as usize;
        if slot >= self.state.players[p].stage.len() {
            return false;
        }
        let Some(card_inst) = self.state.players[p].stage[slot].card else {
            return false;
        };
        if card_inst.instance_id != target.instance_id {
            return false;
        }
        self.remove_modifiers_for_slot(target.player, target.index);
        self.drain_stage_markers_to_waiting_room(target.player, target.index);
        self.state.players[p].stage[slot] = StageSlot::empty();
        self.mark_slot_power_dirty(target.player, target.index);
        self.move_card_between_zones(
            target.player,
            card_inst,
            Zone::Stage,
            Zone::Stock,
            Some(target.index),
            None,
        );
        true
    }

    pub(super) fn move_stage_target_to_clock(&mut self, target: TargetRef) -> bool {
        if target.zone != TargetZone::Stage {
            return false;
        }
        let p = target.player as usize;
        let slot = target.index as usize;
        if slot >= self.state.players[p].stage.len() {
            return false;
        }
        let Some(card_inst) = self.state.players[p].stage[slot].card else {
            return false;
        };
        if card_inst.instance_id != target.instance_id {
            return false;
        }
        self.remove_modifiers_for_slot(target.player, target.index);
        self.drain_stage_markers_to_waiting_room(target.player, target.index);
        self.state.players[p].stage[slot] = StageSlot::empty();
        self.mark_slot_power_dirty(target.player, target.index);
        self.move_card_between_zones(
            target.player,
            card_inst,
            Zone::Stage,
            Zone::Clock,
            Some(target.index),
            None,
        );
        self.check_level_up(target.player);
        true
    }

    pub(in crate::env) fn move_stage_target_to_memory(&mut self, target: TargetRef) -> bool {
        if target.zone != TargetZone::Stage {
            return false;
        }
        let p = target.player as usize;
        let slot = target.index as usize;
        if slot >= self.state.players[p].stage.len() {
            return false;
        }
        let Some(card_inst) = self.state.players[p].stage[slot].card else {
            return false;
        };
        if card_inst.instance_id != target.instance_id {
            return false;
        }
        self.remove_modifiers_for_slot(target.player, target.index);
        self.drain_stage_markers_to_waiting_room(target.player, target.index);
        self.state.players[p].stage[slot] = StageSlot::empty();
        self.mark_slot_power_dirty(target.player, target.index);
        self.move_card_between_zones(
            target.player,
            card_inst,
            Zone::Stage,
            Zone::Memory,
            Some(target.index),
            None,
        );
        true
    }

    pub(super) fn move_bottom_stock_to_waiting_room(&mut self, player: u8) -> bool {
        let p = player as usize;
        if self.state.players[p].stock.is_empty() {
            return false;
        }
        let card = self.state.players[p].stock.remove(0);
        self.move_card_between_zones(player, card, Zone::Stock, Zone::WaitingRoom, None, None);
        true
    }

    pub(super) fn move_top_clock_to_waiting_room(&mut self, player: u8) -> bool {
        let p = player as usize;
        let Some(card) = self.state.players[p].clock.pop() else {
            return false;
        };
        self.move_card_between_zones(player, card, Zone::Clock, Zone::WaitingRoom, None, None);
        self.check_level_up(player);
        true
    }

    fn grant_expiry_turn_number(&self, controller: u8, duration: crate::db::GrantDuration) -> u32 {
        match duration {
            crate::db::GrantDuration::UntilEndOfTurn => self.state.turn.turn_number,
            crate::db::GrantDuration::UntilEndOfOpponentsNextTurn => {
                if self.state.turn.active_player == (1 - controller) {
                    self.state.turn.turn_number.saturating_add(2)
                } else {
                    self.state.turn.turn_number.saturating_add(1)
                }
            }
        }
    }

    pub(super) fn add_granted_ability_to_target(
        &mut self,
        controller: u8,
        target: &TargetRef,
        ability: &crate::db::AbilityDef,
        duration: crate::db::GrantDuration,
    ) {
        if target.zone != TargetZone::Stage {
            return;
        }
        let p = target.player as usize;
        let s = target.index as usize;
        if s >= self.state.players[p].stage.len() {
            return;
        }
        if self.state.players[p].stage[s].card.map(|c| c.instance_id) != Some(target.instance_id) {
            return;
        }
        let grant_id = self.state.turn.next_grant_id;
        self.state.turn.next_grant_id = self.state.turn.next_grant_id.wrapping_add(1);
        let compiled_effects = crate::db::compile_effects_from_def(target.card_id, 0, ability);
        let spec = crate::db::AbilitySpec::from_template(&crate::db::AbilityTemplate::AbilityDef(
            ability.clone(),
        ));
        self.state
            .turn
            .granted_abilities
            .push(GrantedAbilityInstance {
                grant_id,
                target_player: target.player,
                target_slot: target.index,
                target_instance_id: target.instance_id,
                spec,
                compiled_effects,
                expires_turn_number: self.grant_expiry_turn_number(controller, duration),
            });
        self.state.turn.derived_attack = None;
        self.mark_continuous_modifiers_dirty();
        self.touch_player_obs(target.player);
    }

    pub(super) fn reveal_top_deck_card_passes_level_gate(
        &mut self,
        player: u8,
        min_level: u8,
    ) -> bool {
        let p = player as usize;
        let Some(card_inst) = self.state.players[p].deck.last().copied() else {
            return false;
        };
        self.reveal_card(
            player,
            &card_inst,
            RevealReason::AbilityEffect,
            RevealAudience::Public,
        );
        let Some(card) = self.db.get(card_inst.id) else {
            return false;
        };
        let effective_level = if card.card_type == CardType::Climax {
            0
        } else {
            card.level
        };
        effective_level >= min_level
    }
}
