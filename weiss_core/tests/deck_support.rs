#![allow(dead_code)]

use std::collections::HashMap;

use weiss_core::db::CardStatic;
use weiss_core::encode::MAX_DECK;

pub const CLONE_OFFSET: u32 = 1000;
pub const CLONE_GROUPS: usize = 12;
const MAX_COPIES: usize = 4;

pub fn add_clone_cards(cards: &mut Vec<CardStatic>) {
    let base_cards = cards.clone();
    for base in base_cards {
        for idx in 1..=CLONE_GROUPS {
            let mut clone = base.clone();
            clone.id = base.id + CLONE_OFFSET * idx as u32;
            cards.push(clone);
        }
    }
}

pub fn legalize_deck(mut deck: Vec<u32>, filler_pool: &[u32]) -> Vec<u32> {
    if deck.len() > MAX_DECK {
        panic!("deck length {} exceeds MAX_DECK {}", deck.len(), MAX_DECK);
    }
    if filler_pool.is_empty() {
        panic!("filler pool empty");
    }
    let mut counts: HashMap<u32, usize> = HashMap::new();
    let mut next_clone: HashMap<u32, u32> = HashMap::new();
    for card_id in &mut deck {
        *card_id = assign_id(*card_id, &mut counts, &mut next_clone);
    }
    let mut filler_iter = filler_pool.iter().copied().cycle();
    while deck.len() < MAX_DECK {
        let base = filler_iter.next().expect("filler");
        let card_id = assign_id(base, &mut counts, &mut next_clone);
        deck.push(card_id);
    }
    deck
}

fn assign_id(
    base_id: u32,
    counts: &mut HashMap<u32, usize>,
    next_clone: &mut HashMap<u32, u32>,
) -> u32 {
    let count = counts.entry(base_id).or_insert(0);
    if *count < MAX_COPIES {
        *count += 1;
        return base_id;
    }
    loop {
        let idx = next_clone.entry(base_id).or_insert(1);
        if *idx > CLONE_GROUPS as u32 {
            panic!(
                "not enough clone ids for base {} (needed clone group {})",
                base_id, idx
            );
        }
        let clone_id = base_id + CLONE_OFFSET * *idx;
        let clone_count = counts.entry(clone_id).or_insert(0);
        if *clone_count < MAX_COPIES {
            *clone_count += 1;
            return clone_id;
        }
        *idx += 1;
    }
}
