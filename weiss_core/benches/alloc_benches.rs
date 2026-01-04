use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::time::Instant;

use criterion::{black_box, criterion_group, criterion_main, Criterion};

use weiss_core::config::{CurriculumConfig, EnvConfig, RewardConfig};
use weiss_core::db::{CardColor, CardDb, CardStatic, CardType};
use weiss_core::encode::{fill_action_mask, CHOICE_COUNT};
use weiss_core::env::GameEnv;
use weiss_core::legal::{Decision, DecisionKind};
use weiss_core::pool::EnvPool;
use weiss_core::state::{ChoiceOptionRef, ChoiceReason, ChoiceState, ChoiceZone};
use weiss_core::DebugConfig;

struct CountingAlloc;

static ALLOC_COUNT: AtomicUsize = AtomicUsize::new(0);
static DEALLOC_COUNT: AtomicUsize = AtomicUsize::new(0);
static REALLOC_COUNT: AtomicUsize = AtomicUsize::new(0);
static ALLOC_BYTES: AtomicUsize = AtomicUsize::new(0);
static PRINT_LEGAL: AtomicBool = AtomicBool::new(false);
static PRINT_OBS: AtomicBool = AtomicBool::new(false);
static PRINT_MASKS: AtomicBool = AtomicBool::new(false);

#[global_allocator]
static GLOBAL: CountingAlloc = CountingAlloc;

unsafe impl GlobalAlloc for CountingAlloc {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        ALLOC_COUNT.fetch_add(1, Ordering::Relaxed);
        ALLOC_BYTES.fetch_add(layout.size(), Ordering::Relaxed);
        System.alloc(layout)
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        ALLOC_COUNT.fetch_add(1, Ordering::Relaxed);
        ALLOC_BYTES.fetch_add(layout.size(), Ordering::Relaxed);
        System.alloc_zeroed(layout)
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        DEALLOC_COUNT.fetch_add(1, Ordering::Relaxed);
        System.dealloc(ptr, layout)
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        REALLOC_COUNT.fetch_add(1, Ordering::Relaxed);
        ALLOC_BYTES.fetch_add(new_size, Ordering::Relaxed);
        System.realloc(ptr, layout, new_size)
    }
}

fn reset_alloc_counts() {
    ALLOC_COUNT.store(0, Ordering::Relaxed);
    DEALLOC_COUNT.store(0, Ordering::Relaxed);
    REALLOC_COUNT.store(0, Ordering::Relaxed);
    ALLOC_BYTES.store(0, Ordering::Relaxed);
}

fn alloc_snapshot() -> (usize, usize, usize, usize) {
    (
        ALLOC_COUNT.load(Ordering::Relaxed),
        DEALLOC_COUNT.load(Ordering::Relaxed),
        REALLOC_COUNT.load(Ordering::Relaxed),
        ALLOC_BYTES.load(Ordering::Relaxed),
    )
}

fn make_db() -> CardDb {
    let cards = (1u32..=13)
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
    CardDb::new(cards).expect("db build")
}

fn make_choice_db(card_count: u32) -> CardDb {
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
    CardDb::new(cards).expect("db build")
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

fn fill_choice_actions(env: &GameEnv, actions: &mut Vec<weiss_core::legal::ActionDesc>) {
    actions.clear();
    if let Some(choice) = env.state.turn.choice.as_ref() {
        let total = choice.total_candidates as usize;
        let page_start = choice.page_start as usize;
        let safe_start = page_start.min(total);
        let page_end = total.min(safe_start + CHOICE_COUNT);
        for idx in 0..(page_end - safe_start) {
            actions.push(weiss_core::legal::ActionDesc::ChoiceSelect { index: idx as u8 });
        }
        if page_start >= CHOICE_COUNT {
            actions.push(weiss_core::legal::ActionDesc::ChoicePrevPage);
        }
        if page_start + CHOICE_COUNT < total {
            actions.push(weiss_core::legal::ActionDesc::ChoiceNextPage);
        }
    }
}

fn bench_alloc_legal_actions(c: &mut Criterion) {
    let db = make_db();
    let config = make_config();
    let curriculum = CurriculumConfig::default();
    let env = GameEnv::new(
        std::sync::Arc::new(db),
        config,
        curriculum,
        9,
        Default::default(),
        None,
        0,
    );
    let decision = env.decision.clone().unwrap_or(Decision {
        player: 0,
        kind: DecisionKind::Mulligan,
        focus_slot: None,
    });
    c.bench_function("alloc_legal_actions", |b| {
        b.iter_custom(|iters| {
            let start = Instant::now();
            let mut total_allocs = 0usize;
            for _ in 0..iters {
                reset_alloc_counts();
                let actions = weiss_core::legal::legal_actions(
                    &env.state,
                    &decision,
                    &env.db,
                    &env.curriculum,
                );
                black_box(actions);
                let (allocs, _, _, _) = alloc_snapshot();
                total_allocs += allocs;
            }
            if iters > 0 && !PRINT_LEGAL.swap(true, Ordering::Relaxed) {
                println!(
                    "alloc_legal_actions avg_allocs_per_iter={}",
                    total_allocs / iters as usize
                );
            }
            start.elapsed()
        })
    });
}

fn bench_alloc_observation_encode(c: &mut Criterion) {
    let db = make_db();
    let config = make_config();
    let curriculum = CurriculumConfig::default();
    let env = GameEnv::new(
        std::sync::Arc::new(db),
        config,
        curriculum,
        11,
        Default::default(),
        None,
        0,
    );
    c.bench_function("alloc_observation_encode", |b| {
        b.iter_custom(|iters| {
            let start = Instant::now();
            let mut total_allocs = 0usize;
            let mut buf = vec![0i32; weiss_core::encode::OBS_LEN];
            for _ in 0..iters {
                reset_alloc_counts();
                weiss_core::encode::encode_observation(
                    &env.state,
                    &env.db,
                    &env.curriculum,
                    0,
                    env.decision.as_ref(),
                    env.last_action_desc.as_ref(),
                    env.last_action_player,
                    env.config.observation_visibility,
                    &mut buf,
                );
                black_box(buf.as_slice());
                let (allocs, _, _, _) = alloc_snapshot();
                total_allocs += allocs;
            }
            if iters > 0 && !PRINT_OBS.swap(true, Ordering::Relaxed) {
                println!(
                    "alloc_observation_encode avg_allocs_per_iter={}",
                    total_allocs / iters as usize
                );
            }
            start.elapsed()
        })
    });
}

fn bench_alloc_action_masks_batch_into(c: &mut Criterion) {
    let db = make_db();
    let config = make_config();
    let curriculum = CurriculumConfig::default();
    let pool = EnvPool::new_debug(
        64,
        std::sync::Arc::new(db),
        config,
        curriculum,
        7,
        None,
        DebugConfig::default(),
    )
    .expect("pool");
    let mut masks = vec![0u8; pool.envs.len() * weiss_core::encode::ACTION_SPACE_SIZE];
    c.bench_function("alloc_action_masks_batch_into", |b| {
        b.iter_custom(|iters| {
            let start = Instant::now();
            let mut total_allocs = 0usize;
            for _ in 0..iters {
                reset_alloc_counts();
                pool.action_masks_batch_into(&mut masks)
                    .expect("mask buffer");
                black_box(masks.as_slice());
                let (allocs, _, _, _) = alloc_snapshot();
                total_allocs += allocs;
            }
            if iters > 0 && !PRINT_MASKS.swap(true, Ordering::Relaxed) {
                println!(
                    "alloc_action_masks_batch_into avg_allocs_per_iter={}",
                    total_allocs / iters as usize
                );
            }
            start.elapsed()
        })
    });
}

fn bench_alloc_choice_paging_worst_case(c: &mut Criterion) {
    let total = 200usize;
    let db = make_choice_db(total as u32);
    let config = make_config();
    let curriculum = CurriculumConfig::default();
    let mut env = GameEnv::new(
        std::sync::Arc::new(db),
        config,
        curriculum,
        99,
        Default::default(),
        None,
        0,
    );
    install_choice(&mut env, total);
    let mut actions = Vec::with_capacity(64);
    let mut mask = vec![0u8; weiss_core::encode::ACTION_SPACE_SIZE];
    let mut lookup = vec![None; weiss_core::encode::ACTION_SPACE_SIZE];
    c.bench_function("alloc_choice_paging_worst_case", |b| {
        b.iter_custom(|iters| {
            let start = Instant::now();
            let mut total_allocs = 0usize;
            for _ in 0..iters {
                reset_alloc_counts();
                fill_choice_actions(&env, &mut actions);
                fill_action_mask(&actions, &mut mask, &mut lookup);
                black_box(mask.as_slice());
                let (allocs, _, _, _) = alloc_snapshot();
                total_allocs += allocs;
            }
            if iters > 0 && !PRINT_MASKS.swap(true, Ordering::Relaxed) {
                println!(
                    "alloc_choice_paging_worst_case avg_allocs_per_iter={}",
                    total_allocs / iters as usize
                );
            }
            start.elapsed()
        })
    });
}

criterion_group!(
    alloc_benches,
    bench_alloc_legal_actions,
    bench_alloc_observation_encode,
    bench_alloc_action_masks_batch_into,
    bench_alloc_choice_paging_worst_case
);
criterion_main!(alloc_benches);
