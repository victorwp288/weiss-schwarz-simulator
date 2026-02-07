use super::super::GameEnv;
use crate::db::CardId;
use crate::events::*;
use crate::state::*;

impl GameEnv {
    pub(in crate::env) fn move_card_between_zones(
        &mut self,
        player: u8,
        card: CardInstance,
        from: Zone,
        to: Zone,
        from_slot: Option<u8>,
        to_slot: Option<u8>,
    ) {
        let p = player as usize;
        self.touch_player_obs(player);
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

    pub(in crate::env) fn take_resolution_card(
        &mut self,
        player: u8,
        instance_id: CardInstanceId,
    ) -> Option<CardInstance> {
        let p = player as usize;
        let pos = self.state.players[p]
            .resolution
            .iter()
            .position(|card| card.instance_id == instance_id)?;
        self.touch_player_obs(player);
        Some(self.state.players[p].resolution.remove(pos))
    }

    pub(in crate::env) fn cleanup_pending_resolution_cards(&mut self) {
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

    pub(in crate::env) fn place_card_on_stage(
        &mut self,
        player: u8,
        card: CardInstance,
        slot: u8,
        status: StageStatus,
        from: Zone,
        from_slot: Option<u8>,
    ) {
        let p = player as usize;
        let idx = slot as usize;
        if idx >= self.state.players[p].stage.len() {
            return;
        }
        if self.state.players[p].stage[idx].card.is_some() {
            self.send_stage_to_waiting_room(player, slot);
        }
        let mut slot_state = StageSlot::empty();
        slot_state.card = Some(card);
        slot_state.status = status;
        self.state.players[p].stage[idx] = slot_state;
        self.touch_player_obs(player);
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

    pub(in crate::env) fn send_stage_to_waiting_room(&mut self, player: u8, slot: u8) {
        let p = player as usize;
        let s = slot as usize;
        if s >= self.state.players[p].stage.len() {
            return;
        }
        self.remove_modifiers_for_slot(player, slot);
        if let Some(card) = self.state.players[p].stage[s].card.take() {
            self.touch_player_obs(player);
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

    pub(in crate::env) fn swap_stage_slots(&mut self, player: u8, from_slot: u8, to_slot: u8) {
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
        self.touch_player_obs(player);
        self.remove_modifiers_for_slot(player, from_slot);
        self.remove_modifiers_for_slot(player, to_slot);
        self.mark_slot_power_dirty(player, from_slot);
        self.mark_slot_power_dirty(player, to_slot);
        self.mark_rule_actions_dirty();
        self.mark_continuous_modifiers_dirty();
    }

    pub(in crate::env) fn move_waiting_room_to_hand(
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
        let p = player as usize;
        let index = idx as usize;
        if index >= self.state.players[p].waiting_room.len() {
            return;
        }
        let Some(card) = self.state.players[p].waiting_room.get(index).copied() else {
            return;
        };
        if card.instance_id != option.instance_id {
            return;
        }
        let card = self.state.players[p].waiting_room.remove(index);
        self.move_card_between_zones(player, card, Zone::WaitingRoom, Zone::Hand, None, None);
    }

    pub(in crate::env) fn move_stage_to_hand(&mut self, player: u8, option: ChoiceOptionRef) {
        if option.zone != ChoiceZone::Stage {
            return;
        }
        let Some(idx) = option.index else {
            return;
        };
        let Ok(slot_u8) = u8::try_from(idx) else {
            return;
        };
        let p = player as usize;
        let slot = slot_u8 as usize;
        if slot >= self.state.players[p].stage.len() {
            return;
        }
        let Some(card) = self.state.players[p].stage[slot].card else {
            return;
        };
        if card.instance_id != option.instance_id {
            return;
        }
        self.remove_modifiers_for_slot(player, slot_u8);
        self.state.players[p].stage[slot] = StageSlot::empty();
        self.mark_slot_power_dirty(player, slot_u8);
        self.move_card_between_zones(player, card, Zone::Stage, Zone::Hand, Some(slot_u8), None);
    }

    pub(in crate::env) fn move_waiting_room_to_stage_standby(
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
        let index = idx as usize;
        if index >= self.state.players[p].waiting_room.len() {
            return;
        }
        let Some(card) = self.state.players[p].waiting_room.get(index).copied() else {
            return;
        };
        if card.instance_id != option.instance_id {
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
        let card = self.state.players[p].waiting_room.remove(index);
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

    pub(in crate::env) fn move_trigger_card_from_stock_to_hand(
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
}
