#![allow(missing_docs)]

use serde::{Deserialize, Serialize};

use crate::db::{
    BattleOpponentMoveDestination, BattleOpponentMovePreludeAction, BrainstormMode, CardId,
    ConditionTurn, GrantDuration, TriggerIcon, ZoneCountCondition,
};
use crate::events::RevealAudience;
use crate::state::{
    DamageType, ModifierDuration, ModifierKind, TargetSide, TargetSpec, TargetZone,
};

/// Source category for an effect.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EffectSourceKind {
    /// Trigger resolution.
    Trigger,
    /// Auto ability.
    Auto,
    /// Activated ability.
    Activated,
    /// Continuous modifier.
    Continuous,
    /// Event card play.
    EventPlay,
    /// Counter timing.
    Counter,
    /// Replacement effect.
    Replacement,
    /// System-generated effect.
    System,
}

/// Stable identifier for an effect instance.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EffectId {
    /// Effect source category.
    pub source_kind: EffectSourceKind,
    /// Source card id (0 means none; see EffectSourceKind).
    pub source_card: CardId,
    /// Ability index on the source card.
    pub ability_index: u8,
    /// Effect index within the ability.
    pub effect_index: u8,
}

impl EffectId {
    /// Build an effect id from its components.
    pub fn new(
        source_kind: EffectSourceKind,
        source_card: CardId,
        ability_index: u8,
        effect_index: u8,
    ) -> Self {
        Self {
            source_kind,
            source_card,
            ability_index,
            effect_index,
        }
    }
}

/// Fully specified effect with targeting metadata.
#[derive(Clone, Debug, Hash, Serialize, Deserialize)]
pub struct EffectSpec {
    /// Stable effect id.
    pub id: EffectId,
    /// Effect kind.
    pub kind: EffectKind,
    /// Optional target specification.
    pub target: Option<TargetSpec>,
    /// Whether this effect is optional.
    pub optional: bool,
}

/// Effect kinds that can be executed by the engine.
#[derive(Clone, Debug, Hash, Serialize, Deserialize)]
pub enum EffectKind {
    Draw {
        count: u8,
    },
    Damage {
        amount: i32,
        cancelable: bool,
        damage_type: DamageType,
    },
    AddModifier {
        kind: ModifierKind,
        magnitude: i32,
        duration: ModifierDuration,
    },
    GrantAbilityDef {
        ability: Box<crate::db::AbilityDef>,
        duration: GrantDuration,
    },
    AddPowerIfTargetLevelAtLeast {
        amount: i32,
        min_level: u8,
        duration: ModifierDuration,
    },
    AddPowerByTargetLevel {
        multiplier: i32,
        duration: ModifierDuration,
    },
    AddPowerIfBattleOpponentLevelAtLeast {
        amount: i32,
        min_level: u8,
        duration: ModifierDuration,
    },
    AddSoulIfBattleOpponentLevelAtLeast {
        amount: i32,
        min_level: u8,
        duration: ModifierDuration,
    },
    AddPowerIfBattleOpponentLevelExact {
        amount: i32,
        level: u8,
        duration: ModifierDuration,
    },
    AddPowerIfOtherAttackerMatches {
        amount: i32,
        duration: ModifierDuration,
        attacker_card_ids: Vec<CardId>,
    },
    AddSoulIfMiddleCenter {
        amount: i32,
    },
    FacingOpponentAddSoul {
        amount: i32,
    },
    FacingOpponentAddModifier {
        kind: ModifierKind,
        magnitude: i32,
        duration: ModifierDuration,
    },
    SelfAddModifierIfFacingOpponent {
        kind: ModifierKind,
        magnitude: i32,
        duration: ModifierDuration,
        max_level: Option<u8>,
        max_cost: Option<u8>,
        level_gt_source_level: bool,
    },
    ConditionalAddModifier {
        kind: ModifierKind,
        magnitude: i32,
        duration: ModifierDuration,
        turn: Option<ConditionTurn>,
        zone_count: Option<ZoneCountCondition>,
        require_source_marker: bool,
        per_source_marker: bool,
        per_zone_count: bool,
        exclude_source: bool,
    },
    MoveToHand,
    MoveToWaitingRoom,
    MoveToStock,
    MoveToClock,
    MoveToMemory,
    MoveToDeckBottom,
    MoveWaitingRoomCardToSourceSlot,
    RecycleWaitingRoomToDeckShuffle,
    ResetStockFromDeckTop {
        target: TargetSide,
    },
    MoveToMarker,
    MoveTopDeckToMarker,
    Heal,
    HealIfSourcePlayedFromHandThisTurn,
    RestTarget,
    StandTarget,
    StockCharge {
        count: u8,
    },
    MillTop {
        target: TargetSide,
        count: u8,
    },
    MoveStageSlot {
        slot: u8,
    },
    MoveThisToOpenCenter {
        require_facing: bool,
    },
    MoveThisToOpenBack,
    SwapStageSlots,
    RandomDiscardFromHand {
        target: TargetSide,
        count: u8,
    },
    RandomMill {
        target: TargetSide,
        count: u8,
    },
    RevealZoneTop {
        target: TargetSide,
        zone: TargetZone,
        count: u8,
        audience: RevealAudience,
    },
    RevealTopIfLevelAtLeastMoveThisToHand {
        min_level: u8,
    },
    RevealTopIfLevelAtLeastRestThis {
        min_level: u8,
    },
    RevealTopIfLevelAtLeastMoveTopToStock {
        min_level: u8,
    },
    LookTopDeckReorder {
        count: u8,
    },
    LookTopCardTopOrWaitingRoom,
    LookTopCardTopOrBottom,
    SearchTopDeckToHandLevelAtLeastMillRest {
        look_count: u8,
        choose_count: u8,
        min_level: u8,
    },
    RevealTopAndSalvageByRevealedLevel {
        count: u8,
        climax_level: u8,
    },
    MoveTriggerCardToHand,
    MoveTriggerCardToStock,
    ChangeController {
        new_controller: TargetSide,
    },
    Standby {
        target_slot: u8,
    },
    TreasureStock {
        take_stock: bool,
    },
    ModifyPendingAttackDamage {
        delta: i32,
    },
    EnableShotDamage {
        amount: u8,
    },
    TriggerIcon {
        icon: TriggerIcon,
    },
    RevealDeckTop {
        count: u8,
        audience: RevealAudience,
    },
    Brainstorm {
        reveal_count: u8,
        per_climax: u8,
        mode: BrainstormMode,
    },
    BrainstormDrawChoice,
    SetTriggerCheckCount {
        count: u8,
    },
    RestThisIfNoOtherRestCenter,
    BattleOpponentReverseIf {
        max_level: Option<u8>,
        max_cost: Option<u8>,
        level_gt_opponent_level: bool,
    },
    BattleOpponentMoveToDeckBottomIf {
        max_level: Option<u8>,
        max_cost: Option<u8>,
        level_gt_opponent_level: bool,
    },
    BattleOpponentMoveToStockThenBottomStockToWaitingRoomIf {
        max_level: Option<u8>,
        max_cost: Option<u8>,
        level_gt_opponent_level: bool,
    },
    BattleOpponentMoveToClockAfterClockTopToWaitingRoomIf {
        max_level: Option<u8>,
        max_cost: Option<u8>,
        level_gt_opponent_level: bool,
    },
    BattleOpponentMoveToMemoryIf {
        max_level: Option<u8>,
        max_cost: Option<u8>,
        level_gt_opponent_level: bool,
    },
    BattleOpponentMoveToClockIf {
        max_level: Option<u8>,
        max_cost: Option<u8>,
        level_gt_opponent_level: bool,
    },
    BattleOpponentMove {
        destination: BattleOpponentMoveDestination,
        prelude: Option<BattleOpponentMovePreludeAction>,
        max_level: Option<u8>,
        max_cost: Option<u8>,
        level_gt_opponent_level: bool,
    },
    BattleOpponentTopDeckToStockIf {
        min_level: u8,
    },
    CannotUseAutoEncoreForPlayer {
        target: TargetSide,
    },
    CounterBackup {
        power: i32,
    },
    CounterDamageReduce {
        amount: u8,
    },
    CounterDamageCancel,
}

impl EffectKind {
    /// Whether this effect expects a target to be selected.
    pub fn expects_target(&self) -> bool {
        matches!(
            self,
            EffectKind::AddModifier { .. }
                | EffectKind::GrantAbilityDef { .. }
                | EffectKind::AddPowerIfTargetLevelAtLeast { .. }
                | EffectKind::AddPowerByTargetLevel { .. }
                | EffectKind::ConditionalAddModifier { .. }
                | EffectKind::MoveToHand
                | EffectKind::MoveToWaitingRoom
                | EffectKind::MoveToStock
                | EffectKind::MoveToClock
                | EffectKind::MoveToMemory
                | EffectKind::MoveToDeckBottom
                | EffectKind::MoveWaitingRoomCardToSourceSlot
                | EffectKind::MoveToMarker
                | EffectKind::Heal
                | EffectKind::RestTarget
                | EffectKind::StandTarget
                | EffectKind::MoveStageSlot { .. }
                | EffectKind::SwapStageSlots
                | EffectKind::ChangeController { .. }
                | EffectKind::Standby { .. }
                | EffectKind::LookTopDeckReorder { .. }
                | EffectKind::LookTopCardTopOrWaitingRoom
                | EffectKind::LookTopCardTopOrBottom
        )
    }

    pub fn requires_target_zone(&self, zone: TargetZone) -> bool {
        match self {
            EffectKind::MoveToHand => {
                matches!(
                    zone,
                    TargetZone::Stage | TargetZone::WaitingRoom | TargetZone::DeckTop
                )
            }
            EffectKind::MoveToWaitingRoom => matches!(
                zone,
                TargetZone::Stage
                    | TargetZone::Hand
                    | TargetZone::DeckTop
                    | TargetZone::Clock
                    | TargetZone::Level
                    | TargetZone::Stock
                    | TargetZone::Memory
                    | TargetZone::Climax
                    | TargetZone::Resolution
                    | TargetZone::WaitingRoom
            ),
            EffectKind::MoveToStock => matches!(
                zone,
                TargetZone::Stage
                    | TargetZone::Hand
                    | TargetZone::DeckTop
                    | TargetZone::Clock
                    | TargetZone::Level
                    | TargetZone::WaitingRoom
                    | TargetZone::Memory
                    | TargetZone::Climax
                    | TargetZone::Resolution
                    | TargetZone::Stock
            ),
            EffectKind::MoveToClock => matches!(
                zone,
                TargetZone::Stage
                    | TargetZone::Hand
                    | TargetZone::DeckTop
                    | TargetZone::WaitingRoom
                    | TargetZone::Resolution
                    | TargetZone::Clock
            ),
            EffectKind::MoveToMemory => matches!(
                zone,
                TargetZone::Stage
                    | TargetZone::Hand
                    | TargetZone::DeckTop
                    | TargetZone::Clock
                    | TargetZone::Level
                    | TargetZone::Stock
                    | TargetZone::WaitingRoom
                    | TargetZone::Climax
                    | TargetZone::Resolution
                    | TargetZone::Memory
            ),
            EffectKind::MoveToDeckBottom => {
                matches!(zone, TargetZone::Stage | TargetZone::DeckTop)
            }
            EffectKind::MoveWaitingRoomCardToSourceSlot => matches!(zone, TargetZone::WaitingRoom),
            EffectKind::Heal => matches!(zone, TargetZone::Clock),
            EffectKind::ChangeController { .. } => matches!(zone, TargetZone::Stage),
            EffectKind::AddModifier { .. }
            | EffectKind::GrantAbilityDef { .. }
            | EffectKind::AddPowerIfTargetLevelAtLeast { .. }
            | EffectKind::AddPowerByTargetLevel { .. }
            | EffectKind::ConditionalAddModifier { .. } => {
                matches!(zone, TargetZone::Stage)
            }
            EffectKind::MoveToMarker => matches!(zone, TargetZone::WaitingRoom),
            EffectKind::LookTopDeckReorder { .. }
            | EffectKind::LookTopCardTopOrWaitingRoom
            | EffectKind::LookTopCardTopOrBottom => matches!(zone, TargetZone::DeckTop),
            EffectKind::RestTarget
            | EffectKind::StandTarget
            | EffectKind::MoveStageSlot { .. }
            | EffectKind::SwapStageSlots => matches!(zone, TargetZone::Stage),
            EffectKind::Standby { .. } => matches!(zone, TargetZone::WaitingRoom),
            EffectKind::RandomDiscardFromHand { .. } => matches!(zone, TargetZone::Hand),
            EffectKind::RandomMill { .. } => matches!(zone, TargetZone::DeckTop),
            EffectKind::RevealZoneTop {
                zone: reveal_zone, ..
            } => zone == *reveal_zone,
            _ => true,
        }
    }
}

/// Effect with resolved targets ready for execution.
#[derive(Clone, Debug, Hash, Serialize, Deserialize)]
pub struct EffectPayload {
    /// Underlying effect specification.
    pub spec: EffectSpec,
    /// Resolved targets for this effect.
    pub targets: Vec<crate::state::TargetRef>,
    /// Source reference for source-relative effects.
    #[serde(default)]
    pub source_ref: Option<crate::state::TargetRef>,
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
    RedirectDamage { new_target: TargetSide },
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
