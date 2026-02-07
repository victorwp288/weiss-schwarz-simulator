use crate::legal::Decision;

use super::super::GameEnv;

impl GameEnv {
    pub(crate) fn set_decision(&mut self, decision: Decision) {
        self.decision = Some(decision);
        self.decision_id = self.decision_id.wrapping_add(1);
    }

    pub(crate) fn clear_decision(&mut self) {
        self.decision = None;
    }
}
