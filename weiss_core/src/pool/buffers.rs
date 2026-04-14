use crate::encode::{ACTION_SPACE_SIZE, OBS_LEN, SPEC_HASH};

use super::outputs::{BatchOutDebug, BatchOutMinimal, BatchOutMinimalI16, BatchOutMinimalNoMask};

/// Owned buffers for minimal output (Rust-side convenience).
#[derive(Clone, Debug)]
pub struct BatchOutMinimalBuffers {
    /// Observation buffer (len = num_envs * OBS_LEN).
    pub obs: Vec<i32>,
    /// Action mask buffer (len = num_envs * ACTION_SPACE_SIZE).
    pub masks: Vec<u8>,
    /// Reward buffer (len = num_envs).
    pub rewards: Vec<f32>,
    /// Terminal flags (len = num_envs).
    pub terminated: Vec<bool>,
    /// Truncation flags (len = num_envs).
    pub truncated: Vec<bool>,
    /// Actor perspective (len = num_envs).
    pub actor: Vec<i8>,
    /// Decision kind (len = num_envs).
    pub decision_kind: Vec<i8>,
    /// Decision id (len = num_envs).
    pub decision_id: Vec<u32>,
    /// Engine status code (len = num_envs).
    pub engine_status: Vec<u8>,
    /// Encoding spec hash (len = num_envs).
    pub spec_hash: Vec<u64>,
    /// Whether the last action was a main-phase move (len = num_envs).
    pub main_move_action: Vec<bool>,
    /// Whether the last action was a main-phase pass (len = num_envs).
    pub main_pass_action: Vec<bool>,
}

/// Owned buffers for minimal output with i16 observations.
#[derive(Clone, Debug)]
pub struct BatchOutMinimalI16Buffers {
    /// Observation buffer (len = num_envs * OBS_LEN).
    pub obs: Vec<i16>,
    /// Action mask buffer (len = num_envs * ACTION_SPACE_SIZE).
    pub masks: Vec<u8>,
    /// Reward buffer (len = num_envs).
    pub rewards: Vec<f32>,
    /// Terminal flags (len = num_envs).
    pub terminated: Vec<bool>,
    /// Truncation flags (len = num_envs).
    pub truncated: Vec<bool>,
    /// Actor perspective (len = num_envs).
    pub actor: Vec<i8>,
    /// Decision kind (len = num_envs).
    pub decision_kind: Vec<i8>,
    /// Decision id (len = num_envs).
    pub decision_id: Vec<u32>,
    /// Engine status code (len = num_envs).
    pub engine_status: Vec<u8>,
    /// Encoding spec hash (len = num_envs).
    pub spec_hash: Vec<u64>,
    /// Whether the last action was a main-phase move (len = num_envs).
    pub main_move_action: Vec<bool>,
    /// Whether the last action was a main-phase pass (len = num_envs).
    pub main_pass_action: Vec<bool>,
}

impl BatchOutMinimalI16Buffers {
    /// Allocate buffers sized for `num_envs`.
    pub fn new(num_envs: usize) -> Self {
        Self {
            obs: vec![0; num_envs * OBS_LEN],
            masks: vec![0u8; num_envs * ACTION_SPACE_SIZE],
            rewards: vec![0.0; num_envs],
            terminated: vec![false; num_envs],
            truncated: vec![false; num_envs],
            actor: vec![0; num_envs],
            decision_kind: vec![crate::encode::DECISION_KIND_NONE; num_envs],
            decision_id: vec![0; num_envs],
            engine_status: vec![0; num_envs],
            spec_hash: vec![SPEC_HASH; num_envs],
            main_move_action: vec![false; num_envs],
            main_pass_action: vec![false; num_envs],
        }
    }

    /// Borrow buffers as a mutable view.
    pub fn view_mut(&mut self) -> BatchOutMinimalI16<'_> {
        BatchOutMinimalI16 {
            obs: &mut self.obs,
            masks: &mut self.masks,
            rewards: &mut self.rewards,
            terminated: &mut self.terminated,
            truncated: &mut self.truncated,
            actor: &mut self.actor,
            decision_kind: &mut self.decision_kind,
            decision_id: &mut self.decision_id,
            engine_status: &mut self.engine_status,
            spec_hash: &mut self.spec_hash,
            main_move_action: &mut self.main_move_action,
            main_pass_action: &mut self.main_pass_action,
        }
    }
}

impl BatchOutMinimalBuffers {
    /// Allocate buffers sized for `num_envs`.
    pub fn new(num_envs: usize) -> Self {
        Self {
            obs: vec![0; num_envs * OBS_LEN],
            masks: vec![0u8; num_envs * ACTION_SPACE_SIZE],
            rewards: vec![0.0; num_envs],
            terminated: vec![false; num_envs],
            truncated: vec![false; num_envs],
            actor: vec![0; num_envs],
            decision_kind: vec![crate::encode::DECISION_KIND_NONE; num_envs],
            decision_id: vec![0; num_envs],
            engine_status: vec![0; num_envs],
            spec_hash: vec![SPEC_HASH; num_envs],
            main_move_action: vec![false; num_envs],
            main_pass_action: vec![false; num_envs],
        }
    }

    /// Borrow buffers as a mutable view.
    pub fn view_mut(&mut self) -> BatchOutMinimal<'_> {
        BatchOutMinimal {
            obs: &mut self.obs,
            masks: &mut self.masks,
            rewards: &mut self.rewards,
            terminated: &mut self.terminated,
            truncated: &mut self.truncated,
            actor: &mut self.actor,
            decision_kind: &mut self.decision_kind,
            decision_id: &mut self.decision_id,
            engine_status: &mut self.engine_status,
            spec_hash: &mut self.spec_hash,
            main_move_action: &mut self.main_move_action,
            main_pass_action: &mut self.main_pass_action,
        }
    }
}

/// Owned buffers for minimal output without masks (Rust-side convenience).
#[derive(Clone, Debug)]
pub struct BatchOutMinimalNoMaskBuffers {
    /// Observation buffer (len = num_envs * OBS_LEN).
    pub obs: Vec<i32>,
    /// Reward buffer (len = num_envs).
    pub rewards: Vec<f32>,
    /// Terminal flags (len = num_envs).
    pub terminated: Vec<bool>,
    /// Truncation flags (len = num_envs).
    pub truncated: Vec<bool>,
    /// Actor perspective (len = num_envs).
    pub actor: Vec<i8>,
    /// Decision kind (len = num_envs).
    pub decision_kind: Vec<i8>,
    /// Decision id (len = num_envs).
    pub decision_id: Vec<u32>,
    /// Engine status code (len = num_envs).
    pub engine_status: Vec<u8>,
    /// Encoding spec hash (len = num_envs).
    pub spec_hash: Vec<u64>,
    /// Whether the last action was a main-phase move (len = num_envs).
    pub main_move_action: Vec<bool>,
    /// Whether the last action was a main-phase pass (len = num_envs).
    pub main_pass_action: Vec<bool>,
}

impl BatchOutMinimalNoMaskBuffers {
    /// Allocate buffers sized for `num_envs`.
    pub fn new(num_envs: usize) -> Self {
        Self {
            obs: vec![0; num_envs * OBS_LEN],
            rewards: vec![0.0; num_envs],
            terminated: vec![false; num_envs],
            truncated: vec![false; num_envs],
            actor: vec![0; num_envs],
            decision_kind: vec![crate::encode::DECISION_KIND_NONE; num_envs],
            decision_id: vec![0; num_envs],
            engine_status: vec![0; num_envs],
            spec_hash: vec![SPEC_HASH; num_envs],
            main_move_action: vec![false; num_envs],
            main_pass_action: vec![false; num_envs],
        }
    }

    /// Borrow buffers as a mutable view.
    pub fn view_mut(&mut self) -> BatchOutMinimalNoMask<'_> {
        BatchOutMinimalNoMask {
            obs: &mut self.obs,
            rewards: &mut self.rewards,
            terminated: &mut self.terminated,
            truncated: &mut self.truncated,
            actor: &mut self.actor,
            decision_kind: &mut self.decision_kind,
            decision_id: &mut self.decision_id,
            engine_status: &mut self.engine_status,
            spec_hash: &mut self.spec_hash,
            main_move_action: &mut self.main_move_action,
            main_pass_action: &mut self.main_pass_action,
        }
    }
}

/// Owned buffers for debug output (Rust-side convenience).
#[derive(Clone, Debug)]
pub struct BatchOutDebugBuffers {
    /// Minimal output buffers.
    pub minimal: BatchOutMinimalBuffers,
    /// State fingerprint buffer (len = num_envs).
    pub state_fingerprint: Vec<u64>,
    /// Event fingerprint buffer (len = num_envs).
    pub events_fingerprint: Vec<u64>,
    /// Mask fingerprint buffer (len = num_envs).
    pub mask_fingerprint: Vec<u64>,
    /// Event count buffer (len = num_envs).
    pub event_counts: Vec<u16>,
    /// Flattened event codes buffer (len = num_envs * event_capacity).
    pub event_codes: Vec<u32>,
}

impl BatchOutDebugBuffers {
    /// Allocate buffers sized for `num_envs` and event capacity.
    pub fn new(num_envs: usize, event_capacity: usize) -> Self {
        Self {
            minimal: BatchOutMinimalBuffers::new(num_envs),
            state_fingerprint: vec![0; num_envs],
            events_fingerprint: vec![0; num_envs],
            mask_fingerprint: vec![0; num_envs],
            event_counts: vec![0; num_envs],
            event_codes: vec![0; num_envs * event_capacity],
        }
    }

    /// Borrow buffers as a mutable view.
    pub fn view_mut(&mut self) -> BatchOutDebug<'_> {
        BatchOutDebug {
            minimal: self.minimal.view_mut(),
            state_fingerprint: &mut self.state_fingerprint,
            events_fingerprint: &mut self.events_fingerprint,
            mask_fingerprint: &mut self.mask_fingerprint,
            event_counts: &mut self.event_counts,
            event_codes: &mut self.event_codes,
        }
    }
}
