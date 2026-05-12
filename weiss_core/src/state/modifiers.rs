use serde::{Deserialize, Serialize};

use crate::db::{AbilitySpec, CardId};
use crate::effects::EffectSpec;

use super::CardInstanceId;

/// Modifier kinds applied to cards or zones.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ModifierKind {
    /// Modify card power.
    Power,
    /// Modify card soul.
    Soul,
    /// Modify effective level.
    Level,
    /// Modify stock cost to attack.
    AttackCost,
    /// Prevent declaring attacks.
    CannotAttack,
    /// Prevent declaring side attacks.
    CannotSideAttack,
    /// Prevent declaring frontal attacks.
    CannotFrontalAttack,
    /// Prevent a character from becoming reversed.
    CannotBecomeReverse,
    /// Prevent being chosen by opponent effects.
    CannotBeChosenByOpponentEffects,
    /// Prevent moving stage position.
    CannotMoveStagePosition,
    /// Prevent playing events from hand.
    CannotPlayEventsFromHand,
    /// Prevent playing backup from hand.
    CannotPlayBackupFromHand,
    /// Prevent standing during the stand phase.
    CannotStandDuringStandPhase,
    /// On reverse, move the battle opponent to memory.
    BattleOpponentMoveToMemoryOnReverse,
    /// Modify stock cost to encore.
    EncoreStockCost,
}

/// Modifier duration semantics.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ModifierDuration {
    /// Expires during end-of-turn cleanup.
    UntilEndOfTurn,
    /// Persists while the card remains on stage.
    WhileOnStage,
}

/// Modifier layer for ordering purposes.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ModifierLayer {
    /// Continuous effects.
    Continuous,
    /// Effect resolution layer (default).
    #[default]
    Effect,
}

/// Concrete modifier instance applied to a target.
#[derive(Clone, Debug, Hash, Serialize, Deserialize)]
pub struct ModifierInstance {
    /// Unique modifier id.
    pub id: u32,
    /// Source card id that created the modifier.
    pub source: CardId,
    /// Optional source slot when the source is on stage.
    #[serde(default)]
    pub source_slot: Option<u8>,
    /// Target player seat.
    pub target_player: u8,
    /// Target stage slot index.
    pub target_slot: u8,
    /// Target card id (for debugging and validation).
    pub target_card: CardId,
    /// Modifier kind.
    pub kind: ModifierKind,
    /// Modifier magnitude (kind-dependent).
    pub magnitude: i32,
    /// Duration for which the modifier remains active.
    pub duration: ModifierDuration,
    /// Layer used when applying modifiers.
    #[serde(default)]
    pub layer: ModifierLayer,
    /// Insertion order used as a tie-breaker.
    pub insertion: u32,
}

/// Runtime granted ability attached to a specific stage card instance.
#[derive(Clone, Debug, Hash, Serialize, Deserialize)]
pub struct GrantedAbilityInstance {
    /// Stable grant id.
    pub grant_id: u64,
    /// Target player seat.
    pub target_player: u8,
    /// Target stage slot index.
    pub target_slot: u8,
    /// Target card instance id.
    pub target_instance_id: CardInstanceId,
    /// Ability spec (template + conditions + cost).
    pub spec: AbilitySpec,
    /// Compiled effect specs derived from `spec`.
    pub compiled_effects: Vec<EffectSpec>,
    /// Turn number at which this grant expires during end-phase cleanup.
    pub expires_turn_number: u32,
}
