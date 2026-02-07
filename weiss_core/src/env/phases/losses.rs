use super::super::GameEnv;
use crate::config::SimultaneousLossPolicy;
use crate::state::TerminalResult;

impl GameEnv {
    pub(in crate::env) fn register_loss(&mut self, player: u8) {
        debug_assert!(player < 2);
        if player >= 2 {
            return;
        }
        if !self.curriculum.use_alternate_end_conditions {
            self.state.terminal = Some(TerminalResult::Win { winner: 1 - player });
            return;
        }
        self.state.turn.pending_losses[player as usize] = true;
    }

    pub(in crate::env) fn resolve_pending_losses(&mut self) {
        if !self.curriculum.use_alternate_end_conditions {
            return;
        }
        if self.state.terminal.is_some() {
            return;
        }
        let p0 = self.state.turn.pending_losses[0];
        let p1 = self.state.turn.pending_losses[1];
        if !(p0 || p1) {
            return;
        }
        let result = if p0 && p1 {
            match self.config.end_condition_policy.simultaneous_loss {
                SimultaneousLossPolicy::Draw => {
                    if self
                        .config
                        .end_condition_policy
                        .allow_draw_on_simultaneous_loss
                    {
                        TerminalResult::Draw
                    } else {
                        TerminalResult::Win {
                            winner: self.state.turn.active_player,
                        }
                    }
                }
                SimultaneousLossPolicy::ActivePlayerWins => TerminalResult::Win {
                    winner: self.state.turn.active_player,
                },
                SimultaneousLossPolicy::NonActivePlayerWins => TerminalResult::Win {
                    winner: 1 - self.state.turn.active_player,
                },
            }
        } else if p0 {
            TerminalResult::Win { winner: 1 }
        } else {
            TerminalResult::Win { winner: 0 }
        };
        self.state.terminal = Some(result);
    }
}
