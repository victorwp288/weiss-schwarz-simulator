use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use anyhow::{anyhow, Result};
use rayon::prelude::*;

use super::core::EnvPool;
use super::outputs::{
    BatchOutMinimal, BatchOutMinimalI16, BatchOutMinimalI16LegalIds, BatchOutMinimalNoMask,
};

use crate::encode::ACTION_SPACE_SIZE;

impl EnvPool {
    fn sample_actions_from_logits_internal(
        &self,
        logits: &[f32],
        seeds: &[u64],
        out: &mut [u32],
        mut logp_out: Option<&mut [f32]>,
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
        if let Some(ref logp) = logp_out {
            if logp.len() != num_envs {
                anyhow::bail!("logp output size mismatch");
            }
        }

        if let Some(pool) = self.thread_pool.as_ref() {
            let envs = &self.envs;
            let error_flag = Arc::new(AtomicBool::new(false));
            let error_store: Arc<Mutex<Option<anyhow::Error>>> = Arc::new(Mutex::new(None));

            if let Some(logp_slice) = logp_out.as_deref_mut() {
                pool.install(|| {
                    out.par_iter_mut()
                        .zip(logp_slice.par_iter_mut())
                        .zip(envs.par_iter())
                        .zip(seeds.par_iter())
                        .enumerate()
                        .for_each(|(idx, (((action_slot, logp_slot), env), &seed))| {
                            let legal = env.action_ids_cache();
                            if legal.is_empty() {
                                error_flag.store(true, Ordering::Relaxed);
                                let mut guard = error_store
                                    .lock()
                                    .unwrap_or_else(|poison| poison.into_inner());
                                if guard.is_none() {
                                    *guard = Some(anyhow!("no legal actions for env {idx}"));
                                }
                                return;
                            }
                            let base = idx * ACTION_SPACE_SIZE;
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
                                *action_slot = legal[0] as u32;
                                *logp_slot = 0.0;
                                return;
                            }
                            let u = (seed as f64) / (u64::MAX as f64);
                            let mut threshold = u * total;
                            let mut chosen = legal[legal.len() - 1] as u32;
                            let mut chosen_logit = logits[base + chosen as usize] as f64;
                            for &id_u16 in legal.iter() {
                                let logit = logits[base + id_u16 as usize] as f64;
                                threshold -= (logit - max_logit).exp();
                                if threshold <= 0.0 {
                                    chosen = id_u16 as u32;
                                    chosen_logit = logit;
                                    break;
                                }
                            }
                            *action_slot = chosen;
                            *logp_slot = (chosen_logit - max_logit - total.ln()) as f32;
                        });
                });
            } else {
                pool.install(|| {
                    out.par_iter_mut()
                        .zip(envs.par_iter())
                        .zip(seeds.par_iter())
                        .enumerate()
                        .for_each(|(idx, ((action_slot, env), &seed))| {
                            let legal = env.action_ids_cache();
                            if legal.is_empty() {
                                error_flag.store(true, Ordering::Relaxed);
                                let mut guard = error_store
                                    .lock()
                                    .unwrap_or_else(|poison| poison.into_inner());
                                if guard.is_none() {
                                    *guard = Some(anyhow!("no legal actions for env {idx}"));
                                }
                                return;
                            }
                            let base = idx * ACTION_SPACE_SIZE;
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
                                *action_slot = legal[0] as u32;
                                return;
                            }
                            let u = (seed as f64) / (u64::MAX as f64);
                            let mut threshold = u * total;
                            let mut chosen = legal[legal.len() - 1] as u32;
                            for &id_u16 in legal.iter() {
                                let logit = logits[base + id_u16 as usize] as f64;
                                threshold -= (logit - max_logit).exp();
                                if threshold <= 0.0 {
                                    chosen = id_u16 as u32;
                                    break;
                                }
                            }
                            *action_slot = chosen;
                        });
                });
            }

            if error_flag.load(Ordering::Relaxed) {
                let err = error_store
                    .lock()
                    .unwrap_or_else(|poison| poison.into_inner())
                    .take();
                if let Some(err) = err {
                    return Err(err);
                }
                return Err(anyhow!("parallel logits sampling failed"));
            }
            return Ok(());
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
                if let Some(ref mut logp) = logp_out {
                    logp[i] = 0.0;
                }
                continue;
            }
            let u = (seeds[i] as f64) / (u64::MAX as f64);
            let mut threshold = u * total;
            let mut chosen = legal[legal.len() - 1] as u32;
            let mut chosen_logit = logits[base + chosen as usize] as f64;
            for &id_u16 in legal.iter() {
                let logit = logits[base + id_u16 as usize] as f64;
                threshold -= (logit - max_logit).exp();
                if threshold <= 0.0 {
                    chosen = id_u16 as u32;
                    chosen_logit = logit;
                    break;
                }
            }
            out[i] = chosen;
            if let Some(ref mut logp) = logp_out {
                logp[i] = (chosen_logit - max_logit - total.ln()) as f32;
            }
        }
        Ok(())
    }

    /// Select the best legal action per env from logits (argmax).
    pub fn select_actions_from_logits_into(&self, logits: &[f32], out: &mut [u32]) -> Result<()> {
        let num_envs = self.envs.len();
        if out.len() != num_envs {
            anyhow::bail!("output size mismatch");
        }
        if logits.len() != num_envs * ACTION_SPACE_SIZE {
            anyhow::bail!("logits buffer size mismatch");
        }
        if let Some(pool) = self.thread_pool.as_ref() {
            let envs = &self.envs;
            let error_flag = Arc::new(AtomicBool::new(false));
            let error_store: Arc<Mutex<Option<anyhow::Error>>> = Arc::new(Mutex::new(None));
            pool.install(|| {
                out.par_iter_mut()
                    .zip(envs.par_iter())
                    .enumerate()
                    .for_each(|(idx, (slot, env))| {
                        let legal = env.action_ids_cache();
                        if legal.is_empty() {
                            error_flag.store(true, Ordering::Relaxed);
                            let mut guard = error_store
                                .lock()
                                .unwrap_or_else(|poison| poison.into_inner());
                            if guard.is_none() {
                                *guard = Some(anyhow!("no legal actions for env {idx}"));
                            }
                            return;
                        }
                        let base = idx * ACTION_SPACE_SIZE;
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
                        *slot = best_id;
                    });
            });
            if error_flag.load(Ordering::Relaxed) {
                let err = error_store
                    .lock()
                    .unwrap_or_else(|poison| poison.into_inner())
                    .take();
                if let Some(err) = err {
                    return Err(err);
                }
                return Err(anyhow!("parallel logits argmax failed"));
            }
            return Ok(());
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
        self.sample_actions_from_logits_internal(logits, seeds, out, None)
    }

    /// Sample a legal action per env from logits and write sampled-action log-probs.
    pub fn sample_actions_from_logits_with_logp_into(
        &self,
        logits: &[f32],
        seeds: &[u64],
        out: &mut [u32],
        logp_out: &mut [f32],
    ) -> Result<()> {
        self.sample_actions_from_logits_internal(logits, seeds, out, Some(logp_out))
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

    /// Sample from logits, write sampled-action log-probs, and step, filling i16 outputs plus legal-id lists.
    pub fn step_sample_from_logits_with_logp_into_i16_legal_ids(
        &mut self,
        logits: &[f32],
        seeds: &[u64],
        actions: &mut [u32],
        action_logp: &mut [f32],
        out: &mut BatchOutMinimalI16LegalIds<'_>,
    ) -> Result<()> {
        self.sample_actions_from_logits_with_logp_into(logits, seeds, actions, action_logp)?;
        self.step_into_i16_legal_ids(actions, out)
    }
}
