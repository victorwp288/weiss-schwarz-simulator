use crate::config::ObservationVisibility;
use crate::encode::{
    encode_obs_context, encode_obs_header, encode_obs_player_block_into, encode_obs_reason,
    encode_obs_reveal, encode_observation_with_slot_power,
};

use super::{GameEnv, StepOutcome};

impl GameEnv {
    pub(super) fn touch_player_obs(&mut self, player: u8) {
        let p = player as usize;
        if p < 2 {
            self.player_obs_version[p] = self.player_obs_version[p].wrapping_add(1);
        }
    }

    pub(super) fn refresh_player_block_cache(&mut self, player: u8) {
        let p = player as usize;
        if p >= 2 {
            return;
        }
        let visibility = self.config.observation_visibility;
        let memory_self = true;
        let memory_opp =
            visibility == ObservationVisibility::Full || self.curriculum.memory_is_public;
        let hand_self = true;
        let hand_opp = visibility == ObservationVisibility::Full;
        let stock_self = true;
        let stock_opp = visibility == ObservationVisibility::Full;
        let deck_self = true;
        let deck_opp = visibility == ObservationVisibility::Full;
        encode_obs_player_block_into(
            &self.state,
            &self.db,
            player,
            memory_self,
            hand_self,
            stock_self,
            deck_self,
            &self.slot_power_cache,
            &mut self.player_block_cache_self[p],
        );
        encode_obs_player_block_into(
            &self.state,
            &self.db,
            player,
            memory_opp,
            hand_opp,
            stock_opp,
            deck_opp,
            &self.slot_power_cache,
            &mut self.player_block_cache_opp[p],
        );
        self.player_block_cache_version[p] = self.player_obs_version[p];
    }

    pub(crate) fn build_outcome_no_copy(&mut self, reward: f32) -> StepOutcome {
        self.build_outcome_with_obs(reward, false)
    }

    pub(crate) fn build_outcome_with_obs(&mut self, reward: f32, copy_obs: bool) -> StepOutcome {
        let perspective = self
            .decision
            .as_ref()
            .map(|d| d.player)
            .unwrap_or(self.last_perspective);
        let (p0, p1) = match perspective {
            0 => (0usize, 1usize),
            1 => (1usize, 0usize),
            _ => {
                debug_assert!(perspective < 2, "invalid perspective");
                (0usize, 1usize)
            }
        };
        let perspective_changed = self.obs_perspective != perspective;
        let p0_dirty = self.player_block_cache_version[p0] != self.player_obs_version[p0];
        let p1_dirty = self.player_block_cache_version[p1] != self.player_obs_version[p1];
        let full_encode = self.obs_dirty || perspective_changed || (p0_dirty && p1_dirty);
        if full_encode {
            self.refresh_slot_power_cache();
            encode_observation_with_slot_power(
                &self.state,
                &self.db,
                &self.curriculum,
                perspective,
                self.decision.as_ref(),
                self.last_action_desc.as_ref(),
                self.last_action_player,
                self.config.observation_visibility,
                &self.slot_power_cache,
                &mut self.obs_buf,
            );
            self.obs_dirty = false;
            self.obs_perspective = perspective;
            self.refresh_player_block_cache(0);
            self.refresh_player_block_cache(1);
        } else {
            encode_obs_header(
                &self.state,
                perspective,
                self.decision.as_ref(),
                self.last_action_desc.as_ref(),
                self.last_action_player,
                self.config.observation_visibility,
                &mut self.obs_buf,
            );
            encode_obs_reason(
                &self.state,
                &self.db,
                &self.curriculum,
                perspective,
                self.decision.as_ref(),
                &mut self.obs_buf,
            );
            encode_obs_reveal(&self.state, perspective, &mut self.obs_buf);
            encode_obs_context(&self.state, &mut self.obs_buf);
            if p0_dirty || p1_dirty {
                self.refresh_slot_power_cache();
            }
            if p0_dirty {
                self.refresh_player_block_cache(perspective);
            }
            if p1_dirty && !p0_dirty {
                self.refresh_player_block_cache(p1 as u8);
            }
            if p0_dirty || p1_dirty {
                let base0 = crate::encode::OBS_HEADER_LEN;
                let base1 = base0 + crate::encode::PER_PLAYER_BLOCK_LEN;
                self.obs_buf[base0..base0 + crate::encode::PER_PLAYER_BLOCK_LEN]
                    .copy_from_slice(&self.player_block_cache_self[p0]);
                self.obs_buf[base1..base1 + crate::encode::PER_PLAYER_BLOCK_LEN]
                    .copy_from_slice(&self.player_block_cache_opp[p1]);
            }
        }
        let obs = if copy_obs {
            self.obs_buf.clone()
        } else {
            Vec::new()
        };
        let info = super::EnvInfo {
            obs_version: crate::encode::OBS_ENCODING_VERSION,
            action_version: crate::encode::ACTION_ENCODING_VERSION,
            decision_kind: self
                .decision
                .as_ref()
                .map(|d| match d.kind {
                    crate::legal::DecisionKind::Mulligan => 0,
                    crate::legal::DecisionKind::Clock => 1,
                    crate::legal::DecisionKind::Main => 2,
                    crate::legal::DecisionKind::Climax => 3,
                    crate::legal::DecisionKind::AttackDeclaration => 4,
                    crate::legal::DecisionKind::LevelUp => 5,
                    crate::legal::DecisionKind::Encore => 6,
                    crate::legal::DecisionKind::TriggerOrder => 7,
                    crate::legal::DecisionKind::Choice => 8,
                })
                .unwrap_or(crate::encode::DECISION_KIND_NONE),
            current_player: self.decision.as_ref().map(|d| d.player as i8).unwrap_or(-1),
            actor: self.last_perspective as i8,
            decision_count: self.state.turn.decision_count,
            tick_count: self.state.turn.tick_count,
            terminal: self.state.terminal,
            illegal_action: self.last_illegal_action,
            engine_error: self.last_engine_error,
            engine_error_code: self.last_engine_error_code as u8,
        };
        let truncated = matches!(
            self.state.terminal,
            Some(crate::state::TerminalResult::Timeout)
        );
        let terminated = matches!(
            self.state.terminal,
            Some(crate::state::TerminalResult::Win { .. } | crate::state::TerminalResult::Draw)
        );
        StepOutcome {
            obs,
            reward,
            terminated,
            truncated,
            info,
        }
    }
}
