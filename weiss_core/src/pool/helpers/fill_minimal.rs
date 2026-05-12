use std::sync::atomic::Ordering;

use anyhow::Result;

use crate::encode::{
    action_meta_for_id, ACTION_META_UNUSED, ACTION_META_WIDTH, ACTION_SPACE_SIZE, OBS_LEN,
    SPEC_HASH,
};
use crate::env::{EngineErrorCode, GameEnv, RewardBreakdown, StepOutcome};

use super::super::core::EnvPool;
use super::super::outputs::{
    BatchOutMinimal, BatchOutMinimalI16, BatchOutMinimalI16LegalIds,
    BatchOutMinimalI16LegalIdsNoMeta, BatchOutMinimalNoMask,
};

fn empty_info() -> crate::env::EnvInfo {
    crate::env::EnvInfo {
        obs_version: 0,
        action_version: 0,
        decision_kind: crate::encode::DECISION_KIND_NONE,
        current_player: -1,
        actor: -1,
        decision_count: 0,
        tick_count: 0,
        terminal: None,
        illegal_action: false,
        engine_error: false,
        engine_error_code: 0,
        main_move_action: false,
        main_pass_action: false,
    }
}

fn empty_outcome() -> StepOutcome {
    StepOutcome {
        obs: Vec::new(),
        reward: 0.0,
        reward_breakdown: RewardBreakdown::default(),
        terminated: false,
        truncated: false,
        info: empty_info(),
    }
}

#[inline]
fn copy_i32_obs_to_i16(dst: &mut [i16], src: &[i32], clamp: bool, count_overflow: bool) -> u64 {
    debug_assert_eq!(dst.len(), src.len());
    match (clamp, count_overflow) {
        (false, false) => {
            for (dst, &val) in dst.iter_mut().zip(src.iter()) {
                *dst = val as i16;
            }
            0
        }
        (true, false) => {
            for (dst, &val) in dst.iter_mut().zip(src.iter()) {
                *dst = val.clamp(i16::MIN as i32, i16::MAX as i32) as i16;
            }
            0
        }
        (false, true) => {
            let mut overflow_count = 0u64;
            for (dst, &val) in dst.iter_mut().zip(src.iter()) {
                if val < i16::MIN as i32 || val > i16::MAX as i32 {
                    overflow_count = overflow_count.saturating_add(1);
                }
                *dst = val as i16;
            }
            overflow_count
        }
        (true, true) => {
            let mut overflow_count = 0u64;
            for (dst, &val) in dst.iter_mut().zip(src.iter()) {
                if val < i16::MIN as i32 || val > i16::MAX as i32 {
                    overflow_count = overflow_count.saturating_add(1);
                }
                *dst = val.clamp(i16::MIN as i32, i16::MAX as i32) as i16;
            }
            overflow_count
        }
    }
}

impl EnvPool {
    #[inline]
    pub(in crate::pool) fn ensure_outcomes_scratch(&mut self) {
        let len = self.envs.len();
        if self.outcomes_scratch.len() != len {
            self.outcomes_scratch = (0..len).map(|_| empty_outcome()).collect();
        }
    }

    #[inline]
    pub(in crate::pool) fn fill_minimal_out(
        &self,
        outcomes: &[StepOutcome],
        out: &mut BatchOutMinimal<'_>,
    ) -> Result<()> {
        self.validate_minimal_out(out)?;
        let num_envs = self.envs.len();
        debug_assert_eq!(outcomes.len(), num_envs);
        for (i, (env, outcome)) in self.envs.iter().zip(outcomes.iter()).enumerate() {
            let obs_offset = i * OBS_LEN;
            if outcome.obs.is_empty() {
                out.obs[obs_offset..obs_offset + OBS_LEN].copy_from_slice(&env.obs_buf);
            } else {
                out.obs[obs_offset..obs_offset + OBS_LEN].copy_from_slice(&outcome.obs);
            }
            let mask_offset = i * ACTION_SPACE_SIZE;
            if self.output_mask_enabled {
                out.masks[mask_offset..mask_offset + ACTION_SPACE_SIZE]
                    .copy_from_slice(env.action_mask());
            }
            out.rewards[i] = outcome.reward;
            out.terminated[i] = outcome.terminated;
            out.truncated[i] = outcome.truncated;
            let engine_status = if outcome.info.engine_error {
                outcome.info.engine_error_code
            } else {
                env.last_engine_error_code as u8
            };
            out.engine_status[i] = engine_status;
            let keep_fault_actor = engine_status != EngineErrorCode::None as u8
                && (env.fault_actor().is_some() || outcome.info.actor != crate::encode::ACTOR_NONE);
            out.actor[i] = if outcome.terminated || outcome.truncated {
                if keep_fault_actor {
                    env.fault_actor()
                        .or_else(|| {
                            (outcome.info.actor != crate::encode::ACTOR_NONE)
                                .then_some(outcome.info.actor as u8)
                        })
                        .map(|a| a as i8)
                        .unwrap_or(crate::encode::ACTOR_NONE)
                } else {
                    crate::encode::ACTOR_NONE
                }
            } else {
                outcome.info.actor
            };
            out.decision_kind[i] = outcome.info.decision_kind;
            out.decision_id[i] = env.decision_id();
            out.spec_hash[i] = SPEC_HASH;
            let (main_move_action, main_pass_action) = env.last_action_main_flags();
            out.main_move_action[i] = main_move_action;
            out.main_pass_action[i] = main_pass_action;
            debug_assert!(
                out.terminated[i] || out.truncated[i] || (out.actor[i] == 0 || out.actor[i] == 1)
            );
            if self.output_mask_enabled {
                debug_assert!(
                    out.terminated[i]
                        || out.truncated[i]
                        || out.masks[mask_offset..mask_offset + ACTION_SPACE_SIZE].contains(&1)
                );
            }
        }
        Ok(())
    }

    #[inline]
    pub(in crate::pool) fn fill_minimal_out_i16(
        &self,
        outcomes: &[StepOutcome],
        out: &mut BatchOutMinimalI16<'_>,
    ) -> Result<()> {
        self.validate_minimal_out_i16(out)?;
        let num_envs = self.envs.len();
        debug_assert_eq!(outcomes.len(), num_envs);
        let count_overflow = self.i16_overflow_counter_enabled.load(Ordering::Relaxed);
        let mut overflow_count = 0u64;
        for (i, (env, outcome)) in self.envs.iter().zip(outcomes.iter()).enumerate() {
            let obs_offset = i * OBS_LEN;
            let src = if outcome.obs.is_empty() {
                &env.obs_buf
            } else {
                &outcome.obs
            };
            overflow_count = overflow_count.saturating_add(copy_i32_obs_to_i16(
                &mut out.obs[obs_offset..obs_offset + OBS_LEN],
                src,
                self.i16_clamp_enabled,
                count_overflow,
            ));
            let mask_offset = i * ACTION_SPACE_SIZE;
            if self.output_mask_enabled {
                out.masks[mask_offset..mask_offset + ACTION_SPACE_SIZE]
                    .copy_from_slice(env.action_mask());
            }
            out.rewards[i] = outcome.reward;
            out.terminated[i] = outcome.terminated;
            out.truncated[i] = outcome.truncated;
            let engine_status = if outcome.info.engine_error {
                outcome.info.engine_error_code
            } else {
                env.last_engine_error_code as u8
            };
            out.engine_status[i] = engine_status;
            let keep_fault_actor = engine_status != EngineErrorCode::None as u8
                && (env.fault_actor().is_some() || outcome.info.actor != crate::encode::ACTOR_NONE);
            out.actor[i] = if outcome.terminated || outcome.truncated {
                if keep_fault_actor {
                    env.fault_actor()
                        .or_else(|| {
                            (outcome.info.actor != crate::encode::ACTOR_NONE)
                                .then_some(outcome.info.actor as u8)
                        })
                        .map(|a| a as i8)
                        .unwrap_or(crate::encode::ACTOR_NONE)
                } else {
                    crate::encode::ACTOR_NONE
                }
            } else {
                outcome.info.actor
            };
            out.decision_kind[i] = outcome.info.decision_kind;
            out.decision_id[i] = env.decision_id();
            out.spec_hash[i] = SPEC_HASH;
            let (main_move_action, main_pass_action) = env.last_action_main_flags();
            out.main_move_action[i] = main_move_action;
            out.main_pass_action[i] = main_pass_action;
            debug_assert!(
                out.terminated[i] || out.truncated[i] || (out.actor[i] == 0 || out.actor[i] == 1)
            );
            if self.output_mask_enabled {
                debug_assert!(
                    out.terminated[i]
                        || out.truncated[i]
                        || out.masks[mask_offset..mask_offset + ACTION_SPACE_SIZE].contains(&1)
                );
            }
        }
        if count_overflow && overflow_count > 0 {
            self.i16_overflow_count
                .fetch_add(overflow_count, Ordering::Relaxed);
        }
        Ok(())
    }

    #[inline]
    pub(in crate::pool) fn fill_minimal_out_i16_legal_ids(
        &self,
        outcomes: &[StepOutcome],
        out: &mut BatchOutMinimalI16LegalIds<'_>,
    ) -> Result<()> {
        self.validate_minimal_out_i16_legal_ids(out)?;
        let num_envs = self.envs.len();
        debug_assert_eq!(outcomes.len(), num_envs);
        let count_overflow = self.i16_overflow_counter_enabled.load(Ordering::Relaxed);
        let mut overflow_count = 0u64;
        out.legal_offsets[0] = 0;
        let mut legal_cursor = 0usize;
        for (i, (env, outcome)) in self.envs.iter().zip(outcomes.iter()).enumerate() {
            let obs_offset = i * OBS_LEN;
            let src = if outcome.obs.is_empty() {
                &env.obs_buf
            } else {
                &outcome.obs
            };
            overflow_count = overflow_count.saturating_add(copy_i32_obs_to_i16(
                &mut out.obs[obs_offset..obs_offset + OBS_LEN],
                src,
                self.i16_clamp_enabled,
                count_overflow,
            ));
            out.rewards[i] = outcome.reward;
            out.terminated[i] = outcome.terminated;
            out.truncated[i] = outcome.truncated;
            let engine_status = if outcome.info.engine_error {
                outcome.info.engine_error_code
            } else {
                env.last_engine_error_code as u8
            };
            out.engine_status[i] = engine_status;
            let keep_fault_actor = engine_status != EngineErrorCode::None as u8
                && (env.fault_actor().is_some() || outcome.info.actor != crate::encode::ACTOR_NONE);
            out.actor[i] = if outcome.terminated || outcome.truncated {
                if keep_fault_actor {
                    env.fault_actor()
                        .or_else(|| {
                            (outcome.info.actor != crate::encode::ACTOR_NONE)
                                .then_some(outcome.info.actor as u8)
                        })
                        .map(|a| a as i8)
                        .unwrap_or(crate::encode::ACTOR_NONE)
                } else {
                    crate::encode::ACTOR_NONE
                }
            } else {
                outcome.info.actor
            };
            out.decision_kind[i] = outcome.info.decision_kind;
            out.decision_id[i] = env.decision_id();
            out.spec_hash[i] = SPEC_HASH;
            let (main_move_action, main_pass_action) = env.last_action_main_flags();
            out.main_move_action[i] = main_move_action;
            out.main_pass_action[i] = main_pass_action;
            let legal_ids = env.action_ids_cache();
            let next = legal_cursor.saturating_add(legal_ids.len());
            if next > out.legal_ids.len() {
                anyhow::bail!("legal ids buffer size mismatch");
            }
            out.legal_ids[legal_cursor..next].copy_from_slice(legal_ids);
            for (row_index, &action_id) in legal_ids.iter().enumerate() {
                let meta_offset = (legal_cursor + row_index) * ACTION_META_WIDTH;
                let row = action_meta_for_id(action_id as usize)
                    .unwrap_or([ACTION_META_UNUSED; ACTION_META_WIDTH]);
                out.legal_action_meta[meta_offset..meta_offset + ACTION_META_WIDTH]
                    .copy_from_slice(&row);
            }
            out.legal_offsets[i + 1] = next as u32;
            legal_cursor = next;
            debug_assert!(
                out.terminated[i] || out.truncated[i] || (out.actor[i] == 0 || out.actor[i] == 1)
            );
        }
        if count_overflow && overflow_count > 0 {
            self.i16_overflow_count
                .fetch_add(overflow_count, Ordering::Relaxed);
        }
        Ok(())
    }

    #[inline]
    fn fill_minimal_out_i16_legal_ids_nometa_row(
        &self,
        i: usize,
        env: &GameEnv,
        outcome: &StepOutcome,
        out: &mut BatchOutMinimalI16LegalIdsNoMeta<'_>,
        legal_cursor: &mut usize,
        count_overflow: bool,
    ) -> Result<u64> {
        let obs_offset = i * OBS_LEN;
        let src = if outcome.obs.is_empty() {
            &env.obs_buf
        } else {
            &outcome.obs
        };
        let overflow_count = copy_i32_obs_to_i16(
            &mut out.obs[obs_offset..obs_offset + OBS_LEN],
            src,
            self.i16_clamp_enabled,
            count_overflow,
        );
        out.rewards[i] = outcome.reward;
        out.terminated[i] = outcome.terminated;
        out.truncated[i] = outcome.truncated;
        let engine_status = if outcome.info.engine_error {
            outcome.info.engine_error_code
        } else {
            env.last_engine_error_code as u8
        };
        out.engine_status[i] = engine_status;
        let keep_fault_actor = engine_status != EngineErrorCode::None as u8
            && (env.fault_actor().is_some() || outcome.info.actor != crate::encode::ACTOR_NONE);
        out.actor[i] = if outcome.terminated || outcome.truncated {
            if keep_fault_actor {
                env.fault_actor()
                    .or_else(|| {
                        (outcome.info.actor != crate::encode::ACTOR_NONE)
                            .then_some(outcome.info.actor as u8)
                    })
                    .map(|a| a as i8)
                    .unwrap_or(crate::encode::ACTOR_NONE)
            } else {
                crate::encode::ACTOR_NONE
            }
        } else {
            outcome.info.actor
        };
        out.decision_kind[i] = outcome.info.decision_kind;
        out.decision_id[i] = env.decision_id();
        out.spec_hash[i] = SPEC_HASH;
        let (main_move_action, main_pass_action) = env.last_action_main_flags();
        out.main_move_action[i] = main_move_action;
        out.main_pass_action[i] = main_pass_action;
        let legal_ids = env.action_ids_cache();
        let next = legal_cursor.saturating_add(legal_ids.len());
        if next > out.legal_ids.len() {
            anyhow::bail!("legal ids buffer size mismatch");
        }
        out.legal_ids[*legal_cursor..next].copy_from_slice(legal_ids);
        out.legal_offsets[i + 1] = next as u32;
        *legal_cursor = next;
        debug_assert!(
            out.terminated[i] || out.truncated[i] || (out.actor[i] == 0 || out.actor[i] == 1)
        );
        Ok(overflow_count)
    }

    #[inline]
    pub(in crate::pool) fn fill_minimal_out_i16_legal_ids_nometa(
        &self,
        outcomes: &[StepOutcome],
        out: &mut BatchOutMinimalI16LegalIdsNoMeta<'_>,
    ) -> Result<()> {
        self.validate_minimal_out_i16_legal_ids_nometa(out)?;
        let num_envs = self.envs.len();
        debug_assert_eq!(outcomes.len(), num_envs);
        let count_overflow = self.i16_overflow_counter_enabled.load(Ordering::Relaxed);
        let mut overflow_count = 0u64;
        out.legal_offsets[0] = 0;
        let mut legal_cursor = 0usize;
        for (i, (env, outcome)) in self.envs.iter().zip(outcomes.iter()).enumerate() {
            overflow_count =
                overflow_count.saturating_add(self.fill_minimal_out_i16_legal_ids_nometa_row(
                    i,
                    env,
                    outcome,
                    out,
                    &mut legal_cursor,
                    count_overflow,
                )?);
        }
        if count_overflow && overflow_count > 0 {
            self.i16_overflow_count
                .fetch_add(overflow_count, Ordering::Relaxed);
        }
        Ok(())
    }

    #[inline]
    pub(in crate::pool) fn fill_minimal_out_nomask(
        &self,
        outcomes: &[StepOutcome],
        out: &mut BatchOutMinimalNoMask<'_>,
    ) -> Result<()> {
        self.validate_minimal_out_nomask(out)?;
        let num_envs = self.envs.len();
        debug_assert_eq!(outcomes.len(), num_envs);
        for (i, (env, outcome)) in self.envs.iter().zip(outcomes.iter()).enumerate() {
            let obs_offset = i * OBS_LEN;
            if outcome.obs.is_empty() {
                out.obs[obs_offset..obs_offset + OBS_LEN].copy_from_slice(&env.obs_buf);
            } else {
                out.obs[obs_offset..obs_offset + OBS_LEN].copy_from_slice(&outcome.obs);
            }
            out.rewards[i] = outcome.reward;
            out.terminated[i] = outcome.terminated;
            out.truncated[i] = outcome.truncated;
            let engine_status = if outcome.info.engine_error {
                outcome.info.engine_error_code
            } else {
                env.last_engine_error_code as u8
            };
            out.engine_status[i] = engine_status;
            let keep_fault_actor = engine_status != EngineErrorCode::None as u8
                && (env.fault_actor().is_some() || outcome.info.actor != crate::encode::ACTOR_NONE);
            out.actor[i] = if outcome.terminated || outcome.truncated {
                if keep_fault_actor {
                    env.fault_actor()
                        .or_else(|| {
                            (outcome.info.actor != crate::encode::ACTOR_NONE)
                                .then_some(outcome.info.actor as u8)
                        })
                        .map(|a| a as i8)
                        .unwrap_or(crate::encode::ACTOR_NONE)
                } else {
                    crate::encode::ACTOR_NONE
                }
            } else {
                outcome.info.actor
            };
            out.decision_kind[i] = outcome.info.decision_kind;
            out.decision_id[i] = env.decision_id();
            out.spec_hash[i] = SPEC_HASH;
            let (main_move_action, main_pass_action) = env.last_action_main_flags();
            out.main_move_action[i] = main_move_action;
            out.main_pass_action[i] = main_pass_action;
            debug_assert!(
                out.terminated[i] || out.truncated[i] || (out.actor[i] == 0 || out.actor[i] == 1)
            );
        }
        Ok(())
    }
}
