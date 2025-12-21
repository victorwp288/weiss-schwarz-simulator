use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EffectQueue {
    pub pending: Vec<Effect>,
}

impl EffectQueue {
    pub fn new() -> Self {
        Self { pending: Vec::new() }
    }

    pub fn is_empty(&self) -> bool {
        self.pending.is_empty()
    }
}

impl Default for EffectQueue {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Effect {
    Placeholder,
}
