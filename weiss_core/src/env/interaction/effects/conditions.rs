use crate::db::{ConditionTurn, ZoneCountCondition};
use crate::env::GameEnv;
use crate::state::*;

impl GameEnv {
    pub(super) fn source_marker_count(&self, source_ref: Option<&TargetRef>) -> usize {
        let Some(source_ref) = source_ref else {
            return 0;
        };
        if source_ref.zone != TargetZone::Stage {
            return 0;
        }
        let p = source_ref.player as usize;
        let s = source_ref.index as usize;
        if p >= self.state.players.len() {
            return 0;
        }
        if s >= self.state.players[p].stage.len() {
            return 0;
        }
        let slot = &self.state.players[p].stage[s];
        if slot.card.map(|card| card.instance_id) != Some(source_ref.instance_id) {
            return 0;
        }
        slot.markers.len()
    }

    pub(super) fn zone_count_value(&self, controller: u8, condition: &ZoneCountCondition) -> usize {
        self.zone_count_value_for_condition(controller, condition)
    }

    pub(super) fn conditional_modifier_context_satisfied(
        &self,
        controller: u8,
        source_ref: Option<&TargetRef>,
        turn: Option<ConditionTurn>,
        zone_count: Option<&ZoneCountCondition>,
        require_source_marker: bool,
    ) -> bool {
        if let Some(turn_condition) = turn {
            let is_self_turn = self.state.turn.active_player == controller;
            match turn_condition {
                ConditionTurn::SelfTurn if !is_self_turn => return false,
                ConditionTurn::OpponentTurn if is_self_turn => return false,
                _ => {}
            }
        }
        if let Some(zone_condition) = zone_count {
            if !self.zone_count_condition_met(controller, zone_condition) {
                return false;
            }
        }
        if require_source_marker && self.source_marker_count(source_ref) == 0 {
            return false;
        }
        true
    }

    pub(super) fn source_ref_matches_spec(
        &self,
        controller: u8,
        spec: &TargetSpec,
        source: &TargetRef,
    ) -> bool {
        let target_player = match spec.side {
            TargetSide::SelfSide => controller,
            TargetSide::Opponent => controller ^ 1,
        };
        if source.player != target_player {
            return false;
        }
        if source.zone != spec.zone {
            return false;
        }
        match spec.slot_filter {
            TargetSlotFilter::FrontRow if source.index >= 3 => return false,
            TargetSlotFilter::BackRow if source.index < 3 => return false,
            TargetSlotFilter::SpecificSlot(slot) if source.index != slot => return false,
            _ => {}
        }
        if let Some(card_type) = spec.card_type {
            let Some(card) = self.db.get(source.card_id) else {
                return false;
            };
            if card.card_type != card_type {
                return false;
            }
        }
        if let Some(trait_id) = spec.card_trait {
            let Some(card) = self.db.get(source.card_id) else {
                return false;
            };
            if !card.traits.contains(&trait_id) {
                return false;
            }
        }
        if let Some(level_max) = spec.level_max {
            let level = if source.zone == TargetZone::Stage {
                self.compute_slot_level(source.player as usize, source.index as usize)
            } else {
                let Some(card) = self.db.get(source.card_id) else {
                    return false;
                };
                i32::from(card.level)
            };
            if level > i32::from(level_max) {
                return false;
            }
        }
        if let Some(cost_max) = spec.cost_max {
            let Some(card) = self.db.get(source.card_id) else {
                return false;
            };
            if card.cost > cost_max {
                return false;
            }
        }
        if !spec.card_ids.is_empty() && !spec.card_ids.contains(&source.card_id) {
            return false;
        }
        true
    }
}
