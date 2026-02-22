use crate::config::CurriculumConfig;
use crate::db::CardDb;
use crate::legal::{ActionDesc, Decision, DecisionKind, LegalActionIds};
use crate::state::{ChoiceOptionRef, GameState, TargetRef};

use super::actions;

/// Reusable scratch vectors for transient legality/targeting work.
pub(crate) struct EnvScratch {
    pub(crate) targets: Vec<TargetRef>,
    pub(crate) choice_options: Vec<ChoiceOptionRef>,
    pub(crate) priority_actions: Vec<ActionDesc>,
}

impl EnvScratch {
    /// Allocate scratch buffers with stable default capacities.
    pub(crate) fn new() -> Self {
        Self {
            targets: Vec::with_capacity(32),
            choice_options: Vec::with_capacity(32),
            priority_actions: Vec::with_capacity(16),
        }
    }
}

/// Cached legal-action ids and mask views for the active decision.
pub(crate) struct ActionCache {
    pub(crate) mask: Vec<u8>,
    pub(crate) mask_bits: Vec<u64>,
    pub(crate) last_action_ids: LegalActionIds,
    pub(crate) decision_id: u32,
    pub(crate) decision_kind: Option<DecisionKind>,
    pub(crate) decision_player: u8,
}

impl ActionCache {
    /// Create an empty cache sized for the full action space.
    pub(crate) fn new() -> Self {
        Self {
            mask: vec![0u8; crate::encode::ACTION_SPACE_SIZE],
            mask_bits: vec![0u64; crate::encode::ACTION_SPACE_WORDS],
            last_action_ids: LegalActionIds::new(),
            decision_id: u32::MAX,
            decision_kind: None,
            decision_player: 0,
        }
    }

    /// Clear cached ids and zero any enabled mask representations.
    pub(crate) fn clear(&mut self) {
        if self.mask.len() != crate::encode::ACTION_SPACE_SIZE {
            self.mask.resize(crate::encode::ACTION_SPACE_SIZE, 0);
        }
        self.mask.fill(0);
        if self.mask_bits.len() != crate::encode::ACTION_SPACE_WORDS {
            self.mask_bits.resize(crate::encode::ACTION_SPACE_WORDS, 0);
        }
        self.mask_bits.fill(0);
        self.last_action_ids.clear();
        self.decision_id = u32::MAX;
        self.decision_kind = None;
        self.decision_player = 0;
    }

    /// Refresh cache contents when decision identity changes.
    ///
    /// Uses the previous id list to clear only touched mask entries before
    /// writing the newly legal ids, avoiding full-mask rewrites each step.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn update(
        &mut self,
        state: &GameState,
        decision: &Decision,
        decision_id: u32,
        db: &CardDb,
        curriculum: &CurriculumConfig,
        allowed_card_sets: Option<&std::collections::HashSet<String>>,
        write_mask: bool,
        write_mask_bits: bool,
    ) {
        if self.decision_id == decision_id
            && self.decision_kind == Some(decision.kind)
            && self.decision_player == decision.player
        {
            return;
        }
        for &id_u16 in self.last_action_ids.iter() {
            let id = id_u16 as usize;
            if id < crate::encode::ACTION_SPACE_SIZE {
                if write_mask {
                    self.mask[id] = 0;
                }
                if write_mask_bits {
                    let word = id / 64;
                    let bit = id % 64;
                    if word < self.mask_bits.len() {
                        self.mask_bits[word] &= !(1u64 << bit);
                    }
                }
            }
        }
        self.last_action_ids.clear();
        actions::legal_action_ids_cached_into(
            state,
            decision,
            db,
            curriculum,
            allowed_card_sets,
            &mut self.last_action_ids,
        );
        for &id_u16 in self.last_action_ids.iter() {
            let id = id_u16 as usize;
            if id < crate::encode::ACTION_SPACE_SIZE {
                if write_mask {
                    self.mask[id] = 1;
                }
                if write_mask_bits {
                    let word = id / 64;
                    let bit = id % 64;
                    if word < self.mask_bits.len() {
                        self.mask_bits[word] |= 1u64 << bit;
                    }
                }
            }
        }
        self.decision_id = decision_id;
        self.decision_kind = Some(decision.kind);
        self.decision_player = decision.player;
    }
}
