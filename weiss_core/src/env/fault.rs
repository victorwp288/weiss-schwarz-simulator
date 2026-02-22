use crate::state::TerminalResult;

use super::{EngineErrorCode, FaultRecord, FaultSource, GameEnv, StepOutcome};

impl GameEnv {
    /// Whether this env currently has a latched runtime fault.
    pub(crate) fn is_fault_latched(&self) -> bool {
        self.fault_latched.is_some()
    }

    /// Perspective associated with the currently latched fault, if any.
    pub(crate) fn fault_actor(&self) -> Option<u8> {
        self.fault_latched.and_then(|r| r.actor)
    }

    /// Return the current latched fault record.
    pub(crate) fn fault_record(&self) -> Option<FaultRecord> {
        self.fault_latched
    }

    /// Build a fault step outcome without cloning observation bytes.
    pub(crate) fn build_fault_step_outcome_no_copy(&mut self) -> StepOutcome {
        self.build_fault_step_outcome(false)
    }

    /// Build a fault step outcome, replaying the latched fault if present.
    pub(crate) fn build_fault_step_outcome(&mut self, copy_obs: bool) -> StepOutcome {
        if let Some(record) = self.fault_latched {
            self.latch_fault(record.code, record.actor, record.source, copy_obs)
        } else {
            self.build_outcome_with_obs(0.0, copy_obs)
        }
    }

    fn apply_fault_record(&mut self, record: FaultRecord) {
        self.last_engine_error = true;
        self.last_engine_error_code = record.code;
        if let Some(a) = record.actor {
            self.last_perspective = a;
        }
        self.state.terminal = Some(TerminalResult::Timeout);
        self.clear_decision();
        self.update_action_cache();
    }

    /// Latch fault metadata now; emit terminal reward on the next fault outcome build.
    pub(crate) fn latch_fault_deferred(
        &mut self,
        code: EngineErrorCode,
        actor: Option<u8>,
        source: FaultSource,
    ) {
        if self.fault_latched.is_some() {
            return;
        }
        let record = FaultRecord {
            code,
            actor,
            fingerprint: self.synthetic_fault_fingerprint(code, actor, source),
            source,
            reward_emitted: false,
        };
        self.fault_latched = Some(record);
        self.apply_fault_record(record);
    }

    /// Latch a fault and return a terminal `StepOutcome`.
    ///
    /// Reward is emitted at most once per latched fault (`reward_emitted`).
    pub(crate) fn latch_fault(
        &mut self,
        code: EngineErrorCode,
        actor: Option<u8>,
        source: FaultSource,
        copy_obs: bool,
    ) -> StepOutcome {
        let reward = if let Some(mut record) = self.fault_latched {
            self.apply_fault_record(record);
            if record.reward_emitted {
                0.0
            } else {
                record.reward_emitted = true;
                self.fault_latched = Some(record);
                record
                    .actor
                    .map(|_| self.config.reward.terminal_loss)
                    .unwrap_or(self.config.reward.terminal_draw)
            }
        } else {
            let record = FaultRecord {
                code,
                actor,
                fingerprint: self.synthetic_fault_fingerprint(code, actor, source),
                source,
                reward_emitted: true,
            };
            self.fault_latched = Some(record);
            self.apply_fault_record(record);
            record
                .actor
                .map(|_| self.config.reward.terminal_loss)
                .unwrap_or(self.config.reward.terminal_draw)
        };
        self.build_outcome_with_obs(reward, copy_obs)
    }

    /// Build a stable fingerprint for fault deduplication and telemetry.
    pub(crate) fn synthetic_fault_fingerprint(
        &self,
        code: EngineErrorCode,
        actor: Option<u8>,
        source: FaultSource,
    ) -> u64 {
        let mut bytes = Vec::with_capacity(32);
        bytes.extend_from_slice(&self.env_id.to_le_bytes());
        bytes.extend_from_slice(&self.episode_index.to_le_bytes());
        bytes.extend_from_slice(&self.episode_seed.to_le_bytes());
        bytes.extend_from_slice(&self.decision_id.to_le_bytes());
        bytes.push(code as u8);
        match actor {
            Some(a) => {
                bytes.push(1);
                bytes.push(a);
            }
            None => bytes.push(0),
        }
        bytes.push(match source {
            FaultSource::Step => 1,
            FaultSource::Reset => 2,
        });
        crate::fingerprint::hash_bytes(&bytes)
    }
}
