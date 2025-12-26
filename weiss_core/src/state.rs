use crate::db::CardId;
use crate::effects::{EffectId, EffectPayload, ReplacementSpec};
use crate::util::Rng64;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

pub type CardInstanceId = u32;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CardInstance {
    pub id: CardId,
    pub instance_id: CardInstanceId,
    pub owner: u8,
    pub controller: u8,
}

pub const REVEAL_HISTORY_LEN: usize = 8;

#[derive(Clone, Debug, Hash, Serialize, Deserialize)]
pub struct RevealHistory {
    entries: [CardId; REVEAL_HISTORY_LEN],
    len: u8,
    head: u8,
}

impl RevealHistory {
    pub fn new() -> Self {
        Self {
            entries: [0; REVEAL_HISTORY_LEN],
            len: 0,
            head: 0,
        }
    }

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
    pub fn new(id: CardId, owner: u8, instance_id: CardInstanceId) -> Self {
        Self {
            id,
            instance_id,
            owner,
            controller: owner,
        }
    }
}

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

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum StageStatus {
    Stand,
    Rest,
    Reverse,
}

#[derive(Clone, Debug, Hash, Serialize, Deserialize)]
pub struct StageSlot {
    pub card: Option<CardInstance>,
    pub status: StageStatus,
    pub power_mod_battle: i32,
    pub power_mod_turn: i32,
    pub has_attacked: bool,
    pub cannot_attack: bool,
    pub attack_cost: u8,
}

impl StageSlot {
    pub fn empty() -> Self {
        Self {
            card: None,
            status: StageStatus::Stand,
            power_mod_battle: 0,
            power_mod_turn: 0,
            has_attacked: false,
            cannot_attack: false,
            attack_cost: 0,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.card.is_none()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AttackType {
    Frontal,
    Side,
    Direct,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AttackStep {
    Trigger,
    Counter,
    Damage,
    Battle,
    Encore,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DamageType {
    Battle,
    Effect,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DamageModifierKind {
    AddAmount { delta: i32 },
    SetCancelable { cancelable: bool },
    CancelNext,
    SetAmount { amount: i32 },
}

#[derive(Clone, Debug, Hash, Serialize, Deserialize)]
pub struct DamageModifier {
    pub kind: DamageModifierKind,
    pub priority: i16,
    pub insertion: u32,
    pub source_id: u32,
    pub remaining: i32,
    pub used: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TriggerEffect {
    Soul,
    Draw,
    Shot,
    Bounce,
    Treasure,
    Gate,
    Standby,
    AutoAbility { ability_index: u8 },
}

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

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TargetSide {
    SelfSide,
    Opponent,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TargetSlotFilter {
    Any,
    FrontRow,
    BackRow,
    SpecificSlot(u8),
}

#[derive(Clone, Debug, Hash, Serialize, Deserialize)]
pub struct TargetSpec {
    pub zone: TargetZone,
    pub side: TargetSide,
    pub slot_filter: TargetSlotFilter,
    pub card_type: Option<crate::db::CardType>,
    #[serde(default)]
    pub card_trait: Option<u16>,
    #[serde(default)]
    pub level_max: Option<u8>,
    #[serde(default)]
    pub cost_max: Option<u8>,
    pub count: u8,
    #[serde(default)]
    pub limit: Option<u8>,
    #[serde(default)]
    pub source_only: bool,
    #[serde(default)]
    pub reveal_to_controller: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TargetRef {
    pub player: u8,
    pub zone: TargetZone,
    pub index: u8,
    pub card_id: CardId,
    pub instance_id: CardInstanceId,
}

#[derive(Clone, Debug, Hash, Serialize, Deserialize)]
pub enum PendingTargetEffect {
    EffectPending {
        instance_id: u32,
        payload: EffectPayload,
    },
}

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

#[derive(Clone, Debug, Hash, Serialize, Deserialize)]
pub struct StackItem {
    pub id: u32,
    pub controller: u8,
    pub source_id: CardId,
    pub effect_id: EffectId,
    pub payload: EffectPayload,
}

#[derive(Clone, Debug, Hash, Serialize, Deserialize)]
pub struct PriorityState {
    pub holder: u8,
    pub passes: u8,
    pub window: TimingWindow,
    pub used_act_mask: u32,
}

#[derive(Clone, Debug, Hash, Serialize, Deserialize)]
pub struct StackOrderState {
    pub group_id: u32,
    pub controller: u8,
    pub items: Vec<StackItem>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ChoiceReason {
    TriggerStandbySelect,
    TriggerTreasureSelect,
    StackOrderSelect,
    PriorityActionSelect,
    CostPayment,
    TargetSelect,
    EndPhaseDiscard,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CostStepKind {
    RestOther,
    DiscardFromHand,
    ClockFromHand,
    ClockFromDeckTop,
    RevealFromHand,
}

#[derive(Clone, Debug, Hash, Serialize, Deserialize)]
pub struct CostPaymentState {
    pub controller: u8,
    pub source_id: CardId,
    pub source_instance_id: CardInstanceId,
    pub source_slot: Option<u8>,
    pub ability_index: u8,
    pub remaining: crate::db::AbilityCost,
    pub current_step: Option<CostStepKind>,
}

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

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ChoiceOptionRef {
    pub card_id: CardId,
    pub instance_id: CardInstanceId,
    pub zone: ChoiceZone,
    pub index: Option<u8>,
    pub target_slot: Option<u8>,
}

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

#[derive(Clone, Debug, Hash, Serialize, Deserialize)]
pub struct AttackContext {
    pub attacker_slot: u8,
    pub defender_slot: Option<u8>,
    pub attack_type: AttackType,
    pub trigger_card: Option<CardId>,
    pub trigger_instance_id: Option<CardInstanceId>,
    pub damage: i32,
    pub counter_allowed: bool,
    pub counter_played: bool,
    pub counter_power: i32,
    pub damage_modifiers: Vec<DamageModifier>,
    pub next_modifier_id: u32,
    pub last_damage_event_id: Option<u32>,
    pub auto_damage_enqueued: bool,
    pub battle_damage_applied: bool,
    pub step: AttackStep,
    pub decl_window_done: bool,
    pub trigger_window_done: bool,
    pub damage_window_done: bool,
}

#[derive(Clone, Debug, Hash, Serialize, Deserialize)]
pub struct PendingTrigger {
    pub id: u32,
    pub group_id: u32,
    pub player: u8,
    pub source_card: CardId,
    pub effect: TriggerEffect,
    pub effect_id: Option<EffectId>,
}

#[derive(Clone, Debug, Hash, Serialize, Deserialize)]
pub struct TriggerOrderState {
    pub group_id: u32,
    pub player: u8,
    pub choices: Vec<u32>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DerivedAttackSlot {
    pub cannot_attack: bool,
    pub attack_cost: u8,
}

impl DerivedAttackSlot {
    pub fn empty() -> Self {
        Self {
            cannot_attack: false,
            attack_cost: 0,
        }
    }
}

#[derive(Clone, Debug, Hash, Serialize, Deserialize)]
pub struct DerivedAttackState {
    pub per_player: [[DerivedAttackSlot; 5]; 2],
}

impl DerivedAttackState {
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

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EncoreRequest {
    pub player: u8,
    pub slot: u8,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TerminalResult {
    Win { winner: u8 },
    Draw,
    Timeout,
}

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

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ModifierKind {
    Power,
    AttackCost,
    CannotAttack,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ModifierDuration {
    UntilEndOfTurn,
    WhileOnStage,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ModifierLayer {
    Continuous,
    #[default]
    Effect,
}

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
    pub fn new(deck_a: Vec<CardId>, deck_b: Vec<CardId>, seed: u64, starting_player: u8) -> Self {
        assert!(
            deck_a.len() == crate::encode::MAX_DECK,
            "Deck A must contain exactly {} cards",
            crate::encode::MAX_DECK
        );
        assert!(
            deck_b.len() == crate::encode::MAX_DECK,
            "Deck B must contain exactly {} cards",
            crate::encode::MAX_DECK
        );
        let rng = Rng64::new(seed);
        let mut next_instance_id: CardInstanceId = 1;
        let deck_a = Self::build_deck(deck_a, 0, &mut next_instance_id);
        let deck_b = Self::build_deck(deck_b, 1, &mut next_instance_id);
        Self {
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
        }
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
