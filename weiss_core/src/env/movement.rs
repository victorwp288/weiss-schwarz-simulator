use super::GameEnv;
use crate::db::*;
use crate::encode::*;
use crate::events::*;
use crate::state::*;
use anyhow::{anyhow, Result};

impl GameEnv {
    pub(super) fn play_character(
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

    pub(super) fn play_event(&mut self, player: u8, hand_index: u8) -> Result<()> {
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
        let source_ref = resolution_index.map(|index| TargetRef {
            player,
            zone: TargetZone::Resolution,
            index,
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

    pub(super) fn play_climax(&mut self, player: u8, hand_index: u8) -> Result<()> {
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
        Ok(())
    }

    pub(super) fn declare_attack(
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
        let card_inst = attacker_slot
            .card
            .ok_or_else(|| anyhow!("Missing attacker card"))?;
        let card = self
            .db
            .get(card_inst.id)
            .ok_or_else(|| anyhow!("Card missing in db"))?;
        let mut damage = card.soul as i32;
        if attack_type == AttackType::Direct {
            damage += 1;
        } else if attack_type == AttackType::Side {
            let defender_level = self.state.players[defender_player]
                .stage
                .get(s)
                .and_then(|slot| slot.card)
                .and_then(|card| self.db.get(card.id))
                .map(|card| card.level as i32)
                .unwrap_or(0);
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
            damage,
            counter_allowed: attack_type == AttackType::Frontal,
            counter_played: false,
            counter_power: 0,
            damage_modifiers: Vec::new(),
            next_modifier_id: 1,
            last_damage_event_id: None,
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

    pub(super) fn resolve_level_up(&mut self, player: u8, index: u8) -> Result<()> {
        let p = player as usize;
        if self.state.players[p].clock.len() < 7 {
            return Err(anyhow!("Clock has fewer than 7 cards"));
        }
        let idx = index as usize;
        if idx >= 7 {
            return Err(anyhow!("Level up index out of range"));
        }
        let bottom: Vec<CardInstance> = self.state.players[p].clock.drain(0..7).collect();
        if bottom.len() != 7 {
            return Err(anyhow!("Clock underflow on level up"));
        }
        let chosen_id = bottom[idx].id;
        for (i, card) in bottom.into_iter().enumerate() {
            if i == idx {
                self.move_card_between_zones(player, card, Zone::Clock, Zone::Level, None, None);
            } else {
                self.move_card_between_zones(
                    player,
                    card,
                    Zone::Clock,
                    Zone::WaitingRoom,
                    None,
                    None,
                );
            }
        }
        self.log_event(Event::LevelUpChoice {
            player,
            card: chosen_id,
        });
        self.state.turn.pending_level_up = None;
        if self.state.players[p].level.len() >= 4 {
            self.register_loss(player);
        }
        self.check_level_up(player);
        Ok(())
    }

    pub(super) fn resolve_encore(&mut self, player: u8, slot: u8, pay: bool) -> Result<()> {
        let p = player as usize;
        let s = slot as usize;
        if s >= self.state.players[p].stage.len() {
            return Err(anyhow!("Encore slot out of range"));
        }
        if self.state.players[p].stage[s].card.is_none() {
            return Err(anyhow!("Encore slot empty"));
        }
        if self.state.players[p].stage[s].status != StageStatus::Reverse {
            return Err(anyhow!("Encore slot not reversed"));
        }
        let Some(pos) = self
            .state
            .turn
            .encore_queue
            .iter()
            .position(|r| r.player == player && r.slot == slot)
        else {
            return Err(anyhow!("Encore slot not pending"));
        };
        if pay {
            if self.state.players[p].stock.len() < 3 {
                return Err(anyhow!("Encore cost unpaid"));
            }
            for _ in 0..3 {
                if let Some(card) = self.state.players[p].stock.pop() {
                    self.move_card_between_zones(
                        player,
                        card,
                        Zone::Stock,
                        Zone::WaitingRoom,
                        None,
                        None,
                    );
                }
            }
            if let Some(slot_state) = self.state.players[p].stage.get_mut(s) {
                slot_state.status = StageStatus::Rest;
            }
            self.log_event(Event::Encore {
                player,
                slot,
                kept: true,
            });
        } else {
            self.send_stage_to_waiting_room(player, slot);
            self.log_event(Event::Encore {
                player,
                slot,
                kept: false,
            });
        }
        self.state.turn.encore_queue.remove(pos);
        Ok(())
    }

    pub(super) fn move_card_between_zones(
        &mut self,
        player: u8,
        card: CardInstance,
        from: Zone,
        to: Zone,
        from_slot: Option<u8>,
        to_slot: Option<u8>,
    ) {
        let p = player as usize;
        match to {
            Zone::Deck => self.state.players[p].deck.push(card),
            Zone::Hand => self.state.players[p].hand.push(card),
            Zone::WaitingRoom => self.state.players[p].waiting_room.push(card),
            Zone::Clock => self.state.players[p].clock.push(card),
            Zone::Level => self.state.players[p].level.push(card),
            Zone::Stock => self.state.players[p].stock.push(card),
            Zone::Memory => self.state.players[p].memory.push(card),
            Zone::Climax => self.state.players[p].climax.push(card),
            Zone::Resolution => self.state.players[p].resolution.push(card),
            Zone::Stage => panic!("use place_card_on_stage for stage moves"),
        }
        self.on_card_enter_zone(&card, to);
        self.mark_rule_actions_dirty();
        self.mark_continuous_modifiers_dirty();
        self.log_event(Event::ZoneMove {
            player,
            card: card.id,
            from,
            to,
            from_slot,
            to_slot,
        });
    }

    pub(super) fn take_resolution_card(
        &mut self,
        player: u8,
        instance_id: CardInstanceId,
    ) -> Option<CardInstance> {
        let p = player as usize;
        let pos = self.state.players[p]
            .resolution
            .iter()
            .position(|card| card.instance_id == instance_id)?;
        Some(self.state.players[p].resolution.remove(pos))
    }

    pub(super) fn cleanup_pending_resolution_cards(&mut self) {
        if self.state.turn.pending_resolution_cleanup.is_empty() {
            return;
        }
        let pending = std::mem::take(&mut self.state.turn.pending_resolution_cleanup);
        for (player, instance_id) in pending {
            if let Some(card) = self.take_resolution_card(player, instance_id) {
                self.move_card_between_zones(
                    player,
                    card,
                    Zone::Resolution,
                    Zone::WaitingRoom,
                    None,
                    None,
                );
            }
        }
    }

    pub(super) fn place_card_on_stage(
        &mut self,
        player: u8,
        card: CardInstance,
        slot: u8,
        status: StageStatus,
        from: Zone,
        from_slot: Option<u8>,
    ) {
        let p = player as usize;
        if self.state.players[p].stage[slot as usize].card.is_some() {
            self.send_stage_to_waiting_room(player, slot);
        }
        let mut slot_state = StageSlot::empty();
        slot_state.card = Some(card);
        slot_state.status = status;
        self.state.players[p].stage[slot as usize] = slot_state;
        self.mark_slot_power_dirty(player, slot);
        self.mark_rule_actions_dirty();
        self.mark_continuous_modifiers_dirty();
        self.log_event(Event::ZoneMove {
            player,
            card: card.id,
            from,
            to: Zone::Stage,
            from_slot,
            to_slot: Some(slot),
        });
    }

    pub(super) fn send_stage_to_waiting_room(&mut self, player: u8, slot: u8) {
        let p = player as usize;
        let s = slot as usize;
        self.remove_modifiers_for_slot(player, slot);
        if let Some(card) = self.state.players[p].stage[s].card.take() {
            self.move_card_between_zones(
                player,
                card,
                Zone::Stage,
                Zone::WaitingRoom,
                Some(slot),
                None,
            );
        }
        self.state.players[p].stage[s] = StageSlot::empty();
        self.mark_slot_power_dirty(player, slot);
    }

    pub(super) fn swap_stage_slots(&mut self, player: u8, from_slot: u8, to_slot: u8) {
        if from_slot == to_slot {
            return;
        }
        let p = player as usize;
        let fs = from_slot as usize;
        let ts = to_slot as usize;
        if fs >= self.state.players[p].stage.len() || ts >= self.state.players[p].stage.len() {
            return;
        }
        if self.state.players[p].stage[fs].card.is_none() {
            return;
        }
        self.state.players[p].stage.swap(fs, ts);
        self.remove_modifiers_for_slot(player, from_slot);
        self.remove_modifiers_for_slot(player, to_slot);
        self.mark_slot_power_dirty(player, from_slot);
        self.mark_slot_power_dirty(player, to_slot);
        self.mark_rule_actions_dirty();
        self.mark_continuous_modifiers_dirty();
    }

    pub(super) fn move_waiting_room_to_hand(&mut self, player: u8, option: ChoiceOptionRef) {
        if option.zone != ChoiceZone::WaitingRoom {
            return;
        }
        let Some(idx) = option.index else {
            return;
        };
        let p = player as usize;
        let index = idx as usize;
        if index >= self.state.players[p].waiting_room.len() {
            return;
        }
        let card = self.state.players[p].waiting_room.remove(index);
        if card.instance_id != option.instance_id {
            return;
        }
        self.move_card_between_zones(player, card, Zone::WaitingRoom, Zone::Hand, None, None);
    }

    pub(super) fn move_stage_to_hand(&mut self, player: u8, option: ChoiceOptionRef) {
        if option.zone != ChoiceZone::Stage {
            return;
        }
        let Some(idx) = option.index else {
            return;
        };
        let p = player as usize;
        let slot = idx as usize;
        if slot >= self.state.players[p].stage.len() {
            return;
        }
        self.remove_modifiers_for_slot(player, idx);
        let card = self.state.players[p].stage[slot].card.take();
        let Some(card) = card else {
            return;
        };
        if card.instance_id != option.instance_id {
            return;
        }
        self.state.players[p].stage[slot] = StageSlot::empty();
        self.mark_slot_power_dirty(player, idx);
        self.move_card_between_zones(player, card, Zone::Stage, Zone::Hand, Some(idx), None);
    }

    pub(super) fn move_waiting_room_to_stage_standby(
        &mut self,
        player: u8,
        option: ChoiceOptionRef,
    ) {
        if option.zone != ChoiceZone::WaitingRoom {
            return;
        }
        let Some(idx) = option.index else {
            return;
        };
        let Some(target_slot) = option.target_slot else {
            return;
        };
        let p = player as usize;
        let slot = target_slot as usize;
        if slot >= self.state.players[p].stage.len() {
            return;
        }
        if let Some(existing) = self.state.players[p].stage[slot].card {
            self.remove_modifiers_for_slot(player, target_slot);
            self.state.players[p].stage[slot] = StageSlot::empty();
            self.move_card_between_zones(
                player,
                existing,
                Zone::Stage,
                Zone::WaitingRoom,
                Some(target_slot),
                None,
            );
        }
        self.mark_slot_power_dirty(player, target_slot);
        let index = idx as usize;
        if index >= self.state.players[p].waiting_room.len() {
            return;
        }
        let card = self.state.players[p].waiting_room.remove(index);
        if card.instance_id != option.instance_id {
            return;
        }
        let card_id = card.id;
        self.place_card_on_stage(
            player,
            card,
            target_slot,
            StageStatus::Rest,
            Zone::WaitingRoom,
            None,
        );
        self.apply_continuous_modifiers_for_slot(player, target_slot, card_id);
    }

    pub(super) fn move_trigger_card_from_stock_to_hand(
        &mut self,
        player: u8,
        card_id: CardId,
    ) -> bool {
        let p = player as usize;
        if let Some(instance_id) = self
            .state
            .turn
            .attack
            .as_ref()
            .and_then(|ctx| ctx.trigger_instance_id)
        {
            if let Some(pos) = self.state.players[p]
                .resolution
                .iter()
                .position(|c| c.instance_id == instance_id)
            {
                let card = self.state.players[p].resolution.remove(pos);
                self.move_card_between_zones(
                    player,
                    card,
                    Zone::Resolution,
                    Zone::Hand,
                    None,
                    None,
                );
                return true;
            }
            if let Some(pos) = self.state.players[p]
                .stock
                .iter()
                .position(|c| c.instance_id == instance_id)
            {
                let card = self.state.players[p].stock.remove(pos);
                self.move_card_between_zones(player, card, Zone::Stock, Zone::Hand, None, None);
                return true;
            }
        }
        // Deterministic fallback: take the most recent matching card from stock.
        if let Some(pos) = self.state.players[p]
            .stock
            .iter()
            .rposition(|c| c.id == card_id)
        {
            let card = self.state.players[p].stock.remove(pos);
            self.move_card_between_zones(player, card, Zone::Stock, Zone::Hand, None, None);
            return true;
        }
        false
    }

    pub(super) fn treasure_stock_available(&self, player: u8) -> bool {
        let p = player as usize;
        !self.state.players[p].deck.is_empty() || !self.state.players[p].waiting_room.is_empty()
    }

    pub(super) fn compute_slot_power(&mut self, player: usize, slot: usize) -> i32 {
        self.slot_power_cached(player, slot)
    }

    pub(super) fn refresh_slot_power_cache(&mut self) {
        for player in 0..2usize {
            for slot in 0..crate::encode::MAX_STAGE {
                if self.slot_power_dirty[player][slot] {
                    self.recompute_slot_power_cache(player, slot);
                }
            }
        }
    }

    pub(super) fn mark_slot_power_dirty(&mut self, player: u8, slot: u8) {
        let p = player as usize;
        let s = slot as usize;
        if p < 2 && s < crate::encode::MAX_STAGE {
            self.slot_power_dirty[p][s] = true;
        }
    }

    pub(super) fn mark_player_slot_power_dirty(&mut self, player: u8) {
        let p = player as usize;
        if p >= 2 {
            return;
        }
        for slot in 0..crate::encode::MAX_STAGE {
            self.slot_power_dirty[p][slot] = true;
        }
    }

    pub(super) fn mark_all_slot_power_dirty(&mut self) {
        for player in 0..2usize {
            for slot in 0..crate::encode::MAX_STAGE {
                self.slot_power_dirty[player][slot] = true;
            }
        }
    }

    fn slot_power_cached(&mut self, player: usize, slot: usize) -> i32 {
        if player >= 2 || slot >= crate::encode::MAX_STAGE {
            return 0;
        }
        let slot_state = &self.state.players[player].stage[slot];
        let current_card = slot_state.card.map(|c| c.id).unwrap_or(0);
        if current_card != self.slot_power_cache_card[player][slot]
            || slot_state.power_mod_turn != self.slot_power_cache_mod_turn[player][slot]
            || slot_state.power_mod_battle != self.slot_power_cache_mod_battle[player][slot]
        {
            self.slot_power_dirty[player][slot] = true;
        }
        if self.slot_power_dirty[player][slot] {
            return self.recompute_slot_power_cache(player, slot);
        }
        self.slot_power_cache[player][slot]
    }

    fn recompute_slot_power_cache(&mut self, player: usize, slot: usize) -> i32 {
        let power = self.compute_slot_power_uncached(player, slot);
        let slot_state = &self.state.players[player].stage[slot];
        self.slot_power_cache_card[player][slot] = slot_state.card.map(|c| c.id).unwrap_or(0);
        self.slot_power_cache_mod_turn[player][slot] = slot_state.power_mod_turn;
        self.slot_power_cache_mod_battle[player][slot] = slot_state.power_mod_battle;
        self.slot_power_cache[player][slot] = power;
        self.slot_power_dirty[player][slot] = false;
        power
    }

    fn compute_slot_power_uncached(&self, player: usize, slot: usize) -> i32 {
        let slot_state = &self.state.players[player].stage[slot];
        let Some(card_inst) = slot_state.card else {
            return 0;
        };
        let card_id = card_inst.id;
        let Some(card) = self.db.get(card_id) else {
            return 0;
        };
        let mut power = card.power + slot_state.power_mod_turn + slot_state.power_mod_battle;
        for modifier in &self.state.modifiers {
            if modifier.kind != ModifierKind::Power {
                continue;
            }
            if modifier.target_player as usize != player || modifier.target_slot as usize != slot {
                continue;
            }
            if modifier.target_card != card_id {
                continue;
            }
            power += modifier.magnitude;
        }
        power
    }

    pub(super) fn meets_level_requirement(&self, player: u8, card: &CardStatic) -> bool {
        card.level as usize <= self.state.players[player as usize].level.len()
    }

    pub(super) fn meets_cost_requirement(&self, player: u8, card: &CardStatic) -> bool {
        if !self.curriculum.enforce_cost_requirement {
            return true;
        }
        self.state.players[player as usize].stock.len() >= card.cost as usize
    }

    pub(super) fn meets_color_requirement(&self, player: u8, card: &CardStatic) -> bool {
        if !self.curriculum.enforce_color_requirement {
            return true;
        }
        if card.level == 0 || card.color == CardColor::Colorless {
            return true;
        }
        let p = &self.state.players[player as usize];
        for card_id in p.level.iter().chain(p.clock.iter()) {
            if let Some(c) = self.db.get(card_id.id) {
                if c.color == card.color {
                    return true;
                }
            }
        }
        false
    }

    pub(super) fn pay_cost(&mut self, player: u8, cost: usize) -> Result<()> {
        if cost == 0 {
            return Ok(());
        }
        let p = player as usize;
        if self.state.players[p].stock.len() < cost {
            return Err(anyhow!("Insufficient stock"));
        }
        self.state.turn.cost_payment_depth = self.state.turn.cost_payment_depth.saturating_add(1);
        for _ in 0..cost {
            if let Some(card) = self.state.players[p].stock.pop() {
                self.move_card_between_zones(
                    player,
                    card,
                    Zone::Stock,
                    Zone::WaitingRoom,
                    None,
                    None,
                );
            }
        }
        self.state.turn.cost_payment_depth = self.state.turn.cost_payment_depth.saturating_sub(1);
        Ok(())
    }

    pub(super) fn looks_like_event(&self, card: &CardStatic) -> bool {
        matches!(card.card_type, CardType::Event)
    }

    pub(super) fn is_counter_card(&self, card: &CardStatic) -> bool {
        if !card.counter_timing {
            return false;
        }
        self.db
            .iter_card_abilities_in_canonical_order(card.id)
            .iter()
            .any(|spec| {
                matches!(
                    spec.template,
                    AbilityTemplate::CounterBackup { .. }
                        | AbilityTemplate::CounterDamageReduce { .. }
                        | AbilityTemplate::CounterDamageCancel
                )
            })
    }

    pub(super) fn counter_power(&self, card: &CardStatic) -> i32 {
        for spec in self.db.iter_card_abilities_in_canonical_order(card.id) {
            if let AbilityTemplate::CounterBackup { power } = spec.template {
                return power;
            }
        }
        0
    }

    pub(super) fn counter_damage_reductions(&self, card: &CardStatic) -> Vec<i32> {
        let mut out = Vec::new();
        for spec in self.db.iter_card_abilities_in_canonical_order(card.id) {
            if let AbilityTemplate::CounterDamageReduce { amount } = spec.template {
                out.push(amount as i32);
            }
        }
        out
    }

    pub(super) fn counter_damage_cancel(&self, card: &CardStatic) -> bool {
        self.db
            .iter_card_abilities_in_canonical_order(card.id)
            .iter()
            .any(|spec| matches!(spec.template, AbilityTemplate::CounterDamageCancel))
    }

    pub(super) fn shuffle_deck(&mut self, player: u8) {
        let p = player as usize;
        self.state.rng.shuffle(&mut self.state.players[p].deck);
        self.log_event(Event::Shuffle {
            player,
            zone: Zone::Deck,
        });
        if self.curriculum.enable_visibility_policies {
            let instance_ids: Vec<CardInstanceId> = self.state.players[p]
                .deck
                .iter()
                .map(|card| card.instance_id)
                .collect();
            for instance_id in instance_ids {
                self.forget_instance_revealed(instance_id);
            }
        }
    }

    pub(super) fn draw_to_hand(&mut self, player: u8, count: usize) {
        for _ in 0..count {
            if let Some(card) = self.draw_from_deck(player) {
                let card_id = card.id;
                self.move_card_between_zones(player, card, Zone::Deck, Zone::Hand, None, None);
                self.log_event(Event::Draw {
                    player,
                    card: card_id,
                });
            }
        }
    }

    pub(super) fn draw_from_deck(&mut self, player: u8) -> Option<CardInstance> {
        let p = player as usize;
        if self.state.players[p].deck.is_empty() {
            if self.state.turn.cost_payment_depth > 0 {
                return None;
            }
            if !self.refresh_deck(player) {
                return None;
            }
        }
        let card = self.state.players[p].deck.pop()?;
        Some(card)
    }

    pub(super) fn check_level_up(&mut self, player: u8) {
        let p = player as usize;
        if self.state.players[p].clock.len() < 7 {
            return;
        }
        if self.curriculum.enable_level_up_choice {
            if self.state.turn.pending_level_up.is_none() {
                self.state.turn.pending_level_up = Some(player);
            }
        } else {
            let _ = self.resolve_level_up(player, 0);
        }
    }

    pub(super) fn refresh_deck(&mut self, player: u8) -> bool {
        let p = player as usize;
        if self.state.players[p].waiting_room.is_empty() {
            let in_damage = self.state.turn.damage_resolution_target == Some(player);
            if in_damage {
                let has_climax = self.state.players[p].resolution.iter().any(|card| {
                    self.db
                        .get(card.id)
                        .map(|c| c.card_type == CardType::Climax)
                        .unwrap_or(false)
                });
                if has_climax {
                    return false;
                }
            }
            self.register_loss(player);
            return false;
        }
        let mut reshuffle = Vec::new();
        std::mem::swap(&mut reshuffle, &mut self.state.players[p].waiting_room);
        self.state.players[p].deck = reshuffle;
        self.shuffle_deck(player);
        self.log_event(Event::Refresh { player });
        if self.curriculum.enable_refresh_penalty {
            if let Some(card) = self.state.players[p].deck.pop() {
                self.reveal_card(
                    player,
                    &card,
                    RevealReason::RefreshPenalty,
                    RevealAudience::Public,
                );
                let card_id = card.id;
                self.move_card_between_zones(player, card, Zone::Deck, Zone::Clock, None, None);
                self.log_event(Event::RefreshPenalty {
                    player,
                    card: card_id,
                });
                self.check_level_up(player);
            }
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{
        CurriculumConfig, EnvConfig, ErrorPolicy, ObservationVisibility, RewardConfig,
    };
    use crate::db::{CardColor, CardDb, CardStatic, CardType};
    use crate::env::GameEnv;
    use crate::events::Zone;
    use crate::replay::ReplayConfig;
    use crate::state::CardInstance;
    use crate::state::StageStatus;
    use std::sync::Arc;

    fn make_db() -> Arc<CardDb> {
        let mut cards = Vec::new();
        for id in 1..=13u32 {
            cards.push(CardStatic {
                id,
                card_set: None,
                card_type: CardType::Character,
                color: CardColor::Red,
                level: 0,
                cost: 0,
                power: 500,
                soul: 1,
                triggers: vec![],
                traits: vec![],
                abilities: vec![],
                ability_defs: vec![],
                counter_timing: false,
                raw_text: None,
            });
        }
        Arc::new(CardDb::new(cards).expect("db build"))
    }

    fn make_deck() -> Vec<u32> {
        let mut deck = Vec::new();
        for id in 1..=12u32 {
            deck.extend(std::iter::repeat_n(id, 4));
        }
        deck.extend(std::iter::repeat_n(13u32, 2));
        assert_eq!(deck.len(), 50);
        deck
    }

    fn make_env() -> GameEnv {
        let db = make_db();
        let deck = make_deck();
        let config = EnvConfig {
            deck_lists: [deck.clone(), deck],
            deck_ids: [12, 13],
            max_decisions: 10,
            max_ticks: 100,
            reward: RewardConfig::default(),
            error_policy: ErrorPolicy::Strict,
            observation_visibility: ObservationVisibility::Public,
            end_condition_policy: Default::default(),
        };
        GameEnv::new(
            db,
            config,
            CurriculumConfig::default(),
            11,
            ReplayConfig::default(),
            None,
            0,
        )
    }

    #[test]
    fn cached_slot_power_matches_uncached() {
        let mut env = make_env();
        let card = CardInstance::new(1, 0, 1);
        env.place_card_on_stage(0, card, 0, StageStatus::Stand, Zone::Hand, None);
        env.add_modifier(
            1,
            0,
            0,
            ModifierKind::Power,
            1000,
            ModifierDuration::UntilEndOfTurn,
        );

        let cached = env.compute_slot_power(0, 0);
        let uncached = env.compute_slot_power_uncached(0, 0);
        assert_eq!(cached, uncached);

        env.remove_modifiers_for_slot(0, 0);
        let cached_after = env.compute_slot_power(0, 0);
        let uncached_after = env.compute_slot_power_uncached(0, 0);
        assert_eq!(cached_after, uncached_after);
    }
}
