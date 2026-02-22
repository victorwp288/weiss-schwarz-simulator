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
    /// Player stock zone.
    Stock,
    /// Player waiting room zone.
    WaitingRoom,
    /// Player hand zone.
    Hand,
    /// Full stage (front row + back row).
    Stage,
    /// Back row stage slots only.
    BackStage,
    /// Climax cards in the waiting room.
    WaitingRoomClimax,
    /// Sum of printed levels in the player's level zone.
    LevelTotal,
}

/// Simple count condition for zones.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct ZoneCountCondition {
    /// Side to evaluate the count for.
    pub side: TargetSide,
    /// Zone to count cards in.
    pub zone: CountZone,
    /// Comparison operator to apply to the count.
    pub cmp: CountCmp,
    /// Threshold value for the comparison.
    pub value: u8,
    /// Optional card-id filter; empty means any card id.
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
    /// Move to the bottom of deck.
    DeckBottom,
    /// Move to stock, then move the bottom stock card to waiting room.
    StockThenBottomStockToWaitingRoom,
    /// Move to clock.
    Clock,
    /// Move to memory.
    Memory,
}

/// Optional prelude action before generalized battle-opponent movement.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum BattleOpponentMovePreludeAction {
    /// Move the top opponent clock to waiting room before applying the destination move.
    OpponentClockTopToWaitingRoom,
}

/// Terminal outcome specified relative to the effect controller.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
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
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
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
    OppStageSlot {
        /// Stage slot index.
        slot: u8,
    },
    /// Self front row.
    SelfFrontRow,
    /// Self back row.
    SelfBackRow,
    /// Any self stage slot.
    SelfStage,
    /// Specific self stage slot.
    SelfStageSlot {
        /// Stage slot index.
        slot: u8,
    },
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
    Draw {
        /// Number of cards to draw.
        count: u8,
    },
    /// Deal damage (optionally cancelable).
    DealDamage {
        /// Damage amount to deal.
        amount: u8,
        /// Whether damage can be canceled by revealing a climax.
        cancelable: bool,
    },
    /// Add power for a duration.
    AddPower {
        /// Power magnitude to add.
        amount: i32,
        /// If true, expires at end of turn; otherwise lasts while on stage.
        duration_turn: bool,
    },
    /// Add power when the target's level is at least a threshold.
    AddPowerIfTargetLevelAtLeast {
        /// Power magnitude to add.
        amount: i32,
        /// Minimum level threshold for the target.
        min_level: u8,
        /// If true, expires at end of turn; otherwise lasts while on stage.
        duration_turn: bool,
    },
    /// Add power scaled by the target's level for a duration.
    AddPowerByLevel {
        /// Power multiplier per (computed) level.
        multiplier: i32,
        /// If true, expires at end of turn; otherwise lasts while on stage.
        duration_turn: bool,
    },
    /// Add power to the source card if its battle opponent's level is at least a threshold.
    AddPowerIfBattleOpponentLevelAtLeast {
        /// Power magnitude to add.
        amount: i32,
        /// Minimum level threshold for the battle opponent.
        min_level: u8,
        /// If true, expires at end of turn; otherwise lasts while on stage.
        duration_turn: bool,
    },
    /// Add soul to the source card if its battle opponent's level is at least a threshold.
    AddSoulIfBattleOpponentLevelAtLeast {
        /// Soul magnitude to add.
        amount: i32,
        /// Minimum level threshold for the battle opponent.
        min_level: u8,
        /// If true, expires at end of turn; otherwise lasts while on stage.
        duration_turn: bool,
    },
    /// Add power to the source card if its battle opponent's level matches exactly.
    AddPowerIfBattleOpponentLevelExact {
        /// Power magnitude to add.
        amount: i32,
        /// Required battle opponent level.
        level: u8,
        /// If true, expires at end of turn; otherwise lasts while on stage.
        duration_turn: bool,
    },
    /// Add power when another attacking character matches one of the provided card ids.
    AddPowerIfOtherAttackerMatches {
        /// Power magnitude to add.
        amount: i32,
        /// If true, expires at end of turn; otherwise lasts while on stage.
        duration_turn: bool,
        /// Allowed attacker card ids to match against.
        #[serde(default)]
        attacker_card_ids: Vec<CardId>,
    },
    /// Add soul for a duration.
    AddSoul {
        /// Soul magnitude to add.
        amount: i32,
        /// If true, expires at end of turn; otherwise lasts while on stage.
        duration_turn: bool,
    },
    /// Add soul while this card occupies the middle center-stage position.
    AddSoulIfMiddleCenter {
        /// Soul magnitude to add.
        amount: i32,
    },
    /// Add level for a duration.
    AddLevel {
        /// Level magnitude to add.
        amount: i32,
        /// If true, expires at end of turn; otherwise lasts while on stage.
        duration_turn: bool,
    },
    /// Continuous-style soul modifier applied to the character facing the source card.
    FacingOpponentAddSoul {
        /// Soul magnitude to add.
        amount: i32,
    },
    /// Disallow frontal attacks for a duration.
    CannotFrontalAttack {
        /// If true, expires at end of turn; otherwise lasts while on stage.
        duration_turn: bool,
    },
    /// Prevent becoming REVERSE in battle for a duration.
    CannotBecomeReverse {
        /// If true, expires at end of turn; otherwise lasts while on stage.
        duration_turn: bool,
    },
    /// Prevent being chosen by opponent effects for a duration.
    CannotBeChosenByOpponentEffects {
        /// If true, expires at end of turn; otherwise lasts while on stage.
        duration_turn: bool,
    },
    /// Prevent moving to another stage position for a duration.
    CannotMoveStagePosition {
        /// If true, expires at end of turn; otherwise lasts while on stage.
        duration_turn: bool,
    },
    /// Prevent playing events from hand for a duration.
    CannotPlayEventsFromHand {
        /// If true, expires at end of turn; otherwise lasts while on stage.
        duration_turn: bool,
    },
    /// Prevent playing backups from hand for a duration.
    CannotPlayBackupFromHand {
        /// If true, expires at end of turn; otherwise lasts while on stage.
        duration_turn: bool,
    },
    /// Prevent standing during stand phase for a duration.
    CannotStandDuringStandPhase {
        /// If true, expires at end of turn; otherwise lasts while on stage.
        duration_turn: bool,
    },
    /// Move this card's battle opponent to memory when it becomes REVERSE in battle.
    BattleOpponentMoveToMemoryOnReverse {
        /// If true, expires at end of turn; otherwise lasts while on stage.
        duration_turn: bool,
    },
    /// Set encore stock cost for target character.
    EncoreStockCost {
        /// Stock cost to pay for encore.
        cost: u8,
        /// If true, expires at end of turn; otherwise lasts while on stage.
        duration_turn: bool,
    },
    /// Grant an ability definition to the target for a limited duration.
    GrantAbilityDef {
        /// Ability definition to grant.
        ability: Box<AbilityDef>,
        /// Duration for which the ability is granted.
        duration: GrantDuration,
    },
    /// Continuous-style movement lock applied to the character facing the source card.
    FacingOpponentCannotMoveStagePosition,
    /// Continuous-style self protection from becoming REVERSE while facing a matching opponent.
    SelfCannotBecomeReverseIfFacingOpponent {
        /// Optional maximum opponent level allowed.
        #[serde(default)]
        max_level: Option<u8>,
        /// Optional maximum opponent cost allowed.
        #[serde(default)]
        max_cost: Option<u8>,
        /// If true, require opponent level to exceed the source level.
        #[serde(default)]
        level_gt_source_level: bool,
    },
    /// Continuous-style self restriction while facing a higher-level opponent.
    SelfCannotFrontalAttackIfFacingOpponentHigherLevel,
    /// Conditional continuous-style power modifier.
    ConditionalAddPower {
        /// Power magnitude to add.
        amount: i32,
        /// Optional turn condition.
        #[serde(default)]
        turn: Option<ConditionTurn>,
        /// Optional zone-count condition.
        #[serde(default)]
        zone_count: Option<ZoneCountCondition>,
        /// Whether the source must have at least one marker.
        #[serde(default)]
        require_source_marker: bool,
        /// If true, scale magnitude by the number of markers under the source.
        #[serde(default)]
        per_source_marker: bool,
        /// If true, scale magnitude by the zone-count value.
        #[serde(default)]
        per_zone_count: bool,
        /// If true, skip applying the modifier to the source card itself.
        #[serde(default)]
        exclude_source: bool,
        /// Optional target card-id filter.
        #[serde(default)]
        target_ids: Vec<CardId>,
    },
    /// Conditional continuous-style level modifier.
    ConditionalAddLevel {
        /// Level magnitude to add.
        amount: i32,
        /// Optional turn condition.
        #[serde(default)]
        turn: Option<ConditionTurn>,
        /// Optional zone-count condition.
        #[serde(default)]
        zone_count: Option<ZoneCountCondition>,
        /// Whether the source must have at least one marker.
        #[serde(default)]
        require_source_marker: bool,
        /// If true, scale magnitude by the number of markers under the source.
        #[serde(default)]
        per_source_marker: bool,
        /// If true, scale magnitude by the zone-count value.
        #[serde(default)]
        per_zone_count: bool,
        /// If true, skip applying the modifier to the source card itself.
        #[serde(default)]
        exclude_source: bool,
        /// Optional target card-id filter.
        #[serde(default)]
        target_ids: Vec<CardId>,
    },
    /// Conditional power modifier with explicit duration (usable by non-continuous abilities).
    TimedConditionalAddPower {
        /// Power magnitude to add.
        amount: i32,
        /// If true, expires at end of turn; otherwise lasts while on stage.
        duration_turn: bool,
        /// Optional turn condition.
        #[serde(default)]
        turn: Option<ConditionTurn>,
        /// Optional zone-count condition.
        #[serde(default)]
        zone_count: Option<ZoneCountCondition>,
        /// Whether the source must have at least one marker.
        #[serde(default)]
        require_source_marker: bool,
        /// If true, scale magnitude by the number of markers under the source.
        #[serde(default)]
        per_source_marker: bool,
        /// If true, scale magnitude by the zone-count value.
        #[serde(default)]
        per_zone_count: bool,
        /// If true, skip applying the modifier to the source card itself.
        #[serde(default)]
        exclude_source: bool,
        /// Optional target card-id filter.
        #[serde(default)]
        target_ids: Vec<CardId>,
    },
    /// Conditional continuous-style cannot-side-attack modifier.
    ConditionalCannotSideAttack {
        /// Optional turn condition.
        #[serde(default)]
        turn: Option<ConditionTurn>,
        /// Optional zone-count condition.
        #[serde(default)]
        zone_count: Option<ZoneCountCondition>,
        /// Whether the source must have at least one marker.
        #[serde(default)]
        require_source_marker: bool,
        /// If true, skip applying the modifier to the source card itself.
        #[serde(default)]
        exclude_source: bool,
    },
    /// Disallow side attacks for a duration.
    CannotSideAttack {
        /// If true, expires at end of turn; otherwise lasts while on stage.
        duration_turn: bool,
    },
    /// Move target card under the source card as a marker.
    MoveToMarker {
        /// Optional target card-id filter.
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
        /// Optional target card-id filter.
        #[serde(default)]
        target_ids: Vec<CardId>,
    },
    /// Return all cards from waiting room to deck, then shuffle.
    RecycleWaitingRoomToDeckShuffle,
    /// Move all stock to waiting room, then refill stock from deck top by the same count.
    ResetStockFromDeckTop {
        /// Side whose stock is reset.
        target: TargetSide,
    },
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
        #[serde(default)]
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
        #[serde(alias = "minLevel")]
        min_level: u8,
    },
    /// Reveal the top card of your deck; if its level is at least `min_level`,
    /// rest this card. (Climax is treated as level 0.)
    RevealTopIfLevelAtLeastRestThis {
        /// Minimum level threshold for success.
        #[serde(alias = "minLevel")]
        min_level: u8,
    },
    /// Reveal the top card of your deck; if its level is at least `min_level`,
    /// move that revealed card to stock. (Climax is treated as level 0.)
    RevealTopIfLevelAtLeastMoveTopToStock {
        /// Minimum level threshold for success.
        #[serde(alias = "minLevel")]
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
    /// Look at top cards, move up to `choose_count` cards with level at least `min_level` to hand, and send the rest to waiting room.
    SearchTopDeckToHandLevelAtLeastMillRest {
        /// Number of cards to look at from the top of deck.
        look_count: u8,
        /// Maximum number of cards to move to hand.
        choose_count: u8,
        /// Minimum level threshold for cards eligible to move to hand.
        min_level: u8,
    },
    /// Reveal top deck card, then salvage up to `count` waiting-room characters with level at most the revealed card's level.
    RevealTopAndSalvageByRevealedLevel {
        /// Number of cards to salvage.
        count: u8,
        /// Level to treat a climax as during level comparisons.
        #[serde(default = "default_climax_level_zero")]
        climax_level: u8,
    },
    /// Change controller of a card.
    ChangeController,
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
    /// Resolve a trigger icon effect directly.
    TriggerIcon {
        /// Trigger icon to resolve.
        icon: TriggerIcon,
    },
    /// Reverse this card's current battle opponent when a condition is met.
    BattleOpponentReverseIf {
        /// Optional maximum opponent level allowed.
        #[serde(default)]
        max_level: Option<u8>,
        /// Optional maximum opponent cost allowed.
        #[serde(default)]
        max_cost: Option<u8>,
        /// If true, require this card's level to exceed opponent level.
        #[serde(default)]
        level_gt_opponent_level: bool,
    },
    /// Move this card's current battle opponent to the bottom of deck when a condition is met.
    BattleOpponentMoveToDeckBottomIf {
        /// Optional maximum opponent level allowed.
        #[serde(default)]
        max_level: Option<u8>,
        /// Optional maximum opponent cost allowed.
        #[serde(default)]
        max_cost: Option<u8>,
        /// If true, require this card's level to exceed opponent level.
        #[serde(default)]
        level_gt_opponent_level: bool,
    },
    /// Move this card's current battle opponent to stock, then move the bottom stock card to waiting room.
    BattleOpponentMoveToStockThenBottomStockToWaitingRoomIf {
        /// Optional maximum opponent level allowed.
        #[serde(default)]
        max_level: Option<u8>,
        /// Optional maximum opponent cost allowed.
        #[serde(default)]
        max_cost: Option<u8>,
        /// If true, require this card's level to exceed opponent level.
        #[serde(default)]
        level_gt_opponent_level: bool,
    },
    /// Move top opponent clock to waiting room, then move this card's current battle opponent to clock.
    BattleOpponentMoveToClockAfterClockTopToWaitingRoomIf {
        /// Optional maximum opponent level allowed.
        #[serde(default)]
        max_level: Option<u8>,
        /// Optional maximum opponent cost allowed.
        #[serde(default)]
        max_cost: Option<u8>,
        /// If true, require this card's level to exceed opponent level.
        #[serde(default)]
        level_gt_opponent_level: bool,
    },
    /// Move this card's current battle opponent to memory when a condition is met.
    BattleOpponentMoveToMemoryIf {
        /// Optional maximum opponent level allowed.
        #[serde(default)]
        max_level: Option<u8>,
        /// Optional maximum opponent cost allowed.
        #[serde(default)]
        max_cost: Option<u8>,
        /// If true, require this card's level to exceed opponent level.
        #[serde(default)]
        level_gt_opponent_level: bool,
    },
    /// Move this card's current battle opponent to clock when a condition is met.
    BattleOpponentMoveToClockIf {
        /// Optional maximum opponent level allowed.
        #[serde(default)]
        max_level: Option<u8>,
        /// Optional maximum opponent cost allowed.
        #[serde(default)]
        max_cost: Option<u8>,
        /// If true, require this card's level to exceed opponent level.
        #[serde(default)]
        level_gt_opponent_level: bool,
    },
    /// Generalized battle-opponent movement effect.
    BattleOpponentMoveIf {
        /// Destination zone for the battle opponent.
        destination: BattleOpponentMoveDestination,
        /// Optional prelude action applied before the destination move.
        #[serde(default)]
        prelude: Option<BattleOpponentMovePreludeAction>,
        /// Optional maximum opponent level allowed.
        #[serde(default)]
        max_level: Option<u8>,
        /// Optional maximum opponent cost allowed.
        #[serde(default)]
        max_cost: Option<u8>,
        /// If true, require this card's level to exceed opponent level.
        #[serde(default)]
        level_gt_opponent_level: bool,
    },
    /// Put the top card of your deck into your stock if this card's battle opponent meets level threshold.
    BattleOpponentTopDeckToStockIf {
        /// Minimum opponent level threshold for success.
        min_level: u8,
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
    /// Set the total trigger checks to perform this attack's trigger step.
    SetTriggerCheckCount {
        /// Trigger check count to use.
        count: u8,
    },
    /// Rest the source card if no other rested center-stage character is present.
    RestThisIfNoOtherRestCenter,
    /// Prevent a player from using AUTO Encore for the rest of the turn.
    CannotUseAutoEncoreForPlayer {
        /// Side to apply the restriction to.
        target: TargetSide,
    },
    /// Conditional continuous-style soul modifier.
    ///
    /// Appended to preserve postcard discriminants for existing WSDB payloads.
    ConditionalAddSoul {
        /// Soul magnitude to add.
        amount: i32,
        /// Optional turn condition.
        #[serde(default)]
        turn: Option<ConditionTurn>,
        /// Optional zone-count condition.
        #[serde(default)]
        zone_count: Option<ZoneCountCondition>,
        /// Whether the source must have at least one marker.
        #[serde(default)]
        require_source_marker: bool,
        /// If true, scale magnitude by the number of markers under the source.
        #[serde(default)]
        per_source_marker: bool,
        /// If true, scale magnitude by the zone-count value.
        #[serde(default)]
        per_zone_count: bool,
        /// If true, skip applying the modifier to the source card itself.
        #[serde(default)]
        exclude_source: bool,
        /// Optional target card-id filter.
        #[serde(default)]
        target_ids: Vec<CardId>,
    },
    /// Set terminal game outcome immediately.
    ///
    /// Appended to preserve postcard discriminants for existing WSDB payloads.
    SetTerminalOutcome {
        /// Terminal outcome to set.
        outcome: TerminalOutcomeSpec,
    },
    /// Apply a turn-scoped rule-action override.
    ///
    /// Appended to preserve postcard discriminants for existing WSDB payloads.
    ApplyRuleOverride {
        /// Rule override kind to apply.
        kind: RuleOverrideKind,
    },
}

/// Brainstorm payoff mode.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum BrainstormMode {
    /// Draw a card for each climax revealed.
    Draw,
    /// Salvage a character for each climax revealed.
    SalvageCharacter,
    /// Look at the top cards and move eligible cards to hand.
    LookTopToHand,
    /// Look at the top cards, move eligible cards to hand, then discard.
    LookTopToHandThenDiscard,
    /// Salvage characters, then discard.
    SalvageCharacterThenDiscard,
}

const fn default_climax_level_zero() -> u8 {
    0
}
