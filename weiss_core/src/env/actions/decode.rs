use crate::legal::ActionDesc;

use super::super::GameEnv;

impl GameEnv {
    /// Decode an action id into a canonical descriptor, if legal.
    pub fn action_for_id(&self, action_id: usize) -> Option<ActionDesc> {
        if !self.action_id_is_legal(action_id) {
            return None;
        }
        crate::encode::action_desc_for_id(action_id)
    }

    /// Enumerate legal actions in canonical form for the current decision.
    pub fn legal_actions(&self) -> Vec<ActionDesc> {
        let mut out = Vec::with_capacity(self.action_cache.last_action_ids.len());
        for &id in self.action_cache.last_action_ids.iter() {
            if let Some(action) = crate::encode::action_desc_for_id(id as usize) {
                out.push(action);
            }
        }
        out
    }
}
