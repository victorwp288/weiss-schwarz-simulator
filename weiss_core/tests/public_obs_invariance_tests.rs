use std::sync::Arc;

#[path = "engine_support.rs"]
mod engine_support;

use weiss_core::config::CurriculumConfig;
use weiss_core::encode::{build_action_mask, encode_observation, ACTION_SPACE_SIZE, OBS_LEN};
use weiss_core::env::GameEnv;
use weiss_core::legal::legal_actions_cached;
use weiss_core::replay::ReplayConfig;

fn mutate_hidden_zones(env: &mut GameEnv, player: usize, variant: u8) {
    let p = &mut env.state.players[player];
    match variant {
        0 => {
            p.deck.reverse();
            p.hand.reverse();
            p.stock.reverse();
        }
        1 => {
            if p.deck.len() > 1 {
                p.deck.rotate_left(1);
            }
            if p.hand.len() > 1 {
                p.hand.rotate_right(1);
            }
            let stock_len = p.stock.len();
            if stock_len > 1 {
                p.stock.rotate_left(2 % stock_len);
            }
        }
        2 => {
            let deck_len = p.deck.len();
            if deck_len > 1 {
                p.deck.swap(0, deck_len - 1);
            }
            let hand_len = p.hand.len();
            if hand_len > 1 {
                p.hand.swap(0, hand_len - 1);
            }
            let stock_len = p.stock.len();
            if stock_len > 1 {
                p.stock.swap(0, stock_len - 1);
            }
        }
        _ => {
            if p.deck.len() > 2 {
                p.deck.rotate_right(2);
            }
            if p.hand.len() > 2 {
                p.hand.rotate_left(2);
            }
            if p.stock.len() > 2 {
                p.stock.rotate_right(1);
            }
        }
    }
}

fn encode_for(env: &GameEnv, perspective: u8) -> Vec<i32> {
    let mut obs = vec![0; OBS_LEN];
    encode_observation(
        &env.state,
        &env.db,
        &env.curriculum,
        perspective,
        env.decision.as_ref(),
        env.last_action_desc.as_ref(),
        env.last_action_player,
        env.config.observation_visibility,
        &mut obs,
    );
    obs
}

fn mask_for(env: &GameEnv) -> Vec<u8> {
    let mut mask = vec![0u8; ACTION_SPACE_SIZE];
    if let Some(decision) = env.decision.as_ref() {
        let actions = legal_actions_cached(
            &env.state,
            decision,
            &env.db,
            &env.curriculum,
            env.curriculum.allowed_card_sets_cache.as_ref(),
        );
        let (built_mask, _) = build_action_mask(&actions);
        mask.copy_from_slice(&built_mask);
    }
    mask
}

#[test]
fn public_observation_and_masks_invariant_under_opponent_hidden_changes() {
    let db = engine_support::make_db();
    let config = engine_support::make_config(vec![1; 50], vec![1; 50]);
    let curriculum = CurriculumConfig::default();
    let replay = ReplayConfig::default();

    for seed in 0..6u64 {
        let mut actions: Vec<u32> = Vec::new();
        for step in 0..12u32 {
            let mut env_a = GameEnv::new(
                Arc::clone(&db),
                config.clone(),
                curriculum.clone(),
                seed,
                replay.clone(),
                None,
                0,
            );
            let _ = env_a.reset_no_copy();
            for &action_id in &actions {
                let _ = env_a.apply_action_id_no_copy(action_id as usize);
            }
            let mask_actor = mask_for(&env_a);
            if mask_actor.iter().all(|v| *v == 0) {
                break;
            }

            for viewer in 0..=1u8 {
                let obs_view = encode_for(&env_a, viewer);
                for variant in 0..4u8 {
                    let mut env_b = GameEnv::new(
                        Arc::clone(&db),
                        config.clone(),
                        curriculum.clone(),
                        seed,
                        replay.clone(),
                        None,
                        0,
                    );
                    let _ = env_b.reset_no_copy();
                    for &action_id in &actions {
                        let _ = env_b.apply_action_id_no_copy(action_id as usize);
                    }
                    mutate_hidden_zones(&mut env_b, (1 - viewer) as usize, variant);

                    let obs_b = encode_for(&env_b, viewer);
                    assert_eq!(
                        obs_view, obs_b,
                        "seed {seed} step {step} viewer {viewer} v{variant}"
                    );
                    assert_eq!(
                        mask_actor,
                        mask_for(&env_b),
                        "seed {seed} step {step} viewer {viewer} mask v{variant}"
                    );
                }
            }

            let next_action = mask_actor
                .iter()
                .position(|v| *v == 1)
                .expect("legal action");
            actions.push(next_action as u32);
        }
    }
}
