use anyhow::Result;

use super::super::core::EnvPool;

impl EnvPool {
    /// Build a redacted, JSON-serialized human decision view for one env.
    pub fn human_decision_view_json(
        &mut self,
        env_index: usize,
        perspective_seat: Option<u8>,
    ) -> Result<String> {
        let num_envs = self.envs.len();
        let Some(env) = self.envs.get_mut(env_index) else {
            anyhow::bail!("env_index {env_index} out of bounds (num_envs = {num_envs})");
        };
        env.update_action_cache();
        env.human_decision_view_json(perspective_seat)
    }
}
