#![allow(missing_docs)]

use serde::{Deserialize, Serialize};

use super::ability::AbilityDef;
use crate::events::RevealAudience;
use crate::state::{TargetSide, TargetZone};

/// Stable numeric card identifier (non-zero).
pub type CardId = u32;

/// Card type classification.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum CardType {
    /// Character card.
    Character,
    /// Event card.
    Event,
    /// Climax card.
    Climax,
}

/// Card color classification.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum CardColor {
    /// Yellow color.
    Yellow,
    /// Green color.
    Green,
    /// Red color.
    Red,
    /// Blue color.
    Blue,
    /// Colorless/neutral.
    Colorless,
}

/// Trigger icon types.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum TriggerIcon {
    /// +1 soul trigger.
    Soul,
    /// Shot trigger.
    Shot,
    /// Bounce trigger.
    Bounce,
    /// Draw trigger.
    Draw,
    /// Choice trigger.
    Choice,
    /// Pool trigger.
    Pool,
    /// Treasure trigger.
    Treasure,
    /// Gate trigger.
    Gate,
    /// Standby trigger.
    Standby,
}

/// Turn-condition selector for conditional continuous effects.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum ConditionTurn {
    /// Condition is true only during the source controller's turn.
    SelfTurn,
    /// Condition is true only during the opponent's turn.
    OpponentTurn,
}

/// Comparison operator for count-based conditions.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum CountCmp {
    /// Count must be greater than or equal to the threshold.
    AtLeast,
    /// Count must be less than or equal to the threshold.
    AtMost,
}

/// Zone selector for count-based conditional checks.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum CountZone {
    Stock,
    WaitingRoom,
    Hand,
    Stage,
    BackStage,
    WaitingRoomClimax,
    /// Sum of printed levels in the player's level zone.
    LevelTotal,
}

/// Simple count condition for zones.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct ZoneCountCondition {
    pub side: TargetSide,
    pub zone: CountZone,
    pub cmp: CountCmp,
    pub value: u8,
    #[serde(default)]
    pub card_ids: Vec<CardId>,
}

/// Duration selector for temporarily granted abilities.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum GrantDuration {
    /// Expires during end-phase cleanup of the current turn.
    UntilEndOfTurn,
    /// Expires during end-phase cleanup of the target controller's opponent next turn.
    UntilEndOfOpponentsNextTurn,
}

/// Destination selector for generalized battle-opponent movement effects.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum BattleOpponentMoveDestination {
    DeckBottom,
    StockThenBottomStockToWaitingRoom,
    Clock,
    Memory,
}

/// Optional prelude action before generalized battle-opponent movement.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum BattleOpponentMovePreludeAction {
    OpponentClockTopToWaitingRoom,
}

/// Target selection template for effects and abilities.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum TargetTemplate {
    /// Opponent front row.
    OppFrontRow,
    /// Opponent back row.
    OppBackRow,
    /// Any opponent stage slot.
    OppStage,
    /// Specific opponent stage slot.
    OppStageSlot { slot: u8 },
    /// Self front row.
    SelfFrontRow,
    /// Self back row.
    SelfBackRow,
    /// Any self stage slot.
    SelfStage,
    /// Specific self stage slot.
    SelfStageSlot { slot: u8 },
    /// The source card itself.
    This,
    /// Self waiting room.
    SelfWaitingRoom,
    /// Self hand.
    SelfHand,
    /// Top of self deck.
    SelfDeckTop,
    /// Self clock.
    SelfClock,
    /// Self level.
    SelfLevel,
    /// Self stock.
    SelfStock,
    /// Self memory.
    SelfMemory,
}

/// Effect template used by ability definitions.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum EffectTemplate {
    /// Draw cards.
    Draw { count: u8 },
    /// Deal damage (optionally cancelable).
    DealDamage { amount: u8, cancelable: bool },
    /// Add power for a duration.
    AddPower { amount: i32, duration_turn: bool },
    /// Add power when the target's level is at least a threshold.
    AddPowerIfTargetLevelAtLeast {
        amount: i32,
        min_level: u8,
        duration_turn: bool,
    },
    /// Add power scaled by the target's level for a duration.
    AddPowerByLevel {
        multiplier: i32,
        duration_turn: bool,
    },
    /// Add power to the source card if its battle opponent's level is at least a threshold.
    AddPowerIfBattleOpponentLevelAtLeast {
        amount: i32,
        min_level: u8,
        duration_turn: bool,
    },
    /// Add soul to the source card if its battle opponent's level is at least a threshold.
    AddSoulIfBattleOpponentLevelAtLeast {
        amount: i32,
        min_level: u8,
        duration_turn: bool,
    },
    /// Add power to the source card if its battle opponent's level matches exactly.
    AddPowerIfBattleOpponentLevelExact {
        amount: i32,
        level: u8,
        duration_turn: bool,
    },
    /// Add power when another attacking character matches one of the provided card ids.
    AddPowerIfOtherAttackerMatches {
        amount: i32,
        duration_turn: bool,
        #[serde(default)]
        attacker_card_ids: Vec<CardId>,
    },
    /// Add soul for a duration.
    AddSoul { amount: i32, duration_turn: bool },
    /// Add soul while this card occupies the middle center-stage position.
    AddSoulIfMiddleCenter { amount: i32 },
    /// Add level for a duration.
    AddLevel { amount: i32, duration_turn: bool },
    /// Continuous-style soul modifier applied to the character facing the source card.
    FacingOpponentAddSoul { amount: i32 },
    /// Disallow frontal attacks for a duration.
    CannotFrontalAttack { duration_turn: bool },
    /// Prevent becoming REVERSE in battle for a duration.
    CannotBecomeReverse { duration_turn: bool },
    /// Prevent being chosen by opponent effects for a duration.
    CannotBeChosenByOpponentEffects { duration_turn: bool },
    /// Prevent moving to another stage position for a duration.
    CannotMoveStagePosition { duration_turn: bool },
    /// Prevent playing events from hand for a duration.
    CannotPlayEventsFromHand { duration_turn: bool },
    /// Prevent playing backups from hand for a duration.
    CannotPlayBackupFromHand { duration_turn: bool },
    /// Prevent standing during stand phase for a duration.
    CannotStandDuringStandPhase { duration_turn: bool },
    /// Move this card's battle opponent to memory when it becomes REVERSE in battle.
    BattleOpponentMoveToMemoryOnReverse { duration_turn: bool },
    /// Set encore stock cost for target character.
    EncoreStockCost { cost: u8, duration_turn: bool },
    /// Grant an ability definition to the target for a limited duration.
    GrantAbilityDef {
        ability: Box<AbilityDef>,
        duration: GrantDuration,
    },
    /// Continuous-style movement lock applied to the character facing the source card.
    FacingOpponentCannotMoveStagePosition,
    /// Continuous-style self protection from becoming REVERSE while facing a matching opponent.
    SelfCannotBecomeReverseIfFacingOpponent {
        #[serde(default)]
        max_level: Option<u8>,
        #[serde(default)]
        max_cost: Option<u8>,
        #[serde(default)]
        level_gt_source_level: bool,
    },
    /// Continuous-style self restriction while facing a higher-level opponent.
    SelfCannotFrontalAttackIfFacingOpponentHigherLevel,
    /// Conditional continuous-style power modifier.
    ConditionalAddPower {
        amount: i32,
        #[serde(default)]
        turn: Option<ConditionTurn>,
        #[serde(default)]
        zone_count: Option<ZoneCountCondition>,
        #[serde(default)]
        require_source_marker: bool,
        #[serde(default)]
        per_source_marker: bool,
        #[serde(default)]
        per_zone_count: bool,
        #[serde(default)]
        exclude_source: bool,
        #[serde(default)]
        target_ids: Vec<CardId>,
    },
    /// Conditional continuous-style level modifier.
    ConditionalAddLevel {
        amount: i32,
        #[serde(default)]
        turn: Option<ConditionTurn>,
        #[serde(default)]
        zone_count: Option<ZoneCountCondition>,
        #[serde(default)]
        require_source_marker: bool,
        #[serde(default)]
        per_source_marker: bool,
        #[serde(default)]
        per_zone_count: bool,
        #[serde(default)]
        exclude_source: bool,
        #[serde(default)]
        target_ids: Vec<CardId>,
    },
    /// Conditional power modifier with explicit duration (usable by non-continuous abilities).
    TimedConditionalAddPower {
        amount: i32,
        duration_turn: bool,
        #[serde(default)]
        turn: Option<ConditionTurn>,
        #[serde(default)]
        zone_count: Option<ZoneCountCondition>,
        #[serde(default)]
        require_source_marker: bool,
        #[serde(default)]
        per_source_marker: bool,
        #[serde(default)]
        per_zone_count: bool,
        #[serde(default)]
        exclude_source: bool,
        #[serde(default)]
        target_ids: Vec<CardId>,
    },
    /// Conditional continuous-style cannot-side-attack modifier.
    ConditionalCannotSideAttack {
        #[serde(default)]
        turn: Option<ConditionTurn>,
        #[serde(default)]
        zone_count: Option<ZoneCountCondition>,
        #[serde(default)]
        require_source_marker: bool,
        #[serde(default)]
        exclude_source: bool,
    },
    /// Disallow side attacks for a duration.
    CannotSideAttack { duration_turn: bool },
    /// Move target card under the source card as a marker.
    MoveToMarker {
        #[serde(default)]
        target_ids: Vec<CardId>,
    },
    /// Move the top card of your deck under this card as a marker.
    MoveTopDeckToMarker,
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
    /// Move target to the bottom of deck.
    MoveToDeckBottom,
    /// Move a waiting-room card to the source card's stage slot.
    MoveWaitingRoomCardToSourceSlot {
        #[serde(default)]
        target_ids: Vec<CardId>,
    },
    /// Return all cards from waiting room to deck, then shuffle.
    RecycleWaitingRoomToDeckShuffle,
    /// Move all stock to waiting room, then refill stock from deck top by the same count.
    ResetStockFromDeckTop { target: TargetSide },
    /// Heal (move top clock to waiting room).
    Heal,
    /// Heal only if the source card was played from hand this turn and remains on stage.
    HealIfSourcePlayedFromHandThisTurn,
    /// Rest the target.
    RestTarget,
    /// Stand the target.
    StandTarget,
    /// Stock charge by count.
    StockCharge { count: u8 },
    /// Mill top cards from deck.
    MillTop { target: TargetSide, count: u8 },
    /// Move target to a specific stage slot.
    MoveStageSlot { slot: u8 },
    /// Move the source card to the first open center-stage slot.
    MoveThisToOpenCenter {
        #[serde(default)]
        require_facing: bool,
    },
    /// Move the source card to the first open back-stage slot.
    MoveThisToOpenBack,
    /// Swap two stage slots.
    SwapStageSlots,
    /// Random discard from hand.
    RandomDiscardFromHand { target: TargetSide, count: u8 },
    /// Random mill from deck.
    RandomMill { target: TargetSide, count: u8 },
    /// Reveal the top of a zone.
    RevealZoneTop {
        target: TargetSide,
        zone: TargetZone,
        count: u8,
        audience: RevealAudience,
    },
    /// Reveal the top card of your deck; if its level is at least `min_level`,
    /// move this card to hand. (Climax is treated as level 0.)
    RevealTopIfLevelAtLeastMoveThisToHand {
        #[serde(alias = "minLevel")]
        min_level: u8,
    },
    /// Reveal the top card of your deck; if its level is at least `min_level`,
    /// rest this card. (Climax is treated as level 0.)
    RevealTopIfLevelAtLeastRestThis {
        #[serde(alias = "minLevel")]
        min_level: u8,
    },
    /// Reveal the top card of your deck; if its level is at least `min_level`,
    /// move that revealed card to stock. (Climax is treated as level 0.)
    RevealTopIfLevelAtLeastMoveTopToStock {
        #[serde(alias = "minLevel")]
        min_level: u8,
    },
    /// Look at the top `count` cards of your deck and reorder them on top.
    LookTopDeckReorder { count: u8 },
    /// Look at the top card and either leave it on top or move it to waiting room.
    LookTopCardTopOrWaitingRoom,
    /// Look at the top card and either leave it on top or move it to deck bottom.
    LookTopCardTopOrBottom,
    /// Look at top cards, move up to `choose_count` cards with level at least `min_level` to hand, and send the rest to waiting room.
    SearchTopDeckToHandLevelAtLeastMillRest {
        look_count: u8,
        choose_count: u8,
        min_level: u8,
    },
    /// Reveal top deck card, then salvage up to `count` waiting-room characters with level at most the revealed card's level.
    RevealTopAndSalvageByRevealedLevel {
        count: u8,
        #[serde(default = "default_climax_level_zero")]
        climax_level: u8,
    },
    /// Change controller of a card.
    ChangeController,
    /// Counter backup (power).
    CounterBackup { power: i32 },
    /// Counter damage reduction.
    CounterDamageReduce { amount: u8 },
    /// Counter damage cancel.
    CounterDamageCancel,
    /// Resolve a trigger icon effect directly.
    TriggerIcon { icon: TriggerIcon },
    /// Reverse this card's current battle opponent when a condition is met.
    BattleOpponentReverseIf {
        #[serde(default)]
        max_level: Option<u8>,
        #[serde(default)]
        max_cost: Option<u8>,
        #[serde(default)]
        level_gt_opponent_level: bool,
    },
    /// Move this card's current battle opponent to the bottom of deck when a condition is met.
    BattleOpponentMoveToDeckBottomIf {
        #[serde(default)]
        max_level: Option<u8>,
        #[serde(default)]
        max_cost: Option<u8>,
        #[serde(default)]
        level_gt_opponent_level: bool,
    },
    /// Move this card's current battle opponent to stock, then move the bottom stock card to waiting room.
    BattleOpponentMoveToStockThenBottomStockToWaitingRoomIf {
        #[serde(default)]
        max_level: Option<u8>,
        #[serde(default)]
        max_cost: Option<u8>,
        #[serde(default)]
        level_gt_opponent_level: bool,
    },
    /// Move top opponent clock to waiting room, then move this card's current battle opponent to clock.
    BattleOpponentMoveToClockAfterClockTopToWaitingRoomIf {
        #[serde(default)]
        max_level: Option<u8>,
        #[serde(default)]
        max_cost: Option<u8>,
        #[serde(default)]
        level_gt_opponent_level: bool,
    },
    /// Move this card's current battle opponent to memory when a condition is met.
    BattleOpponentMoveToMemoryIf {
        #[serde(default)]
        max_level: Option<u8>,
        #[serde(default)]
        max_cost: Option<u8>,
        #[serde(default)]
        level_gt_opponent_level: bool,
    },
    /// Move this card's current battle opponent to clock when a condition is met.
    BattleOpponentMoveToClockIf {
        #[serde(default)]
        max_level: Option<u8>,
        #[serde(default)]
        max_cost: Option<u8>,
        #[serde(default)]
        level_gt_opponent_level: bool,
    },
    /// Generalized battle-opponent movement effect.
    BattleOpponentMoveIf {
        destination: BattleOpponentMoveDestination,
        #[serde(default)]
        prelude: Option<BattleOpponentMovePreludeAction>,
        #[serde(default)]
        max_level: Option<u8>,
        #[serde(default)]
        max_cost: Option<u8>,
        #[serde(default)]
        level_gt_opponent_level: bool,
    },
    /// Put the top card of your deck into your stock if this card's battle opponent meets level threshold.
    BattleOpponentTopDeckToStockIf { min_level: u8 },
    /// Brainstorm resolver (reveal/mill then payoff per climax).
    Brainstorm {
        reveal_count: u8,
        per_climax: u8,
        mode: BrainstormMode,
    },
    /// Set the total trigger checks to perform this attack's trigger step.
    SetTriggerCheckCount { count: u8 },
    /// Rest the source card if no other rested center-stage character is present.
    RestThisIfNoOtherRestCenter,
    /// Prevent a player from using AUTO Encore for the rest of the turn.
    CannotUseAutoEncoreForPlayer { target: TargetSide },
}

/// Brainstorm payoff mode.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum BrainstormMode {
    Draw,
    SalvageCharacter,
    LookTopToHand,
    LookTopToHandThenDiscard,
    SalvageCharacterThenDiscard,
}

const fn default_climax_level_zero() -> u8 {
    0
}
