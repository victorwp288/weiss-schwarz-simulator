use std::sync::Arc;

use criterion::{black_box, criterion_group, criterion_main, BatchSize, Criterion};

use weiss_core::config::{CurriculumConfig, EnvConfig, RewardConfig};
use weiss_core::db::{AbilityTemplate, CardColor, CardDb, CardStatic, CardType};
use weiss_core::encode::{fill_action_mask, CHOICE_COUNT};
use weiss_core::env::GameEnv;
use weiss_core::legal::{ActionDesc, Decision, DecisionKind};
use weiss_core::pool::{BatchOutMinimalBuffers, EnvPool};
use weiss_core::state::{
    AttackType, ChoiceOptionRef, ChoiceReason, ChoiceState, ChoiceZone, Phase, StageSlot,
    StageStatus,
};
use weiss_core::DebugConfig;

fn make_base_cards() -> Vec<CardStatic> {
    (1u32..=13)
        .map(|id| CardStatic {
            id,
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
            ability_defs: vec![],
            counter_timing: false,
            raw_text: None,
        })
        .collect()
}

fn make_db() -> Arc<CardDb> {
    Arc::new(CardDb::new(make_base_cards()).expect("db build"))
}

fn make_db_on_reverse() -> Arc<CardDb> {
    let mut cards = make_base_cards();
    if let Some(card) = cards.iter_mut().find(|card| card.id == 2) {
        card.color = CardColor::Blue;
        card.power = 0;
        card.abilities = vec![AbilityTemplate::AutoOnReverseDraw { count: 1 }];
    }
    Arc::new(CardDb::new(cards).expect("db build"))
}

fn make_choice_db(card_count: u32) -> Arc<CardDb> {
    let cards = (1u32..=card_count)
        .map(|id| CardStatic {
            id,
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
            ability_defs: vec![],
            counter_timing: false,
            raw_text: None,
        })
        .collect();
    Arc::new(CardDb::new(cards).expect("db build"))
}

fn make_deck_list() -> Vec<u32> {
    let mut deck = Vec::with_capacity(50);
    for id in 1u32..=12 {
        deck.extend(std::iter::repeat_n(id, 4));
    }
    deck.extend(std::iter::repeat_n(13u32, 2));
    deck
}

fn make_config() -> EnvConfig {
    let deck = make_deck_list();
    EnvConfig {
        deck_lists: [deck.clone(), deck],
        deck_ids: [1, 2],
        max_decisions: 2000,
        max_ticks: 100_000,
        reward: RewardConfig::default(),
        error_policy: weiss_core::config::ErrorPolicy::LenientTerminate,
        observation_visibility: weiss_core::config::ObservationVisibility::Public,
        end_condition_policy: Default::default(),
    }
}

fn make_curriculum(enable_priority_windows: bool) -> CurriculumConfig {
    CurriculumConfig {
        enable_priority_windows,
        ..Default::default()
    }
}

fn setup_reversal_env(env: &mut GameEnv) {
    for player in 0..2usize {
        env.state.players[player].deck.clear();
        env.state.players[player].hand.clear();
        env.state.players[player].waiting_room.clear();
        env.state.players[player].clock.clear();
        env.state.players[player].level.clear();
        env.state.players[player].stock.clear();
        env.state.players[player].memory.clear();
        env.state.players[player].climax.clear();
        env.state.players[player].resolution.clear();
        env.state.players[player].stage = [
            StageSlot::empty(),
            StageSlot::empty(),
            StageSlot::empty(),
            StageSlot::empty(),
            StageSlot::empty(),
        ];
    }

    let mut atk_slot = StageSlot::empty();
    atk_slot.card = Some(weiss_core::state::CardInstance::new(1, 0, 1));
    atk_slot.status = StageStatus::Stand;
    env.state.players[0].stage[0] = atk_slot;

    let mut def_slot = StageSlot::empty();
    def_slot.card = Some(weiss_core::state::CardInstance::new(2, 1, 2));
    def_slot.status = StageStatus::Stand;
    env.state.players[1].stage[0] = def_slot;

    env.state.players[1]
        .deck
        .push(weiss_core::state::CardInstance::new(1, 1, 3));

    env.state.turn.phase = Phase::Attack;
    env.state.turn.active_player = 0;
    env.state.turn.starting_player = 0;
    env.state.turn.turn_number = 1;
    env.state.turn.mulligan_done = [true, true];
    env.state.turn.attack = None;
    env.state.turn.pending_level_up = None;
    env.state.turn.encore_queue.clear();
    env.state.turn.pending_triggers.clear();
    env.state.turn.trigger_order = None;
    env.state.turn.choice = None;
    env.state.turn.priority = None;
    env.state.turn.stack.clear();
    env.state.turn.pending_stack_groups.clear();
    env.state.turn.stack_order = None;
    env.state.turn.derived_attack = None;
    env.state.turn.end_phase_pending = false;
    env.state.turn.main_passed = false;
    env.decision = Some(Decision {
        player: 0,
        kind: DecisionKind::AttackDeclaration,
        focus_slot: None,
    });
}

fn install_choice(env: &mut GameEnv, total: usize) {
    let options = (0..total)
        .map(|idx| ChoiceOptionRef {
            card_id: (idx + 1) as u32,
            instance_id: (idx + 1) as u32,
            zone: ChoiceZone::WaitingRoom,
            index: Some(idx as u8),
            target_slot: None,
        })
        .collect::<Vec<_>>();
    env.state.turn.choice = Some(ChoiceState {
        id: 1,
        reason: ChoiceReason::TriggerTreasureSelect,
        player: 0,
        options,
        total_candidates: total as u16,
        page_start: 0,
        pending_trigger: None,
    });
    env.decision = Some(Decision {
        player: 0,
        kind: DecisionKind::Choice,
        focus_slot: None,
    });
}

fn bench_advance_until_decision(c: &mut Criterion) {
    let db = make_db();
    let config = make_config();
    let curriculum = CurriculumConfig::default();
    c.bench_function("advance_until_decision", |b| {
        b.iter(|| {
            let mut env = GameEnv::new(
                db.clone(),
                config.clone(),
                curriculum.clone(),
                42,
                Default::default(),
                None,
                0,
            );
            for _ in 0..50 {
                if let Some(decision) = env.decision.clone() {
                    let actions = weiss_core::legal::legal_actions(
                        &env.state,
                        &decision,
                        &env.db,
                        &env.curriculum,
                    );
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
    let mut pool = EnvPool::new_debug(
        64,
        db.clone(),
        config,
        curriculum,
        7,
        None,
        DebugConfig::default(),
    )
    .expect("pool");
    let mut out = BatchOutMinimalBuffers::new(pool.envs.len());
    let mut masks = vec![0u8; pool.envs.len() * weiss_core::encode::ACTION_SPACE_SIZE];
    c.bench_function("step_batch_64", |b| {
        b.iter(|| {
            pool.action_masks_batch_into(&mut masks)
                .expect("mask buffer");
            let mut actions = vec![0u32; pool.envs.len()];
            for (i, action) in actions.iter_mut().enumerate() {
                let offset = i * weiss_core::encode::ACTION_SPACE_SIZE;
                let slice = &masks[offset..offset + weiss_core::encode::ACTION_SPACE_SIZE];
                let mut chosen = 0u32;
                for (id, &m) in slice.iter().enumerate() {
                    if m == 1 {
                        chosen = id as u32;
                        break;
                    }
                }
                *action = chosen;
            }
            pool.step_into(black_box(&actions), black_box(&mut out.view_mut()))
                .unwrap();
        })
    });
}

fn bench_step_batch_fast_priority_off(c: &mut Criterion) {
    let db = make_db();
    let config = make_config();
    let curriculum = make_curriculum(false);
    let mut pool = EnvPool::new_debug(
        256,
        db,
        config,
        curriculum,
        21,
        None,
        DebugConfig::default(),
    )
    .expect("pool");
    let mut actions = vec![0u32; pool.envs.len()];
    let mut out = BatchOutMinimalBuffers::new(pool.envs.len());
    let mut masks = vec![0u8; pool.envs.len() * weiss_core::encode::ACTION_SPACE_SIZE];
    c.bench_function("step_batch_fast_256_priority_off", |b| {
        b.iter(|| {
            pool.action_masks_batch_into(&mut masks)
                .expect("mask buffer");
            for (i, action) in actions.iter_mut().enumerate() {
                let offset = i * weiss_core::encode::ACTION_SPACE_SIZE;
                let slice = &masks[offset..offset + weiss_core::encode::ACTION_SPACE_SIZE];
                let mut chosen = 0u32;
                for (id, &m) in slice.iter().enumerate() {
                    if m == 1 {
                        chosen = id as u32;
                        break;
                    }
                }
                *action = chosen;
            }
            pool.step_into(black_box(&actions), black_box(&mut out.view_mut()))
                .unwrap();
        })
    });
}

fn bench_step_batch_fast_priority_on(c: &mut Criterion) {
    let db = make_db();
    let config = make_config();
    let curriculum = make_curriculum(true);
    let mut pool = EnvPool::new_debug(
        256,
        db,
        config,
        curriculum,
        22,
        None,
        DebugConfig::default(),
    )
    .expect("pool");
    let mut actions = vec![0u32; pool.envs.len()];
    let mut out = BatchOutMinimalBuffers::new(pool.envs.len());
    let mut masks = vec![0u8; pool.envs.len() * weiss_core::encode::ACTION_SPACE_SIZE];
    c.bench_function("step_batch_fast_256_priority_on", |b| {
        b.iter(|| {
            pool.action_masks_batch_into(&mut masks)
                .expect("mask buffer");
            for (i, action) in actions.iter_mut().enumerate() {
                let offset = i * weiss_core::encode::ACTION_SPACE_SIZE;
                let slice = &masks[offset..offset + weiss_core::encode::ACTION_SPACE_SIZE];
                let mut chosen = 0u32;
                for (id, &m) in slice.iter().enumerate() {
                    if m == 1 {
                        chosen = id as u32;
                        break;
                    }
                }
                *action = chosen;
            }
            pool.step_into(black_box(&actions), black_box(&mut out.view_mut()))
                .unwrap();
        })
    });
}

fn bench_legal_actions(c: &mut Criterion) {
    let db = make_db();
    let config = make_config();
    let curriculum = CurriculumConfig::default();
    let env = GameEnv::new(
        db.clone(),
        config,
        curriculum,
        9,
        Default::default(),
        None,
        0,
    );
    c.bench_function("legal_actions", |b| {
        b.iter(|| {
            if let Some(decision) = black_box(env.decision.clone()) {
                let _ = weiss_core::legal::legal_actions(
                    black_box(&env.state),
                    black_box(&decision),
                    black_box(&env.db),
                    black_box(&env.curriculum),
                );
            }
        })
    });
}

fn bench_legal_actions_forced(c: &mut Criterion) {
    let db = make_db();
    let config = make_config();
    let curriculum = CurriculumConfig::default();
    let env = GameEnv::new(
        db.clone(),
        config,
        curriculum,
        9,
        Default::default(),
        None,
        0,
    );
    let mut state = env.state.clone();
    let mut decision = env.decision.clone().expect("decision");
    c.bench_function("legal_actions_forced", |b| {
        b.iter(|| {
            state.turn.active_player ^= 1;
            decision.player = state.turn.active_player;
            let actions = weiss_core::legal::legal_actions(
                black_box(&state),
                black_box(&decision),
                black_box(&env.db),
                black_box(&env.curriculum),
            );
            black_box(actions.len());
        })
    });
}

fn bench_on_reverse_decision_frequency(c: &mut Criterion) {
    let db = make_db_on_reverse();
    let config = make_config();
    let curriculum_on = CurriculumConfig {
        enable_on_reverse_triggers: true,
        ..Default::default()
    };
    let curriculum_off = CurriculumConfig {
        enable_on_reverse_triggers: false,
        ..Default::default()
    };
    let seed = 123u64;

    c.bench_function("on_reverse_decision_frequency_on", |b| {
        b.iter_batched(
            || {
                let mut env = GameEnv::new(
                    db.clone(),
                    config.clone(),
                    curriculum_on.clone(),
                    seed,
                    Default::default(),
                    None,
                    0,
                );
                setup_reversal_env(&mut env);
                env
            },
            |mut env| {
                let _ = env
                    .apply_action(ActionDesc::Attack {
                        slot: 0,
                        attack_type: AttackType::Frontal,
                    })
                    .expect("attack");
                black_box(env.state.turn.decision_count);
            },
            BatchSize::SmallInput,
        )
    });

    c.bench_function("on_reverse_decision_frequency_off", |b| {
        b.iter_batched(
            || {
                let mut env = GameEnv::new(
                    db.clone(),
                    config.clone(),
                    curriculum_off.clone(),
                    seed,
                    Default::default(),
                    None,
                    0,
                );
                setup_reversal_env(&mut env);
                env
            },
            |mut env| {
                let _ = env
                    .apply_action(ActionDesc::Attack {
                        slot: 0,
                        attack_type: AttackType::Frontal,
                    })
                    .expect("attack");
                black_box(env.state.turn.decision_count);
            },
            BatchSize::SmallInput,
        )
    });
}

fn bench_observation_encode(c: &mut Criterion) {
    let db = make_db();
    let config = make_config();
    let curriculum = CurriculumConfig::default();
    let env = GameEnv::new(
        db.clone(),
        config,
        curriculum,
        11,
        Default::default(),
        None,
        0,
    );
    c.bench_function("observation_encode", |b| {
        b.iter(|| {
            let mut buf = vec![0i32; weiss_core::encode::OBS_LEN];
            weiss_core::encode::encode_observation(
                black_box(&env.state),
                black_box(&env.db),
                black_box(&env.curriculum),
                black_box(0),
                black_box(env.decision.as_ref()),
                black_box(env.last_action_desc.as_ref()),
                black_box(env.last_action_player),
                black_box(env.config.observation_visibility),
                black_box(&mut buf),
            );
            black_box(buf);
        })
    });
}

fn bench_observation_encode_forced(c: &mut Criterion) {
    let db = make_db();
    let config = make_config();
    let curriculum = CurriculumConfig::default();
    let env = GameEnv::new(
        db.clone(),
        config,
        curriculum,
        11,
        Default::default(),
        None,
        0,
    );
    let mut state = env.state.clone();
    c.bench_function("observation_encode_forced", |b| {
        b.iter(|| {
            state.turn.tick_count = state.turn.tick_count.wrapping_add(1);
            let mut buf = vec![0i32; weiss_core::encode::OBS_LEN];
            weiss_core::encode::encode_observation(
                black_box(&state),
                black_box(&env.db),
                black_box(&env.curriculum),
                black_box(0),
                black_box(env.decision.as_ref()),
                black_box(env.last_action_desc.as_ref()),
                black_box(env.last_action_player),
                black_box(env.config.observation_visibility),
                black_box(&mut buf),
            );
            black_box(buf);
        })
    });
}

fn bench_mask_construction(c: &mut Criterion) {
    let db = make_db();
    let config = make_config();
    let curriculum = CurriculumConfig::default();
    let env = GameEnv::new(
        db.clone(),
        config,
        curriculum,
        13,
        Default::default(),
        None,
        0,
    );
    c.bench_function("mask_construction", |b| {
        b.iter(|| {
            if let Some(decision) = black_box(env.decision.clone()) {
                let actions = weiss_core::legal::legal_actions(
                    black_box(&env.state),
                    black_box(&decision),
                    black_box(&env.db),
                    black_box(&env.curriculum),
                );
                let mask = weiss_core::encode::build_action_mask(black_box(&actions));
                black_box(mask);
            }
        })
    });
}

fn bench_mask_construction_forced(c: &mut Criterion) {
    let db = make_db();
    let config = make_config();
    let curriculum = CurriculumConfig::default();
    let env = GameEnv::new(
        db.clone(),
        config,
        curriculum,
        13,
        Default::default(),
        None,
        0,
    );
    let mut state = env.state.clone();
    let mut decision = env.decision.clone().expect("decision");
    c.bench_function("mask_construction_forced", |b| {
        b.iter(|| {
            state.turn.active_player ^= 1;
            decision.player = state.turn.active_player;
            let actions = weiss_core::legal::legal_actions(
                black_box(&state),
                black_box(&decision),
                black_box(&env.db),
                black_box(&env.curriculum),
            );
            let mask = weiss_core::encode::build_action_mask(black_box(&actions));
            black_box(mask);
        })
    });
}

fn fill_choice_actions(env: &GameEnv, actions: &mut Vec<ActionDesc>) {
    actions.clear();
    if let Some(choice) = env.state.turn.choice.as_ref() {
        let total = choice.total_candidates as usize;
        let page_start = choice.page_start as usize;
        let safe_start = page_start.min(total);
        let page_end = total.min(safe_start + CHOICE_COUNT);
        for idx in 0..(page_end - safe_start) {
            actions.push(ActionDesc::ChoiceSelect { index: idx as u8 });
        }
        if page_start >= CHOICE_COUNT {
            actions.push(ActionDesc::ChoicePrevPage);
        }
        if page_start + CHOICE_COUNT < total {
            actions.push(ActionDesc::ChoiceNextPage);
        }
    }
    if env.curriculum.allow_concede {
        actions.push(ActionDesc::Concede);
    }
}

fn bench_choice_paging_worst_case(c: &mut Criterion) {
    let total = 200usize;
    let db = make_choice_db(total as u32);
    let config = make_config();
    let curriculum = CurriculumConfig::default();
    let mut env = GameEnv::new(db, config, curriculum, 123, Default::default(), None, 0);
    install_choice(&mut env, total);
    let mut actions = Vec::with_capacity(64);
    let mut mask = vec![0u8; weiss_core::encode::ACTION_SPACE_SIZE];
    let mut lookup = vec![None; weiss_core::encode::ACTION_SPACE_SIZE];
    c.bench_function("choice_paging_worst_case_mask", |b| {
        b.iter(|| {
            fill_choice_actions(&env, &mut actions);
            fill_action_mask(black_box(&actions), &mut mask, &mut lookup);
            black_box(mask.as_slice());
        })
    });
}

criterion_group!(
    benches,
    bench_advance_until_decision,
    bench_step_batch,
    bench_step_batch_fast_priority_off,
    bench_step_batch_fast_priority_on,
    bench_legal_actions,
    bench_legal_actions_forced,
    bench_on_reverse_decision_frequency,
    bench_observation_encode,
    bench_observation_encode_forced,
    bench_mask_construction,
    bench_mask_construction_forced,
    bench_choice_paging_worst_case
);
criterion_main!(benches);
