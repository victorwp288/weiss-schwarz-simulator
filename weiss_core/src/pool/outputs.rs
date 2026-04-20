/// Minimal RL batch output, filled in-place.
pub struct BatchOutMinimal<'a> {
    /// Observation buffer (len = num_envs * OBS_LEN).
    pub obs: &'a mut [i32],
    /// Action mask buffer (len = num_envs * ACTION_SPACE_SIZE).
    pub masks: &'a mut [u8],
    /// Reward per env (len = num_envs).
    pub rewards: &'a mut [f32],
    /// Terminal flags per env (len = num_envs).
    pub terminated: &'a mut [bool],
    /// Truncation flags per env (len = num_envs).
    pub truncated: &'a mut [bool],
    /// Actor perspective per env (len = num_envs).
    pub actor: &'a mut [i8],
    /// Decision kind per env (len = num_envs).
    pub decision_kind: &'a mut [i8],
    /// Decision id per env (len = num_envs).
    pub decision_id: &'a mut [u32],
    /// Engine error code per env (len = num_envs).
    pub engine_status: &'a mut [u8],
    /// Encoding spec hash per env (len = num_envs).
    pub spec_hash: &'a mut [u64],
    /// Whether the last action was a main-phase move per env (len = num_envs).
    pub main_move_action: &'a mut [bool],
    /// Whether the last action was a main-phase pass per env (len = num_envs).
    pub main_pass_action: &'a mut [bool],
}

/// Minimal RL batch output with i16 observations, filled in-place.
pub struct BatchOutMinimalI16<'a> {
    /// Observation buffer (len = num_envs * OBS_LEN).
    pub obs: &'a mut [i16],
    /// Action mask buffer (len = num_envs * ACTION_SPACE_SIZE).
    pub masks: &'a mut [u8],
    /// Reward per env (len = num_envs).
    pub rewards: &'a mut [f32],
    /// Terminal flags per env (len = num_envs).
    pub terminated: &'a mut [bool],
    /// Truncation flags per env (len = num_envs).
    pub truncated: &'a mut [bool],
    /// Actor perspective per env (len = num_envs).
    pub actor: &'a mut [i8],
    /// Decision kind per env (len = num_envs).
    pub decision_kind: &'a mut [i8],
    /// Decision id per env (len = num_envs).
    pub decision_id: &'a mut [u32],
    /// Engine error code per env (len = num_envs).
    pub engine_status: &'a mut [u8],
    /// Encoding spec hash per env (len = num_envs).
    pub spec_hash: &'a mut [u64],
    /// Whether the last action was a main-phase move per env (len = num_envs).
    pub main_move_action: &'a mut [bool],
    /// Whether the last action was a main-phase pass per env (len = num_envs).
    pub main_pass_action: &'a mut [bool],
}

/// Minimal RL batch output with i16 observations and legal id lists, filled in-place.
pub struct BatchOutMinimalI16LegalIds<'a> {
    /// Observation buffer (len = num_envs * OBS_LEN).
    pub obs: &'a mut [i16],
    /// Flattened legal action ids (len = num_envs * ACTION_SPACE_SIZE).
    pub legal_ids: &'a mut [u16],
    /// Packed legal-action metadata aligned 1:1 with `legal_ids` (len = rows * 4).
    pub legal_action_meta: &'a mut [u16],
    /// Offsets into `legal_ids` (len = num_envs + 1).
    pub legal_offsets: &'a mut [u32],
    /// Reward per env (len = num_envs).
    pub rewards: &'a mut [f32],
    /// Terminal flags per env (len = num_envs).
    pub terminated: &'a mut [bool],
    /// Truncation flags per env (len = num_envs).
    pub truncated: &'a mut [bool],
    /// Actor perspective per env (len = num_envs).
    pub actor: &'a mut [i8],
    /// Decision kind per env (len = num_envs).
    pub decision_kind: &'a mut [i8],
    /// Decision id per env (len = num_envs).
    pub decision_id: &'a mut [u32],
    /// Engine error code per env (len = num_envs).
    pub engine_status: &'a mut [u8],
    /// Encoding spec hash per env (len = num_envs).
    pub spec_hash: &'a mut [u64],
    /// Whether the last action was a main-phase move per env (len = num_envs).
    pub main_move_action: &'a mut [bool],
    /// Whether the last action was a main-phase pass per env (len = num_envs).
    pub main_pass_action: &'a mut [bool],
}

/// Minimal RL batch output without masks, filled in-place.
pub struct BatchOutMinimalNoMask<'a> {
    /// Observation buffer (len = num_envs * OBS_LEN).
    pub obs: &'a mut [i32],
    /// Reward per env (len = num_envs).
    pub rewards: &'a mut [f32],
    /// Terminal flags per env (len = num_envs).
    pub terminated: &'a mut [bool],
    /// Truncation flags per env (len = num_envs).
    pub truncated: &'a mut [bool],
    /// Actor perspective per env (len = num_envs).
    pub actor: &'a mut [i8],
    /// Decision kind per env (len = num_envs).
    pub decision_kind: &'a mut [i8],
    /// Decision id per env (len = num_envs).
    pub decision_id: &'a mut [u32],
    /// Engine error code per env (len = num_envs).
    pub engine_status: &'a mut [u8],
    /// Encoding spec hash per env (len = num_envs).
    pub spec_hash: &'a mut [u64],
    /// Whether the last action was a main-phase move per env (len = num_envs).
    pub main_move_action: &'a mut [bool],
    /// Whether the last action was a main-phase pass per env (len = num_envs).
    pub main_pass_action: &'a mut [bool],
}

/// Trajectory output with masks, filled in-place.
pub struct BatchOutTrajectory<'a> {
    /// Observation buffer (len = steps * num_envs * OBS_LEN).
    pub obs: &'a mut [i32],
    /// Action mask buffer (len = steps * num_envs * ACTION_SPACE_SIZE).
    pub masks: &'a mut [u8],
    /// Reward per step/env (len = steps * num_envs).
    pub rewards: &'a mut [f32],
    /// Terminal flags per step/env (len = steps * num_envs).
    pub terminated: &'a mut [bool],
    /// Truncation flags per step/env (len = steps * num_envs).
    pub truncated: &'a mut [bool],
    /// Actor perspective per step/env (len = steps * num_envs).
    pub actor: &'a mut [i8],
    /// Decision kind per step/env (len = steps * num_envs).
    pub decision_kind: &'a mut [i8],
    /// Decision id per step/env (len = steps * num_envs).
    pub decision_id: &'a mut [u32],
    /// Engine error code per step/env (len = steps * num_envs).
    pub engine_status: &'a mut [u8],
    /// Encoding spec hash per step/env (len = steps * num_envs).
    pub spec_hash: &'a mut [u64],
    /// Whether the last action was a main-phase move per step/env.
    pub main_move_action: &'a mut [bool],
    /// Whether the last action was a main-phase pass per step/env.
    pub main_pass_action: &'a mut [bool],
    /// Actions applied at each step (len = steps * num_envs).
    pub actions: &'a mut [u32],
}

/// Trajectory output with masks and i16 observations, filled in-place.
pub struct BatchOutTrajectoryI16<'a> {
    /// Observation buffer (len = steps * num_envs * OBS_LEN).
    pub obs: &'a mut [i16],
    /// Action mask buffer (len = steps * num_envs * ACTION_SPACE_SIZE).
    pub masks: &'a mut [u8],
    /// Reward per step/env (len = steps * num_envs).
    pub rewards: &'a mut [f32],
    /// Terminal flags per step/env (len = steps * num_envs).
    pub terminated: &'a mut [bool],
    /// Truncation flags per step/env (len = steps * num_envs).
    pub truncated: &'a mut [bool],
    /// Actor perspective per step/env (len = steps * num_envs).
    pub actor: &'a mut [i8],
    /// Decision kind per step/env (len = steps * num_envs).
    pub decision_kind: &'a mut [i8],
    /// Decision id per step/env (len = steps * num_envs).
    pub decision_id: &'a mut [u32],
    /// Engine error code per step/env (len = steps * num_envs).
    pub engine_status: &'a mut [u8],
    /// Encoding spec hash per step/env (len = steps * num_envs).
    pub spec_hash: &'a mut [u64],
    /// Whether the last action was a main-phase move per step/env.
    pub main_move_action: &'a mut [bool],
    /// Whether the last action was a main-phase pass per step/env.
    pub main_pass_action: &'a mut [bool],
    /// Actions applied at each step (len = steps * num_envs).
    pub actions: &'a mut [u32],
}

/// Trajectory output with i16 observations and legal id lists, filled in-place.
pub struct BatchOutTrajectoryI16LegalIds<'a> {
    /// Observation buffer (len = steps * num_envs * OBS_LEN).
    pub obs: &'a mut [i16],
    /// Flattened legal action ids (len = steps * num_envs * ACTION_SPACE_SIZE).
    pub legal_ids: &'a mut [u16],
    /// Packed legal-action metadata aligned 1:1 with `legal_ids` (len = rows * 4).
    pub legal_action_meta: &'a mut [u16],
    /// Offsets into `legal_ids` (len = steps * num_envs + 1).
    pub legal_offsets: &'a mut [u32],
    /// Reward per step/env (len = steps * num_envs).
    pub rewards: &'a mut [f32],
    /// Terminal flags per step/env (len = steps * num_envs).
    pub terminated: &'a mut [bool],
    /// Truncation flags per step/env (len = steps * num_envs).
    pub truncated: &'a mut [bool],
    /// Actor perspective per step/env (len = steps * num_envs).
    pub actor: &'a mut [i8],
    /// Decision kind per step/env (len = steps * num_envs).
    pub decision_kind: &'a mut [i8],
    /// Decision id per step/env (len = steps * num_envs).
    pub decision_id: &'a mut [u32],
    /// Engine error code per step/env (len = steps * num_envs).
    pub engine_status: &'a mut [u8],
    /// Episode seed per step/env (len = steps * num_envs).
    pub episode_seed: &'a mut [u64],
    /// Encoding spec hash per step/env (len = steps * num_envs).
    pub spec_hash: &'a mut [u64],
    /// Whether the last action was a main-phase move per step/env.
    pub main_move_action: &'a mut [bool],
    /// Whether the last action was a main-phase pass per step/env.
    pub main_pass_action: &'a mut [bool],
    /// Actions applied at each step (len = steps * num_envs).
    pub actions: &'a mut [u32],
}

/// Trajectory output without masks, filled in-place.
pub struct BatchOutTrajectoryNoMask<'a> {
    /// Observation buffer (len = steps * num_envs * OBS_LEN).
    pub obs: &'a mut [i32],
    /// Reward per step/env (len = steps * num_envs).
    pub rewards: &'a mut [f32],
    /// Terminal flags per step/env (len = steps * num_envs).
    pub terminated: &'a mut [bool],
    /// Truncation flags per step/env (len = steps * num_envs).
    pub truncated: &'a mut [bool],
    /// Actor perspective per step/env (len = steps * num_envs).
    pub actor: &'a mut [i8],
    /// Decision kind per step/env (len = steps * num_envs).
    pub decision_kind: &'a mut [i8],
    /// Decision id per step/env (len = steps * num_envs).
    pub decision_id: &'a mut [u32],
    /// Engine error code per step/env (len = steps * num_envs).
    pub engine_status: &'a mut [u8],
    /// Encoding spec hash per step/env (len = steps * num_envs).
    pub spec_hash: &'a mut [u64],
    /// Whether the last action was a main-phase move per step/env.
    pub main_move_action: &'a mut [bool],
    /// Whether the last action was a main-phase pass per step/env.
    pub main_pass_action: &'a mut [bool],
    /// Actions applied at each step (len = steps * num_envs).
    pub actions: &'a mut [u32],
}

/// Debug batch output, filled in-place.
pub struct BatchOutDebug<'a> {
    /// Minimal outputs for the batch.
    pub minimal: BatchOutMinimal<'a>,
    /// State fingerprint per env.
    pub state_fingerprint: &'a mut [u64],
    /// Event fingerprint per env.
    pub events_fingerprint: &'a mut [u64],
    /// Mask fingerprint per env.
    pub mask_fingerprint: &'a mut [u64],
    /// Count of debug events per env.
    pub event_counts: &'a mut [u16],
    /// Flattened debug event codes (len = num_envs * event_capacity).
    pub event_codes: &'a mut [u32],
}
