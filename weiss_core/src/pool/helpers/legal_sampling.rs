use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use anyhow::{anyhow, Result};
use rayon::prelude::*;

use crate::encode::ACTION_SPACE_SIZE;
use crate::legal::ActionDesc;

use super::super::core::EnvPool;

impl EnvPool {
    pub(super) fn ensure_legal_counts_scratch(&mut self) {
        let len = self.envs.len();
        if self.legal_counts_scratch.len() != len {
            self.legal_counts_scratch = vec![0usize; len];
        }
    }

    /// Sample a legal action id uniformly per env.
    pub fn sample_legal_action_ids_uniform(&self, seeds: &[u64]) -> Result<Vec<u32>> {
        let mut out = vec![0u32; self.envs.len()];
        self.sample_legal_action_ids_uniform_into(seeds, &mut out)?;
        Ok(out)
    }

    /// Sample a legal action id uniformly per env into a buffer.
    pub fn sample_legal_action_ids_uniform_into(
        &self,
        seeds: &[u64],
        out: &mut [u32],
    ) -> Result<()> {
        let num_envs = self.envs.len();
        if seeds.len() != num_envs || out.len() != num_envs {
            anyhow::bail!("seed/output size mismatch");
        }
        if let Some(pool) = self.thread_pool.as_ref() {
            let envs = &self.envs;
            let error_flag = Arc::new(AtomicBool::new(false));
            let error_store: Arc<Mutex<Option<anyhow::Error>>> = Arc::new(Mutex::new(None));
            pool.install(|| {
                out.par_iter_mut()
                    .zip(envs.par_iter())
                    .zip(seeds.par_iter())
                    .enumerate()
                    .for_each(|(idx, ((slot, env), &seed))| {
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
                        let pick = (seed % legal.len() as u64) as usize;
                        *slot = legal[pick] as u32;
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
                return Err(anyhow!("parallel sampling failed"));
            }
        } else {
            for (i, ((slot, env), &seed)) in out
                .iter_mut()
                .zip(self.envs.iter())
                .zip(seeds.iter())
                .enumerate()
            {
                let legal = env.action_ids_cache();
                if legal.is_empty() {
                    anyhow::bail!("no legal actions for env {i}");
                }
                let pick = (seed % legal.len() as u64) as usize;
                *slot = legal[pick] as u32;
            }
        }
        Ok(())
    }

    /// Write the first legal action id per env into a buffer.
    pub fn first_legal_action_ids_into(&self, out: &mut [u32]) -> Result<()> {
        let num_envs = self.envs.len();
        if out.len() != num_envs {
            anyhow::bail!("output size mismatch");
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
                        *slot = legal[0] as u32;
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
                return Err(anyhow!("parallel sampling failed"));
            }
        } else {
            for (i, (slot, env)) in out.iter_mut().zip(self.envs.iter()).enumerate() {
                let legal = env.action_ids_cache();
                if legal.is_empty() {
                    anyhow::bail!("no legal actions for env {i}");
                }
                *slot = legal[0] as u32;
            }
        }
        Ok(())
    }

    /// Fill legal-id buffers and sample one action per env.
    pub fn legal_action_ids_and_sample_uniform_into(
        &mut self,
        ids: &mut [u16],
        offsets: &mut [u32],
        seeds: &[u64],
        sampled: &mut [u32],
    ) -> Result<usize> {
        let num_envs = self.envs.len();
        if seeds.len() != num_envs || sampled.len() != num_envs {
            anyhow::bail!("seed/output size mismatch");
        }
        if offsets.len() != num_envs + 1 {
            anyhow::bail!("offset buffer size mismatch");
        }
        if ACTION_SPACE_SIZE > u16::MAX as usize {
            anyhow::bail!("action space too large for u16 ids");
        }
        if self.thread_pool.is_none() {
            offsets[0] = 0;
            let mut cursor = 0usize;
            for (i, ((env, &seed), slot)) in self
                .envs
                .iter()
                .zip(seeds.iter())
                .zip(sampled.iter_mut())
                .enumerate()
            {
                let legal = env.action_ids_cache();
                if legal.is_empty() {
                    anyhow::bail!("no legal actions for env {i}");
                }
                let pick = (seed % legal.len() as u64) as usize;
                *slot = legal[pick] as u32;
                let next = cursor.saturating_add(legal.len());
                if next > ids.len() {
                    anyhow::bail!("ids buffer size mismatch");
                }
                ids[cursor..next].copy_from_slice(legal);
                offsets[i + 1] = next as u32;
                cursor = next;
            }
            return Ok(cursor);
        }
        let total = self.legal_action_ids_batch_into(ids, offsets)?;
        if let Some(pool) = self.thread_pool.as_ref() {
            let envs = &self.envs;
            let error_flag = Arc::new(AtomicBool::new(false));
            let error_store: Arc<Mutex<Option<anyhow::Error>>> = Arc::new(Mutex::new(None));
            pool.install(|| {
                sampled
                    .par_iter_mut()
                    .zip(envs.par_iter())
                    .zip(seeds.par_iter())
                    .enumerate()
                    .for_each(|(idx, ((slot, env), &seed))| {
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
                        let pick = (seed % legal.len() as u64) as usize;
                        *slot = legal[pick] as u32;
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
                return Err(anyhow!("parallel sampling failed"));
            }
        }
        Ok(total)
    }

    /// Fill legal-id buffers for all envs.
    pub fn legal_action_ids_batch_into(
        &mut self,
        ids: &mut [u16],
        offsets: &mut [u32],
    ) -> Result<usize> {
        let num_envs = self.envs.len();
        if offsets.len() != num_envs + 1 {
            anyhow::bail!("offset buffer size mismatch");
        }
        if ACTION_SPACE_SIZE > u16::MAX as usize {
            anyhow::bail!("action space too large for u16 ids");
        }
        self.ensure_legal_counts_scratch();
        let counts = &mut self.legal_counts_scratch;
        // This path is called every policy step in legal-id workflows.
        // Per-env work here is tiny (cache length read), and rayon setup/coordination
        // dominates at typical batch sizes, so keep this pass serial.
        for (slot, env) in counts.iter_mut().zip(self.envs.iter()) {
            *slot = env.action_ids_cache().len();
        }
        offsets[0] = 0;
        let mut total = 0usize;
        for (i, &count) in counts.iter().enumerate() {
            total = match total.checked_add(count) {
                Some(value) => value,
                None => anyhow::bail!("ids offset total overflow"),
            };
            if total > ids.len() {
                anyhow::bail!("ids buffer size mismatch");
            }
            offsets[i + 1] = total as u32;
        }
        let mut cursor = 0usize;
        for (i, env) in self.envs.iter().enumerate() {
            for &action_id in env.action_ids_cache() {
                ids[cursor] = action_id;
                cursor += 1;
            }
            debug_assert_eq!(cursor, offsets[i + 1] as usize);
        }
        Ok(total)
    }

    /// Compute legal action descriptors for all envs.
    pub fn legal_actions_batch(&self) -> Vec<Vec<ActionDesc>> {
        self.envs.iter().map(|env| env.legal_actions()).collect()
    }

    /// Current decision player per env (-1 if none).
    pub fn get_current_player_batch(&self) -> Vec<i8> {
        self.envs
            .iter()
            .map(|env| env.decision.as_ref().map(|d| d.player as i8).unwrap_or(-1))
            .collect()
    }
}
