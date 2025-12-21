use std::sync::Arc;

use criterion::{criterion_group, criterion_main, Criterion};

use weiss_core::config::{CurriculumConfig, EnvConfig, RewardConfig};
use weiss_core::db::{CardDb, CardStatic, CardType, CardColor};
use weiss_core::env::GameEnv;
use weiss_core::pool::EnvPool;

fn make_db() -> Arc<CardDb> {
    let cards = vec![
        CardStatic {
            id: 1,
            card_set: None,
            card_type: CardType::Character,
            color: CardColor::Red,
            level: 0,
            cost: 0,
            power: 500,
            soul: 1,
            triggers: vec![],
            traits: vec![],
            abilities: vec![],
            counter_timing: false,
            raw_text: None,
        },
    ];
    Arc::new(CardDb::new(cards).expect("db build"))
}

fn make_config() -> EnvConfig {
    EnvConfig {
        deck_lists: [vec![1; 50], vec![1; 50]],
        deck_ids: [1, 2],
        max_decisions: 2000,
        max_ticks: 100_000,
        reward: RewardConfig::default(),
        error_policy: weiss_core::config::ErrorPolicy::LenientTerminate,
        observation_visibility: weiss_core::config::ObservationVisibility::Public,
    }
}

fn bench_advance_until_decision(c: &mut Criterion) {
    let db = make_db();
    let config = make_config();
    let curriculum = CurriculumConfig::default();
    c.bench_function("advance_until_decision", |b| {
        b.iter(|| {
            let mut env = GameEnv::new(db.clone(), config.clone(), curriculum.clone(), 42, Default::default(), None);
            for _ in 0..50 {
                if let Some(decision) = env.decision.clone() {
                    let actions = weiss_core::legal::legal_actions(&env.state, &decision, &env.db, &env.curriculum);
                    env.apply_action(actions[0].clone()).unwrap();
                }
            }
        })
    });
}

fn bench_step_batch(c: &mut Criterion) {
    let db = make_db();
    let config = make_config();
    let curriculum = CurriculumConfig::default();
    let mut pool = EnvPool::new(64, db.clone(), config, curriculum, 7);
    c.bench_function("step_batch_64", |b| {
        b.iter(|| {
            let masks = pool.action_masks_batch();
            let mut actions = vec![0u32; pool.envs.len()];
            for i in 0..pool.envs.len() {
                let offset = i * weiss_core::encode::ACTION_SPACE_SIZE;
                let slice = &masks[offset..offset + weiss_core::encode::ACTION_SPACE_SIZE];
                let mut chosen = 0u32;
                for (id, &m) in slice.iter().enumerate() {
                    if m == 1 {
                        chosen = id as u32;
                        break;
                    }
                }
                actions[i] = chosen;
            }
            let _ = pool.step_batch(&actions).unwrap();
        })
    });
}

fn bench_legal_actions(c: &mut Criterion) {
    let db = make_db();
    let config = make_config();
    let curriculum = CurriculumConfig::default();
    let env = GameEnv::new(db.clone(), config, curriculum, 9, Default::default(), None);
    c.bench_function("legal_actions", |b| {
        b.iter(|| {
            if let Some(decision) = env.decision.clone() {
                let _ = weiss_core::legal::legal_actions(&env.state, &decision, &env.db, &env.curriculum);
            }
        })
    });
}

fn bench_observation_encode(c: &mut Criterion) {
    let db = make_db();
    let config = make_config();
    let curriculum = CurriculumConfig::default();
    let env = GameEnv::new(db.clone(), config, curriculum, 11, Default::default(), None);
    c.bench_function("observation_encode", |b| {
        b.iter(|| {
            let mut buf = vec![0i32; weiss_core::encode::OBS_LEN];
            weiss_core::encode::encode_observation(
                &env.state,
                &env.db,
                &env.curriculum,
                0,
                env.decision.as_ref(),
                env.last_action_desc.as_ref(),
                env.config.observation_visibility,
                &mut buf,
            );
        })
    });
}

fn bench_mask_construction(c: &mut Criterion) {
    let db = make_db();
    let config = make_config();
    let curriculum = CurriculumConfig::default();
    let env = GameEnv::new(db.clone(), config, curriculum, 13, Default::default(), None);
    c.bench_function("mask_construction", |b| {
        b.iter(|| {
            if let Some(decision) = env.decision.clone() {
                let actions = weiss_core::legal::legal_actions(&env.state, &decision, &env.db, &env.curriculum);
                let _ = weiss_core::encode::build_action_mask(&actions);
            }
        })
    });
}

criterion_group!(benches, bench_advance_until_decision, bench_step_batch, bench_legal_actions, bench_observation_encode, bench_mask_construction);
criterion_main!(benches);
