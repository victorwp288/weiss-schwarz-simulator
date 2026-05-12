use serde::{Deserialize, Serialize};

use crate::db::{
    BattleOpponentMoveDestination, BattleOpponentMovePreludeAction, BrainstormMode, CardId,
    ConditionTurn, GrantDuration, TriggerIcon, ZoneCountCondition,
};
use crate::events::RevealAudience;
use crate::state::{DamageType, ModifierDuration, ModifierKind, TargetSide, TargetZone};

mod id;
mod payload;
mod replacement;

pub use id::{EffectId, EffectSourceKind, EffectSpec};
pub use payload::EffectPayload;
pub use replacement::{
    ReplacementHook, ReplacementKind, ReplacementSpec, RuleOverrideKind, TerminalOutcomeSpec,
};

/// Effect kinds that can be executed by the engine.
#[derive(Clone, Debug, Hash, Serialize, Deserialize)]
pub enum EffectKind {
    /// Draw cards.
    Draw {
        /// Number of cards to draw.
        count: u8,
    },
    /// Deal damage.
    Damage {
        /// Damage magnitude to deal.
        amount: i32,
        /// Whether damage can be canceled by revealing a climax.
        cancelable: bool,
        /// Damage classification (e.g., battle vs effect damage).
        damage_type: DamageType,
    },
    /// Add a modifier to the target.
    AddModifier {
        /// Modifier kind (power, soul, etc).
        kind: ModifierKind,
        /// Signed magnitude of the modifier.
        magnitude: i32,
        /// Duration of the modifier.
        duration: ModifierDuration,
    },
    /// Grant an ability definition to the target.
    GrantAbilityDef {
        /// Ability definition to grant.
        ability: Box<crate::db::AbilityDef>,
        /// Duration for which the ability is granted.
        duration: GrantDuration,
    },
    /// Add power when the target's level is at least a threshold.
    AddPowerIfTargetLevelAtLeast {
        /// Power magnitude to add.
        amount: i32,
        /// Minimum level threshold for the target.
        min_level: u8,
        /// Duration of the modifier.
        duration: ModifierDuration,
    },
    /// Add power scaled by the target's (computed) level.
    AddPowerByTargetLevel {
        /// Power multiplier per level.
        multiplier: i32,
        /// Duration of the modifier.
        duration: ModifierDuration,
    },
    /// Add power if the battle opponent's level is at least a threshold.
    AddPowerIfBattleOpponentLevelAtLeast {
        /// Power magnitude to add.
        amount: i32,
        /// Minimum level threshold for the battle opponent.
        min_level: u8,
        /// Duration of the modifier.
        duration: ModifierDuration,
    },
    /// Add soul if the battle opponent's level is at least a threshold.
    AddSoulIfBattleOpponentLevelAtLeast {
        /// Soul magnitude to add.
        amount: i32,
        /// Minimum level threshold for the battle opponent.
        min_level: u8,
        /// Duration of the modifier.
        duration: ModifierDuration,
    },
    /// Add power if the battle opponent's level matches exactly.
    AddPowerIfBattleOpponentLevelExact {
        /// Power magnitude to add.
        amount: i32,
        /// Required battle opponent level.
        level: u8,
        /// Duration of the modifier.
        duration: ModifierDuration,
    },
    /// Add power when another attacking character matches one of the provided card ids.
    AddPowerIfOtherAttackerMatches {
        /// Power magnitude to add.
        amount: i32,
        /// Duration of the modifier.
        duration: ModifierDuration,
        /// Allowed attacker card ids to match against.
        attacker_card_ids: Vec<CardId>,
    },
    /// Add soul while this card occupies the middle center-stage position.
    AddSoulIfMiddleCenter {
        /// Soul magnitude to add.
        amount: i32,
    },
    /// Add soul to the character facing the source card.
    FacingOpponentAddSoul {
        /// Soul magnitude to add.
        amount: i32,
    },
    /// Add a modifier to the character facing the source card.
    FacingOpponentAddModifier {
        /// Modifier kind to apply.
        kind: ModifierKind,
        /// Signed magnitude of the modifier.
        magnitude: i32,
        /// Duration of the modifier.
        duration: ModifierDuration,
    },
    /// Add a modifier to the source card if it is facing a matching opponent.
    SelfAddModifierIfFacingOpponent {
        /// Modifier kind to apply.
        kind: ModifierKind,
        /// Signed magnitude of the modifier.
        magnitude: i32,
        /// Duration of the modifier.
        duration: ModifierDuration,
        /// Optional maximum opponent level allowed.
        max_level: Option<u8>,
        /// Optional maximum opponent cost allowed.
        max_cost: Option<u8>,
        /// If true, require opponent level to exceed the source level.
        level_gt_source_level: bool,
    },
    /// Add a modifier to targets when a conditional context is satisfied.
    ConditionalAddModifier {
        /// Modifier kind to apply.
        kind: ModifierKind,
        /// Signed magnitude of the modifier.
        magnitude: i32,
        /// Duration of the modifier.
        duration: ModifierDuration,
        /// Optional turn condition.
        turn: Option<ConditionTurn>,
        /// Optional zone-count condition.
        zone_count: Option<ZoneCountCondition>,
        /// Whether the source must have at least one marker.
        require_source_marker: bool,
        /// If true, scale magnitude by the number of markers under the source.
        per_source_marker: bool,
        /// If true, scale magnitude by the zone-count value.
        per_zone_count: bool,
        /// If true, skip applying the modifier to the source card itself.
        exclude_source: bool,
    },
    /// Move target to hand.
    MoveToHand,
    /// Move target to waiting room.
    MoveToWaitingRoom,
    /// Move target to stock.
    MoveToStock,
    /// Move target to clock.
    MoveToClock,
    /// Move target to memory.
    MoveToMemory,
    /// Move target to bottom of deck.
    MoveToDeckBottom,
    /// Move a waiting-room card to the source card's stage slot.
    MoveWaitingRoomCardToSourceSlot,
    /// Return all cards from waiting room to deck, then shuffle.
    RecycleWaitingRoomToDeckShuffle,
    /// Move all stock to waiting room, then refill stock from deck top by the same count.
    ResetStockFromDeckTop {
        /// Side whose stock is reset.
        target: TargetSide,
    },
    /// Move target card under the source card as a marker.
    MoveToMarker,
    /// Move the top card of your deck under this card as a marker.
    MoveTopDeckToMarker,
    /// Heal (move top clock to waiting room).
    Heal,
    /// Heal only if the source card was played from hand this turn and remains on stage.
    HealIfSourcePlayedFromHandThisTurn,
    /// Rest the target.
    RestTarget,
    /// Stand the target.
    StandTarget,
    /// Stock charge by count.
    StockCharge {
        /// Number of cards to stock-charge.
        count: u8,
    },
    /// Mill top cards from deck.
    MillTop {
        /// Side whose deck is milled.
        target: TargetSide,
        /// Number of cards to mill.
        count: u8,
    },
    /// Move target to a specific stage slot.
    MoveStageSlot {
        /// Stage slot index.
        slot: u8,
    },
    /// Move the source card to the first open center-stage slot.
    MoveThisToOpenCenter {
        /// Whether the source must currently be facing an opponent.
        require_facing: bool,
    },
    /// Move the source card to the first open back-stage slot.
    MoveThisToOpenBack,
    /// Swap two stage slots.
    SwapStageSlots,
    /// Random discard from hand.
    RandomDiscardFromHand {
        /// Side whose hand is discarded from.
        target: TargetSide,
        /// Number of cards to discard.
        count: u8,
    },
    /// Random mill from deck.
    RandomMill {
        /// Side whose deck is milled.
        target: TargetSide,
        /// Number of cards to mill.
        count: u8,
    },
    /// Reveal the top of a zone.
    RevealZoneTop {
        /// Side whose zone is revealed.
        target: TargetSide,
        /// Zone to reveal cards from.
        zone: TargetZone,
        /// Number of cards to reveal.
        count: u8,
        /// Reveal visibility audience.
        audience: RevealAudience,
    },
    /// Reveal the top card of your deck; if its level is at least `min_level`,
    /// move this card to hand. (Climax is treated as level 0.)
    RevealTopIfLevelAtLeastMoveThisToHand {
        /// Minimum level threshold for success.
        min_level: u8,
    },
    /// Reveal the top card of your deck; if its level is at least `min_level`,
    /// rest this card. (Climax is treated as level 0.)
    RevealTopIfLevelAtLeastRestThis {
        /// Minimum level threshold for success.
        min_level: u8,
    },
    /// Reveal the top card of your deck; if its level is at least `min_level`,
    /// move that revealed card to stock. (Climax is treated as level 0.)
    RevealTopIfLevelAtLeastMoveTopToStock {
        /// Minimum level threshold for success.
        min_level: u8,
    },
    /// Look at the top `count` cards of your deck and reorder them on top.
    LookTopDeckReorder {
        /// Number of cards to look at and reorder.
        count: u8,
    },
    /// Look at the top card and either leave it on top or move it to waiting room.
    LookTopCardTopOrWaitingRoom,
    /// Look at the top card and either leave it on top or move it to deck bottom.
    LookTopCardTopOrBottom,
    /// Look at top cards, move up to `choose_count` cards with level at least `min_level` to hand,
    /// and send the rest to waiting room.
    SearchTopDeckToHandLevelAtLeastMillRest {
        /// Number of cards to look at from the top of deck.
        look_count: u8,
        /// Maximum number of cards to move to hand.
        choose_count: u8,
        /// Minimum level threshold for cards eligible to move to hand.
        min_level: u8,
    },
    /// Reveal top deck card, then salvage up to `count` waiting-room characters with level at most
    /// the revealed card's level.
    RevealTopAndSalvageByRevealedLevel {
        /// Number of cards to salvage.
        count: u8,
        /// Level to treat a climax as during level comparisons.
        climax_level: u8,
    },
    /// Move the trigger card to hand.
    MoveTriggerCardToHand,
    /// Move the trigger card to stock.
    MoveTriggerCardToStock,
    /// Change controller of a card.
    ChangeController {
        /// New controller side.
        new_controller: TargetSide,
    },
    /// Standby trigger resolution (place a character from waiting room onto stage).
    Standby {
        /// Destination stage slot index.
        target_slot: u8,
    },
    /// Treasure trigger resolution (optionally take the stock).
    TreasureStock {
        /// Whether to take stock when resolving the treasure trigger.
        take_stock: bool,
    },
    /// Modify pending attack damage by a delta.
    ModifyPendingAttackDamage {
        /// Signed delta to apply.
        delta: i32,
    },
    /// Enable shot damage for the current attack.
    EnableShotDamage {
        /// Shot damage amount.
        amount: u8,
    },
    /// Resolve a trigger icon effect directly.
    TriggerIcon {
        /// Trigger icon to resolve.
        icon: TriggerIcon,
    },
    /// Reveal the top of the deck.
    RevealDeckTop {
        /// Number of cards to reveal.
        count: u8,
        /// Reveal visibility audience.
        audience: RevealAudience,
    },
    /// Brainstorm resolver (reveal/mill then payoff per climax).
    Brainstorm {
        /// Number of cards to reveal/mill.
        reveal_count: u8,
        /// Payoff multiplier per climax revealed.
        per_climax: u8,
        /// Brainstorm payoff mode.
        mode: BrainstormMode,
    },
    /// Optional brainstorm choice hook for draw-mode resolution.
    BrainstormDrawChoice,
    /// Set the total trigger checks to perform this attack's trigger step.
    SetTriggerCheckCount {
        /// Trigger check count to use.
        count: u8,
    },
    /// Rest the source card if no other rested center-stage character is present.
    RestThisIfNoOtherRestCenter,
    /// Reverse this card's current battle opponent when a condition is met.
    BattleOpponentReverseIf {
        /// Optional maximum opponent level allowed.
        max_level: Option<u8>,
        /// Optional maximum opponent cost allowed.
        max_cost: Option<u8>,
        /// If true, require this card's level to exceed opponent level.
        level_gt_opponent_level: bool,
    },
    /// Move this card's current battle opponent to the bottom of deck when a condition is met.
    BattleOpponentMoveToDeckBottomIf {
        /// Optional maximum opponent level allowed.
        max_level: Option<u8>,
        /// Optional maximum opponent cost allowed.
        max_cost: Option<u8>,
        /// If true, require this card's level to exceed opponent level.
        level_gt_opponent_level: bool,
    },
    /// Move this card's current battle opponent to stock, then move the bottom stock card to
    /// waiting room.
    BattleOpponentMoveToStockThenBottomStockToWaitingRoomIf {
        /// Optional maximum opponent level allowed.
        max_level: Option<u8>,
        /// Optional maximum opponent cost allowed.
        max_cost: Option<u8>,
        /// If true, require this card's level to exceed opponent level.
        level_gt_opponent_level: bool,
    },
    /// Move top opponent clock to waiting room, then move this card's current battle opponent to
    /// clock when a condition is met.
    BattleOpponentMoveToClockAfterClockTopToWaitingRoomIf {
        /// Optional maximum opponent level allowed.
        max_level: Option<u8>,
        /// Optional maximum opponent cost allowed.
        max_cost: Option<u8>,
        /// If true, require this card's level to exceed opponent level.
        level_gt_opponent_level: bool,
    },
    /// Move this card's current battle opponent to memory when a condition is met.
    BattleOpponentMoveToMemoryIf {
        /// Optional maximum opponent level allowed.
        max_level: Option<u8>,
        /// Optional maximum opponent cost allowed.
        max_cost: Option<u8>,
        /// If true, require this card's level to exceed opponent level.
        level_gt_opponent_level: bool,
    },
    /// Move this card's current battle opponent to clock when a condition is met.
    BattleOpponentMoveToClockIf {
        /// Optional maximum opponent level allowed.
        max_level: Option<u8>,
        /// Optional maximum opponent cost allowed.
        max_cost: Option<u8>,
        /// If true, require this card's level to exceed opponent level.
        level_gt_opponent_level: bool,
    },
    /// Generalized battle-opponent movement effect.
    BattleOpponentMove {
        /// Destination zone for the battle opponent.
        destination: BattleOpponentMoveDestination,
        /// Optional prelude action applied before the destination move.
        prelude: Option<BattleOpponentMovePreludeAction>,
        /// Optional maximum opponent level allowed.
        max_level: Option<u8>,
        /// Optional maximum opponent cost allowed.
        max_cost: Option<u8>,
        /// If true, require this card's level to exceed opponent level.
        level_gt_opponent_level: bool,
    },
    /// Put the top card of your deck into your stock if this card's battle opponent meets a level
    /// threshold.
    BattleOpponentTopDeckToStockIf {
        /// Minimum opponent level threshold for success.
        min_level: u8,
    },
    /// Prevent a player from using AUTO Encore for the rest of the turn.
    CannotUseAutoEncoreForPlayer {
        /// Side to apply the restriction to.
        target: TargetSide,
    },
    /// Counter backup (power).
    CounterBackup {
        /// Power magnitude to add.
        power: i32,
    },
    /// Counter damage reduction.
    CounterDamageReduce {
        /// Damage reduction magnitude.
        amount: u8,
    },
    /// Counter damage cancel.
    CounterDamageCancel,
    /// Set terminal game outcome immediately.
    SetTerminalOutcome {
        /// Terminal outcome to set.
        outcome: TerminalOutcomeSpec,
    },
    /// Apply a turn-scoped rule-action override.
    ApplyRuleOverride {
        /// Rule override kind to apply.
        kind: RuleOverrideKind,
    },
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

    /// Whether this effect can target a card in the given zone.
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
