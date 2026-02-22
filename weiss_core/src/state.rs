use crate::db::{AbilitySpec, CardId};
use crate::effects::{EffectId, EffectPayload, EffectSpec, ReplacementSpec};
use crate::error::StateError;
use crate::util::Rng64;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

/// Unique identifier for a card instance within a game.
pub type CardInstanceId = u32;

/// Concrete card instance with ownership and controller.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CardInstance {
    /// Static card identifier in the card database.
    pub id: CardId,
    /// Stable per-game identifier for this physical card instance.
    pub instance_id: CardInstanceId,
    /// Owning seat (0 or 1).
    pub owner: u8,
    /// Current controlling seat (0 or 1).
    pub controller: u8,
}

/// Max number of reveal history entries tracked per player.
pub const REVEAL_HISTORY_LEN: usize = 8;

/// Ring buffer of recently revealed cards.
#[derive(Clone, Debug, Hash, Serialize, Deserialize)]
pub struct RevealHistory {
    entries: [CardId; REVEAL_HISTORY_LEN],
    len: u8,
    head: u8,
}

impl RevealHistory {
    /// Create an empty reveal history.
    pub fn new() -> Self {
        Self {
            entries: [0; REVEAL_HISTORY_LEN],
            len: 0,
            head: 0,
        }
    }

    /// Push a newly revealed card into the history.
    pub fn push(&mut self, card: CardId) {
        let head = self.head as usize;
        self.entries[head] = card;
        if (self.len as usize) < REVEAL_HISTORY_LEN {
            self.len = self.len.saturating_add(1);
        }
        self.head = ((head + 1) % REVEAL_HISTORY_LEN) as u8;
    }

    /// Write entries in chronological order into `out`.
    pub fn write_chronological(&self, out: &mut [i32]) {
        out.fill(0);
        let len = self.len as usize;
        if len == 0 {
            return;
        }
        let start = if len < REVEAL_HISTORY_LEN {
            0
        } else {
            self.head as usize
        };
        for idx in 0..len.min(out.len()) {
            let entry_idx = if len < REVEAL_HISTORY_LEN {
                idx
            } else {
                (start + idx) % REVEAL_HISTORY_LEN
            };
            out[idx] = self.entries[entry_idx] as i32;
        }
    }
}

impl Default for RevealHistory {
    fn default() -> Self {
        Self::new()
    }
}

impl CardInstance {
    /// Create a new card instance owned by `owner`.
    pub fn new(id: CardId, owner: u8, instance_id: CardInstanceId) -> Self {
        Self {
            id,
            instance_id,
            owner,
            controller: owner,
        }
    }
}

/// Turn phase.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Phase {
    /// Mulligan step before the first turn begins.
    Mulligan,
    /// Stand phase: stand rested characters.
    Stand,
    /// Draw phase: draw a card.
    Draw,
    /// Clock phase: optionally place a card into clock.
    Clock,
    /// Main phase: play cards and use main-phase abilities.
    Main,
    /// Climax phase: optionally place a climax.
    Climax,
    /// Attack phase.
    Attack,
    /// End phase cleanup.
    End,
}

/// Timing window for triggered effects.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TimingWindow {
    /// Main phase timing window.
    MainWindow,
    /// Climax phase timing window.
    ClimaxWindow,
    /// After an attack is declared.
    AttackDeclarationWindow,
    /// During trigger reveal/resolution.
    TriggerResolutionWindow,
    /// During counter timing.
    CounterWindow,
    /// During damage resolution.
    DamageResolutionWindow,
    /// During encore timing.
    EncoreWindow,
    /// During end phase timing.
    EndPhaseWindow,
}

/// Stage slot status.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum StageStatus {
    /// Standing (upright) character.
    Stand,
    /// Rested (tapped) character.
    Rest,
    /// Reversed (defeated) character.
    Reverse,
}

/// Stage slot containing a character or empty.
#[derive(Clone, Debug, Hash, Serialize, Deserialize)]
pub struct StageSlot {
    /// Occupying card instance, if any.
    pub card: Option<CardInstance>,
    /// Marker cards attached to the occupying character.
    #[serde(default)]
    pub markers: Vec<CardInstance>,
    /// Current stand/rest/reverse status.
    pub status: StageStatus,
    /// Whether the current card was played from hand this turn.
    #[serde(default)]
    pub played_from_hand_this_turn: bool,
    /// Battle-only power modifier.
    pub power_mod_battle: i32,
    /// Turn-long power modifier.
    pub power_mod_turn: i32,
    /// Whether this slot has attacked this turn.
    pub has_attacked: bool,
    /// Whether this slot is prevented from attacking.
    pub cannot_attack: bool,
    /// Additional stock cost required to declare an attack.
    pub attack_cost: u8,
}

impl StageSlot {
    /// Create an empty stage slot.
    pub fn empty() -> Self {
        Self {
            card: None,
            markers: Vec::new(),
            status: StageStatus::Stand,
            played_from_hand_this_turn: false,
            power_mod_battle: 0,
            power_mod_turn: 0,
            has_attacked: false,
            cannot_attack: false,
            attack_cost: 0,
        }
    }

    /// Whether the slot is empty.
    pub fn is_empty(&self) -> bool {
        self.card.is_none()
    }
}

/// Attack types available during the attack step.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AttackType {
    /// Frontal attack against an opposing character.
    Frontal,
    /// Side attack against an opposing character.
    Side,
    /// Direct attack (no opposing character).
    Direct,
}

/// Attack step sub-phase.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AttackStep {
    /// Trigger reveal/resolution step.
    Trigger,
    /// Counter timing step.
    Counter,
    /// Damage resolution step.
    Damage,
    /// Battle comparison step.
    Battle,
    /// Encore timing step.
    Encore,
}

/// Damage type classification.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DamageType {
    /// Damage caused by an attack.
    Battle,
    /// Damage caused by an effect.
    Effect,
}

/// Modifier categories for damage processing.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DamageModifierKind {
    /// Add `delta` to the damage amount.
    AddAmount {
        /// Signed delta to apply.
        delta: i32,
    },
    /// Set whether the damage is cancelable.
    SetCancelable {
        /// New cancelable flag.
        cancelable: bool,
    },
    /// Cancel the next damage instance.
    CancelNext,
    /// Set the damage amount to an absolute value.
    SetAmount {
        /// Absolute damage amount.
        amount: i32,
    },
}

/// Applied damage modifier instance.
#[derive(Clone, Debug, Hash, Serialize, Deserialize)]
pub struct DamageModifier {
    /// Modifier behavior.
    pub kind: DamageModifierKind,
    /// Ordering priority for application.
    pub priority: i16,
    /// Insertion order used as a tie-breaker.
    pub insertion: u32,
    /// Source identifier for debugging/auditing.
    pub source_id: u32,
    /// Remaining applications or magnitude budget (variant-dependent).
    pub remaining: i32,
    /// Whether this modifier has been applied at least once.
    pub used: bool,
}

/// Trigger effects resolved from trigger icons.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TriggerEffect {
    /// Add soul for the current attack.
    Soul,
    /// Draw a card.
    Draw,
    /// Deal shot damage.
    Shot,
    /// Return a character to hand.
    Bounce,
    /// Perform a choice selection.
    Choice,
    /// Add the revealed card to stock ("Pool").
    Pool,
    /// Add the revealed card to hand ("Treasure").
    Treasure,
    /// Salvage from waiting room ("Gate").
    Gate,
    /// Place a character from deck ("Standby").
    Standby,
    /// Resolve an auto ability on the trigger source card.
    AutoAbility {
        /// Index into the card's ability list.
        ability_index: u8,
    },
    /// Resolve an auto ability that was granted at runtime.
    GrantedAutoAbility {
        /// Stable grant identifier referencing a `GrantedAbilityInstance`.
        grant_id: u64,
    },
}

/// Zones that can be targeted by effects.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TargetZone {
    /// Stage (front/back row slots).
    Stage,
    /// Hand.
    Hand,
    /// Top of deck.
    DeckTop,
    /// Clock.
    Clock,
    /// Level zone.
    Level,
    /// Stock.
    Stock,
    /// Memory.
    Memory,
    /// Waiting room.
    WaitingRoom,
    /// Climax zone.
    Climax,
    /// Resolution zone (temporary).
    Resolution,
}

/// Side selection for targeting.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TargetSide {
    /// Current player / controller side.
    SelfSide,
    /// Opponent side.
    Opponent,
}

/// Slot filter for targeting.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TargetSlotFilter {
    /// Any slot (no restriction).
    Any,
    /// Front-row slots only.
    FrontRow,
    /// Back-row slots only.
    BackRow,
    /// A specific slot index.
    SpecificSlot(
        /// Slot index in `[0, 4]`.
        u8,
    ),
}

/// Targeting specification for effects.
#[derive(Clone, Debug, Hash, Serialize, Deserialize)]
pub struct TargetSpec {
    /// Target zone to search/select from.
    pub zone: TargetZone,
    /// Which side to target.
    pub side: TargetSide,
    /// Optional slot filter (primarily for stage targeting).
    pub slot_filter: TargetSlotFilter,
    /// Optional card type restriction.
    pub card_type: Option<crate::db::CardType>,
    /// Optional trait restriction (packed trait id).
    #[serde(default)]
    pub card_trait: Option<u16>,
    /// Optional inclusive maximum level restriction.
    pub level_max: Option<u8>,
    /// Optional inclusive maximum cost restriction.
    #[serde(default)]
    pub cost_max: Option<u8>,
    /// Optional card id whitelist restriction.
    #[serde(default)]
    pub card_ids: Vec<CardId>,
    /// Number of cards/targets to select.
    pub count: u8,
    /// Optional hard limit for search-like effects.
    #[serde(default)]
    pub limit: Option<u8>,
    /// If true, only the source card is eligible.
    #[serde(default)]
    pub source_only: bool,
    /// If true, reveal selected cards to the controller.
    #[serde(default)]
    pub reveal_to_controller: bool,
}

/// Concrete target reference.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TargetRef {
    /// Owning/located player seat (0 or 1).
    pub player: u8,
    /// Zone containing the card.
    pub zone: TargetZone,
    /// Index within the zone (slot or list position).
    pub index: u8,
    /// Static card id.
    pub card_id: CardId,
    /// Stable per-game card instance id.
    pub instance_id: CardInstanceId,
}

/// Pending target effect awaiting resolution.
#[derive(Clone, Debug, Hash, Serialize, Deserialize)]
pub enum PendingTargetEffect {
    /// Resolve an effect payload once targeting is complete.
    EffectPending {
        /// Effect instance id to associate with the payload.
        instance_id: u32,
        /// Effect payload to execute.
        payload: EffectPayload,
    },
}

/// State for a target-selection prompt.
#[derive(Clone, Debug, Hash, Serialize, Deserialize)]
pub struct TargetSelectionState {
    /// Controlling player making selections.
    pub controller: u8,
    /// Source card id producing the prompt.
    pub source_id: CardId,
    /// Targeting specification to satisfy.
    pub spec: TargetSpec,
    /// Remaining selections required.
    pub remaining: u8,
    /// Selected targets so far.
    pub selected: Vec<TargetRef>,
    /// Optional precomputed candidate list (for pagination/debugging).
    #[serde(default)]
    pub candidates: Vec<TargetRef>,
    /// Effect to apply once selection completes.
    pub effect: PendingTargetEffect,
    /// Whether the controller may skip instead of selecting.
    pub allow_skip: bool,
}

/// Item on the effect stack.
#[derive(Clone, Debug, Hash, Serialize, Deserialize)]
pub struct StackItem {
    /// Unique stack item id.
    pub id: u32,
    /// Controlling player for resolution ordering.
    pub controller: u8,
    /// Source card id that created the stack item.
    pub source_id: CardId,
    /// Compiled effect id for this item.
    pub effect_id: EffectId,
    /// Payload to resolve.
    pub payload: EffectPayload,
}

/// Priority window state.
#[derive(Clone, Debug, Hash, Serialize, Deserialize)]
pub struct PriorityState {
    /// Current priority holder seat.
    pub holder: u8,
    /// Consecutive pass count (for window termination).
    pub passes: u8,
    /// Active timing window.
    pub window: TimingWindow,
    /// Bitmask of already-used ACT abilities (by index).
    pub used_act_mask: u32,
}

/// Stack ordering state when multiple items are pending.
#[derive(Clone, Debug, Hash, Serialize, Deserialize)]
pub struct StackOrderState {
    /// Group identifier for the ordering prompt.
    pub group_id: u32,
    /// Controller making the ordering decision.
    pub controller: u8,
    /// Items to order.
    pub items: Vec<StackItem>,
}

/// Reasons for prompting a choice.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ChoiceReason {
    /// Choose a standby target from trigger effect.
    TriggerStandbySelect,
    /// Choose a treasure target from trigger effect.
    TriggerTreasureSelect,
    /// Choose whether to resolve a draw trigger (and/or its target).
    TriggerDrawSelect,
    /// Choose a target for a choice trigger.
    TriggerChoiceSelect,
    /// Choose targets for an auto ability cost step.
    TriggerAutoCostSelect,
    /// Choose whether to draw after a brainstorm effect.
    BrainstormDrawSelect,
    /// Choose an ordering for simultaneous stack items.
    StackOrderSelect,
    /// Choose an action during a priority window.
    PriorityActionSelect,
    /// Choose a payment option for a staged cost.
    CostPayment,
    /// Choose effect targets.
    TargetSelect,
    /// Choose cards to discard during end phase.
    EndPhaseDiscard,
}

/// Cost payment step kinds.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CostStepKind {
    /// Rest another character.
    RestOther,
    /// Sacrifice a character from stage.
    SacrificeFromStage,
    /// Discard a card from hand.
    DiscardFromHand,
    /// Clock a card from hand.
    ClockFromHand,
    /// Clock the top card(s) of the deck.
    ClockFromDeckTop,
    /// Reveal a card from hand.
    RevealFromHand,
}

/// Result to execute when staged cost payment finishes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum CostPaymentOutcome {
    /// Continue by resolving the originating ability.
    #[default]
    ResolveAbility,
    /// Keep a character for encore by selecting the specified slot.
    EncoreKeep {
        /// Stage slot index being kept for encore.
        slot: u8,
    },
}

/// State for a multi-step cost payment.
#[derive(Clone, Debug, Hash, Serialize, Deserialize)]
pub struct CostPaymentState {
    /// Controlling player paying the cost.
    pub controller: u8,
    /// Source card id of the ability being paid for.
    pub source_id: CardId,
    /// Source card instance id (stable per-game).
    pub source_instance_id: CardInstanceId,
    /// Optional source slot when the source is on stage.
    pub source_slot: Option<u8>,
    /// Ability index on the source card.
    pub ability_index: u8,
    /// Remaining cost to pay.
    pub remaining: crate::db::AbilityCost,
    /// Currently active staged step, if any.
    pub current_step: Option<CostStepKind>,
    /// Outcome to execute once the cost is fully paid.
    #[serde(default)]
    pub outcome: CostPaymentOutcome,
}

/// Zones that choices can draw from.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ChoiceZone {
    /// Waiting room.
    WaitingRoom,
    /// Stage.
    Stage,
    /// Hand.
    Hand,
    /// Top of deck.
    DeckTop,
    /// Clock.
    Clock,
    /// Level zone.
    Level,
    /// Stock.
    Stock,
    /// Memory.
    Memory,
    /// Climax zone.
    Climax,
    /// Resolution zone.
    Resolution,
    /// Effect stack.
    Stack,
    /// Priority window: counter action.
    PriorityCounter,
    /// Priority window: ACT action.
    PriorityAct,
    /// Priority window: pass action.
    PriorityPass,
    /// Skip / decline an optional choice.
    Skip,
}

/// Reference to a concrete choice option.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ChoiceOptionRef {
    /// Static card id for the referenced option.
    pub card_id: CardId,
    /// Stable per-game card instance id.
    pub instance_id: CardInstanceId,
    /// Zone the option is sourced from.
    pub zone: ChoiceZone,
    /// Optional index within the zone (for list-like zones).
    pub index: Option<u16>,
    /// Optional target slot index (for stage-based options).
    pub target_slot: Option<u8>,
}

/// Choice prompt state for a player.
#[derive(Clone, Debug, Hash, Serialize, Deserialize)]
pub struct ChoiceState {
    /// Unique choice id.
    pub id: u32,
    /// Reason this choice is being requested.
    pub reason: ChoiceReason,
    /// Player seat making the choice.
    pub player: u8,
    /// Candidate options in the current page.
    pub options: Vec<ChoiceOptionRef>,
    /// Total candidates available across all pages.
    pub total_candidates: u16,
    /// Start offset of the current page.
    pub page_start: u16,
    /// Optional trigger associated with this choice (when choosing trigger order/targets).
    pub pending_trigger: Option<PendingTrigger>,
}

/// Context for an ongoing attack.
#[derive(Clone, Debug, Hash, Serialize, Deserialize)]
pub struct AttackContext {
    /// Attacker stage slot index.
    pub attacker_slot: u8,
    /// Optional defender stage slot index (None for direct attacks).
    pub defender_slot: Option<u8>,
    /// Declared attack type.
    pub attack_type: AttackType,
    /// Trigger-check revealed card id (if any).
    pub trigger_card: Option<CardId>,
    /// Trigger-check revealed card instance id (if any).
    pub trigger_instance_id: Option<CardInstanceId>,
    /// Total trigger checks required.
    pub trigger_checks_total: u8,
    /// Trigger checks already resolved.
    pub trigger_checks_resolved: u8,
    /// Current damage amount.
    pub damage: i32,
    /// Whether counter is allowed.
    pub counter_allowed: bool,
    /// Whether a counter was played.
    pub counter_played: bool,
    /// Power added by counters.
    pub counter_power: i32,
    /// Active damage modifiers.
    pub damage_modifiers: Vec<DamageModifier>,
    /// Pending shot damage remaining to apply.
    pub pending_shot_damage: u8,
    /// Next id for damage modifier instances within this attack.
    pub next_modifier_id: u32,
    /// Last damage event id emitted for this attack (if any).
    pub last_damage_event_id: Option<u32>,
    /// Whether auto triggers were enqueued for this attack.
    pub auto_trigger_enqueued: bool,
    /// Whether auto damage effects were enqueued for this attack.
    pub auto_damage_enqueued: bool,
    /// Whether battle damage has been applied.
    pub battle_damage_applied: bool,
    /// Current sub-step within the attack.
    pub step: AttackStep,
    /// Whether the declaration timing window is complete.
    pub decl_window_done: bool,
    /// Whether the trigger timing window is complete.
    pub trigger_window_done: bool,
    /// Whether the damage timing window is complete.
    pub damage_window_done: bool,
}

/// Trigger pending resolution.
#[derive(Clone, Debug, Hash, Serialize, Deserialize)]
pub struct PendingTrigger {
    /// Unique trigger id.
    pub id: u32,
    /// Group id for simultaneous triggers.
    pub group_id: u32,
    /// Player seat that owns the trigger.
    pub player: u8,
    /// Source card id that produced the trigger.
    pub source_card: CardId,
    /// Trigger effect kind.
    pub effect: TriggerEffect,
    /// Optional effect id for auto/granted abilities.
    pub effect_id: Option<EffectId>,
}

/// Ordering state for multiple triggers.
#[derive(Clone, Debug, Hash, Serialize, Deserialize)]
pub struct TriggerOrderState {
    /// Group id for the set of triggers being ordered.
    pub group_id: u32,
    /// Player seat choosing the order.
    pub player: u8,
    /// Remaining trigger ids to choose from.
    pub choices: Vec<u32>,
}

/// Derived attack information for a single slot.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DerivedAttackSlot {
    /// Whether the slot is unable to declare any attack.
    pub cannot_attack: bool,
    /// Whether side attacks are disallowed.
    #[serde(default)]
    pub cannot_side_attack: bool,
    /// Whether frontal attacks are disallowed.
    #[serde(default)]
    pub cannot_frontal_attack: bool,
    /// Additional stock cost required to attack from this slot.
    pub attack_cost: u8,
}

impl DerivedAttackSlot {
    /// Create an empty derived attack slot.
    pub fn empty() -> Self {
        Self {
            cannot_attack: false,
            cannot_side_attack: false,
            cannot_frontal_attack: false,
            attack_cost: 0,
        }
    }
}

/// Derived attack state for a turn.
#[derive(Clone, Debug, Hash, Serialize, Deserialize)]
pub struct DerivedAttackState {
    /// Per-player derived slot info for each stage slot.
    pub per_player: [[DerivedAttackSlot; 5]; 2],
}

impl DerivedAttackState {
    /// Create a default derived attack state.
    pub fn new() -> Self {
        Self {
            per_player: [[DerivedAttackSlot::empty(); 5]; 2],
        }
    }
}

impl Default for DerivedAttackState {
    fn default() -> Self {
        Self::new()
    }
}

/// Encore request tracking for a character.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EncoreRequest {
    /// Player seat that owns the encore request.
    pub player: u8,
    /// Stage slot index of the character requesting encore.
    pub slot: u8,
}

/// Terminal outcome for an episode.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TerminalResult {
    /// A player won the episode.
    Win {
        /// Winning player seat (0 or 1).
        winner: u8,
    },
    /// Both players reached a draw condition.
    Draw,
    /// Episode ended due to reaching a tick/decision limit.
    Timeout,
}

/// Full per-player state.
#[derive(Clone, Debug, Hash)]
pub struct PlayerState {
    /// Deck (top at end of vector).
    pub deck: Vec<CardInstance>,
    /// Hand.
    pub hand: Vec<CardInstance>,
    /// Waiting room.
    pub waiting_room: Vec<CardInstance>,
    /// Clock.
    pub clock: Vec<CardInstance>,
    /// Level zone.
    pub level: Vec<CardInstance>,
    /// Stock (top at end of vector).
    pub stock: Vec<CardInstance>,
    /// Memory.
    pub memory: Vec<CardInstance>,
    /// Climax zone.
    pub climax: Vec<CardInstance>,
    /// Resolution zone (temporary).
    pub resolution: Vec<CardInstance>,
    /// Stage slots (5 total).
    pub stage: [StageSlot; 5],
}

impl PlayerState {
    /// Create a new player state with an initial deck.
    pub fn new(deck: Vec<CardInstance>) -> Self {
        Self {
            deck,
            hand: Vec::new(),
            waiting_room: Vec::new(),
            clock: Vec::new(),
            level: Vec::new(),
            stock: Vec::new(),
            memory: Vec::new(),
            climax: Vec::new(),
            resolution: Vec::new(),
            stage: [
                StageSlot::empty(),
                StageSlot::empty(),
                StageSlot::empty(),
                StageSlot::empty(),
                StageSlot::empty(),
            ],
        }
    }
}

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

/// Turn-level state tracking.
#[derive(Clone, Debug, Hash)]
pub struct TurnState {
    /// Seat whose turn it currently is.
    pub active_player: u8,
    /// Seat that started the game.
    pub starting_player: u8,
    /// Turn counter (starting at 0 for mulligan).
    pub turn_number: u32,
    /// Current phase within the turn.
    pub phase: Phase,
    /// Per-seat mulligan completion flags.
    pub mulligan_done: [bool; 2],
    /// Per-seat packed mulligan selections.
    pub mulligan_selected: [u64; 2],
    /// Whether the main phase has been passed.
    pub main_passed: bool,
    /// Decision counter for the episode.
    pub decision_count: u32,
    /// Tick counter for the episode.
    pub tick_count: u32,
    /// Active attack context, if any.
    pub attack: Option<AttackContext>,
    /// Counter used to advance attack subphases deterministically.
    pub attack_subphase_count: u8,
    /// Seat that must level up next, if any.
    pub pending_level_up: Option<u8>,
    /// Queue of encore requests to resolve.
    pub encore_queue: Vec<EncoreRequest>,
    /// Seat currently choosing encore, if any.
    pub encore_step_player: Option<u8>,
    /// Pending triggers awaiting ordering/resolution.
    pub pending_triggers: Vec<PendingTrigger>,
    /// Whether `pending_triggers` is already sorted for resolution.
    pub pending_triggers_sorted: bool,
    /// Current timing window for priority/auto effects.
    pub active_window: Option<TimingWindow>,
    /// Whether end-phase window has been processed.
    pub end_phase_window_done: bool,
    /// Whether end-phase discard has been processed.
    pub end_phase_discard_done: bool,
    /// Whether end-phase climax cleanup has been processed.
    pub end_phase_climax_done: bool,
    /// Whether end-phase general cleanup has been processed.
    pub end_phase_cleanup_done: bool,
    /// Whether encore window has been processed.
    pub encore_window_done: bool,
    /// Per-seat pending loss flags.
    pub pending_losses: [bool; 2],
    /// Seat currently taking damage, if any.
    pub damage_resolution_target: Option<u8>,
    /// Nested cost payment depth (re-entrancy guard).
    pub cost_payment_depth: u8,
    /// Pending cleanup operations for the resolution zone.
    pub pending_resolution_cleanup: Vec<(u8, CardInstanceId)>,
    /// Per-seat flag to disable auto-encore.
    pub cannot_use_auto_encore: [bool; 2],
    /// Active rule overrides for this turn.
    pub rule_overrides: Vec<crate::effects::RuleOverrideKind>,
    /// Runtime granted abilities active this turn.
    pub granted_abilities: Vec<GrantedAbilityInstance>,
    /// Next grant id allocator.
    pub next_grant_id: u64,
    /// Internal step counter within the current phase.
    pub phase_step: u8,
    /// Whether attack-phase begin effects have run.
    pub attack_phase_begin_done: bool,
    /// Whether attack declaration legality checks have run.
    pub attack_decl_check_done: bool,
    /// Whether encore begin effects have run.
    pub encore_begin_done: bool,
    /// Active trigger ordering prompt, if any.
    pub trigger_order: Option<TriggerOrderState>,
    /// Active choice prompt, if any.
    pub choice: Option<ChoiceState>,
    /// Active target selection prompt, if any.
    pub target_selection: Option<TargetSelectionState>,
    /// Active cost payment state, if any.
    pub pending_cost: Option<CostPaymentState>,
    /// Active priority window, if any.
    pub priority: Option<PriorityState>,
    /// Effect stack items awaiting resolution.
    pub stack: Vec<StackItem>,
    /// Pending stack groups awaiting an ordering decision.
    pub pending_stack_groups: VecDeque<StackOrderState>,
    /// Active stack ordering prompt, if any.
    pub stack_order: Option<StackOrderState>,
    /// Cached derived attack state, if any.
    pub derived_attack: Option<DerivedAttackState>,
    /// Next trigger id allocator.
    pub next_trigger_id: u32,
    /// Next trigger group id allocator.
    pub next_trigger_group_id: u32,
    /// Next choice id allocator.
    pub next_choice_id: u32,
    /// Next stack group id allocator.
    pub next_stack_group_id: u32,
    /// Next damage event id allocator.
    pub next_damage_event_id: u32,
    /// Next effect instance id allocator.
    pub next_effect_instance_id: u32,
    /// Whether the end phase is currently pending execution.
    pub end_phase_pending: bool,
}

/// Complete game state for an environment.
#[derive(Clone, Debug, Hash)]
pub struct GameState {
    /// Per-seat player state.
    pub players: [PlayerState; 2],
    /// Per-seat reveal history.
    pub reveal_history: [RevealHistory; 2],
    /// Turn-level state.
    pub turn: TurnState,
    /// Deterministic RNG state.
    pub rng: Rng64,
    /// Active modifier instances.
    pub modifiers: Vec<ModifierInstance>,
    /// Next modifier id allocator.
    pub next_modifier_id: u32,
    /// Active replacement specs.
    pub replacements: Vec<ReplacementSpec>,
    /// Insertion order counter for replacements.
    pub next_replacement_insertion: u32,
    /// Terminal result for the episode, if any.
    pub terminal: Option<TerminalResult>,
}

impl GameState {
    /// Build a new game state with the given decks and seed.
    pub fn new(
        deck_a: Vec<CardId>,
        deck_b: Vec<CardId>,
        seed: u64,
        starting_player: u8,
    ) -> Result<Self, StateError> {
        if starting_player > 1 {
            return Err(StateError::InvalidStartingPlayer {
                got: starting_player,
            });
        }
        if deck_a.len() != crate::encode::MAX_DECK {
            return Err(StateError::DeckLength {
                owner: 0,
                got: deck_a.len(),
                expected: crate::encode::MAX_DECK,
            });
        }
        if deck_b.len() != crate::encode::MAX_DECK {
            return Err(StateError::DeckLength {
                owner: 1,
                got: deck_b.len(),
                expected: crate::encode::MAX_DECK,
            });
        }
        let rng = Rng64::new(seed);
        let mut next_instance_id: CardInstanceId = 1;
        let deck_a = Self::build_deck(deck_a, 0, &mut next_instance_id);
        let deck_b = Self::build_deck(deck_b, 1, &mut next_instance_id);
        Ok(Self {
            players: [PlayerState::new(deck_a), PlayerState::new(deck_b)],
            reveal_history: [RevealHistory::new(), RevealHistory::new()],
            turn: TurnState {
                active_player: starting_player,
                starting_player,
                turn_number: 0,
                phase: Phase::Mulligan,
                mulligan_done: [false; 2],
                mulligan_selected: [0; 2],
                main_passed: false,
                decision_count: 0,
                tick_count: 0,
                attack: None,
                attack_subphase_count: 0,
                pending_level_up: None,
                encore_queue: Vec::new(),
                encore_step_player: None,
                pending_triggers: Vec::new(),
                pending_triggers_sorted: true,
                trigger_order: None,
                choice: None,
                target_selection: None,
                pending_cost: None,
                priority: None,
                stack: Vec::new(),
                pending_stack_groups: VecDeque::new(),
                stack_order: None,
                derived_attack: None,
                next_trigger_id: 1,
                next_trigger_group_id: 1,
                next_choice_id: 1,
                next_stack_group_id: 1,
                next_damage_event_id: 1,
                next_effect_instance_id: 1,
                active_window: None,
                end_phase_window_done: false,
                end_phase_discard_done: false,
                end_phase_climax_done: false,
                end_phase_cleanup_done: false,
                encore_window_done: false,
                pending_losses: [false; 2],
                damage_resolution_target: None,
                cost_payment_depth: 0,
                pending_resolution_cleanup: Vec::new(),
                cannot_use_auto_encore: [false; 2],
                rule_overrides: Vec::new(),
                granted_abilities: Vec::new(),
                next_grant_id: 1,
                phase_step: 0,
                attack_phase_begin_done: false,
                attack_decl_check_done: false,
                encore_begin_done: false,
                end_phase_pending: false,
            },
            rng,
            modifiers: Vec::new(),
            next_modifier_id: 1,
            replacements: Vec::new(),
            next_replacement_insertion: 1,
            terminal: None,
        })
    }

    /// Compatibility helper for test/bench scaffolding.
    pub fn new_or_panic(
        deck_a: Vec<CardId>,
        deck_b: Vec<CardId>,
        seed: u64,
        starting_player: u8,
    ) -> Self {
        Self::new(deck_a, deck_b, seed, starting_player).expect("GameState::new_or_panic failed")
    }

    fn build_deck(
        deck: Vec<CardId>,
        owner: u8,
        next_instance_id: &mut CardInstanceId,
    ) -> Vec<CardInstance> {
        deck.into_iter()
            .map(|id| {
                let instance_id = *next_instance_id;
                *next_instance_id = next_instance_id.wrapping_add(1);
                CardInstance::new(id, owner, instance_id)
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::{RevealHistory, REVEAL_HISTORY_LEN};

    #[test]
    fn reveal_history_chronology_before_wrap() {
        let mut history = RevealHistory::new();
        history.push(10);
        history.push(20);
        history.push(30);
        let mut out = [0i32; REVEAL_HISTORY_LEN];
        history.write_chronological(&mut out);
        assert_eq!(&out[..3], &[10, 20, 30]);
        assert!(out[3..].iter().all(|entry| *entry == 0));
    }

    #[test]
    fn reveal_history_chronology_after_wrap_keeps_latest_entries() {
        let mut history = RevealHistory::new();
        for card in 1..=(REVEAL_HISTORY_LEN as u32 + 3) {
            history.push(card);
        }
        let mut out = [0i32; REVEAL_HISTORY_LEN];
        history.write_chronological(&mut out);
        let expected: Vec<i32> = (4..=(REVEAL_HISTORY_LEN as i32 + 3)).collect();
        assert_eq!(out.as_slice(), expected.as_slice());
    }
}
