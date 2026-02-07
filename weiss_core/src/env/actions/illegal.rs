use anyhow::{anyhow, Result};

use crate::config::ErrorPolicy;
use crate::state::TerminalResult;

use super::super::{GameEnv, StepOutcome};

impl GameEnv {
    pub(super) fn handle_illegal_action(
        &mut self,
        acting_player: u8,
        reason: &str,
        copy_obs: bool,
    ) -> Result<StepOutcome> {
        self.last_illegal_action = true;
        self.last_perspective = acting_player;
        match self.config.error_policy {
            ErrorPolicy::Strict => Err(anyhow!("Illegal action: {reason}")),
            ErrorPolicy::LenientTerminate => {
                debug_assert!(acting_player <= 1, "invalid acting_player");
                let winner = if acting_player == 0 { 1 } else { 0 };
                self.state.terminal = Some(TerminalResult::Win { winner });
                self.decision = None;
                self.update_action_cache();
                Ok(self.build_outcome_with_obs(self.terminal_reward_for(acting_player), copy_obs))
            }
            ErrorPolicy::LenientNoop => {
                self.update_action_cache();
                Ok(self.build_outcome_with_obs(0.0, copy_obs))
            }
        }
    }
}
