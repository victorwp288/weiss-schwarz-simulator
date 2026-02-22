#![allow(dead_code)]

#[path = "deck_support.rs"]
mod deck_support;

use std::sync::{Arc, OnceLock};

use weiss_core::db::{CardDb, CardStatic};
use weiss_core::env::GameEnv;
use weiss_core::legal::ActionDesc;
use weiss_core::replay::ReplayConfig;
use weiss_core::state::{CardInstance, ChoiceReason, ChoiceZone, StageSlot, StageStatus};

pub const CLONE_OFFSET: u32 = deck_support::CLONE_OFFSET;

fn make_instance(card_id: u32, owner: u8, zone_tag: u32, index: usize) -> CardInstance {
    let instance_id = ((owner as u32) << 24) | (zone_tag << 16) | (index as u32);
    CardInstance::new(card_id, owner, instance_id)
}

pub fn enable_validate() {
    static VALIDATE_ONCE: OnceLock<()> = OnceLock::new();
    VALIDATE_ONCE.get_or_init(|| {
        std::env::set_var("WEISS_VALIDATE_STATE", "1");
    });
}

pub fn replay_config() -> ReplayConfig {
    let mut config = ReplayConfig {
        enabled: true,
        sample_rate: 1.0,
        out_dir: std::env::temp_dir(),
        compress: false,
        include_trigger_card_id: true,
        ..Default::default()
    };
    config.rebuild_cache();
    config
}

pub fn make_db(mut cards: Vec<CardStatic>) -> Arc<CardDb> {
    deck_support::add_clone_cards(&mut cards);
    Arc::new(CardDb::new(cards).expect("db build"))
}

pub fn legalize_deck(deck: Vec<u32>, pool: &[u32]) -> Vec<u32> {
    deck_support::legalize_deck(deck, pool)
}

pub fn build_deck_list(size: usize, extras: &[u32], filler: u32) -> Vec<u32> {
    let mut deck = extras.to_vec();
    while deck.len() < size {
        deck.push(filler);
    }
    legalize_deck(deck, &[filler])
}

pub fn build_deck_list_with_clone_padding(
    size: usize,
    extras: &[u32],
    base_id: u32,
    target_len: usize,
) -> Vec<u32> {
    let mut deck = extras.to_vec();
    if deck.len() > size {
        deck.truncate(size);
    }
    extend_with_filler(&mut deck, base_id, size);
    extend_with_filler(&mut deck, base_id, target_len);
    deck
}

fn extend_with_filler(deck: &mut Vec<u32>, base_id: u32, target_len: usize) {
    use std::collections::HashMap;
    let mut counts: HashMap<u32, usize> = HashMap::new();
    let mut next_clone: HashMap<u32, u32> = HashMap::new();
    for &card_id in deck.iter() {
        *counts.entry(card_id).or_insert(0) += 1;
    }
    while deck.len() < target_len {
        let card_id = assign_id(base_id, &mut counts, &mut next_clone);
        deck.push(card_id);
    }
}

fn assign_id(
    base_id: u32,
    counts: &mut std::collections::HashMap<u32, usize>,
    next_clone: &mut std::collections::HashMap<u32, u32>,
) -> u32 {
    let count = counts.entry(base_id).or_insert(0);
    if *count < 4 {
        *count += 1;
        return base_id;
    }
    loop {
        let idx = next_clone.entry(base_id).or_insert(1);
        if *idx > deck_support::CLONE_GROUPS as u32 {
            panic!(
                "not enough clone ids for base {} (needed clone group {})",
                base_id, idx
            );
        }
        let clone_id = base_id + deck_support::CLONE_OFFSET * *idx;
        let clone_count = counts.entry(clone_id).or_insert(0);
        if *clone_count < 4 {
            *clone_count += 1;
            return clone_id;
        }
        *idx += 1;
    }
}

pub fn choose_priority_activation(env: &mut GameEnv) {
    if let Some(choice) = env.state.turn.choice.as_ref() {
        if choice.reason == ChoiceReason::PriorityActionSelect {
            let idx = choice
                .options
                .iter()
                .enumerate()
                .filter(|(_, opt)| opt.zone == ChoiceZone::PriorityAct)
                .min_by_key(|(_, opt)| {
                    (
                        opt.index.unwrap_or(u16::MAX),
                        opt.target_slot.unwrap_or(u8::MAX),
                    )
                })
                .map(|(idx, _)| idx)
                .expect("priority activation");
            env.apply_action(ActionDesc::ChoiceSelect { index: idx as u8 })
                .unwrap();
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub fn setup_player_state(
    env: &mut GameEnv,
    player: usize,
    hand: Vec<u32>,
    stock: Vec<u32>,
    stage_cards: Vec<(usize, u32)>,
    deck_top: Vec<u32>,
    clock: Vec<u32>,
    level: Vec<u32>,
    waiting_room: Vec<u32>,
    memory: Vec<u32>,
    climax: Vec<u32>,
) {
    setup_player_state_with_stage_zone_tag(
        env,
        player,
        hand,
        stock,
        stage_cards,
        deck_top,
        clock,
        level,
        waiting_room,
        memory,
        climax,
        4,
    );
}

#[allow(clippy::too_many_arguments)]
pub fn setup_player_state_with_stage_zone_tag(
    env: &mut GameEnv,
    player: usize,
    hand: Vec<u32>,
    stock: Vec<u32>,
    stage_cards: Vec<(usize, u32)>,
    deck_top: Vec<u32>,
    clock: Vec<u32>,
    level: Vec<u32>,
    waiting_room: Vec<u32>,
    memory: Vec<u32>,
    climax: Vec<u32>,
    stage_zone_tag: u32,
) {
    use std::collections::HashMap;
    let mut counts: HashMap<u32, i32> = HashMap::new();
    for &card in &env.config.deck_lists[player] {
        *counts.entry(card).or_insert(0) += 1;
    }
    let mut consume = |card: u32, zone: &str| {
        let entry = counts.entry(card).or_insert(0);
        *entry -= 1;
        if *entry < 0 {
            panic!("card {card} overused in {zone}");
        }
    };

    for &card in &hand {
        consume(card, "hand");
    }
    for &card in &stock {
        consume(card, "stock");
    }
    for &card in &deck_top {
        consume(card, "deck_top");
    }
    for &card in &clock {
        consume(card, "clock");
    }
    for &card in &level {
        consume(card, "level");
    }
    for &card in &waiting_room {
        consume(card, "waiting_room");
    }
    for &card in &memory {
        consume(card, "memory");
    }
    for &card in &climax {
        consume(card, "climax");
    }
    for &(_, card) in &stage_cards {
        consume(card, "stage");
    }

    let mut remaining = Vec::new();
    for (card, count) in counts {
        if count < 0 {
            panic!("card {card} negative count");
        }
        for _ in 0..count {
            remaining.push(card);
        }
    }

    let mut deck = remaining;
    let mut top = deck_top;
    top.reverse();
    deck.extend(top);

    let owner = player as u8;
    let p = &mut env.state.players[player];
    p.hand = hand
        .into_iter()
        .enumerate()
        .map(|(idx, id)| make_instance(id, owner, 1, idx))
        .collect();
    p.stock = stock
        .into_iter()
        .enumerate()
        .map(|(idx, id)| make_instance(id, owner, 2, idx))
        .collect();
    p.clock = clock
        .into_iter()
        .enumerate()
        .map(|(idx, id)| make_instance(id, owner, 3, idx))
        .collect();
    p.level = level
        .into_iter()
        .enumerate()
        .map(|(idx, id)| make_instance(id, owner, 4, idx))
        .collect();
    p.waiting_room = waiting_room
        .into_iter()
        .enumerate()
        .map(|(idx, id)| make_instance(id, owner, 5, idx))
        .collect();
    p.memory = memory
        .into_iter()
        .enumerate()
        .map(|(idx, id)| make_instance(id, owner, 6, idx))
        .collect();
    p.climax = climax
        .into_iter()
        .enumerate()
        .map(|(idx, id)| make_instance(id, owner, 7, idx))
        .collect();
    p.deck = deck
        .into_iter()
        .enumerate()
        .map(|(idx, id)| make_instance(id, owner, 8, idx))
        .collect();
    p.stage = [
        StageSlot::empty(),
        StageSlot::empty(),
        StageSlot::empty(),
        StageSlot::empty(),
        StageSlot::empty(),
    ];
    for (slot, card) in stage_cards {
        let mut slot_state = StageSlot::empty();
        slot_state.card = Some(make_instance(card, owner, stage_zone_tag, slot));
        slot_state.status = StageStatus::Stand;
        p.stage[slot] = slot_state;
    }
}
