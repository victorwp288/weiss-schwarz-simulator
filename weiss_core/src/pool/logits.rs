use anyhow::Result;

use super::core::EnvPool;
use super::outputs::{
    BatchOutMinimal, BatchOutMinimalI16, BatchOutMinimalI16LegalIds, BatchOutMinimalNoMask,
};

use crate::encode::ACTION_SPACE_SIZE;

impl EnvPool {
    /// Select the best legal action per env from logits (argmax).
    pub fn select_actions_from_logits_into(&self, logits: &[f32], out: &mut [u32]) -> Result<()> {
        let num_envs = self.envs.len();
        if out.len() != num_envs {
            anyhow::bail!("output size mismatch");
        }
        if logits.len() != num_envs * ACTION_SPACE_SIZE {
            anyhow::bail!("logits buffer size mismatch");
        }
        for (i, env) in self.envs.iter().enumerate() {
            let legal = env.action_ids_cache();
            if legal.is_empty() {
                anyhow::bail!("no legal actions for env {i}");
            }
            let base = i * ACTION_SPACE_SIZE;
            let mut best_id = legal[0] as u32;
            let mut best_logit = logits[base + best_id as usize];
            for &id_u16 in legal.iter().skip(1) {
                let id = id_u16 as usize;
                let logit = logits[base + id];
                if logit > best_logit {
                    best_logit = logit;
                    best_id = id_u16 as u32;
                }
            }
            out[i] = best_id;
        }
        Ok(())
    }

    /// Sample a legal action per env from logits using softmax.
    pub fn sample_actions_from_logits_into(
        &self,
        logits: &[f32],
        seeds: &[u64],
        out: &mut [u32],
    ) -> Result<()> {
        let num_envs = self.envs.len();
        if out.len() != num_envs {
            anyhow::bail!("output size mismatch");
        }
        if logits.len() != num_envs * ACTION_SPACE_SIZE {
            anyhow::bail!("logits buffer size mismatch");
        }
        if seeds.len() != num_envs {
            anyhow::bail!("seed buffer size mismatch");
        }
        for (i, env) in self.envs.iter().enumerate() {
            let legal = env.action_ids_cache();
            if legal.is_empty() {
                anyhow::bail!("no legal actions for env {i}");
            }
            let base = i * ACTION_SPACE_SIZE;
            let mut max_logit = f64::NEG_INFINITY;
            for &id_u16 in legal.iter() {
                let logit = logits[base + id_u16 as usize] as f64;
                if logit > max_logit {
                    max_logit = logit;
                }
            }
            let mut total = 0.0f64;
            for &id_u16 in legal.iter() {
                let logit = logits[base + id_u16 as usize] as f64;
                total += (logit - max_logit).exp();
            }
            if total <= 0.0 || !total.is_finite() {
                out[i] = legal[0] as u32;
                continue;
            }
            let u = (seeds[i] as f64) / (u64::MAX as f64);
            let mut threshold = u * total;
            for &id_u16 in legal.iter() {
                let logit = logits[base + id_u16 as usize] as f64;
                threshold -= (logit - max_logit).exp();
                if threshold <= 0.0 {
                    out[i] = id_u16 as u32;
                    break;
                }
            }
            if threshold > 0.0 {
                out[i] = legal[legal.len() - 1] as u32;
            }
        }
        Ok(())
    }

    /// Select from logits and step, filling minimal outputs.
    pub fn step_select_from_logits_into(
        &mut self,
        logits: &[f32],
        actions: &mut [u32],
        out: &mut BatchOutMinimal<'_>,
    ) -> Result<()> {
        self.select_actions_from_logits_into(logits, actions)?;
        self.step_into(actions, out)
    }

    /// Select from logits and step, filling i16 outputs.
    pub fn step_select_from_logits_into_i16(
        &mut self,
        logits: &[f32],
        actions: &mut [u32],
        out: &mut BatchOutMinimalI16<'_>,
    ) -> Result<()> {
        self.select_actions_from_logits_into(logits, actions)?;
        self.step_into_i16(actions, out)
    }

    /// Select from logits and step, filling outputs without masks.
    pub fn step_select_from_logits_into_nomask(
        &mut self,
        logits: &[f32],
        actions: &mut [u32],
        out: &mut BatchOutMinimalNoMask<'_>,
    ) -> Result<()> {
        self.select_actions_from_logits_into(logits, actions)?;
        self.step_into_nomask(actions, out)
    }

    /// Select from logits and step, filling i16 outputs plus legal-id lists.
    ///
    /// Requires output masks to be disabled.
    pub fn step_select_from_logits_into_i16_legal_ids(
        &mut self,
        logits: &[f32],
        actions: &mut [u32],
        out: &mut BatchOutMinimalI16LegalIds<'_>,
    ) -> Result<()> {
        self.select_actions_from_logits_into(logits, actions)?;
        self.step_into_i16_legal_ids(actions, out)
    }

    /// Sample from logits and step, filling minimal outputs.
    pub fn step_sample_from_logits_into(
        &mut self,
        logits: &[f32],
        seeds: &[u64],
        actions: &mut [u32],
        out: &mut BatchOutMinimal<'_>,
    ) -> Result<()> {
        self.sample_actions_from_logits_into(logits, seeds, actions)?;
        self.step_into(actions, out)
    }

    /// Sample from logits and step, filling i16 outputs.
    pub fn step_sample_from_logits_into_i16(
        &mut self,
        logits: &[f32],
        seeds: &[u64],
        actions: &mut [u32],
        out: &mut BatchOutMinimalI16<'_>,
    ) -> Result<()> {
        self.sample_actions_from_logits_into(logits, seeds, actions)?;
        self.step_into_i16(actions, out)
    }

    /// Sample from logits and step, filling outputs without masks.
    pub fn step_sample_from_logits_into_nomask(
        &mut self,
        logits: &[f32],
        seeds: &[u64],
        actions: &mut [u32],
        out: &mut BatchOutMinimalNoMask<'_>,
    ) -> Result<()> {
        self.sample_actions_from_logits_into(logits, seeds, actions)?;
        self.step_into_nomask(actions, out)
    }

    /// Sample from logits and step, filling i16 outputs plus legal-id lists.
    ///
    /// Requires output masks to be disabled.
    pub fn step_sample_from_logits_into_i16_legal_ids(
        &mut self,
        logits: &[f32],
        seeds: &[u64],
        actions: &mut [u32],
        out: &mut BatchOutMinimalI16LegalIds<'_>,
    ) -> Result<()> {
        self.sample_actions_from_logits_into(logits, seeds, actions)?;
        self.step_into_i16_legal_ids(actions, out)
    }
}
