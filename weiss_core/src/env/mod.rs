//! Game environment and advance-until-decision loop.
//!
//! Related docs:
//! - <https://github.com/victorwp288/weiss-schwarz-simulator/blob/main/docs/README.md>
//! - <https://github.com/victorwp288/weiss-schwarz-simulator/blob/main/docs/engine_architecture.md>
//! - <https://github.com/victorwp288/weiss-schwarz-simulator/blob/main/docs/rl_contract.md>

use crate::config::RewardConfig;
use crate::db::{CardId, CardStatic};
use crate::state::{ModifierDuration, ModifierKind, TerminalResult};

mod actions;
mod advance;
mod cache;
mod constants;
mod core;
mod debug_events;
mod debug_fingerprints;
mod debug_validate;
mod lifecycle;
mod obs;
mod shared;
mod types;

mod interaction;
mod modifiers;
mod movement;
mod phases;
mod visibility;

#[cfg(feature = "test-harness")]
pub mod harness;

pub use actions::legal_action_ids_cached_into;
pub use core::GameEnv;
pub use types::{DebugConfig, EngineErrorCode, EnvInfo, StepOutcome};

pub(crate) use cache::{ActionCache, EnvScratch};
pub use constants::{CHECK_TIMING_QUIESCENCE_CAP, HAND_LIMIT, STACK_AUTO_RESOLVE_CAP};
pub(crate) use constants::{
    MAX_CHOICE_OPTIONS, TRIGGER_EFFECT_BOUNCE, TRIGGER_EFFECT_DRAW, TRIGGER_EFFECT_GATE,
    TRIGGER_EFFECT_SHOT, TRIGGER_EFFECT_SOUL, TRIGGER_EFFECT_STANDBY, TRIGGER_EFFECT_TREASURE_MOVE,
    TRIGGER_EFFECT_TREASURE_STOCK,
};
pub(crate) use types::{DamageIntentLocal, TriggerCompileContext, VisibilityContext};

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

    pub(crate) fn mark_rule_actions_dirty(&mut self) {
        self.rule_actions_dirty = true;
    }

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

    pub(crate) fn terminal_reward_for(&self, perspective: u8) -> f32 {
        let RewardConfig {
            terminal_win,
            terminal_loss,
            terminal_draw,
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
            Some(TerminalResult::Draw | TerminalResult::Timeout) => *terminal_draw,
            None => 0.0,
        }
    }
}

#[cfg(test)]
mod tests;
