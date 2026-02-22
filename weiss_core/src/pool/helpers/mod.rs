use super::core::EnvPool;

mod fill_minimal;
mod fill_trajectory;
mod fingerprint;
mod legal_sampling;
mod masks;
mod unsafe_bytes;
mod validate;

impl EnvPool {
    /// Debug event ring capacity configured for the pool.
    pub fn debug_event_ring_capacity(&self) -> usize {
        self.debug_config.event_ring_capacity
    }

    /// Render a single env to an ANSI string for debugging.
    pub fn render_ansi(&self, env_index: usize, perspective: u8) -> String {
        if env_index >= self.envs.len() {
            return "Invalid env index".to_string();
        }
        if perspective > 1 {
            return "Invalid perspective".to_string();
        }
        let env = &self.envs[env_index];
        let p0 = perspective as usize;
        let p1 = 1 - p0;
        let state = &env.state;
        let mut out = String::new();
        out.push_str(&format!("Phase: {:?}\n", state.turn.phase));
        out.push_str(&format!("Active: {}\n", state.turn.active_player));
        out.push_str(&format!(
            "P{} Level: {} Clock: {} Hand: {} Deck: {}\n",
            p0,
            state.players[p0].level.len(),
            state.players[p0].clock.len(),
            state.players[p0].hand.len(),
            state.players[p0].deck.len()
        ));
        out.push_str(&format!(
            "P{} Level: {} Clock: {} Hand: {} Deck: {}\n",
            p1,
            state.players[p1].level.len(),
            state.players[p1].clock.len(),
            state.players[p1].hand.len(),
            state.players[p1].deck.len()
        ));
        out.push_str("Stage:\n");
        out.push_str(&format!(
            " P{}: {}\n",
            p0,
            Self::format_stage(&state.players[p0].stage)
        ));
        out.push_str(&format!(
            " P{}: {}\n",
            p1,
            Self::format_stage(&state.players[p1].stage)
        ));
        if let Some(action) = &env.last_action_desc {
            let hide_action = env.curriculum.enable_visibility_policies
                && env.config.observation_visibility
                    == crate::config::ObservationVisibility::Public
                && env
                    .last_action_player
                    .map(|p| p != perspective)
                    .unwrap_or(false);
            if !hide_action {
                out.push_str(&format!("Last action: {:?}\n", action));
            }
        }
        out
    }

    fn format_stage(stage: &[crate::state::StageSlot; 5]) -> String {
        let mut parts = Vec::with_capacity(stage.len());
        for slot in stage {
            if let Some(card) = slot.card {
                parts.push(format!("{}:{:?}", card.id, slot.status));
            } else {
                parts.push("Empty".to_string());
            }
        }
        format!("[{}]", parts.join(", "))
    }
}
