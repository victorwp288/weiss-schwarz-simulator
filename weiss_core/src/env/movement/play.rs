use super::super::GameEnv;
use crate::db::*;
use crate::encode::MAX_STAGE;
use crate::events::*;
use crate::state::*;
use anyhow::{anyhow, Result};

impl GameEnv {
    fn compute_slot_effective_soul(&self, player: usize, slot: usize) -> i32 {
        if player >= 2 || slot >= MAX_STAGE {
            return 0;
        }
        let slot_state = &self.state.players[player].stage[slot];
        let Some(card_inst) = slot_state.card else {
            return 0;
        };
        let mut soul = self.db.soul_by_id(card_inst.id) as i32;
        for modifier in &self.state.modifiers {
            if modifier.kind != ModifierKind::Soul {
                continue;
            }
            if modifier.target_player as usize != player || modifier.target_slot as usize != slot {
                continue;
            }
            if modifier.target_card != card_inst.id {
                continue;
            }
            soul = soul.saturating_add(modifier.magnitude);
        }
        soul.max(0)
    }

    pub(in crate::env) fn play_character(
        &mut self,
        player: u8,
        hand_index: u8,
        stage_slot: u8,
    ) -> Result<()> {
        let p = player as usize;
        let hi = hand_index as usize;
        let ss = stage_slot as usize;
        if hi >= self.state.players[p].hand.len() {
            return Err(anyhow!("Hand index out of range"));
        }
        if ss >= MAX_STAGE || (self.curriculum.reduced_stage_mode && ss > 0) {
            return Err(anyhow!("Stage slot invalid"));
        }
        let card_inst = self.state.players[p].hand[hi];
        let card_id = card_inst.id;
        let card = self
            .db
            .get(card_id)
            .ok_or_else(|| anyhow!("Card missing in db"))?;
        if !self.card_set_allowed(card) {
            return Err(anyhow!("Card set not allowed"));
        }
        if card.card_type != CardType::Character {
            return Err(anyhow!("Card is not a character"));
        }
        if !self.curriculum.allow_character {
            return Err(anyhow!("Character play disabled"));
        }
        if !self.meets_level_requirement(player, card)
            || !self.meets_color_requirement(player, card)
            || !self.meets_cost_requirement(player, card)
        {
            return Err(anyhow!("Play requirements not met"));
        }
        let cost = card.cost as usize;
        self.pay_cost(player, cost)?;
        if self.state.players[p].stage[ss].card.is_some() {
            self.send_stage_to_waiting_room(player, stage_slot);
        }
        let card_inst = self.state.players[p].hand.remove(hi);
        let card_id = card_inst.id;
        self.reveal_card(
            player,
            &card_inst,
            RevealReason::Play,
            RevealAudience::Public,
        );
        self.place_card_on_stage(
            player,
            card_inst,
            stage_slot,
            StageStatus::Stand,
            Zone::Hand,
            Some(hand_index),
        );
        self.log_event(Event::Play {
            player,
            card: card_id,
            slot: stage_slot,
        });
        self.apply_continuous_modifiers_for_slot(player, stage_slot, card_id);
        let source_ref = TargetRef {
            player,
            zone: TargetZone::Stage,
            index: stage_slot,
            card_id,
            instance_id: card_inst.instance_id,
        };
        self.resolve_on_play_abilities(player, card_id, Some(source_ref));
        Ok(())
    }

    pub(in crate::env) fn play_event(&mut self, player: u8, hand_index: u8) -> Result<()> {
        let p = player as usize;
        let hi = hand_index as usize;
        if hi >= self.state.players[p].hand.len() {
            return Err(anyhow!("Event hand index out of range"));
        }
        let card_inst = self.state.players[p].hand[hi];
        let card_id = card_inst.id;
        let db = self.db.clone();
        let card = db
            .get(card_id)
            .ok_or_else(|| anyhow!("Card missing in db"))?;
        if !self.card_set_allowed(card) {
            return Err(anyhow!("Card set not allowed"));
        }
        let events_locked =
            self.state.players[p]
                .stage
                .iter()
                .enumerate()
                .any(|(slot, slot_state)| {
                    slot_state.card.is_some()
                        && self.slot_has_active_modifier_kind(
                            player,
                            slot as u8,
                            ModifierKind::CannotPlayEventsFromHand,
                        )
                });
        if events_locked {
            return Err(anyhow!("Cannot play events from hand"));
        }
        if !self.looks_like_event(card) {
            return Err(anyhow!("Card is not an event"));
        }
        if !self.curriculum.allow_event {
            return Err(anyhow!("Event play disabled"));
        }
        if !self.meets_level_requirement(player, card)
            || !self.meets_color_requirement(player, card)
            || !self.meets_cost_requirement(player, card)
        {
            return Err(anyhow!("Event requirements not met"));
        }
        let cost = card.cost as usize;
        self.pay_cost(player, cost)?;
        let card_inst = self.state.players[p].hand.remove(hi);
        self.reveal_card(
            player,
            &card_inst,
            RevealReason::Play,
            RevealAudience::Public,
        );
        self.move_card_between_zones(
            player,
            card_inst,
            Zone::Hand,
            Zone::Resolution,
            Some(hand_index),
            None,
        );
        self.log_event(Event::PlayEvent {
            player,
            card: card_inst.id,
        });
        let resolution_index = self.state.players[p]
            .resolution
            .iter()
            .position(|card| card.instance_id == card_inst.instance_id)
            .and_then(|idx| {
                if idx <= u8::MAX as usize {
                    Some(idx as u8)
                } else {
                    None
                }
            });
        let Some(resolution_index) = resolution_index else {
            return Err(anyhow!(
                "Resolution index missing for card {} (instance {})",
                card_id,
                card_inst.instance_id
            ));
        };
        let source_ref = Some(TargetRef {
            player,
            zone: TargetZone::Resolution,
            index: resolution_index,
            card_id,
            instance_id: card_inst.instance_id,
        });
        self.resolve_on_play_abilities(player, card_id, source_ref);
        self.state
            .turn
            .pending_resolution_cleanup
            .push((player, card_inst.instance_id));
        Ok(())
    }

    pub(in crate::env) fn play_climax(&mut self, player: u8, hand_index: u8) -> Result<()> {
        let p = player as usize;
        let hi = hand_index as usize;
        if hi >= self.state.players[p].hand.len() {
            return Err(anyhow!("Climax hand index out of range"));
        }
        if !self.curriculum.allow_climax {
            return Err(anyhow!("Climax play disabled"));
        }
        if !self.state.players[p].climax.is_empty() {
            return Err(anyhow!("Climax zone occupied"));
        }
        let card_inst = self.state.players[p].hand[hi];
        let card = self
            .db
            .get(card_inst.id)
            .ok_or_else(|| anyhow!("Card missing in db"))?;
        if !self.card_set_allowed(card) {
            return Err(anyhow!("Card set not allowed"));
        }
        if card.card_type != CardType::Climax {
            return Err(anyhow!("Card is not a climax"));
        }
        if !self.meets_level_requirement(player, card)
            || !self.meets_color_requirement(player, card)
            || !self.meets_cost_requirement(player, card)
        {
            return Err(anyhow!("Climax requirements not met"));
        }
        let cost = card.cost as usize;
        self.pay_cost(player, cost)?;
        let card_inst = self.state.players[p].hand.remove(hi);
        self.reveal_card(
            player,
            &card_inst,
            RevealReason::Play,
            RevealAudience::Public,
        );
        let card_id = card_inst.id;
        self.move_card_between_zones(
            player,
            card_inst,
            Zone::Hand,
            Zone::Climax,
            Some(hand_index),
            None,
        );
        self.log_event(Event::PlayClimax {
            player,
            card: card_id,
        });
        self.resolve_on_play_abilities(player, card_id, None);
        Ok(())
    }

    pub(in crate::env) fn declare_attack(
        &mut self,
        player: u8,
        slot: u8,
        attack_type: AttackType,
    ) -> Result<()> {
        if let Err(reason) = crate::legal::can_declare_attack(
            &self.state,
            player,
            slot,
            attack_type,
            &self.curriculum,
        ) {
            return Err(anyhow!(reason));
        }
        let p = player as usize;
        let s = slot as usize;
        let defender_player = 1 - p;
        let defender_slot = self.state.players[defender_player].stage[s].card.is_some();
        let mut damage = self.compute_slot_effective_soul(p, s);
        let attack_cost = self
            .state
            .turn
            .derived_attack
            .as_ref()
            .map(|d| d.per_player[p][s].attack_cost as usize)
            .unwrap_or(self.state.players[p].stage[s].attack_cost as usize);
        if attack_cost > 0 {
            self.pay_cost(player, attack_cost)?;
        }
        let attacker_slot = &mut self.state.players[p].stage[s];
        attacker_slot.status = StageStatus::Rest;
        attacker_slot.has_attacked = true;
        let _card_inst = attacker_slot
            .card
            .ok_or_else(|| anyhow!("Missing attacker card"))?;
        self.touch_player_obs(player);
        if attack_type == AttackType::Direct {
            damage += 1;
        } else if attack_type == AttackType::Side {
            let defender_level = self.compute_slot_level(defender_player, s);
            damage = (damage - defender_level).max(0);
        }
        self.log_event(Event::Attack { player, slot });
        self.log_event(Event::AttackType {
            player,
            attacker_slot: slot,
            attack_type,
        });
        self.state.turn.attack_subphase_count =
            self.state.turn.attack_subphase_count.saturating_add(1);
        let ctx = AttackContext {
            attacker_slot: slot,
            defender_slot: if defender_slot { Some(slot) } else { None },
            attack_type,
            trigger_card: None,
            trigger_instance_id: None,
            trigger_checks_total: 1,
            trigger_checks_resolved: 0,
            damage,
            counter_allowed: attack_type == AttackType::Frontal,
            counter_played: false,
            counter_power: 0,
            damage_modifiers: Vec::new(),
            pending_shot_damage: 0,
            next_modifier_id: 1,
            last_damage_event_id: None,
            auto_trigger_enqueued: false,
            auto_damage_enqueued: false,
            battle_damage_applied: false,
            step: AttackStep::Trigger,
            decl_window_done: false,
            trigger_window_done: false,
            damage_window_done: false,
        };
        self.state.turn.attack = Some(ctx);
        Ok(())
    }
}
