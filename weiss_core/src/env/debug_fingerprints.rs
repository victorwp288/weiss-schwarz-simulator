use crate::state::GameState;

pub(crate) fn state_fingerprint(state: &GameState) -> u64 {
    crate::fingerprint::state_fingerprint(state)
}
