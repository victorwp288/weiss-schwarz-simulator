use std::collections::VecDeque;

use serde::{Deserialize, Serialize};

use crate::db::CardId;
use crate::effects::ReplacementSpec;
use crate::error::StateError;
use crate::util::Rng64;

use super::{
    AttackContext, CardInstance, CardInstanceId, ChoiceState, CostPaymentState, DerivedAttackState,
    EncoreRequest, GrantedAbilityInstance, ModifierInstance, PendingTrigger, Phase, PlayerState,
    PriorityState, RevealHistory, StackItem, StackOrderState, TargetSelectionState, TimingWindow,
    TriggerOrderState,
};

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
    /// Whether a main-phase move has already been used this turn.
    pub main_move_used: bool,
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
                main_move_used: false,
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
