use anyhow::Result;

use crate::encode::ACTION_SPACE_SIZE;
use crate::env::{EngineErrorCode, StepOutcome, REWARD_COMPONENT_WIDTH};

use super::super::core::EnvPool;
use super::super::outputs::BatchOutDebug;
use super::unsafe_bytes;

impl EnvPool {
    pub(in crate::pool) fn panic_fingerprint_from_meta(
        env_id: u32,
        episode_index: u32,
        episode_seed: u64,
        decision_id: u32,
        code: EngineErrorCode,
    ) -> u64 {
        let mut bytes = Vec::with_capacity(32);
        bytes.extend_from_slice(&env_id.to_le_bytes());
        bytes.extend_from_slice(&episode_index.to_le_bytes());
        bytes.extend_from_slice(&episode_seed.to_le_bytes());
        bytes.extend_from_slice(&decision_id.to_le_bytes());
        bytes.push(code as u8);
        crate::fingerprint::hash_bytes(&bytes)
    }

    pub(in crate::pool) fn debug_compute_fingerprints(&mut self) -> bool {
        if self.debug_config.fingerprint_every_n == 0 {
            return false;
        }
        self.debug_step_counter = self.debug_step_counter.wrapping_add(1);
        self.debug_step_counter
            .is_multiple_of(self.debug_config.fingerprint_every_n as u64)
    }

    /// Compute state fingerprints for each env.
    pub fn state_fingerprint_batch(&self) -> Vec<u64> {
        self.envs
            .iter()
            .map(|env| crate::fingerprint::state_fingerprint(&env.state))
            .collect()
    }

    /// Compute event-stream fingerprints for each env.
    pub fn events_fingerprint_batch(&self) -> Vec<u64> {
        self.envs
            .iter()
            .map(|env| crate::fingerprint::events_fingerprint(env.canonical_events()))
            .collect()
    }

    /// Compute observation fingerprints for each env.
    pub fn obs_fingerprint_batch(&self) -> Vec<u64> {
        self.envs
            .iter()
            .map(|env| {
                let bytes = unsafe_bytes::i32_slice_as_bytes(&env.obs_buf);
                crate::fingerprint::hash_bytes(bytes)
            })
            .collect()
    }

    pub(in crate::pool) fn fill_debug_out(
        &self,
        outcomes: &[StepOutcome],
        out: &mut BatchOutDebug<'_>,
        compute_fingerprints: bool,
    ) -> Result<()> {
        let num_envs = self.envs.len();
        if out.state_fingerprint.len() != num_envs
            || out.reward_components.len() != num_envs * REWARD_COMPONENT_WIDTH
            || out.events_fingerprint.len() != num_envs
            || out.mask_fingerprint.len() != num_envs
            || out.event_counts.len() != num_envs
        {
            anyhow::bail!("debug buffer size mismatch");
        }
        if self.output_mask_enabled && out.minimal.masks.len() != num_envs * ACTION_SPACE_SIZE {
            anyhow::bail!("mask buffer size mismatch");
        }
        let event_capacity = if num_envs == 0 {
            0
        } else if !out.event_codes.len().is_multiple_of(num_envs) {
            anyhow::bail!("event code buffer size mismatch");
        } else {
            out.event_codes.len() / num_envs
        };
        for (i, (env, outcome)) in self.envs.iter().zip(outcomes.iter()).enumerate() {
            let component_offset = i * REWARD_COMPONENT_WIDTH;
            out.reward_components[component_offset..component_offset + REWARD_COMPONENT_WIDTH]
                .copy_from_slice(&outcome.reward_breakdown.as_array());
            if compute_fingerprints {
                out.state_fingerprint[i] = crate::fingerprint::state_fingerprint(&env.state);
                out.events_fingerprint[i] =
                    crate::fingerprint::events_fingerprint(env.canonical_events());
                if self.output_mask_enabled {
                    let mask_offset = i * ACTION_SPACE_SIZE;
                    let mask = &out.minimal.masks[mask_offset..mask_offset + ACTION_SPACE_SIZE];
                    out.mask_fingerprint[i] = crate::fingerprint::hash_bytes(mask);
                } else if self.output_mask_bits_enabled {
                    let bits = env.action_mask_bits();
                    let bytes = unsafe_bytes::u64_slice_as_bytes(bits);
                    out.mask_fingerprint[i] = crate::fingerprint::hash_bytes(bytes);
                } else {
                    let ids = env.action_ids_cache();
                    let bytes = unsafe_bytes::u16_slice_as_bytes(ids);
                    out.mask_fingerprint[i] = crate::fingerprint::hash_bytes(bytes);
                }
            } else {
                out.state_fingerprint[i] = 0;
                out.events_fingerprint[i] = 0;
                out.mask_fingerprint[i] = 0;
            }
            if event_capacity == 0 {
                out.event_counts[i] = 0;
            } else {
                let actor = outcome.info.actor;
                let viewer = if actor < 0 { 0 } else { actor as u8 };
                let offset = i * event_capacity;
                let count = env.debug_event_ring_codes(
                    viewer,
                    &mut out.event_codes[offset..offset + event_capacity],
                );
                out.event_counts[i] = count;
            }
        }
        Ok(())
    }
}
