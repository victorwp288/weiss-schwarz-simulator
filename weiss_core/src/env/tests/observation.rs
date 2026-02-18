use super::*;
use crate::config::ObservationVisibility;
use crate::encode::{
    encode_observation, OBS_CONTEXT_BASE, OBS_CONTEXT_CHOICE_ACTIVE, OBS_CONTEXT_ENCORE_PENDING,
    OBS_CONTEXT_LEN, OBS_CONTEXT_PRIORITY_WINDOW, OBS_CONTEXT_STACK_NONEMPTY, OBS_HEADER_LEN,
    OBS_LEN, PER_PLAYER_BLOCK_LEN, PER_PLAYER_CLIMAX_TOP, PER_PLAYER_CLOCK_TOP, PER_PLAYER_COUNTS,
    PER_PLAYER_DECK, PER_PLAYER_HAND, PER_PLAYER_LEVEL, PER_PLAYER_RESOLUTION_TOP,
    PER_PLAYER_STAGE, PER_PLAYER_STOCK_TOP, PER_PLAYER_WAITING_TOP,
};
use crate::legal::ActionDesc;
use crate::state::{
    ChoiceReason, ChoiceState, DerivedAttackState, EncoreRequest, ModifierDuration, ModifierKind,
    PriorityState, TimingWindow,
};

#[test]
fn public_observation_masks_opponent_last_action_params() {
    let mut env = make_env();
    env.curriculum.enable_visibility_policies = false;
    env.last_action_desc = Some(ActionDesc::MainPlayCharacter {
        hand_index: 4,
        stage_slot: 1,
    });
    env.last_action_player = Some(1);
    let mut obs = vec![0; OBS_LEN];
    encode_observation(
        &env.state,
        &env.db,
        &env.curriculum,
        0,
        env.decision.as_ref(),
        env.last_action_desc.as_ref(),
        env.last_action_player,
        env.config.observation_visibility,
        &mut obs,
    );
    assert_eq!(obs[5], 6);
    assert_eq!(obs[6], -1);
    assert_eq!(obs[7], 1);
}

#[test]
fn public_observation_masks_hidden_zones_without_policies() {
    let mut env = make_env();
    env.curriculum.enable_visibility_policies = false;
    env.config.observation_visibility = ObservationVisibility::Public;

    let mut obs = vec![0; OBS_LEN];
    encode_observation(
        &env.state,
        &env.db,
        &env.curriculum,
        0,
        env.decision.as_ref(),
        env.last_action_desc.as_ref(),
        env.last_action_player,
        env.config.observation_visibility,
        &mut obs,
    );

    let opponent_base = OBS_HEADER_LEN + PER_PLAYER_BLOCK_LEN;
    let stock_start = opponent_base
        + PER_PLAYER_COUNTS
        + PER_PLAYER_STAGE
        + PER_PLAYER_CLIMAX_TOP
        + PER_PLAYER_LEVEL
        + PER_PLAYER_CLOCK_TOP
        + PER_PLAYER_WAITING_TOP
        + PER_PLAYER_RESOLUTION_TOP;
    let hand_start = stock_start + PER_PLAYER_STOCK_TOP;
    let deck_start = hand_start + PER_PLAYER_HAND;

    assert!(obs[stock_start..stock_start + PER_PLAYER_STOCK_TOP]
        .iter()
        .all(|v| *v == -1));
    assert!(obs[hand_start..hand_start + PER_PLAYER_HAND]
        .iter()
        .all(|v| *v == -1));
    assert!(obs[deck_start..deck_start + PER_PLAYER_DECK]
        .iter()
        .all(|v| *v == -1));

    let own_hand_start = OBS_HEADER_LEN
        + PER_PLAYER_COUNTS
        + PER_PLAYER_STAGE
        + PER_PLAYER_CLIMAX_TOP
        + PER_PLAYER_LEVEL
        + PER_PLAYER_CLOCK_TOP
        + PER_PLAYER_WAITING_TOP
        + PER_PLAYER_RESOLUTION_TOP
        + PER_PLAYER_STOCK_TOP;
    assert!(obs[own_hand_start..own_hand_start + PER_PLAYER_HAND]
        .iter()
        .any(|v| *v > 0));
}

#[test]
fn public_observation_masks_hidden_zones_cached_path() {
    let mut env = make_env();
    env.curriculum.enable_visibility_policies = false;
    env.config.observation_visibility = ObservationVisibility::Public;

    // Prime caches with a full encode.
    let _ = env.reset();

    // Mark only the opponent block dirty to force the cached encode path.
    let perspective = env.last_perspective as usize;
    let opponent = 1 - perspective;
    env.player_obs_version[opponent] = env.player_obs_version[opponent].wrapping_add(1);
    env.obs_dirty = false;
    env.obs_perspective = env.last_perspective;

    let outcome = env.build_outcome_with_obs(0.0, true);
    let obs = outcome.obs;

    let opponent_base = OBS_HEADER_LEN + PER_PLAYER_BLOCK_LEN;
    let stock_start = opponent_base
        + PER_PLAYER_COUNTS
        + PER_PLAYER_STAGE
        + PER_PLAYER_CLIMAX_TOP
        + PER_PLAYER_LEVEL
        + PER_PLAYER_CLOCK_TOP
        + PER_PLAYER_WAITING_TOP
        + PER_PLAYER_RESOLUTION_TOP;
    let hand_start = stock_start + PER_PLAYER_STOCK_TOP;
    let deck_start = hand_start + PER_PLAYER_HAND;

    assert!(obs[stock_start..stock_start + PER_PLAYER_STOCK_TOP]
        .iter()
        .all(|v| *v == -1));
    assert!(obs[hand_start..hand_start + PER_PLAYER_HAND]
        .iter()
        .all(|v| *v == -1));
    assert!(obs[deck_start..deck_start + PER_PLAYER_DECK]
        .iter()
        .all(|v| *v == -1));
}

#[test]
fn cached_path_refreshes_and_uses_opponent_block() {
    let mut env = make_env();
    env.curriculum.enable_visibility_policies = false;
    env.config.observation_visibility = ObservationVisibility::Public;

    // Prime caches.
    let _ = env.build_outcome_with_obs(0.0, true);

    let perspective = env.last_perspective as usize;
    let opponent = 1 - perspective;

    // Keep perspective cache clean; mutate only opponent data and mark that block dirty.
    env.state.players[opponent].hand.truncate(1);
    env.touch_player_obs(opponent as u8);
    env.obs_dirty = false;
    env.obs_perspective = perspective as u8;

    let outcome = env.build_outcome_with_obs(0.0, true);
    let obs = outcome.obs;

    let self_hand = obs[OBS_HEADER_LEN + 3];
    let opp_hand = obs[OBS_HEADER_LEN + PER_PLAYER_BLOCK_LEN + 3];
    let opp_stock = obs[OBS_HEADER_LEN + PER_PLAYER_BLOCK_LEN + 4];

    assert_eq!(self_hand, env.state.players[perspective].hand.len() as i32);
    assert_eq!(opp_hand, 0);
    assert_eq!(opp_stock, 0);
}

#[test]
fn observation_context_bits_reflect_turn_state() {
    let mut env = make_env();
    env.config.observation_visibility = ObservationVisibility::Public;
    env.advance_until_decision();
    env.update_action_cache();

    env.state.turn.priority = Some(PriorityState {
        holder: 0,
        passes: 0,
        window: TimingWindow::MainWindow,
        used_act_mask: 0,
    });
    env.state.turn.choice = Some(ChoiceState {
        id: 1,
        reason: ChoiceReason::EndPhaseDiscard,
        player: 0,
        options: Vec::new(),
        total_candidates: 0,
        page_start: 0,
        pending_trigger: None,
    });
    env.state.turn.stack.push(make_noop_stack_item(1));
    env.state
        .turn
        .encore_queue
        .push(EncoreRequest { player: 0, slot: 0 });

    let mut obs = vec![0; OBS_LEN];
    encode_observation(
        &env.state,
        &env.db,
        &env.curriculum,
        env.last_perspective,
        env.decision.as_ref(),
        env.last_action_desc.as_ref(),
        env.last_action_player,
        env.config.observation_visibility,
        &mut obs,
    );
    let ctx = &obs[OBS_CONTEXT_BASE..OBS_CONTEXT_BASE + OBS_CONTEXT_LEN];
    assert_eq!(ctx[OBS_CONTEXT_PRIORITY_WINDOW], 1);
    assert_eq!(ctx[OBS_CONTEXT_CHOICE_ACTIVE], 1);
    assert_eq!(ctx[OBS_CONTEXT_STACK_NONEMPTY], 1);
    assert_eq!(ctx[OBS_CONTEXT_ENCORE_PENDING], 1);
}

#[test]
fn observation_encoded_from_actor_and_self_block_first() {
    let mut env = make_env();
    env.advance_until_decision();
    let decision_player = env.decision.as_ref().expect("decision").player as usize;
    env.state.players[0].hand.truncate(2);
    env.state.players[1].hand.truncate(5);
    env.touch_player_obs(0);
    env.touch_player_obs(1);

    let outcome = env.build_outcome_with_obs(0.0, true);
    let mut manual = vec![0; OBS_LEN];
    encode_observation(
        &env.state,
        &env.db,
        &env.curriculum,
        decision_player as u8,
        env.decision.as_ref(),
        env.last_action_desc.as_ref(),
        env.last_action_player,
        env.config.observation_visibility,
        &mut manual,
    );
    assert_eq!(outcome.obs, manual);

    let self_hand = outcome.obs[OBS_HEADER_LEN + 3];
    let opp_hand = outcome.obs[OBS_HEADER_LEN + PER_PLAYER_BLOCK_LEN + 3];
    let opp_stock = outcome.obs[OBS_HEADER_LEN + PER_PLAYER_BLOCK_LEN + 4];
    assert_eq!(
        self_hand,
        env.state.players[decision_player].hand.len() as i32
    );
    assert_eq!(opp_hand, 0);
    assert_eq!(opp_stock, 0);
}

#[test]
fn observation_can_expose_opponent_hand_count_with_override() {
    let mut env = make_env();
    env.curriculum.reveal_opponent_hand_stock_counts = true;
    env.advance_until_decision();
    let decision_player = env.decision.as_ref().expect("decision").player as usize;
    env.state.players[0].hand.truncate(2);
    env.state.players[1].hand.truncate(5);
    while env.state.players[0].stock.len() < 2 {
        let card = env.state.players[0].deck.pop().expect("player 0 deck");
        env.state.players[0].stock.push(card);
    }
    while env.state.players[1].stock.len() < 3 {
        let card = env.state.players[1].deck.pop().expect("player 1 deck");
        env.state.players[1].stock.push(card);
    }
    env.touch_player_obs(0);
    env.touch_player_obs(1);

    let outcome = env.build_outcome_with_obs(0.0, true);
    let self_hand = outcome.obs[OBS_HEADER_LEN + 3];
    let opp_hand = outcome.obs[OBS_HEADER_LEN + PER_PLAYER_BLOCK_LEN + 3];
    let self_stock = outcome.obs[OBS_HEADER_LEN + 4];
    let opp_stock = outcome.obs[OBS_HEADER_LEN + PER_PLAYER_BLOCK_LEN + 4];
    assert_eq!(
        self_hand,
        env.state.players[decision_player].hand.len() as i32
    );
    assert_eq!(
        opp_hand,
        env.state.players[1 - decision_player].hand.len() as i32
    );
    assert_eq!(
        self_stock,
        env.state.players[decision_player].stock.len() as i32
    );
    assert_eq!(
        opp_stock,
        env.state.players[1 - decision_player].stock.len() as i32
    );
}

#[test]
fn observation_exposes_effective_soul_and_side_attack_flag() {
    let mut env = make_env();
    let _ = env.reset_no_copy();
    let mut next_id = 1u32;
    let card = make_instance(1, 0, &mut next_id);
    env.state.players[0].stage[0].card = Some(card);
    env.state.players[0].stage[0].status = crate::state::StageStatus::Stand;
    let _ = env.add_modifier(
        1,
        0,
        0,
        ModifierKind::Soul,
        2,
        ModifierDuration::UntilEndOfTurn,
    );
    let _ = env.add_modifier(
        1,
        0,
        0,
        ModifierKind::CannotSideAttack,
        1,
        ModifierDuration::UntilEndOfTurn,
    );

    let mut obs = vec![0; OBS_LEN];
    encode_observation(
        &env.state,
        &env.db,
        &env.curriculum,
        0,
        env.decision.as_ref(),
        env.last_action_desc.as_ref(),
        env.last_action_player,
        env.config.observation_visibility,
        &mut obs,
    );

    let slot0 = OBS_HEADER_LEN + PER_PLAYER_COUNTS;
    assert_eq!(obs[slot0 + 4], 1);
    assert_eq!(obs[slot0 + 5], 3);
    assert_eq!(obs[slot0 + 6], 0);
}

#[test]
fn observation_exposes_default_soul_and_side_attack_without_modifiers() {
    let mut env = make_env();
    let _ = env.reset_no_copy();
    let mut next_id = 1u32;
    let card = make_instance(1, 0, &mut next_id);
    env.state.players[0].stage[0].card = Some(card);
    env.state.players[0].stage[0].status = crate::state::StageStatus::Stand;

    let mut obs = vec![0; OBS_LEN];
    encode_observation(
        &env.state,
        &env.db,
        &env.curriculum,
        0,
        env.decision.as_ref(),
        env.last_action_desc.as_ref(),
        env.last_action_player,
        env.config.observation_visibility,
        &mut obs,
    );

    let slot0 = OBS_HEADER_LEN + PER_PLAYER_COUNTS;
    assert_eq!(obs[slot0 + 4], 1);
    assert_eq!(obs[slot0 + 5], 1);
    assert_eq!(obs[slot0 + 6], 1);
}

#[test]
fn observation_prefers_derived_attack_side_flag_over_modifier_scan() {
    let mut env = make_env();
    let _ = env.reset_no_copy();
    let mut next_id = 1u32;
    let card = make_instance(1, 0, &mut next_id);
    env.state.players[0].stage[0].card = Some(card);
    env.state.players[0].stage[0].status = crate::state::StageStatus::Stand;
    let _ = env.add_modifier(
        1,
        0,
        0,
        ModifierKind::CannotSideAttack,
        1,
        ModifierDuration::UntilEndOfTurn,
    );

    let mut derived = DerivedAttackState::new();
    derived.per_player[0][0].cannot_side_attack = false;
    env.state.turn.derived_attack = Some(derived);

    let mut obs = vec![0; OBS_LEN];
    encode_observation(
        &env.state,
        &env.db,
        &env.curriculum,
        0,
        env.decision.as_ref(),
        env.last_action_desc.as_ref(),
        env.last_action_player,
        env.config.observation_visibility,
        &mut obs,
    );

    let slot0 = OBS_HEADER_LEN + PER_PLAYER_COUNTS;
    assert_eq!(obs[slot0 + 6], 1);
}
