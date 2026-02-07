use super::super::GameEnv;
use crate::encode::{action_id_for, ACTION_ENCODING_VERSION, OBS_ENCODING_VERSION};
use crate::events::Event;
use crate::legal::ActionDesc;
use crate::replay::{
    EpisodeBody, EpisodeHeader, ReplayData, ReplayEvent, ReplayFinal, ReplayVisibilityMode,
    REPLAY_ACTION_ID_UNKNOWN, REPLAY_SCHEMA_VERSION,
};
use crate::state::TerminalResult;

impl GameEnv {
    pub(in crate::env) fn log_event(&mut self, event: Event) {
        if self.recording {
            let ctx = self.replay_visibility_context();
            self.canonical_events.push(event.clone());
            let replay_event = self.sanitize_event_for_viewer(&event, ctx);
            self.replay_events.push(replay_event);
        }
        if self.debug_event_ring.is_some() {
            let mut sanitized = [None, None];
            for viewer in 0..2u8 {
                let ctx = self.debug_visibility_context(viewer);
                sanitized[viewer as usize] = Some(self.sanitize_event_for_viewer(&event, ctx));
            }
            if let Some(rings) = self.debug_event_ring.as_mut() {
                for viewer in 0..2u8 {
                    if let Some(entry) = sanitized[viewer as usize].take() {
                        rings[viewer as usize].push(entry);
                    }
                }
            }
        }
    }

    pub(in crate::env) fn log_action(&mut self, actor: u8, action: ActionDesc) {
        if !self.recording || !self.replay_config.store_actions {
            return;
        }
        let ctx = self.replay_visibility_context();
        self.replay_actions_raw.push(action.clone());
        let logged = self.sanitize_action_for_viewer(&action, actor, ctx);
        self.replay_actions.push(logged);
        let raw_id = action_id_for(&action)
            .and_then(|id| u16::try_from(id).ok())
            .unwrap_or(REPLAY_ACTION_ID_UNKNOWN);
        let public_id = action_id_for(self.replay_actions.last().expect("logged action"))
            .and_then(|id| u16::try_from(id).ok())
            .unwrap_or(REPLAY_ACTION_ID_UNKNOWN);
        self.replay_action_ids_raw.push(raw_id);
        self.replay_action_ids.push(public_id);
    }

    /// Finalize and flush replay output for the current episode.
    pub fn finish_episode_replay(&mut self) {
        if !self.recording {
            return;
        }
        if self.state.terminal.is_some() {
            let need_terminal = !self
                .replay_events
                .iter()
                .any(|e| matches!(e, ReplayEvent::Terminal { .. }));
            if need_terminal {
                let winner = match self.state.terminal {
                    Some(TerminalResult::Win { winner }) => Some(winner),
                    Some(TerminalResult::Draw | TerminalResult::Timeout) => None,
                    None => None,
                };
                self.log_event(Event::Terminal { winner });
            }
        }
        let writer = self.replay_writer.clone();
        if let Some(writer) = writer {
            let header = EpisodeHeader {
                obs_version: OBS_ENCODING_VERSION,
                action_version: ACTION_ENCODING_VERSION,
                replay_version: REPLAY_SCHEMA_VERSION,
                seed: self.episode_seed,
                base_seed: self.base_seed,
                episode_seed: self.episode_seed,
                spec_hash: crate::encode::SPEC_HASH,
                starting_player: self.state.turn.starting_player,
                deck_ids: self.config.deck_ids,
                curriculum_id: "default".to_string(),
                config_hash: self.config.config_hash(&self.curriculum),
                fingerprint_algo: crate::fingerprint::FINGERPRINT_ALGO.to_string(),
                env_id: self.env_id,
                episode_index: self.episode_index,
            };
            let (actions, action_ids, events) = match self.replay_config.visibility_mode {
                ReplayVisibilityMode::Full => (
                    if self.replay_config.store_actions {
                        self.replay_actions_raw.clone()
                    } else {
                        Vec::new()
                    },
                    if self.replay_config.store_actions {
                        self.replay_action_ids_raw.clone()
                    } else {
                        Vec::new()
                    },
                    Some(self.canonical_events.clone()),
                ),
                ReplayVisibilityMode::Public => (
                    if self.replay_config.store_actions {
                        self.replay_actions.clone()
                    } else {
                        Vec::new()
                    },
                    if self.replay_config.store_actions {
                        self.replay_action_ids.clone()
                    } else {
                        Vec::new()
                    },
                    Some(self.replay_events.clone()),
                ),
            };
            let body = EpisodeBody {
                actions,
                action_ids,
                events,
                steps: self.replay_steps.clone(),
                final_state: Some(ReplayFinal {
                    terminal: self.state.terminal,
                    state_hash: crate::fingerprint::state_fingerprint(&self.state),
                    decision_count: self.state.turn.decision_count,
                    tick_count: self.state.turn.tick_count,
                }),
            };
            if let Err(err) = writer.send(ReplayData { header, body }) {
                eprintln!("Replay enqueue failed: {err}");
            }
        }
        self.recording = false;
    }
}
