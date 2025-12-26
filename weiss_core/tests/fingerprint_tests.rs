mod engine_support;

use engine_support::*;
use weiss_core::config::CurriculumConfig;
use weiss_core::env::GameEnv;
use weiss_core::fingerprint::{config_fingerprint, state_fingerprint, FINGERPRINT_ALGO};

#[test]
fn config_hash_stable() {
    let deck_a = vec![1; 50];
    let deck_b = vec![1; 50];
    let config = make_config(deck_a, deck_b);
    let curriculum_a = CurriculumConfig {
        allowed_card_sets: vec!["A".to_string(), "B".to_string()],
        ..Default::default()
    };
    let curriculum_b = CurriculumConfig {
        allowed_card_sets: vec!["B".to_string(), "A".to_string()],
        ..Default::default()
    };

    let hash_a = config_fingerprint(&config, &curriculum_a);
    let hash_b = config_fingerprint(&config, &curriculum_b);
    assert_eq!(hash_a, hash_b);
}

#[test]
fn state_hash_stable_across_runs() {
    let db = make_db();
    let deck_a = vec![1; 50];
    let deck_b = vec![1; 50];
    let config = make_config(deck_a, deck_b);
    let curriculum = CurriculumConfig::default();
    let mut env_a = GameEnv::new(
        db.clone(),
        config.clone(),
        curriculum.clone(),
        99,
        Default::default(),
        None,
        0,
    );
    let mut env_b = GameEnv::new(db, config, curriculum, 99, Default::default(), None, 0);

    for _ in 0..6 {
        let action = env_a
            .legal_actions()
            .first()
            .cloned()
            .expect("legal action");
        env_a.apply_action(action.clone()).unwrap();
        env_b.apply_action(action).unwrap();
    }

    assert_eq!(
        state_fingerprint(&env_a.state),
        state_fingerprint(&env_b.state)
    );
}

#[test]
fn fingerprint_algo_constant_present() {
    assert!(FINGERPRINT_ALGO.contains("blake3"));
}
