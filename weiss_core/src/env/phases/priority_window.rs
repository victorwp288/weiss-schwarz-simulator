use super::super::{EngineErrorCode, GameEnv};
use crate::legal::{Decision, DecisionKind};

impl GameEnv {
    pub(in crate::env) fn handle_priority_window(&mut self) -> bool {
        let Some(priority) = self.state.turn.priority.clone() else {
            return false;
        };
        if self.decision.is_some() {
            return true;
        }
        self.collect_priority_actions(priority.holder);
        if self.scratch.priority_actions.is_empty() {
            self.priority_pass(priority.holder);
            return true;
        }
        if self.scratch.priority_actions.len() == 1
            && self.curriculum.priority_autopick_single_action
        {
            let action = self.scratch.priority_actions[0].clone();
            match self.apply_priority_action(priority.holder, action) {
                Ok(()) => return true,
                Err(err) => {
                    self.last_engine_error = true;
                    self.last_engine_error_code = EngineErrorCode::ActionError;
                    eprintln!("Priority autopick action failed: {err}");
                    return false;
                }
            }
        }
        if self.start_priority_choice(priority.holder) {
            self.set_decision(Decision {
                player: priority.holder,
                kind: DecisionKind::Choice,
                focus_slot: None,
            });
        }
        true
    }
}
