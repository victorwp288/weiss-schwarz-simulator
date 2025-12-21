use crate::db::CardId;
use crate::util::Rng64;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
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
pub enum StageStatus {
    Stand,
    Rest,
    Reverse,
}

#[derive(Clone, Debug, Hash, Serialize, Deserialize)]
pub struct StageSlot {
    pub card: Option<CardId>,
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
    EndPhaseDraw { count: u8 },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ChoiceReason {
    TriggerGateSelect,
    TriggerBounceSelect,
    TriggerStandbySelect,
    TriggerTreasureSelect,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ChoiceZone {
    WaitingRoom,
    Stage,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ChoiceOptionRef {
    pub card_id: CardId,
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
    pub pending_trigger: Option<PendingTrigger>,
}

#[derive(Clone, Debug, Hash, Serialize, Deserialize)]
pub struct AttackContext {
    pub attacker_slot: u8,
    pub defender_slot: Option<u8>,
    pub attack_type: AttackType,
    pub trigger_card: Option<CardId>,
    pub damage: i32,
    pub counter_allowed: bool,
    pub counter_power: i32,
    pub damage_modifiers: Vec<DamageModifier>,
    pub next_modifier_id: u32,
    pub last_damage_event_id: Option<u32>,
    pub step: AttackStep,
}

#[derive(Clone, Debug, Hash, Serialize, Deserialize)]
pub struct PendingTrigger {
    pub id: u32,
    pub group_id: u32,
    pub player: u8,
    pub source_card: CardId,
    pub effect: TriggerEffect,
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
        Self { cannot_attack: false, attack_cost: 0 }
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
    pub deck: Vec<CardId>,
    pub hand: Vec<CardId>,
    pub waiting_room: Vec<CardId>,
    pub clock: Vec<CardId>,
    pub level: Vec<CardId>,
    pub stock: Vec<CardId>,
    pub memory: Vec<CardId>,
    pub climax: Vec<CardId>,
    pub stage: [StageSlot; 5],
}

impl PlayerState {
    pub fn new(deck: Vec<CardId>) -> Self {
        Self {
            deck,
            hand: Vec::new(),
            waiting_room: Vec::new(),
            clock: Vec::new(),
            level: Vec::new(),
            stock: Vec::new(),
            memory: Vec::new(),
            climax: Vec::new(),
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

#[derive(Clone, Debug, Hash, Serialize, Deserialize)]
pub struct ModifierInstance {
    pub id: u32,
    pub source: CardId,
    pub target_player: u8,
    pub target_slot: u8,
    pub target_card: CardId,
    pub kind: ModifierKind,
    pub magnitude: i32,
    pub duration: ModifierDuration,
    pub insertion: u32,
}

#[derive(Clone, Debug, Hash)]
pub struct TurnState {
    pub active_player: u8,
    pub starting_player: u8,
    pub phase: Phase,
    pub mulligan_done: [bool; 2],
    pub decision_count: u32,
    pub tick_count: u32,
    pub attack: Option<AttackContext>,
    pub pending_level_up: Option<u8>,
    pub encore_queue: Vec<EncoreRequest>,
    pub pending_triggers: Vec<PendingTrigger>,
    pub trigger_order: Option<TriggerOrderState>,
    pub choice: Option<ChoiceState>,
    pub derived_attack: Option<DerivedAttackState>,
    pub next_trigger_id: u32,
    pub next_trigger_group_id: u32,
    pub next_choice_id: u32,
    pub next_damage_event_id: u32,
    pub end_phase_pending: bool,
}

#[derive(Clone, Debug, Hash)]
pub struct GameState {
    pub players: [PlayerState; 2],
    pub turn: TurnState,
    pub rng: Rng64,
    pub modifiers: Vec<ModifierInstance>,
    pub next_modifier_id: u32,
    pub terminal: Option<TerminalResult>,
}

impl GameState {
    pub fn new(deck_a: Vec<CardId>, deck_b: Vec<CardId>, seed: u64, starting_player: u8) -> Self {
        let rng = Rng64::new(seed);
        Self {
            players: [PlayerState::new(deck_a), PlayerState::new(deck_b)],
            turn: TurnState {
                active_player: starting_player,
                starting_player,
                phase: Phase::Mulligan,
                mulligan_done: [false; 2],
                decision_count: 0,
                tick_count: 0,
                attack: None,
                pending_level_up: None,
                encore_queue: Vec::new(),
                pending_triggers: Vec::new(),
                trigger_order: None,
                choice: None,
                derived_attack: None,
                next_trigger_id: 1,
                next_trigger_group_id: 1,
                next_choice_id: 1,
                next_damage_event_id: 1,
                end_phase_pending: false,
            },
            rng,
            modifiers: Vec::new(),
            next_modifier_id: 1,
            terminal: None,
        }
    }
}
