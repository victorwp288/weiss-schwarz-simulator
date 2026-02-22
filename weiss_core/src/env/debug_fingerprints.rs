use crate::state::GameState;

/// Convenience wrapper for debug-state fingerprinting.
pub(crate) fn state_fingerprint(state: &GameState) -> u64 {
    crate::fingerprint::state_fingerprint(state)
}
