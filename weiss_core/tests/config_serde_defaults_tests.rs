use weiss_core::config::{CurriculumConfig, EndConditionPolicy};

fn assert_curriculum_eq(lhs: &CurriculumConfig, rhs: &CurriculumConfig) {
    assert_eq!(lhs.allowed_card_sets, rhs.allowed_card_sets);
    assert_eq!(lhs.allow_character, rhs.allow_character);
    assert_eq!(lhs.allow_event, rhs.allow_event);
    assert_eq!(lhs.allow_climax, rhs.allow_climax);
    assert_eq!(lhs.enable_clock_phase, rhs.enable_clock_phase);
    assert_eq!(lhs.enable_climax_phase, rhs.enable_climax_phase);
    assert_eq!(lhs.enable_side_attacks, rhs.enable_side_attacks);
    assert_eq!(lhs.enable_direct_attacks, rhs.enable_direct_attacks);
    assert_eq!(lhs.enable_counters, rhs.enable_counters);
    assert_eq!(lhs.enable_triggers, rhs.enable_triggers);
    assert_eq!(lhs.enable_trigger_soul, rhs.enable_trigger_soul);
    assert_eq!(lhs.enable_trigger_draw, rhs.enable_trigger_draw);
    assert_eq!(lhs.enable_trigger_shot, rhs.enable_trigger_shot);
    assert_eq!(lhs.enable_trigger_bounce, rhs.enable_trigger_bounce);
    assert_eq!(lhs.enable_trigger_treasure, rhs.enable_trigger_treasure);
    assert_eq!(lhs.enable_trigger_gate, rhs.enable_trigger_gate);
    assert_eq!(lhs.enable_trigger_standby, rhs.enable_trigger_standby);
    assert_eq!(
        lhs.enable_on_reverse_triggers,
        rhs.enable_on_reverse_triggers
    );
    assert_eq!(lhs.enable_backup, rhs.enable_backup);
    assert_eq!(lhs.enable_encore, rhs.enable_encore);
    assert_eq!(lhs.enable_refresh_penalty, rhs.enable_refresh_penalty);
    assert_eq!(lhs.enable_level_up_choice, rhs.enable_level_up_choice);
    assert_eq!(
        lhs.enable_activated_abilities,
        rhs.enable_activated_abilities
    );
    assert_eq!(
        lhs.enable_continuous_modifiers,
        rhs.enable_continuous_modifiers
    );
    assert_eq!(lhs.enable_approx_effects, rhs.enable_approx_effects);
    assert_eq!(lhs.enable_priority_windows, rhs.enable_priority_windows);
    assert_eq!(
        lhs.enable_visibility_policies,
        rhs.enable_visibility_policies
    );
    assert_eq!(
        lhs.use_alternate_end_conditions,
        rhs.use_alternate_end_conditions
    );
    assert_eq!(
        lhs.priority_autopick_single_action,
        rhs.priority_autopick_single_action
    );
    assert_eq!(lhs.priority_allow_pass, rhs.priority_allow_pass);
    assert_eq!(lhs.strict_priority_mode, rhs.strict_priority_mode);
    assert_eq!(lhs.enable_legacy_cost_order, rhs.enable_legacy_cost_order);
    assert_eq!(
        lhs.enable_legacy_shot_damage_step_only,
        rhs.enable_legacy_shot_damage_step_only
    );
    assert_eq!(lhs.reduced_stage_mode, rhs.reduced_stage_mode);
    assert_eq!(lhs.enforce_color_requirement, rhs.enforce_color_requirement);
    assert_eq!(lhs.enforce_cost_requirement, rhs.enforce_cost_requirement);
    assert_eq!(lhs.allow_concede, rhs.allow_concede);
    assert_eq!(
        lhs.reveal_opponent_hand_stock_counts,
        rhs.reveal_opponent_hand_stock_counts
    );
    assert_eq!(lhs.memory_is_public, rhs.memory_is_public);
    assert_eq!(lhs.allowed_card_sets_cache, rhs.allowed_card_sets_cache);
}

#[test]
fn curriculum_config_serde_omitted_fields_match_default() {
    let parsed: CurriculumConfig =
        serde_json::from_str("{}").expect("empty curriculum json should deserialize");
    let expected = CurriculumConfig::default();
    assert_curriculum_eq(&parsed, &expected);
}

#[test]
fn end_condition_policy_serde_omitted_fields_match_default() {
    let parsed: EndConditionPolicy =
        serde_json::from_str("{}").expect("empty end condition policy json should deserialize");
    assert_eq!(parsed, EndConditionPolicy::default());
}
