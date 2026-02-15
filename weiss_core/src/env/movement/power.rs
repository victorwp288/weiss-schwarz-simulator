use super::super::GameEnv;
use crate::encode::MAX_STAGE;
use crate::state::ModifierKind;

impl GameEnv {
    pub(in crate::env::movement) fn compute_slot_power_uncached(
        &self,
        player: usize,
        slot: usize,
    ) -> i32 {
        if player >= 2 || slot >= MAX_STAGE {
            return 0;
        }
        let slot_state = &self.state.players[player].stage[slot];
        let Some(card_inst) = slot_state.card else {
            return 0;
        };
        let card_id = card_inst.id;
        if !self.db.is_valid_id(card_id) {
            return 0;
        }
        let mut power =
            self.db.power_by_id(card_id) + slot_state.power_mod_turn + slot_state.power_mod_battle;
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

    pub(in crate::env) fn compute_slot_level(&self, player: usize, slot: usize) -> i32 {
        if player >= 2 || slot >= MAX_STAGE {
            return 0;
        }
        let slot_state = &self.state.players[player].stage[slot];
        let Some(card_inst) = slot_state.card else {
            return 0;
        };
        let card_id = card_inst.id;
        if !self.db.is_valid_id(card_id) {
            return 0;
        }
        let mut level = i32::from(self.db.level_by_id(card_id));
        for modifier in &self.state.modifiers {
            if modifier.kind != ModifierKind::Level {
                continue;
            }
            if modifier.target_player as usize != player || modifier.target_slot as usize != slot {
                continue;
            }
            if modifier.target_card != card_id {
                continue;
            }
            level = level.saturating_add(modifier.magnitude);
        }
        level.max(0)
    }

    pub(in crate::env) fn compute_slot_power(&mut self, player: usize, slot: usize) -> i32 {
        self.slot_power_cached(player, slot)
    }

    pub(in crate::env) fn refresh_slot_power_cache(&mut self) {
        for player in 0..2usize {
            for slot in 0..MAX_STAGE {
                if self.slot_power_cache_modifiers_version[player][slot] != self.modifiers_version {
                    self.slot_power_dirty[player][slot] = true;
                }
                if self.slot_power_dirty[player][slot] {
                    self.recompute_slot_power_cache(player, slot);
                }
            }
        }
    }

    pub(in crate::env) fn mark_slot_power_dirty(&mut self, player: u8, slot: u8) {
        let p = player as usize;
        let s = slot as usize;
        if p < 2 && s < MAX_STAGE {
            self.slot_power_dirty[p][s] = true;
            self.touch_player_obs(player);
        }
    }

    pub(in crate::env) fn mark_player_slot_power_dirty(&mut self, player: u8) {
        let p = player as usize;
        if p >= 2 {
            return;
        }
        for slot in 0..MAX_STAGE {
            self.slot_power_dirty[p][slot] = true;
        }
        self.touch_player_obs(player);
    }

    pub(in crate::env) fn mark_all_slot_power_dirty(&mut self) {
        for player in 0..2usize {
            for slot in 0..MAX_STAGE {
                self.slot_power_dirty[player][slot] = true;
            }
        }
        self.touch_player_obs(0);
        self.touch_player_obs(1);
    }

    fn slot_power_cached(&mut self, player: usize, slot: usize) -> i32 {
        if player >= 2 || slot >= MAX_STAGE {
            return 0;
        }
        let slot_state = &self.state.players[player].stage[slot];
        let current_card = slot_state.card.map(|c| c.id);
        if current_card != self.slot_power_cache_card[player][slot]
            || slot_state.power_mod_turn != self.slot_power_cache_mod_turn[player][slot]
            || slot_state.power_mod_battle != self.slot_power_cache_mod_battle[player][slot]
            || self.slot_power_cache_modifiers_version[player][slot] != self.modifiers_version
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
        self.slot_power_cache_card[player][slot] = slot_state.card.map(|c| c.id);
        self.slot_power_cache_mod_turn[player][slot] = slot_state.power_mod_turn;
        self.slot_power_cache_mod_battle[player][slot] = slot_state.power_mod_battle;
        self.slot_power_cache_modifiers_version[player][slot] = self.modifiers_version;
        self.slot_power_cache[player][slot] = power;
        self.slot_power_dirty[player][slot] = false;
        power
    }
}
