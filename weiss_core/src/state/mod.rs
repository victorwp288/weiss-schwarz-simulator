//! Serializable game-state data and domain concepts.
//!
//! This module keeps the public `weiss_core::state::*` surface stable while the
//! internal files are grouped by simulator concepts.

mod attack;
mod cards;
mod choice;
mod game;
mod modifiers;
mod player;
mod reveal;
mod stack;
mod stage;
mod target;
mod turn;

pub use attack::{
    AttackContext, AttackStep, AttackType, DamageModifier, DamageModifierKind, DamageType,
    DerivedAttackSlot, DerivedAttackState, EncoreRequest, PendingTrigger, TriggerEffect,
    TriggerOrderState,
};
pub use cards::{CardInstance, CardInstanceId};
pub use choice::{
    ChoiceOptionRef, ChoiceReason, ChoiceState, ChoiceZone, CostPaymentOutcome, CostPaymentState,
    CostStepKind,
};
pub use game::{GameState, TerminalResult, TurnState};
pub use modifiers::{
    GrantedAbilityInstance, ModifierDuration, ModifierInstance, ModifierKind, ModifierLayer,
};
pub use player::PlayerState;
pub use reveal::{RevealHistory, REVEAL_HISTORY_LEN};
pub use stack::{PriorityState, StackItem, StackOrderState};
pub use stage::{StageSlot, StageStatus};
pub use target::{
    PendingTargetEffect, TargetRef, TargetSelectionState, TargetSide, TargetSlotFilter, TargetSpec,
    TargetZone,
};
pub use turn::{Phase, TimingWindow};
