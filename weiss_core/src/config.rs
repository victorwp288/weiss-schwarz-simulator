use crate::db::{CardDb, CardId, CardType};
use crate::error::ConfigError;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// Policy for handling illegal actions or engine errors during stepping.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Hash, Default)]
pub enum ErrorPolicy {
    /// Return an error to the caller and preserve strict correctness.
    Strict,
    #[default]
    /// Convert errors into a terminal loss for the acting player.
    LenientTerminate,
    /// Ignore the illegal action and return a no-op outcome.
    LenientNoop,
}

/// Visibility policy for observations.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Hash, Default)]
pub enum ObservationVisibility {
    #[default]
    /// Hide private information and sanitize hidden zones.
    Public,
    /// Expose full state without sanitization.
    Full,
}

/// Reward shaping configuration for RL training.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RewardConfig {
    /// Reward for winning the episode.
    pub terminal_win: f32,
    /// Reward for losing the episode.
    pub terminal_loss: f32,
    /// Reward for a draw or timeout.
    pub terminal_draw: f32,
    /// Whether to include shaping rewards during the episode.
    pub enable_shaping: bool,
    /// Per-damage shaping reward (scaled by damage dealt).
    pub damage_reward: f32,
}

impl Default for RewardConfig {
    fn default() -> Self {
        Self {
            terminal_win: 1.0,
            terminal_loss: -1.0,
            terminal_draw: 0.0,
            enable_shaping: false,
            damage_reward: 0.1,
        }
    }
}

impl RewardConfig {
    /// Validate that terminal rewards are finite and zero-sum.
    pub fn validate_zero_sum(&self) -> Result<(), String> {
        const EPS: f32 = 1e-6;
        if !self.terminal_win.is_finite()
            || !self.terminal_loss.is_finite()
            || !self.terminal_draw.is_finite()
        {
            return Err(format!(
                "terminal rewards must be finite (terminal_win={}, terminal_loss={}, terminal_draw={})",
                self.terminal_win, self.terminal_loss, self.terminal_draw
            ));
        }
        let terminal_sum = self.terminal_win + self.terminal_loss;
        if terminal_sum.abs() > EPS {
            return Err(format!(
                "terminal rewards must be zero-sum (terminal_win + terminal_loss = {terminal_sum})"
            ));
        }
        if self.terminal_draw.abs() > EPS {
            return Err(format!(
                "terminal_draw must be 0 for zero-sum (terminal_draw = {})",
                self.terminal_draw
            ));
        }
        Ok(())
    }
}

/// Top-level environment configuration shared by all envs in a pool.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EnvConfig {
    /// Deck lists for both players, as card IDs.
    pub deck_lists: [Vec<CardId>; 2],
    /// Deck identifiers for replay metadata.
    pub deck_ids: [u32; 2],
    /// Max number of decisions before truncation.
    pub max_decisions: u32,
    /// Max number of engine ticks before truncation.
    pub max_ticks: u32,
    /// Reward shaping settings.
    pub reward: RewardConfig,
    #[serde(default)]
    /// Policy for illegal actions and engine errors.
    pub error_policy: ErrorPolicy,
    #[serde(default)]
    /// Observation sanitization policy.
    pub observation_visibility: ObservationVisibility,
    #[serde(default)]
    /// End-condition rules for simultaneous losses.
    pub end_condition_policy: EndConditionPolicy,
}

impl EnvConfig {
    /// Compute a stable hash for this config and curriculum pair.
    pub fn config_hash(&self, curriculum: &CurriculumConfig) -> u64 {
        crate::fingerprint::config_fingerprint(self, curriculum)
    }

    /// Validate deck lists against hard constraints and collect all issues.
    pub fn validate_with_db_all_issues(&self, db: &CardDb) -> Vec<ConfigError> {
        let mut issues: Vec<ConfigError> = Vec::new();
        for (player, deck) in self.deck_lists.iter().enumerate() {
            if deck.len() != crate::encode::MAX_DECK {
                issues.push(ConfigError::DeckLength {
                    player: player as u8,
                    got: deck.len(),
                    expected: crate::encode::MAX_DECK,
                });
            }
            let mut climax_count = 0usize;
            let mut counts: std::collections::HashMap<CardId, usize> =
                std::collections::HashMap::new();
            let mut seen_unknown: std::collections::HashSet<CardId> =
                std::collections::HashSet::new();
            for &card_id in deck {
                let Some(card) = db.get(card_id) else {
                    if seen_unknown.insert(card_id) {
                        issues.push(ConfigError::UnknownCardId {
                            player: player as u8,
                            card_id,
                        });
                    }
                    continue;
                };
                if card.card_type == CardType::Climax {
                    climax_count += 1;
                }
                *counts.entry(card_id).or_insert(0) += 1;
            }
            if climax_count > 8 {
                issues.push(ConfigError::ClimaxCount {
                    player: player as u8,
                    got: climax_count,
                    max: 8,
                });
            }
            let mut excessive: Vec<(CardId, usize)> =
                counts.into_iter().filter(|(_, count)| *count > 4).collect();
            excessive.sort_by_key(|(card_id, _)| *card_id);
            for (card_id, count) in excessive {
                issues.push(ConfigError::CardCopyCount {
                    player: player as u8,
                    card_id,
                    got: count,
                    max: 4,
                });
            }
        }
        issues
    }

    /// Validate deck lists against hard game constraints and known card ids.
    pub fn validate_with_db(&self, db: &CardDb) -> Result<(), ConfigError> {
        if let Some(issue) = self.validate_with_db_all_issues(db).into_iter().next() {
            return Err(issue);
        }
        Ok(())
    }
}

/// Policy for resolving simultaneous loss conditions.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Hash, Default)]
pub enum SimultaneousLossPolicy {
    /// Active player wins when both players would lose.
    ActivePlayerWins,
    /// Non-active player wins when both players would lose.
    NonActivePlayerWins,
    #[default]
    /// Treat simultaneous loss as a draw.
    Draw,
}

/// End-condition behavior for edge cases such as simultaneous loss.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct EndConditionPolicy {
    #[serde(default)]
    /// Winner selection strategy for simultaneous losses.
    pub simultaneous_loss: SimultaneousLossPolicy,
    #[serde(default = "default_true")]
    /// Allow a draw when simultaneous losses occur.
    pub allow_draw_on_simultaneous_loss: bool,
}

impl Default for EndConditionPolicy {
    fn default() -> Self {
        Self {
            simultaneous_loss: SimultaneousLossPolicy::Draw,
            allow_draw_on_simultaneous_loss: true,
        }
    }
}

/// Curriculum toggles for enabling/disabling engine subsystems.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CurriculumConfig {
    #[serde(default)]
    /// Optional whitelist of allowed card set identifiers.
    pub allowed_card_sets: Vec<String>,
    #[serde(default = "default_true")]
    /// Allow character cards to be played.
    pub allow_character: bool,
    #[serde(default = "default_true")]
    /// Allow event cards to be played.
    pub allow_event: bool,
    #[serde(default = "default_true")]
    /// Allow climax cards to be played.
    pub allow_climax: bool,
    #[serde(default = "default_true")]
    /// Enable the clock phase.
    pub enable_clock_phase: bool,
    #[serde(default = "default_true")]
    /// Enable the climax phase.
    pub enable_climax_phase: bool,
    #[serde(default = "default_true")]
    /// Enable side attacks.
    pub enable_side_attacks: bool,
    #[serde(default = "default_true")]
    /// Enable direct attacks.
    pub enable_direct_attacks: bool,
    #[serde(default = "default_true")]
    /// Enable counter play.
    pub enable_counters: bool,
    #[serde(default = "default_true")]
    /// Enable trigger checks.
    pub enable_triggers: bool,
    #[serde(default = "default_true")]
    /// Enable soul trigger effect.
    pub enable_trigger_soul: bool,
    #[serde(default = "default_true")]
    /// Enable draw trigger effect.
    pub enable_trigger_draw: bool,
    #[serde(default = "default_true")]
    /// Enable shot trigger effect.
    pub enable_trigger_shot: bool,
    #[serde(default = "default_true")]
    /// Enable bounce trigger effect.
    pub enable_trigger_bounce: bool,
    #[serde(default = "default_true")]
    /// Enable treasure trigger effect.
    pub enable_trigger_treasure: bool,
    #[serde(default = "default_true")]
    /// Enable gate trigger effect.
    pub enable_trigger_gate: bool,
    #[serde(default = "default_true")]
    /// Enable standby trigger effect.
    pub enable_trigger_standby: bool,
    #[serde(default = "default_true")]
    /// Enable on-reverse triggers.
    pub enable_on_reverse_triggers: bool,
    #[serde(default = "default_true")]
    /// Enable backup effects.
    pub enable_backup: bool,
    #[serde(default = "default_true")]
    /// Enable encore step.
    pub enable_encore: bool,
    #[serde(default = "default_true")]
    /// Enable refresh penalty on deck refresh.
    pub enable_refresh_penalty: bool,
    #[serde(default = "default_true")]
    /// Enable level-up choice step.
    pub enable_level_up_choice: bool,
    #[serde(default = "default_true")]
    /// Enable activated abilities.
    pub enable_activated_abilities: bool,
    #[serde(default = "default_true")]
    /// Enable continuous modifiers.
    pub enable_continuous_modifiers: bool,
    #[serde(default)]
    /// Enable approximated non-combat effects listed in docs/approximation_policy.md.
    pub enable_approx_effects: bool,
    #[serde(default)]
    /// Enable explicit priority windows.
    pub enable_priority_windows: bool,
    #[serde(default)]
    /// Enable visibility policies and sanitization.
    pub enable_visibility_policies: bool,
    #[serde(default)]
    /// Use alternate end-condition handling rules.
    pub use_alternate_end_conditions: bool,
    #[serde(default = "default_true")]
    /// Auto-pick when only one action is available in priority.
    pub priority_autopick_single_action: bool,
    #[serde(default = "default_true")]
    /// Allow pass actions during priority windows.
    pub priority_allow_pass: bool,
    #[serde(default)]
    /// Enforce strict priority legality (debug/audit mode).
    pub strict_priority_mode: bool,
    #[serde(default)]
    /// Use legacy fixed ability-cost step ordering.
    pub enable_legacy_cost_order: bool,
    #[serde(default)]
    /// Restrict shot trigger bonus damage to battle-damage cancel timing only.
    pub enable_legacy_shot_damage_step_only: bool,
    #[serde(default)]
    /// Reduce stage size for curriculum experiments.
    pub reduced_stage_mode: bool,
    #[serde(default = "default_true")]
    /// Enforce color requirements on play.
    pub enforce_color_requirement: bool,
    #[serde(default = "default_true")]
    /// Enforce cost requirements on play.
    pub enforce_cost_requirement: bool,
    #[serde(default)]
    /// Allow players to concede.
    pub allow_concede: bool,
    #[serde(default)]
    /// Expose opponent hand/stock counts in public observations.
    pub reveal_opponent_hand_stock_counts: bool,
    #[serde(default = "default_true")]
    /// Treat memory zone as public information.
    pub memory_is_public: bool,
    #[serde(skip)]
    /// Cached set whitelist derived from `allowed_card_sets`.
    pub allowed_card_sets_cache: Option<HashSet<String>>,
}

impl Default for CurriculumConfig {
    fn default() -> Self {
        Self {
            allowed_card_sets: Vec::new(),
            allow_character: true,
            allow_event: true,
            allow_climax: true,
            enable_clock_phase: true,
            enable_climax_phase: true,
            enable_side_attacks: true,
            enable_direct_attacks: true,
            enable_counters: true,
            enable_triggers: true,
            enable_trigger_soul: true,
            enable_trigger_draw: true,
            enable_trigger_shot: true,
            enable_trigger_bounce: true,
            enable_trigger_treasure: true,
            enable_trigger_gate: true,
            enable_trigger_standby: true,
            enable_on_reverse_triggers: true,
            enable_backup: true,
            enable_encore: true,
            enable_refresh_penalty: true,
            enable_level_up_choice: true,
            enable_activated_abilities: true,
            enable_continuous_modifiers: true,
            enable_approx_effects: false,
            enable_priority_windows: false,
            enable_visibility_policies: false,
            use_alternate_end_conditions: false,
            priority_autopick_single_action: true,
            priority_allow_pass: true,
            strict_priority_mode: false,
            enable_legacy_cost_order: false,
            enable_legacy_shot_damage_step_only: false,
            reduced_stage_mode: false,
            enforce_color_requirement: true,
            enforce_cost_requirement: true,
            allow_concede: false,
            reveal_opponent_hand_stock_counts: false,
            memory_is_public: true,
            allowed_card_sets_cache: None,
        }
    }
}

impl CurriculumConfig {
    /// Rebuild derived caches after changing configuration fields.
    pub fn rebuild_cache(&mut self) {
        if self.allowed_card_sets.is_empty() {
            self.allowed_card_sets_cache = None;
        } else {
            self.allowed_card_sets_cache = Some(self.allowed_card_sets.iter().cloned().collect());
        }
    }
}

fn default_true() -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::RewardConfig;

    #[test]
    fn reward_config_zero_sum_defaults_validate() {
        assert!(RewardConfig::default().validate_zero_sum().is_ok());
    }

    #[test]
    fn reward_config_rejects_non_finite_terminal_values() {
        let invalid_configs = [
            RewardConfig {
                terminal_win: f32::NAN,
                ..RewardConfig::default()
            },
            RewardConfig {
                terminal_loss: f32::INFINITY,
                ..RewardConfig::default()
            },
            RewardConfig {
                terminal_draw: f32::NEG_INFINITY,
                ..RewardConfig::default()
            },
        ];

        for cfg in invalid_configs {
            let err = cfg
                .validate_zero_sum()
                .expect_err("non-finite rewards must fail");
            assert!(err.contains("must be finite"), "unexpected error: {err}");
        }
    }

    #[test]
    fn reward_config_rejects_non_zero_sum_win_loss() {
        let cfg = RewardConfig {
            terminal_win: 1.0,
            terminal_loss: -0.75,
            terminal_draw: 0.0,
            ..RewardConfig::default()
        };
        let err = cfg
            .validate_zero_sum()
            .expect_err("non-zero-sum win/loss must fail");
        assert!(err.contains("must be zero-sum"), "unexpected error: {err}");
    }

    #[test]
    fn reward_config_rejects_non_zero_draw() {
        let cfg = RewardConfig {
            terminal_draw: 0.25,
            ..RewardConfig::default()
        };
        let err = cfg
            .validate_zero_sum()
            .expect_err("non-zero draw must fail");
        assert!(
            err.contains("terminal_draw must be 0"),
            "unexpected error: {err}"
        );
    }
}
