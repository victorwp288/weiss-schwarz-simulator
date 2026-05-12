//! Canonical action descriptors and legality helpers.
//!
//! The public `weiss_core::legal::*` surface is preserved through re-exports
//! while implementation details are grouped by legality concern.

mod attack;
mod descriptors;
pub(crate) mod hand_play_requirements;
mod helpers;
mod ids;
mod types;

const MAX_HAND: usize = crate::encode::MAX_HAND;
const MAX_STAGE: usize = 5;

pub use attack::{can_declare_attack, legal_attack_actions, legal_attack_actions_into};
pub use descriptors::{legal_actions, legal_actions_cached, legal_actions_cached_into};
pub use ids::legal_action_ids_cached_into;
pub use types::{ActionDesc, Decision, DecisionKind, LegalActionIds, LegalActions};
