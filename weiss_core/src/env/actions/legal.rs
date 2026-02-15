use std::collections::HashSet;

use crate::config::CurriculumConfig;
use crate::db::CardDb;
use crate::legal::{Decision, LegalActionIds, LegalActions};
use crate::state::GameState;

use super::super::GameEnv;

/// Compute legal action ids into a reusable output buffer.
pub fn legal_action_ids_cached_into(
    state: &GameState,
    decision: &Decision,
    db: &CardDb,
    curriculum: &CurriculumConfig,
    allowed_card_sets: Option<&HashSet<String>>,
    out: &mut LegalActionIds,
) {
    crate::legal::legal_action_ids_cached_into(
        state,
        decision,
        db,
        curriculum,
        allowed_card_sets,
        out,
    );
}

/// Compute legal action descriptors for a decision.
pub fn legal_actions_cached(
    state: &GameState,
    decision: &Decision,
    db: &CardDb,
    curriculum: &CurriculumConfig,
    allowed_card_sets: Option<&HashSet<String>>,
) -> LegalActions {
    crate::legal::legal_actions_cached(state, decision, db, curriculum, allowed_card_sets)
}

impl GameEnv {
    /// Action mask for the current decision (1 = legal).
    pub fn action_mask(&self) -> &[u8] {
        &self.action_cache.mask
    }

    /// Bitset action mask for the current decision.
    pub fn action_mask_bits(&self) -> &[u64] {
        &self.action_cache.mask_bits
    }

    /// Whether an action id is legal for the current decision.
    pub fn action_id_is_legal(&self, action_id: usize) -> bool {
        if action_id >= crate::encode::ACTION_SPACE_SIZE {
            return false;
        }
        if !self.output_mask_bits_enabled {
            return self
                .action_cache
                .last_action_ids
                .iter()
                .any(|&id| id as usize == action_id);
        }
        let word = action_id / 64;
        let bit = action_id % 64;
        self.action_cache
            .mask_bits
            .get(word)
            .map(|w| (w >> bit) & 1 == 1)
            .unwrap_or(false)
    }

    /// Cached legal action ids for the current decision.
    pub fn action_ids_cache(&self) -> &[u16] {
        &self.action_cache.last_action_ids
    }

    pub(crate) fn update_action_cache(&mut self) {
        let (decision_kind, _) = match self.decision.as_ref() {
            Some(decision) => (decision.kind, decision.player),
            None => {
                self.action_cache.clear();
                return;
            }
        };
        if decision_kind == crate::legal::DecisionKind::AttackDeclaration
            && self.state.turn.derived_attack.is_none()
        {
            self.recompute_derived_attack();
        }
        if let Some(decision) = self.decision.as_ref() {
            self.last_perspective = decision.player;
            self.action_cache.update(
                &self.state,
                decision,
                self.decision_id,
                &self.db,
                &self.curriculum,
                self.curriculum.allowed_card_sets_cache.as_ref(),
                self.output_mask_enabled,
                self.output_mask_bits_enabled,
            );
        } else {
            self.action_cache.clear();
        }
    }
}
