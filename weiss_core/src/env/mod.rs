//! Game environment and advance-until-decision loop.
//!
//! Related docs:
//! - <https://github.com/victorwp288/weiss-schwarz-simulator/blob/main/docs/README.md>
//! - <https://github.com/victorwp288/weiss-schwarz-simulator/blob/main/docs/architecture.md>
//! - <https://github.com/victorwp288/weiss-schwarz-simulator/blob/main/docs/rl_contract.md>

use crate::config::RewardConfig;
use crate::db::{
    AbilitySpec, AbilityTemplate, CardId, CardStatic, CountCmp, CountZone, ZoneCountCondition,
};
use crate::state::{ModifierDuration, ModifierKind, TerminalResult};

mod actions;
mod advance;
mod cache;
mod constants;
mod core;
mod debug_events;
mod debug_fingerprints;
mod debug_validate;
mod fault;
pub(crate) mod heuristic_public;
mod human_view;
mod lifecycle;
mod live_abilities;
mod obs;
mod shared;
mod types;

mod interaction;
mod modifiers;
mod movement;
mod phases;
mod visibility;

#[cfg(any(feature = "test-harness", test))]
/// Test harness helpers exposed when the `test-harness` feature or crate tests are enabled.
pub mod harness;

pub use actions::legal_action_ids_cached_into;
pub use core::GameEnv;
pub use types::{
    DebugConfig, EngineErrorCode, EnvInfo, FaultRecord, FaultSource, RewardBreakdown, StepOutcome,
    REWARD_COMPONENT_WIDTH,
};

pub(crate) use cache::{ActionCache, EnvScratch};
pub use constants::{CHECK_TIMING_QUIESCENCE_CAP, HAND_LIMIT, STACK_AUTO_RESOLVE_CAP};
pub(crate) use constants::{
    MAX_CHOICE_OPTIONS, TRIGGER_EFFECT_BOUNCE, TRIGGER_EFFECT_DRAW, TRIGGER_EFFECT_GATE,
    TRIGGER_EFFECT_POOL_MOVE, TRIGGER_EFFECT_POOL_STOCK, TRIGGER_EFFECT_SHOT, TRIGGER_EFFECT_SOUL,
    TRIGGER_EFFECT_STANDBY, TRIGGER_EFFECT_TREASURE_MOVE, TRIGGER_EFFECT_TREASURE_STOCK,
};
pub(crate) use types::{
    DamageIntentLocal, DamageResolveResult, TriggerCompileContext, VisibilityContext,
};

impl GameEnv {
    /// Add a temporary or permanent modifier to a stage slot.
    pub fn add_modifier(
        &mut self,
        source: CardId,
        target_player: u8,
        target_slot: u8,
        kind: ModifierKind,
        magnitude: i32,
        duration: ModifierDuration,
    ) -> Option<u32> {
        self.add_modifier_instance(
            source,
            None,
            target_player,
            target_slot,
            kind,
            magnitude,
            duration,
            crate::state::ModifierLayer::Effect,
        )
    }

    /// Mark rule-action fixpoint work as stale for the next engine advance.
    ///
    /// Any mutation that can enable or disable rule actions should call this so
    /// `run_rule_actions_if_needed` re-evaluates invariants before the next
    /// decision boundary.
    pub(crate) fn mark_rule_actions_dirty(&mut self) {
        self.rule_actions_dirty = true;
    }

    /// Mark continuous-modifier derived state as stale for recomputation.
    ///
    /// This is paired with power/cache invalidation and ensures continuous
    /// effects are reapplied exactly once on the next advance pass.
    pub(crate) fn mark_continuous_modifiers_dirty(&mut self) {
        self.continuous_modifiers_dirty = true;
    }

    fn run_rule_actions_if_needed(&mut self) {
        if self.state.turn.phase != self.last_rule_action_phase {
            self.rule_actions_dirty = true;
            self.last_rule_action_phase = self.state.turn.phase;
        }
        if !self.rule_actions_dirty {
            return;
        }
        self.rule_actions_dirty = false;
        self.resolve_rule_actions_until_stable();
    }

    fn card_set_allowed(&self, card: &CardStatic) -> bool {
        match (&self.curriculum.allowed_card_sets_cache, &card.card_set) {
            (None, _) => true,
            (Some(set), Some(set_id)) => set.contains(set_id),
            (Some(_), None) => false,
        }
    }

    fn ability_conditions_met(
        &self,
        controller: u8,
        conditions: &crate::db::AbilityDefConditions,
    ) -> bool {
        if conditions.requires_approx_effects && !self.curriculum.enable_approx_effects {
            return false;
        }
        if let Some(turn) = conditions.turn {
            let is_self_turn = self.state.turn.active_player == controller;
            match turn {
                crate::db::ConditionTurn::SelfTurn if !is_self_turn => return false,
                crate::db::ConditionTurn::OpponentTurn if is_self_turn => return false,
                _ => {}
            }
        }
        if let Some(max_memory) = conditions.self_memory_at_most {
            if self.state.players[controller as usize].memory.len() > max_memory as usize {
                return false;
            }
        }
        if !conditions.self_memory_card_ids_any.is_empty() {
            let has_required = self.state.players[controller as usize]
                .memory
                .iter()
                .any(|card_inst| conditions.self_memory_card_ids_any.contains(&card_inst.id));
            if !has_required {
                return false;
            }
        }
        if conditions.trigger_check_revealed_climax {
            let Some(ctx) = self.state.turn.attack.as_ref() else {
                return false;
            };
            if self.state.turn.active_player != controller {
                return false;
            }
            let Some(trigger_card) = ctx.trigger_card else {
                return false;
            };
            let Some(card) = self.db.get(trigger_card) else {
                return false;
            };
            if card.card_type != crate::db::CardType::Climax {
                return false;
            }
        }
        if let Some(required_icon) = conditions.trigger_check_revealed_icon {
            let Some(ctx) = self.state.turn.attack.as_ref() else {
                return false;
            };
            if self.state.turn.active_player != controller {
                return false;
            }
            let Some(trigger_card) = ctx.trigger_card else {
                return false;
            };
            let Some(card) = self.db.get(trigger_card) else {
                return false;
            };
            if !card.triggers.contains(&required_icon) {
                return false;
            }
        }
        if let Some(zone_condition) = conditions.zone_count.as_ref() {
            if !self.zone_count_condition_met(controller, zone_condition) {
                return false;
            }
        }
        let Some(climax_condition) = conditions.climax_area.as_ref() else {
            return true;
        };
        let target_player = match climax_condition.side {
            crate::state::TargetSide::SelfSide => controller,
            crate::state::TargetSide::Opponent => 1 - controller,
        } as usize;
        let climax = self
            .state
            .players
            .get(target_player)
            .map(|player| &player.climax);
        let Some(climax) = climax else {
            return false;
        };
        if climax.is_empty() {
            return false;
        }
        if climax_condition.card_ids.is_empty() {
            return true;
        }
        climax
            .iter()
            .any(|card_inst| climax_condition.card_ids.contains(&card_inst.id))
    }

    pub(in crate::env) fn auto_ability_conditions_met(
        &self,
        controller: u8,
        _source_card: CardId,
        spec: &AbilitySpec,
    ) -> bool {
        let AbilityTemplate::AbilityDef(def) = &spec.template else {
            return true;
        };
        self.ability_conditions_met(controller, &def.conditions)
    }

    /// Compute terminal reward from `perspective` using the configured reward table.
    ///
    /// Non-terminal states always produce `0.0`; timeouts use the dedicated
    /// timeout reward while faults remain draw-equivalent.
    pub(crate) fn terminal_reward_for(&self, perspective: u8) -> f32 {
        let RewardConfig {
            terminal_win,
            terminal_loss,
            terminal_draw,
            terminal_timeout,
            ..
        } = &self.config.reward;
        match self.state.terminal {
            Some(TerminalResult::Win { winner }) => {
                if winner == perspective {
                    *terminal_win
                } else {
                    *terminal_loss
                }
            }
            Some(TerminalResult::Draw) => *terminal_draw,
            Some(TerminalResult::Timeout) => *terminal_timeout,
            None => 0.0,
        }
    }

    pub(in crate::env) fn zone_count_value_for_condition(
        &self,
        controller: u8,
        condition: &ZoneCountCondition,
    ) -> usize {
        let player = match condition.side {
            crate::state::TargetSide::SelfSide => controller,
            crate::state::TargetSide::Opponent => 1 - controller,
        } as usize;
        let matches_id = |card_id: CardId| {
            condition.card_ids.is_empty() || condition.card_ids.contains(&card_id)
        };
        match condition.zone {
            CountZone::Stock => self.state.players[player]
                .stock
                .iter()
                .filter(|card| matches_id(card.id))
                .count(),
            CountZone::WaitingRoom => self.state.players[player]
                .waiting_room
                .iter()
                .filter(|card| matches_id(card.id))
                .count(),
            CountZone::Hand => self.state.players[player]
                .hand
                .iter()
                .filter(|card| matches_id(card.id))
                .count(),
            CountZone::Stage => self.state.players[player]
                .stage
                .iter()
                .filter(|slot| slot.card.map(|card| matches_id(card.id)).unwrap_or(false))
                .count(),
            CountZone::BackStage => {
                if self.curriculum.reduced_stage_mode {
                    0
                } else {
                    self.state.players[player]
                        .stage
                        .iter()
                        .enumerate()
                        .filter(|(idx, slot)| {
                            *idx >= 3 && slot.card.map(|card| matches_id(card.id)).unwrap_or(false)
                        })
                        .count()
                }
            }
            CountZone::WaitingRoomClimax => self.state.players[player]
                .waiting_room
                .iter()
                .filter(|card| {
                    if !matches_id(card.id) {
                        return false;
                    }
                    self.db
                        .get(card.id)
                        .map(|static_card| static_card.card_type == crate::db::CardType::Climax)
                        .unwrap_or(false)
                })
                .count(),
            CountZone::LevelTotal => self.state.players[player]
                .level
                .iter()
                .filter(|card| matches_id(card.id))
                .map(|card| {
                    self.db
                        .get(card.id)
                        .map(|static_card| static_card.level as usize)
                        .unwrap_or(0)
                })
                .sum(),
        }
    }

    pub(in crate::env) fn zone_count_condition_met(
        &self,
        controller: u8,
        condition: &ZoneCountCondition,
    ) -> bool {
        let count = self.zone_count_value_for_condition(controller, condition);
        match condition.cmp {
            CountCmp::AtLeast => count >= condition.value as usize,
            CountCmp::AtMost => count <= condition.value as usize,
        }
    }
}

#[cfg(test)]
mod tests;
