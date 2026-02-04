use crate::db::CardId;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Hash, Default)]
pub enum ErrorPolicy {
    Strict,
    #[default]
    LenientTerminate,
    LenientNoop,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Hash, Default)]
pub enum ObservationVisibility {
    #[default]
    Public,
    Full,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RewardConfig {
    pub terminal_win: f32,
    pub terminal_loss: f32,
    pub terminal_draw: f32,
    pub enable_shaping: bool,
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

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EnvConfig {
    pub deck_lists: [Vec<CardId>; 2],
    pub deck_ids: [u32; 2],
    pub max_decisions: u32,
    pub max_ticks: u32,
    pub reward: RewardConfig,
    #[serde(default)]
    pub error_policy: ErrorPolicy,
    #[serde(default)]
    pub observation_visibility: ObservationVisibility,
    #[serde(default)]
    pub end_condition_policy: EndConditionPolicy,
}

impl EnvConfig {
    pub fn config_hash(&self, curriculum: &CurriculumConfig) -> u64 {
        crate::fingerprint::config_fingerprint(self, curriculum)
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Hash, Default)]
pub enum SimultaneousLossPolicy {
    ActivePlayerWins,
    NonActivePlayerWins,
    #[default]
    Draw,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct EndConditionPolicy {
    #[serde(default)]
    pub simultaneous_loss: SimultaneousLossPolicy,
    #[serde(default)]
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

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CurriculumConfig {
    #[serde(default)]
    pub allowed_card_sets: Vec<String>,
    #[serde(default = "default_true")]
    pub allow_character: bool,
    #[serde(default = "default_true")]
    pub allow_event: bool,
    #[serde(default = "default_true")]
    pub allow_climax: bool,
    #[serde(default = "default_true")]
    pub enable_clock_phase: bool,
    #[serde(default = "default_true")]
    pub enable_climax_phase: bool,
    #[serde(default = "default_true")]
    pub enable_side_attacks: bool,
    #[serde(default = "default_true")]
    pub enable_direct_attacks: bool,
    #[serde(default = "default_true")]
    pub enable_counters: bool,
    #[serde(default = "default_true")]
    pub enable_triggers: bool,
    #[serde(default = "default_true")]
    pub enable_trigger_soul: bool,
    #[serde(default = "default_true")]
    pub enable_trigger_draw: bool,
    #[serde(default = "default_true")]
    pub enable_trigger_shot: bool,
    #[serde(default = "default_true")]
    pub enable_trigger_bounce: bool,
    #[serde(default = "default_true")]
    pub enable_trigger_treasure: bool,
    #[serde(default = "default_true")]
    pub enable_trigger_gate: bool,
    #[serde(default = "default_true")]
    pub enable_trigger_standby: bool,
    #[serde(default = "default_true")]
    pub enable_on_reverse_triggers: bool,
    #[serde(default = "default_true")]
    pub enable_backup: bool,
    #[serde(default = "default_true")]
    pub enable_encore: bool,
    #[serde(default = "default_true")]
    pub enable_refresh_penalty: bool,
    #[serde(default = "default_true")]
    pub enable_level_up_choice: bool,
    #[serde(default = "default_true")]
    pub enable_activated_abilities: bool,
    #[serde(default = "default_true")]
    pub enable_continuous_modifiers: bool,
    #[serde(default)]
    pub enable_priority_windows: bool,
    #[serde(default)]
    pub enable_visibility_policies: bool,
    #[serde(default)]
    pub use_alternate_end_conditions: bool,
    #[serde(default = "default_true")]
    pub priority_autopick_single_action: bool,
    #[serde(default = "default_true")]
    pub priority_allow_pass: bool,
    #[serde(default)]
    pub strict_priority_mode: bool,
    #[serde(default)]
    pub reduced_stage_mode: bool,
    #[serde(default = "default_true")]
    pub enforce_color_requirement: bool,
    #[serde(default = "default_true")]
    pub enforce_cost_requirement: bool,
    #[serde(default)]
    pub allow_concede: bool,
    #[serde(default = "default_true")]
    pub memory_is_public: bool,
    #[serde(skip)]
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
            enable_priority_windows: false,
            enable_visibility_policies: false,
            use_alternate_end_conditions: false,
            priority_autopick_single_action: true,
            priority_allow_pass: true,
            strict_priority_mode: false,
            reduced_stage_mode: false,
            enforce_color_requirement: true,
            enforce_cost_requirement: true,
            allow_concede: false,
            memory_is_public: true,
            allowed_card_sets_cache: None,
        }
    }
}

impl CurriculumConfig {
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
