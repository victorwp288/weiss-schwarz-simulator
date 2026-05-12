use serde::{Deserialize, Serialize};

use crate::db::CardId;
use crate::state::TargetSide;

use super::EffectId;

/// Terminal outcome specified relative to the effect controller.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TerminalOutcomeSpec {
    /// Controller wins.
    WinSelf,
    /// Controller loses (opponent wins).
    WinOpponent,
    /// Game ends in draw.
    Draw,
    /// Game ends in timeout.
    Timeout,
}

/// Turn-scoped rule-action override selectors.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RuleOverrideKind {
    /// Skip deck-empty refresh/loss processing in rule actions.
    SkipDeckRefreshOrLoss,
    /// Skip level-4 loss checks in rule actions.
    SkipLevelFourLoss,
    /// Skip non-character stage cleanup in rule actions.
    SkipNonCharacterStageCleanup,
    /// Skip non-positive-power stage cleanup in rule actions.
    SkipZeroOrNegativePowerCleanup,
}

/// Hook point for replacement effects.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ReplacementHook {
    /// Damage resolution hook.
    Damage,
}

/// Replacement behavior for a hook.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ReplacementKind {
    /// Cancel damage entirely.
    CancelDamage,
    /// Redirect damage to a new target.
    RedirectDamage {
        /// Target side to receive redirected damage.
        new_target: TargetSide,
    },
}

/// Replacement specification with priority ordering.
#[derive(Clone, Debug, Hash, Serialize, Deserialize)]
pub struct ReplacementSpec {
    /// Stable effect id.
    pub id: EffectId,
    /// Source card id.
    pub source: CardId,
    /// Hook point for the replacement.
    pub hook: ReplacementHook,
    /// Replacement behavior.
    pub kind: ReplacementKind,
    /// Priority ordering (higher first).
    pub priority: i16,
    /// Insertion order for stable sorting.
    pub insertion: u32,
}
