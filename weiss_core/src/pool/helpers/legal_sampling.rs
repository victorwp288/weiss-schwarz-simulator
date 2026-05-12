use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use anyhow::{anyhow, Result};
use rayon::prelude::*;

use crate::db::{CardColor, CardId, CardType};
use crate::encode::{
    action_meta_for_id, ACTION_META_UNUSED, ACTION_META_WIDTH, ACTION_SPACE_SIZE,
    LEGAL_ACTION_CONTEXT_UNUSED, LEGAL_ACTION_CONTEXT_V1_WIDTH,
};
use crate::env::heuristic_public::HeuristicPublicProfile;
use crate::legal::{ActionDesc, DecisionKind};
use crate::state::{ChoiceOptionRef, ChoiceReason, ChoiceState, ChoiceZone, TargetSide};

use super::super::core::EnvPool;

const CONTEXT_ZONE_NONE: i32 = 0;
const CONTEXT_ZONE_HAND: i32 = 1;
const CONTEXT_ZONE_STAGE: i32 = 2;
const CONTEXT_ZONE_CLOCK: i32 = 3;
const CONTEXT_ZONE_LEVEL: i32 = 4;
const CONTEXT_ZONE_CHOICE: i32 = 5;
const CONTEXT_ZONE_DECK_TOP: i32 = 6;
const CONTEXT_ZONE_STOCK: i32 = 7;
const CONTEXT_ZONE_MEMORY: i32 = 8;
const CONTEXT_ZONE_WAITING_ROOM: i32 = 9;
const CONTEXT_ZONE_CLIMAX: i32 = 10;
const CONTEXT_ZONE_RESOLUTION: i32 = 11;

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

    /// Fill packed legal-action metadata for all envs.
    pub fn legal_action_meta_batch_into(&self, meta: &mut [u16]) -> Result<usize> {
        let num_envs = self.envs.len();
        if meta.len() != num_envs * ACTION_SPACE_SIZE * ACTION_META_WIDTH {
            anyhow::bail!("legal action meta buffer size mismatch");
        }
        let mut cursor = 0usize;
        for env in &self.envs {
            for &action_id in env.action_ids_cache() {
                let Some(row) = action_meta_for_id(action_id as usize) else {
                    meta[cursor * ACTION_META_WIDTH
                        ..cursor * ACTION_META_WIDTH + ACTION_META_WIDTH]
                        .copy_from_slice(&[ACTION_META_UNUSED; ACTION_META_WIDTH]);
                    cursor += 1;
                    continue;
                };
                meta[cursor * ACTION_META_WIDTH..cursor * ACTION_META_WIDTH + ACTION_META_WIDTH]
                    .copy_from_slice(&row);
                cursor += 1;
            }
        }
        Ok(cursor)
    }

    /// Fill optional per-legal-row context for all envs.
    pub fn legal_action_context_v1_batch_into(&self, context: &mut [i32]) -> Result<usize> {
        let num_envs = self.envs.len();
        if context.len() != num_envs * ACTION_SPACE_SIZE * LEGAL_ACTION_CONTEXT_V1_WIDTH {
            anyhow::bail!("legal action context buffer size mismatch");
        }
        let mut cursor = 0usize;
        for env in &self.envs {
            for &action_id in env.action_ids_cache() {
                let row_offset = cursor * LEGAL_ACTION_CONTEXT_V1_WIDTH;
                fill_legal_action_context_row(
                    env,
                    action_id,
                    &mut context[row_offset..row_offset + LEGAL_ACTION_CONTEXT_V1_WIDTH],
                );
                cursor += 1;
            }
        }
        Ok(cursor)
    }

    /// Choose deterministic public-only heuristic actions for the selected env rows.
    pub fn choose_heuristic_public_actions_into(
        &mut self,
        env_indices: &[usize],
        out: &mut [u16],
    ) -> Result<()> {
        self.choose_heuristic_public_profile_actions_into(env_indices, out, "base")
    }

    /// Choose deterministic public-only heuristic actions for the selected env rows using a named profile.
    pub fn choose_heuristic_public_profile_actions_into(
        &mut self,
        env_indices: &[usize],
        out: &mut [u16],
        profile_name: &str,
    ) -> Result<()> {
        if env_indices.len() != out.len() {
            anyhow::bail!("output length must match env_indices length");
        }
        let profile = HeuristicPublicProfile::from_name(profile_name)?;
        for (slot, &env_index) in env_indices.iter().enumerate() {
            let Some(env) = self.envs.get_mut(env_index) else {
                anyhow::bail!("env_index {env_index} out of bounds");
            };
            out[slot] = env.choose_heuristic_public_action_id_for_profile(profile);
        }
        Ok(())
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

fn decision_kind_code(kind: DecisionKind) -> i32 {
    match kind {
        DecisionKind::Mulligan => 0,
        DecisionKind::Clock => 1,
        DecisionKind::Main => 2,
        DecisionKind::Climax => 3,
        DecisionKind::AttackDeclaration => 4,
        DecisionKind::LevelUp => 5,
        DecisionKind::Encore => 6,
        DecisionKind::TriggerOrder => 7,
        DecisionKind::Choice => 8,
    }
}

fn card_type_code(card_type: CardType) -> i32 {
    match card_type {
        CardType::Character => 0,
        CardType::Event => 1,
        CardType::Climax => 2,
    }
}

fn color_code(color: CardColor) -> i32 {
    match color {
        CardColor::Yellow => 0,
        CardColor::Green => 1,
        CardColor::Red => 2,
        CardColor::Blue => 3,
        CardColor::Colorless => 4,
    }
}

fn choice_zone_code(zone: ChoiceZone) -> i32 {
    match zone {
        ChoiceZone::WaitingRoom => CONTEXT_ZONE_WAITING_ROOM,
        ChoiceZone::Stage => CONTEXT_ZONE_STAGE,
        ChoiceZone::Hand => CONTEXT_ZONE_HAND,
        ChoiceZone::DeckTop => CONTEXT_ZONE_DECK_TOP,
        ChoiceZone::Clock => CONTEXT_ZONE_CLOCK,
        ChoiceZone::Level => CONTEXT_ZONE_LEVEL,
        ChoiceZone::Stock => CONTEXT_ZONE_STOCK,
        ChoiceZone::Memory => CONTEXT_ZONE_MEMORY,
        ChoiceZone::Climax => CONTEXT_ZONE_CLIMAX,
        ChoiceZone::Resolution => CONTEXT_ZONE_RESOLUTION,
        ChoiceZone::Stack | ChoiceZone::PriorityCounter | ChoiceZone::PriorityAct => {
            CONTEXT_ZONE_CHOICE
        }
        ChoiceZone::PriorityPass | ChoiceZone::Skip => CONTEXT_ZONE_NONE,
    }
}

fn card_id_to_i32(card_id: CardId) -> i32 {
    i32::try_from(card_id).unwrap_or(i32::MAX)
}

fn set_card_fields(row: &mut [i32], env: &crate::env::GameEnv, card_id: Option<CardId>) {
    let Some(card_id) = card_id else {
        return;
    };
    if card_id == 0 || !env.db.is_valid_id(card_id) {
        return;
    }
    row[8] = card_id_to_i32(card_id);
    row[9] = card_type_code(env.db.card_type_by_id(card_id));
    row[10] = color_code(env.db.color_by_id(card_id));
    row[11] = i32::from(env.db.level_by_id(card_id));
    row[12] = i32::from(env.db.cost_by_id(card_id));
    row[13] = env.db.power_by_id(card_id);
    row[14] = i32::from(env.db.soul_by_id(card_id));
}

fn opponent_seat(seat: u8) -> u8 {
    match seat {
        0 => 1,
        1 => 0,
        _ => seat,
    }
}

fn choice_option_owner_for_context(env: &crate::env::GameEnv, choice: &ChoiceState) -> u8 {
    if choice.reason != ChoiceReason::TargetSelect {
        return choice.player;
    }
    let Some(selection) = env.state.turn.target_selection.as_ref() else {
        return choice.player;
    };
    match selection.spec.side {
        TargetSide::SelfSide => selection.controller,
        TargetSide::Opponent => opponent_seat(selection.controller),
    }
}

fn choice_option_zone_hidden_for_opponent(
    env: &crate::env::GameEnv,
    option: &ChoiceOptionRef,
) -> bool {
    matches!(
        option.zone,
        ChoiceZone::Hand | ChoiceZone::DeckTop | ChoiceZone::Stock | ChoiceZone::PriorityCounter
    ) || (option.zone == ChoiceZone::Memory && !env.curriculum.memory_is_public)
}

fn choice_option_source_for_actor(
    env: &crate::env::GameEnv,
    actor: usize,
    choice: &ChoiceState,
    page_index: usize,
    option: &ChoiceOptionRef,
) -> (i32, i32, Option<CardId>) {
    let zone = choice_zone_code(option.zone);
    let owner = choice_option_owner_for_context(env, choice);
    if actor as u8 != owner && choice_option_zone_hidden_for_opponent(env, option) {
        return (zone, LEGAL_ACTION_CONTEXT_UNUSED, None);
    }
    (
        zone,
        option.index.map(i32::from).unwrap_or(page_index as i32),
        (option.card_id != 0).then_some(option.card_id),
    )
}

fn source_for_action(
    env: &crate::env::GameEnv,
    actor: usize,
    action: &ActionDesc,
) -> (i32, i32, Option<CardId>) {
    match action {
        ActionDesc::MulliganSelect { hand_index }
        | ActionDesc::Clock { hand_index }
        | ActionDesc::MainPlayEvent { hand_index }
        | ActionDesc::ClimaxPlay { hand_index }
        | ActionDesc::CounterPlay { hand_index } => {
            let idx = *hand_index as usize;
            let card_id = env.state.players[actor].hand.get(idx).map(|card| card.id);
            (CONTEXT_ZONE_HAND, idx as i32, card_id)
        }
        ActionDesc::MainPlayCharacter {
            hand_index,
            stage_slot: _,
        } => {
            let idx = *hand_index as usize;
            let card_id = env.state.players[actor].hand.get(idx).map(|card| card.id);
            (CONTEXT_ZONE_HAND, idx as i32, card_id)
        }
        ActionDesc::MainMove { from_slot, .. } => {
            let idx = *from_slot as usize;
            let card_id = env.state.players[actor].stage[idx].card.map(|card| card.id);
            (CONTEXT_ZONE_STAGE, idx as i32, card_id)
        }
        ActionDesc::MainActivateAbility { slot, .. }
        | ActionDesc::Attack { slot, .. }
        | ActionDesc::EncorePay { slot }
        | ActionDesc::EncoreDecline { slot } => {
            let idx = *slot as usize;
            let card_id = env.state.players[actor].stage[idx].card.map(|card| card.id);
            (CONTEXT_ZONE_STAGE, idx as i32, card_id)
        }
        ActionDesc::LevelUp { index } => {
            let idx = *index as usize;
            let card_id = env.state.players[actor].clock.get(idx).map(|card| card.id);
            (CONTEXT_ZONE_CLOCK, idx as i32, card_id)
        }
        ActionDesc::ChoiceSelect { index } => {
            let idx = *index as usize;
            if let Some(choice) = env.state.turn.choice.as_ref() {
                if let Some(option) = choice.options.get(idx) {
                    return choice_option_source_for_actor(env, actor, choice, idx, option);
                }
            }
            (CONTEXT_ZONE_CHOICE, idx as i32, None)
        }
        ActionDesc::TriggerOrder { index } => (CONTEXT_ZONE_CHOICE, i32::from(*index), None),
        ActionDesc::MulliganConfirm
        | ActionDesc::Pass
        | ActionDesc::ChoicePrevPage
        | ActionDesc::ChoiceNextPage
        | ActionDesc::Concede => (CONTEXT_ZONE_NONE, LEGAL_ACTION_CONTEXT_UNUSED, None),
    }
}

fn fill_legal_action_context_row(env: &crate::env::GameEnv, action_id: u16, row: &mut [i32]) {
    row.fill(LEGAL_ACTION_CONTEXT_UNUSED);
    let meta =
        action_meta_for_id(action_id as usize).unwrap_or([ACTION_META_UNUSED; ACTION_META_WIDTH]);
    for (dst, &value) in row.iter_mut().take(ACTION_META_WIDTH).zip(meta.iter()) {
        *dst = if value == ACTION_META_UNUSED {
            LEGAL_ACTION_CONTEXT_UNUSED
        } else {
            i32::from(value)
        };
    }
    if let Some(decision) = env.decision.as_ref() {
        row[4] = decision_kind_code(decision.kind);
        row[5] = i32::from(decision.player);
        if let Some(action) = crate::encode::action_desc_for_id(action_id as usize) {
            let (source_zone, source_index, card_id) =
                source_for_action(env, decision.player as usize, &action);
            row[6] = source_zone;
            row[7] = source_index;
            set_card_fields(row, env, card_id);
        }
    }
}
