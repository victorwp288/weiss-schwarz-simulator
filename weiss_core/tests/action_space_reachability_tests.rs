use std::collections::HashSet;
use std::sync::Arc;

use weiss_core::config::CurriculumConfig;
use weiss_core::db::{CardColor, CardDb, CardStatic, CardType};
use weiss_core::encode::{
    fill_action_mask, ACTION_SPACE_SIZE, ATTACK_SLOT_COUNT, CHOICE_COUNT, MAX_HAND, MAX_STAGE,
    TRIGGER_ORDER_COUNT,
};
use weiss_core::legal::{legal_actions, Decision, DecisionKind};
use weiss_core::state::{
    CardInstance, ChoiceOptionRef, ChoiceReason, ChoiceState, ChoiceZone, GameState, StageSlot,
    StageStatus, TriggerOrderState,
};

const CARD_CHAR: u32 = 1;
const CARD_EVENT: u32 = 2;
const CARD_CLIMAX: u32 = 3;

fn build_db() -> Arc<CardDb> {
    let cards = vec![
        CardStatic {
            id: CARD_CHAR,
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
        },
        CardStatic {
            id: CARD_EVENT,
            card_set: None,
            card_type: CardType::Event,
            color: CardColor::Red,
            level: 0,
            cost: 0,
            power: 0,
            soul: 0,
            triggers: vec![],
            traits: vec![],
            abilities: vec![],
            ability_defs: vec![],
            counter_timing: false,
            raw_text: None,
        },
        CardStatic {
            id: CARD_CLIMAX,
            card_set: None,
            card_type: CardType::Climax,
            color: CardColor::Red,
            level: 0,
            cost: 0,
            power: 0,
            soul: 0,
            triggers: vec![],
            traits: vec![],
            abilities: vec![],
            ability_defs: vec![],
            counter_timing: false,
            raw_text: None,
        },
    ];
    Arc::new(CardDb::new(cards).expect("db build"))
}

fn base_state(seed: u64) -> GameState {
    let mut state = GameState::new(
        vec![CARD_CHAR; weiss_core::encode::MAX_DECK],
        vec![CARD_CHAR; weiss_core::encode::MAX_DECK],
        seed,
        0,
    );
    for player in 0..2 {
        let p = &mut state.players[player];
        p.deck.clear();
        p.hand.clear();
        p.waiting_room.clear();
        p.clock.clear();
        p.level.clear();
        p.stock.clear();
        p.memory.clear();
        p.climax.clear();
        p.resolution.clear();
        p.stage = [
            StageSlot::empty(),
            StageSlot::empty(),
            StageSlot::empty(),
            StageSlot::empty(),
            StageSlot::empty(),
        ];
    }
    state.turn.turn_number = 1;
    state.turn.attack_subphase_count = 0;
    state
}

fn make_instance(card_id: u32, owner: u8, zone_tag: u32, index: usize) -> CardInstance {
    let instance_id = ((owner as u32) << 24) | (zone_tag << 16) | (index as u32);
    CardInstance::new(card_id, owner, instance_id)
}

fn set_hand(state: &mut GameState, player: usize, card_id: u32, count: usize) {
    let owner = player as u8;
    state.players[player].hand = (0..count)
        .map(|idx| make_instance(card_id, owner, 1, idx))
        .collect();
}

fn set_clock(state: &mut GameState, player: usize, card_id: u32, count: usize) {
    let owner = player as u8;
    state.players[player].clock = (0..count)
        .map(|idx| make_instance(card_id, owner, 2, idx))
        .collect();
}

fn set_stock(state: &mut GameState, player: usize, card_id: u32, count: usize) {
    let owner = player as u8;
    state.players[player].stock = (0..count)
        .map(|idx| make_instance(card_id, owner, 3, idx))
        .collect();
}

fn set_stage_full(state: &mut GameState, player: usize, card_id: u32) {
    let owner = player as u8;
    for slot in 0..MAX_STAGE {
        let mut slot_state = StageSlot::empty();
        slot_state.card = Some(make_instance(card_id, owner, 4, slot));
        slot_state.status = StageStatus::Stand;
        state.players[player].stage[slot] = slot_state;
    }
}

fn set_stage_front_row(state: &mut GameState, player: usize, card_id: u32) {
    let owner = player as u8;
    for slot in 0..ATTACK_SLOT_COUNT {
        let mut slot_state = StageSlot::empty();
        slot_state.card = Some(make_instance(card_id, owner, 4, slot));
        slot_state.status = StageStatus::Stand;
        state.players[player].stage[slot] = slot_state;
    }
}

fn record_mask(
    state: &GameState,
    decision: &Decision,
    db: &CardDb,
    curriculum: &CurriculumConfig,
    seen: &mut HashSet<usize>,
) {
    let actions = legal_actions(state, decision, db, curriculum);
    let mut mask = vec![0u8; ACTION_SPACE_SIZE];
    let mut lookup = vec![None; ACTION_SPACE_SIZE];
    fill_action_mask(&actions, &mut mask, &mut lookup);
    for (idx, value) in mask.iter().enumerate() {
        if *value == 1 {
            seen.insert(idx);
        }
    }
}

#[test]
fn action_space_ids_are_reachable() {
    let db = build_db();
    let base = CurriculumConfig {
        allow_concede: true,
        ..Default::default()
    };
    let mut curricula = vec![base];
    curricula.push(CurriculumConfig {
        allow_concede: true,
        reduced_stage_mode: true,
        ..Default::default()
    });
    let seeds = [1u64, 2u64, 3u64];
    let mut seen: HashSet<usize> = HashSet::new();

    for seed in seeds {
        for curriculum in &curricula {
            // Mulligan confirm + all select indices.
            let mut state = base_state(seed);
            set_hand(&mut state, 0, CARD_CHAR, MAX_HAND);
            let decision = Decision {
                player: 0,
                kind: DecisionKind::Mulligan,
                focus_slot: None,
            };
            record_mask(&state, &decision, &db, curriculum, &mut seen);

            // Clock pass + all hand indices.
            let mut state = base_state(seed);
            set_hand(&mut state, 0, CARD_CHAR, MAX_HAND);
            let decision = Decision {
                player: 0,
                kind: DecisionKind::Clock,
                focus_slot: None,
            };
            record_mask(&state, &decision, &db, curriculum, &mut seen);

            // Main: play characters into every slot.
            let mut state = base_state(seed);
            set_hand(&mut state, 0, CARD_CHAR, MAX_HAND);
            let decision = Decision {
                player: 0,
                kind: DecisionKind::Main,
                focus_slot: None,
            };
            record_mask(&state, &decision, &db, curriculum, &mut seen);

            // Main: play events from every hand index.
            let mut state = base_state(seed);
            set_hand(&mut state, 0, CARD_EVENT, MAX_HAND);
            let decision = Decision {
                player: 0,
                kind: DecisionKind::Main,
                focus_slot: None,
            };
            record_mask(&state, &decision, &db, curriculum, &mut seen);

            // Main: move between every occupied slot.
            let mut state = base_state(seed);
            set_stage_full(&mut state, 0, CARD_CHAR);
            let decision = Decision {
                player: 0,
                kind: DecisionKind::Main,
                focus_slot: None,
            };
            record_mask(&state, &decision, &db, curriculum, &mut seen);

            // Climax: pass + play from every hand index.
            let mut state = base_state(seed);
            set_hand(&mut state, 0, CARD_CLIMAX, MAX_HAND);
            let decision = Decision {
                player: 0,
                kind: DecisionKind::Climax,
                focus_slot: None,
            };
            record_mask(&state, &decision, &db, curriculum, &mut seen);

            // Attack pass when no attackers.
            let state = base_state(seed);
            let decision = Decision {
                player: 0,
                kind: DecisionKind::AttackDeclaration,
                focus_slot: None,
            };
            record_mask(&state, &decision, &db, curriculum, &mut seen);

            // Attack frontal + side with defenders present.
            let mut state = base_state(seed);
            set_stage_front_row(&mut state, 0, CARD_CHAR);
            set_stage_front_row(&mut state, 1, CARD_CHAR);
            let decision = Decision {
                player: 0,
                kind: DecisionKind::AttackDeclaration,
                focus_slot: None,
            };
            record_mask(&state, &decision, &db, curriculum, &mut seen);

            // Attack direct with defenders absent.
            let mut state = base_state(seed);
            set_stage_front_row(&mut state, 0, CARD_CHAR);
            let decision = Decision {
                player: 0,
                kind: DecisionKind::AttackDeclaration,
                focus_slot: None,
            };
            record_mask(&state, &decision, &db, curriculum, &mut seen);

            // Level up choices (7).
            let mut state = base_state(seed);
            set_clock(&mut state, 0, CARD_CHAR, 7);
            let decision = Decision {
                player: 0,
                kind: DecisionKind::LevelUp,
                focus_slot: None,
            };
            record_mask(&state, &decision, &db, curriculum, &mut seen);

            // Encore select per reversed slot.
            let mut state = base_state(seed);
            let max_slot = if curriculum.reduced_stage_mode {
                1
            } else {
                MAX_STAGE
            };
            for slot in 0..max_slot {
                state.players[0].stage[slot] = StageSlot {
                    card: Some(make_instance(CARD_CHAR, 0, 9, slot)),
                    status: StageStatus::Reverse,
                    power_mod_battle: 0,
                    power_mod_turn: 0,
                    has_attacked: false,
                    cannot_attack: false,
                    attack_cost: 0,
                };
            }
            set_stock(&mut state, 0, CARD_CHAR, 3);
            let decision = Decision {
                player: 0,
                kind: DecisionKind::Encore,
                focus_slot: None,
            };
            record_mask(&state, &decision, &db, curriculum, &mut seen);

            // Trigger order choices.
            let mut state = base_state(seed);
            state.turn.trigger_order = Some(TriggerOrderState {
                group_id: 1,
                player: 0,
                choices: (0..TRIGGER_ORDER_COUNT).map(|idx| idx as u32).collect(),
            });
            let decision = Decision {
                player: 0,
                kind: DecisionKind::TriggerOrder,
                focus_slot: None,
            };
            record_mask(&state, &decision, &db, curriculum, &mut seen);

            // Choice selections (16).
            let mut state = base_state(seed);
            let choice_total = CHOICE_COUNT + 4;
            let options: Vec<ChoiceOptionRef> = (0..choice_total)
                .map(|idx| ChoiceOptionRef {
                    card_id: CARD_CHAR,
                    instance_id: (idx + 1) as u32,
                    zone: ChoiceZone::Hand,
                    index: Some(idx as u8),
                    target_slot: None,
                })
                .collect();
            state.turn.choice = Some(ChoiceState {
                id: 1,
                reason: ChoiceReason::TargetSelect,
                player: 0,
                options: options.clone(),
                total_candidates: choice_total as u16,
                page_start: 0,
                pending_trigger: None,
            });
            let decision = Decision {
                player: 0,
                kind: DecisionKind::Choice,
                focus_slot: None,
            };
            record_mask(&state, &decision, &db, curriculum, &mut seen);
            if choice_total > CHOICE_COUNT {
                state.turn.choice = Some(ChoiceState {
                    id: 1,
                    reason: ChoiceReason::TargetSelect,
                    player: 0,
                    options,
                    total_candidates: choice_total as u16,
                    page_start: CHOICE_COUNT as u16,
                    pending_trigger: None,
                });
                record_mask(&state, &decision, &db, curriculum, &mut seen);
            }
        }
    }

    let mut missing = Vec::new();
    for id in 0..ACTION_SPACE_SIZE {
        if !seen.contains(&id) {
            missing.push(id);
        }
    }
    assert!(missing.is_empty(), "unreachable action ids: {:?}", missing);
}
