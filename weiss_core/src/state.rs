#![allow(missing_docs)]

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
    pub id: CardId,
    pub instance_id: CardInstanceId,
    pub owner: u8,
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
        if REVEAL_HISTORY_LEN == 0 {
            return;
        }
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
        if len == 0 || REVEAL_HISTORY_LEN == 0 {
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
    Mulligan,
    Stand,
    Draw,
    Clock,
    Main,
    Climax,
    Attack,
    End,
}

/// Timing window for triggered effects.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TimingWindow {
    MainWindow,
    ClimaxWindow,
    AttackDeclarationWindow,
    TriggerResolutionWindow,
    CounterWindow,
    DamageResolutionWindow,
    EncoreWindow,
    EndPhaseWindow,
}

/// Stage slot status.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum StageStatus {
    Stand,
    Rest,
    Reverse,
}

/// Stage slot containing a character or empty.
#[derive(Clone, Debug, Hash, Serialize, Deserialize)]
pub struct StageSlot {
    pub card: Option<CardInstance>,
    #[serde(default)]
    pub markers: Vec<CardInstance>,
    pub status: StageStatus,
    #[serde(default)]
    pub played_from_hand_this_turn: bool,
    pub power_mod_battle: i32,
    pub power_mod_turn: i32,
    pub has_attacked: bool,
    pub cannot_attack: bool,
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
    Frontal,
    Side,
    Direct,
}

/// Attack step sub-phase.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AttackStep {
    Trigger,
    Counter,
    Damage,
    Battle,
    Encore,
}

/// Damage type classification.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DamageType {
    Battle,
    Effect,
}

/// Modifier categories for damage processing.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DamageModifierKind {
    AddAmount { delta: i32 },
    SetCancelable { cancelable: bool },
    CancelNext,
    SetAmount { amount: i32 },
}

/// Applied damage modifier instance.
#[derive(Clone, Debug, Hash, Serialize, Deserialize)]
pub struct DamageModifier {
    pub kind: DamageModifierKind,
    pub priority: i16,
    pub insertion: u32,
    pub source_id: u32,
    pub remaining: i32,
    pub used: bool,
}

/// Trigger effects resolved from trigger icons.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TriggerEffect {
    Soul,
    Draw,
    Shot,
    Bounce,
    Choice,
    Pool,
    Treasure,
    Gate,
    Standby,
    AutoAbility { ability_index: u8 },
    GrantedAutoAbility { grant_id: u64 },
}

/// Zones that can be targeted by effects.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TargetZone {
    Stage,
    Hand,
    DeckTop,
    Clock,
    Level,
    Stock,
    Memory,
    WaitingRoom,
    Climax,
    Resolution,
}

/// Side selection for targeting.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TargetSide {
    SelfSide,
    Opponent,
}

/// Slot filter for targeting.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TargetSlotFilter {
    Any,
    FrontRow,
    BackRow,
    SpecificSlot(u8),
}

/// Targeting specification for effects.
#[derive(Clone, Debug, Hash, Serialize, Deserialize)]
pub struct TargetSpec {
    pub zone: TargetZone,
    pub side: TargetSide,
    pub slot_filter: TargetSlotFilter,
    pub card_type: Option<crate::db::CardType>,
    #[serde(default)]
    pub card_trait: Option<u16>,
    pub level_max: Option<u8>,
    #[serde(default)]
    pub cost_max: Option<u8>,
    #[serde(default)]
    pub card_ids: Vec<CardId>,
    pub count: u8,
    #[serde(default)]
    pub limit: Option<u8>,
    #[serde(default)]
    pub source_only: bool,
    #[serde(default)]
    pub reveal_to_controller: bool,
}

/// Concrete target reference.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TargetRef {
    pub player: u8,
    pub zone: TargetZone,
    pub index: u8,
    pub card_id: CardId,
    pub instance_id: CardInstanceId,
}

/// Pending target effect awaiting resolution.
#[derive(Clone, Debug, Hash, Serialize, Deserialize)]
pub enum PendingTargetEffect {
    EffectPending {
        instance_id: u32,
        payload: EffectPayload,
    },
}

/// State for a target-selection prompt.
#[derive(Clone, Debug, Hash, Serialize, Deserialize)]
pub struct TargetSelectionState {
    pub controller: u8,
    pub source_id: CardId,
    pub spec: TargetSpec,
    pub remaining: u8,
    pub selected: Vec<TargetRef>,
    #[serde(default)]
    pub candidates: Vec<TargetRef>,
    pub effect: PendingTargetEffect,
    pub allow_skip: bool,
}

/// Item on the effect stack.
#[derive(Clone, Debug, Hash, Serialize, Deserialize)]
pub struct StackItem {
    pub id: u32,
    pub controller: u8,
    pub source_id: CardId,
    pub effect_id: EffectId,
    pub payload: EffectPayload,
}

/// Priority window state.
#[derive(Clone, Debug, Hash, Serialize, Deserialize)]
pub struct PriorityState {
    pub holder: u8,
    pub passes: u8,
    pub window: TimingWindow,
    pub used_act_mask: u32,
}

/// Stack ordering state when multiple items are pending.
#[derive(Clone, Debug, Hash, Serialize, Deserialize)]
pub struct StackOrderState {
    pub group_id: u32,
    pub controller: u8,
    pub items: Vec<StackItem>,
}

/// Reasons for prompting a choice.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ChoiceReason {
    TriggerStandbySelect,
    TriggerTreasureSelect,
    TriggerDrawSelect,
    TriggerChoiceSelect,
    TriggerAutoCostSelect,
    BrainstormDrawSelect,
    StackOrderSelect,
    PriorityActionSelect,
    CostPayment,
    TargetSelect,
    EndPhaseDiscard,
}

/// Cost payment step kinds.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CostStepKind {
    RestOther,
    SacrificeFromStage,
    DiscardFromHand,
    ClockFromHand,
    ClockFromDeckTop,
    RevealFromHand,
}

/// Result to execute when staged cost payment finishes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum CostPaymentOutcome {
    #[default]
    ResolveAbility,
    EncoreKeep {
        slot: u8,
    },
}

/// State for a multi-step cost payment.
#[derive(Clone, Debug, Hash, Serialize, Deserialize)]
pub struct CostPaymentState {
    pub controller: u8,
    pub source_id: CardId,
    pub source_instance_id: CardInstanceId,
    pub source_slot: Option<u8>,
    pub ability_index: u8,
    pub remaining: crate::db::AbilityCost,
    pub current_step: Option<CostStepKind>,
    #[serde(default)]
    pub outcome: CostPaymentOutcome,
}

/// Zones that choices can draw from.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ChoiceZone {
    WaitingRoom,
    Stage,
    Hand,
    DeckTop,
    Clock,
    Level,
    Stock,
    Memory,
    Climax,
    Resolution,
    Stack,
    PriorityCounter,
    PriorityAct,
    PriorityPass,
    Skip,
}

/// Reference to a concrete choice option.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ChoiceOptionRef {
    pub card_id: CardId,
    pub instance_id: CardInstanceId,
    pub zone: ChoiceZone,
    pub index: Option<u16>,
    pub target_slot: Option<u8>,
}

/// Choice prompt state for a player.
#[derive(Clone, Debug, Hash, Serialize, Deserialize)]
pub struct ChoiceState {
    pub id: u32,
    pub reason: ChoiceReason,
    pub player: u8,
    pub options: Vec<ChoiceOptionRef>,
    pub total_candidates: u16,
    pub page_start: u16,
    pub pending_trigger: Option<PendingTrigger>,
}

/// Context for an ongoing attack.
#[derive(Clone, Debug, Hash, Serialize, Deserialize)]
pub struct AttackContext {
    pub attacker_slot: u8,
    pub defender_slot: Option<u8>,
    pub attack_type: AttackType,
    pub trigger_card: Option<CardId>,
    pub trigger_instance_id: Option<CardInstanceId>,
    pub trigger_checks_total: u8,
    pub trigger_checks_resolved: u8,
    pub damage: i32,
    pub counter_allowed: bool,
    pub counter_played: bool,
    pub counter_power: i32,
    pub damage_modifiers: Vec<DamageModifier>,
    pub pending_shot_damage: u8,
    pub next_modifier_id: u32,
    pub last_damage_event_id: Option<u32>,
    pub auto_trigger_enqueued: bool,
    pub auto_damage_enqueued: bool,
    pub battle_damage_applied: bool,
    pub step: AttackStep,
    pub decl_window_done: bool,
    pub trigger_window_done: bool,
    pub damage_window_done: bool,
}

/// Trigger pending resolution.
#[derive(Clone, Debug, Hash, Serialize, Deserialize)]
pub struct PendingTrigger {
    pub id: u32,
    pub group_id: u32,
    pub player: u8,
    pub source_card: CardId,
    pub effect: TriggerEffect,
    pub effect_id: Option<EffectId>,
}

/// Ordering state for multiple triggers.
#[derive(Clone, Debug, Hash, Serialize, Deserialize)]
pub struct TriggerOrderState {
    pub group_id: u32,
    pub player: u8,
    pub choices: Vec<u32>,
}

/// Derived attack information for a single slot.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DerivedAttackSlot {
    pub cannot_attack: bool,
    #[serde(default)]
    pub cannot_side_attack: bool,
    #[serde(default)]
    pub cannot_frontal_attack: bool,
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
    pub player: u8,
    pub slot: u8,
}

/// Terminal outcome for an episode.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TerminalResult {
    Win { winner: u8 },
    Draw,
    Timeout,
}

/// Full per-player state.
#[derive(Clone, Debug, Hash)]
pub struct PlayerState {
    pub deck: Vec<CardInstance>,
    pub hand: Vec<CardInstance>,
    pub waiting_room: Vec<CardInstance>,
    pub clock: Vec<CardInstance>,
    pub level: Vec<CardInstance>,
    pub stock: Vec<CardInstance>,
    pub memory: Vec<CardInstance>,
    pub climax: Vec<CardInstance>,
    pub resolution: Vec<CardInstance>,
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
    Power,
    Soul,
    Level,
    AttackCost,
    CannotAttack,
    CannotSideAttack,
    CannotFrontalAttack,
    CannotBecomeReverse,
    CannotBeChosenByOpponentEffects,
    CannotMoveStagePosition,
    CannotPlayEventsFromHand,
    CannotPlayBackupFromHand,
    CannotStandDuringStandPhase,
    BattleOpponentMoveToMemoryOnReverse,
    EncoreStockCost,
}

/// Modifier duration semantics.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ModifierDuration {
    UntilEndOfTurn,
    WhileOnStage,
}

/// Modifier layer for ordering purposes.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ModifierLayer {
    Continuous,
    #[default]
    Effect,
}

/// Concrete modifier instance applied to a target.
#[derive(Clone, Debug, Hash, Serialize, Deserialize)]
pub struct ModifierInstance {
    pub id: u32,
    pub source: CardId,
    #[serde(default)]
    pub source_slot: Option<u8>,
    pub target_player: u8,
    pub target_slot: u8,
    pub target_card: CardId,
    pub kind: ModifierKind,
    pub magnitude: i32,
    pub duration: ModifierDuration,
    #[serde(default)]
    pub layer: ModifierLayer,
    pub insertion: u32,
}

/// Runtime granted ability attached to a specific stage card instance.
#[derive(Clone, Debug, Hash, Serialize, Deserialize)]
pub struct GrantedAbilityInstance {
    pub grant_id: u64,
    pub target_player: u8,
    pub target_slot: u8,
    pub target_instance_id: CardInstanceId,
    pub spec: AbilitySpec,
    pub compiled_effects: Vec<EffectSpec>,
    /// Turn number at which this grant expires during end-phase cleanup.
    pub expires_turn_number: u32,
}

/// Turn-level state tracking.
#[derive(Clone, Debug, Hash)]
pub struct TurnState {
    pub active_player: u8,
    pub starting_player: u8,
    pub turn_number: u32,
    pub phase: Phase,
    pub mulligan_done: [bool; 2],
    pub mulligan_selected: [u64; 2],
    pub main_passed: bool,
    pub decision_count: u32,
    pub tick_count: u32,
    pub attack: Option<AttackContext>,
    pub attack_subphase_count: u8,
    pub pending_level_up: Option<u8>,
    pub encore_queue: Vec<EncoreRequest>,
    pub encore_step_player: Option<u8>,
    pub pending_triggers: Vec<PendingTrigger>,
    pub pending_triggers_sorted: bool,
    pub active_window: Option<TimingWindow>,
    pub end_phase_window_done: bool,
    pub end_phase_discard_done: bool,
    pub end_phase_climax_done: bool,
    pub end_phase_cleanup_done: bool,
    pub encore_window_done: bool,
    pub pending_losses: [bool; 2],
    pub damage_resolution_target: Option<u8>,
    pub cost_payment_depth: u8,
    pub pending_resolution_cleanup: Vec<(u8, CardInstanceId)>,
    pub cannot_use_auto_encore: [bool; 2],
    pub granted_abilities: Vec<GrantedAbilityInstance>,
    pub next_grant_id: u64,
    pub phase_step: u8,
    pub attack_phase_begin_done: bool,
    pub attack_decl_check_done: bool,
    pub encore_begin_done: bool,
    pub trigger_order: Option<TriggerOrderState>,
    pub choice: Option<ChoiceState>,
    pub target_selection: Option<TargetSelectionState>,
    pub pending_cost: Option<CostPaymentState>,
    pub priority: Option<PriorityState>,
    pub stack: Vec<StackItem>,
    pub pending_stack_groups: VecDeque<StackOrderState>,
    pub stack_order: Option<StackOrderState>,
    pub derived_attack: Option<DerivedAttackState>,
    pub next_trigger_id: u32,
    pub next_trigger_group_id: u32,
    pub next_choice_id: u32,
    pub next_stack_group_id: u32,
    pub next_damage_event_id: u32,
    pub next_effect_instance_id: u32,
    pub end_phase_pending: bool,
}

/// Complete game state for an environment.
#[derive(Clone, Debug, Hash)]
pub struct GameState {
    pub players: [PlayerState; 2],
    pub reveal_history: [RevealHistory; 2],
    pub turn: TurnState,
    pub rng: Rng64,
    pub modifiers: Vec<ModifierInstance>,
    pub next_modifier_id: u32,
    pub replacements: Vec<ReplacementSpec>,
    pub next_replacement_insertion: u32,
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
