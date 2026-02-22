use crate::legal::Decision;

use super::super::GameEnv;

impl GameEnv {
    /// Set the active decision and advance the monotonic decision id.
    ///
    /// The counter intentionally wraps; reset seeds it with `u32::MAX` so the
    /// first decision in each episode becomes `0`.
    pub(crate) fn set_decision(&mut self, decision: Decision) {
        self.decision = Some(decision);
        self.decision_id = self.decision_id.wrapping_add(1);
    }

    /// Clear the active decision.
    pub(crate) fn clear_decision(&mut self) {
        self.decision = None;
    }
}
