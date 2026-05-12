use serde::{Deserialize, Serialize};

use crate::db::CardId;
use crate::effects::EffectId;

use super::CardInstanceId;

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
