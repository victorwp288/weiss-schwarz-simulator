use serde::{Deserialize, Serialize};

use super::EffectSpec;

/// Effect with resolved targets ready for execution.
#[derive(Clone, Debug, Hash, Serialize, Deserialize)]
pub struct EffectPayload {
    /// Underlying effect specification.
    pub spec: EffectSpec,
    /// Resolved targets for this effect.
    pub targets: Vec<crate::state::TargetRef>,
    /// Source reference for source-relative effects.
    #[serde(default)]
    pub source_ref: Option<crate::state::TargetRef>,
}
