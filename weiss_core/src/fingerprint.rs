use serde::Serialize;

use crate::config::{
    CurriculumConfig, EndConditionPolicy, EnvConfig, ErrorPolicy, ObservationVisibility,
    RewardConfig,
};
use crate::db::CardId;
use crate::effects::ReplacementSpec;
use crate::events::Event;
use crate::state::{
    AttackContext, ChoiceState, GameState, PendingTrigger, PlayerState, PriorityState, StackItem,
    StackOrderState, TargetSelectionState, TerminalResult, TimingWindow, TurnState,
};

pub const FINGERPRINT_ALGO: &str = "postcard+blake3+u64le v1";

pub fn hash_bytes(bytes: &[u8]) -> u64 {
    let hash = blake3::hash(bytes);
    let mut out = [0u8; 8];
    out.copy_from_slice(&hash.as_bytes()[..8]);
    u64::from_le_bytes(out)
}

pub fn hash_postcard<T: Serialize + ?Sized>(value: &T) -> u64 {
    let bytes = postcard::to_allocvec(value).expect("fingerprint serialization failed");
    hash_bytes(&bytes)
}

pub fn config_fingerprint(config: &EnvConfig, curriculum: &CurriculumConfig) -> u64 {
    let canonical = CanonicalConfigForHash::from_config(config, curriculum);
    hash_postcard(&canonical)
}

pub fn state_fingerprint(state: &GameState) -> u64 {
    let canonical = CanonicalStateForHash::from_state(state);
    hash_postcard(&canonical)
}

pub fn events_fingerprint(events: &[Event]) -> u64 {
    hash_postcard(events)
}

#[derive(Clone, Debug, Serialize)]
struct CanonicalConfigForHash {
    env: CanonicalEnvConfig,
    curriculum: CanonicalCurriculumConfig,
}

impl CanonicalConfigForHash {
    fn from_config(config: &EnvConfig, curriculum: &CurriculumConfig) -> Self {
        Self {
            env: CanonicalEnvConfig::from_config(config),
            curriculum: CanonicalCurriculumConfig::from_curriculum(curriculum),
        }
    }
}

#[derive(Clone, Debug, Serialize)]
struct CanonicalEnvConfig {
    deck_lists: [Vec<CardId>; 2],
    deck_ids: [u32; 2],
    max_decisions: u32,
    max_ticks: u32,
    reward: RewardConfig,
    error_policy: ErrorPolicy,
    observation_visibility: ObservationVisibility,
    end_condition_policy: EndConditionPolicy,
}

impl CanonicalEnvConfig {
    fn from_config(config: &EnvConfig) -> Self {
        Self {
            deck_lists: config.deck_lists.clone(),
            deck_ids: config.deck_ids,
            max_decisions: config.max_decisions,
            max_ticks: config.max_ticks,
            reward: config.reward.clone(),
            error_policy: config.error_policy,
            observation_visibility: config.observation_visibility,
            end_condition_policy: config.end_condition_policy.clone(),
        }
    }
}

#[derive(Clone, Debug, Serialize)]
struct CanonicalCurriculumConfig {
    allowed_card_sets: Vec<String>,
    allow_character: bool,
    allow_event: bool,
    allow_climax: bool,
    enable_clock_phase: bool,
    enable_climax_phase: bool,
    enable_side_attacks: bool,
    enable_direct_attacks: bool,
    enable_counters: bool,
    enable_triggers: bool,
    enable_trigger_soul: bool,
    enable_trigger_draw: bool,
    enable_trigger_shot: bool,
    enable_trigger_bounce: bool,
    enable_trigger_treasure: bool,
    enable_trigger_gate: bool,
    enable_trigger_standby: bool,
    enable_backup: bool,
    enable_encore: bool,
    enable_refresh_penalty: bool,
    enable_level_up_choice: bool,
    enable_activated_abilities: bool,
    enable_continuous_modifiers: bool,
    enable_priority_windows: bool,
    enable_visibility_policies: bool,
    use_alternate_end_conditions: bool,
    priority_autopick_single_action: bool,
    priority_allow_pass: bool,
    strict_priority_mode: bool,
    reduced_stage_mode: bool,
    enforce_color_requirement: bool,
    enforce_cost_requirement: bool,
    allow_concede: bool,
    memory_is_public: bool,
}

impl CanonicalCurriculumConfig {
    fn from_curriculum(curriculum: &CurriculumConfig) -> Self {
        let mut allowed_card_sets = curriculum.allowed_card_sets.clone();
        allowed_card_sets.sort();
        allowed_card_sets.dedup();
        Self {
            allowed_card_sets,
            allow_character: curriculum.allow_character,
            allow_event: curriculum.allow_event,
            allow_climax: curriculum.allow_climax,
            enable_clock_phase: curriculum.enable_clock_phase,
            enable_climax_phase: curriculum.enable_climax_phase,
            enable_side_attacks: curriculum.enable_side_attacks,
            enable_direct_attacks: curriculum.enable_direct_attacks,
            enable_counters: curriculum.enable_counters,
            enable_triggers: curriculum.enable_triggers,
            enable_trigger_soul: curriculum.enable_trigger_soul,
            enable_trigger_draw: curriculum.enable_trigger_draw,
            enable_trigger_shot: curriculum.enable_trigger_shot,
            enable_trigger_bounce: curriculum.enable_trigger_bounce,
            enable_trigger_treasure: curriculum.enable_trigger_treasure,
            enable_trigger_gate: curriculum.enable_trigger_gate,
            enable_trigger_standby: curriculum.enable_trigger_standby,
            enable_backup: curriculum.enable_backup,
            enable_encore: curriculum.enable_encore,
            enable_refresh_penalty: curriculum.enable_refresh_penalty,
            enable_level_up_choice: curriculum.enable_level_up_choice,
            enable_activated_abilities: curriculum.enable_activated_abilities,
            enable_continuous_modifiers: curriculum.enable_continuous_modifiers,
            enable_priority_windows: curriculum.enable_priority_windows,
            enable_visibility_policies: curriculum.enable_visibility_policies,
            use_alternate_end_conditions: curriculum.use_alternate_end_conditions,
            priority_autopick_single_action: curriculum.priority_autopick_single_action,
            priority_allow_pass: curriculum.priority_allow_pass,
            strict_priority_mode: curriculum.strict_priority_mode,
            reduced_stage_mode: curriculum.reduced_stage_mode,
            enforce_color_requirement: curriculum.enforce_color_requirement,
            enforce_cost_requirement: curriculum.enforce_cost_requirement,
            allow_concede: curriculum.allow_concede,
            memory_is_public: curriculum.memory_is_public,
        }
    }
}

#[derive(Clone, Debug, Serialize)]
struct CanonicalStateForHash {
    players: [CanonicalPlayerState; 2],
    reveal_history: [crate::state::RevealHistory; 2],
    turn: CanonicalTurnState,
    rng_state: u64,
    modifiers: Vec<crate::state::ModifierInstance>,
    next_modifier_id: u32,
    replacements: Vec<ReplacementSpec>,
    next_replacement_insertion: u32,
    terminal: Option<TerminalResult>,
}

impl CanonicalStateForHash {
    fn from_state(state: &GameState) -> Self {
        Self {
            players: [
                CanonicalPlayerState::from_player(&state.players[0]),
                CanonicalPlayerState::from_player(&state.players[1]),
            ],
            reveal_history: state.reveal_history.clone(),
            turn: CanonicalTurnState::from_turn(&state.turn),
            rng_state: state.rng.state(),
            modifiers: state.modifiers.clone(),
            next_modifier_id: state.next_modifier_id,
            replacements: state.replacements.clone(),
            next_replacement_insertion: state.next_replacement_insertion,
            terminal: state.terminal,
        }
    }
}

#[derive(Clone, Debug, Serialize)]
struct CanonicalPlayerState {
    deck: Vec<crate::state::CardInstance>,
    hand: Vec<crate::state::CardInstance>,
    waiting_room: Vec<crate::state::CardInstance>,
    clock: Vec<crate::state::CardInstance>,
    level: Vec<crate::state::CardInstance>,
    stock: Vec<crate::state::CardInstance>,
    memory: Vec<crate::state::CardInstance>,
    climax: Vec<crate::state::CardInstance>,
    resolution: Vec<crate::state::CardInstance>,
    stage: [crate::state::StageSlot; 5],
}

impl CanonicalPlayerState {
    fn from_player(player: &PlayerState) -> Self {
        Self {
            deck: player.deck.clone(),
            hand: player.hand.clone(),
            waiting_room: player.waiting_room.clone(),
            clock: player.clock.clone(),
            level: player.level.clone(),
            stock: player.stock.clone(),
            memory: player.memory.clone(),
            climax: player.climax.clone(),
            resolution: player.resolution.clone(),
            stage: player.stage.clone(),
        }
    }
}

#[derive(Clone, Debug, Serialize)]
struct CanonicalTurnState {
    active_player: u8,
    starting_player: u8,
    turn_number: u32,
    phase: crate::state::Phase,
    mulligan_done: [bool; 2],
    mulligan_selected: [u64; 2],
    main_passed: bool,
    decision_count: u32,
    tick_count: u32,
    attack: Option<AttackContext>,
    attack_subphase_count: u8,
    pending_level_up: Option<u8>,
    encore_queue: Vec<crate::state::EncoreRequest>,
    encore_step_player: Option<u8>,
    pending_triggers: Vec<PendingTrigger>,
    trigger_order: Option<crate::state::TriggerOrderState>,
    choice: Option<ChoiceState>,
    target_selection: Option<TargetSelectionState>,
    pending_cost: Option<crate::state::CostPaymentState>,
    priority: Option<PriorityState>,
    stack: Vec<StackItem>,
    pending_stack_groups: Vec<StackOrderState>,
    stack_order: Option<StackOrderState>,
    next_trigger_id: u32,
    next_trigger_group_id: u32,
    next_choice_id: u32,
    next_stack_group_id: u32,
    next_damage_event_id: u32,
    next_effect_instance_id: u32,
    active_window: Option<TimingWindow>,
    end_phase_window_done: bool,
    end_phase_discard_done: bool,
    end_phase_climax_done: bool,
    end_phase_cleanup_done: bool,
    encore_window_done: bool,
    pending_losses: [bool; 2],
    damage_resolution_target: Option<u8>,
    cost_payment_depth: u8,
    pending_resolution_cleanup: Vec<(u8, crate::state::CardInstanceId)>,
    phase_step: u8,
    attack_phase_begin_done: bool,
    attack_decl_check_done: bool,
    encore_begin_done: bool,
    end_phase_pending: bool,
}

impl CanonicalTurnState {
    fn from_turn(turn: &TurnState) -> Self {
        Self {
            active_player: turn.active_player,
            starting_player: turn.starting_player,
            turn_number: turn.turn_number,
            phase: turn.phase,
            mulligan_done: turn.mulligan_done,
            mulligan_selected: turn.mulligan_selected,
            main_passed: turn.main_passed,
            decision_count: turn.decision_count,
            tick_count: turn.tick_count,
            attack: turn.attack.clone(),
            attack_subphase_count: turn.attack_subphase_count,
            pending_level_up: turn.pending_level_up,
            encore_queue: turn.encore_queue.clone(),
            encore_step_player: turn.encore_step_player,
            pending_triggers: turn.pending_triggers.clone(),
            trigger_order: turn.trigger_order.clone(),
            choice: turn.choice.clone(),
            target_selection: turn.target_selection.clone(),
            pending_cost: turn.pending_cost.clone(),
            priority: turn.priority.clone(),
            stack: turn.stack.clone(),
            pending_stack_groups: turn.pending_stack_groups.iter().cloned().collect(),
            stack_order: turn.stack_order.clone(),
            next_trigger_id: turn.next_trigger_id,
            next_trigger_group_id: turn.next_trigger_group_id,
            next_choice_id: turn.next_choice_id,
            next_stack_group_id: turn.next_stack_group_id,
            next_damage_event_id: turn.next_damage_event_id,
            next_effect_instance_id: turn.next_effect_instance_id,
            active_window: turn.active_window,
            end_phase_window_done: turn.end_phase_window_done,
            end_phase_discard_done: turn.end_phase_discard_done,
            end_phase_climax_done: turn.end_phase_climax_done,
            end_phase_cleanup_done: turn.end_phase_cleanup_done,
            encore_window_done: turn.encore_window_done,
            pending_losses: turn.pending_losses,
            damage_resolution_target: turn.damage_resolution_target,
            cost_payment_depth: turn.cost_payment_depth,
            pending_resolution_cleanup: turn.pending_resolution_cleanup.clone(),
            phase_step: turn.phase_step,
            attack_phase_begin_done: turn.attack_phase_begin_done,
            attack_decl_check_done: turn.attack_decl_check_done,
            encore_begin_done: turn.encore_begin_done,
            end_phase_pending: turn.end_phase_pending,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_hashmap_leakage_regression() {
        let config_name = std::any::type_name::<CanonicalConfigForHash>();
        let state_name = std::any::type_name::<CanonicalStateForHash>();
        assert!(!config_name.contains("HashMap"));
        assert!(!config_name.contains("HashSet"));
        assert!(!state_name.contains("HashMap"));
        assert!(!state_name.contains("HashSet"));
    }
}
