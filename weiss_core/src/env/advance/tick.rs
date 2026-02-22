use crate::events::Event;
use crate::legal::{Decision, DecisionKind};
use crate::state::TerminalResult;

use super::super::{EngineErrorCode, GameEnv, CHECK_TIMING_QUIESCENCE_CAP};

impl GameEnv {
    pub(super) fn advance_tick(&mut self) -> bool {
        if self.state.turn.tick_count >= self.config.max_ticks {
            self.state.terminal = Some(TerminalResult::Timeout);
            return false;
        }
        self.state.turn.tick_count += 1;
        true
    }

    /// Drain immediate rule/trigger/priority work until a decision is reached.
    ///
    /// This is a quiescence loop used by timing/check paths that should settle
    /// pending effects without running the full phase machine. When priority
    /// windows are disabled it may auto-resolve stack items, bounded by
    /// `CHECK_TIMING_QUIESCENCE_CAP`.
    pub(crate) fn resolve_quiescence_until_decision(&mut self) {
        let mut auto_resolve_steps: u32 = 0;
        loop {
            if self.state.terminal.is_some() || self.decision.is_some() {
                return;
            }
            self.run_rule_actions_if_needed();
            self.refresh_continuous_modifiers_if_needed();
            if let Some(player) = self.state.turn.pending_level_up {
                self.set_decision(Decision {
                    player,
                    kind: DecisionKind::LevelUp,
                    focus_slot: None,
                });
                return;
            }
            if self.handle_trigger_pipeline() {
                if self.decision.is_some() {
                    return;
                }
                continue;
            }
            if self.handle_priority_window() {
                if self.decision.is_some() {
                    return;
                }
                continue;
            }
            if !self.curriculum.enable_priority_windows
                && self.state.turn.priority.is_none()
                && self.state.turn.choice.is_none()
                && self.state.turn.stack_order.is_none()
                && !self.state.turn.stack.is_empty()
            {
                auto_resolve_steps = auto_resolve_steps.saturating_add(1);
                if auto_resolve_steps > CHECK_TIMING_QUIESCENCE_CAP {
                    self.log_event(Event::AutoResolveCapExceeded {
                        cap: CHECK_TIMING_QUIESCENCE_CAP,
                        stack_len: self.state.turn.stack.len() as u32,
                        window: self.state.turn.active_window,
                    });
                    self.last_engine_error = true;
                    self.last_engine_error_code = EngineErrorCode::TriggerQuiescenceCap;
                    self.state.terminal = Some(TerminalResult::Timeout);
                    return;
                }
                if let Some(item) = self.state.turn.stack.pop() {
                    self.resolve_stack_item(&item);
                    self.log_event(Event::StackResolved { item });
                    continue;
                }
            }
            break;
        }
    }
}
