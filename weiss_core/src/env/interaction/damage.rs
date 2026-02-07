use super::super::GameEnv;
use crate::effects::{ReplacementHook, ReplacementKind};
use crate::state::TargetSide;

impl GameEnv {
    pub(in crate::env) fn apply_replacements_to_damage(
        &mut self,
        source_player: u8,
        target_player: u8,
        amount: i32,
    ) -> (i32, u8) {
        let mut amount = amount;
        let mut target = target_player;
        if amount <= 0 {
            return (0, target);
        }
        self.scratch_replacement_indices.clear();
        for (idx, replacement) in self.state.replacements.iter().enumerate() {
            if matches!(replacement.hook, ReplacementHook::Damage) {
                self.scratch_replacement_indices.push(idx);
            }
        }
        self.scratch_replacement_indices.sort_by_key(|idx| {
            let replacement = &self.state.replacements[*idx];
            (
                replacement.priority,
                replacement.insertion,
                replacement.source,
            )
        });
        for idx in self.scratch_replacement_indices.iter().copied() {
            let replacement = &self.state.replacements[idx];
            match replacement.kind {
                ReplacementKind::CancelDamage => {
                    amount = 0;
                    break;
                }
                ReplacementKind::RedirectDamage { new_target } => {
                    debug_assert!(
                        source_player <= 1,
                        "RedirectDamage assumes a two-player game"
                    );
                    target = match new_target {
                        TargetSide::SelfSide => source_player,
                        TargetSide::Opponent => source_player ^ 1,
                    };
                }
            }
        }
        (amount, target)
    }
}
