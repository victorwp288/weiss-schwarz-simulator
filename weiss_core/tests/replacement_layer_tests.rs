use std::sync::{Arc, OnceLock};

#[path = "deck_support.rs"]
mod deck_support;

use weiss_core::config::{
    CurriculumConfig, EnvConfig, ErrorPolicy, ObservationVisibility, RewardConfig,
};
use weiss_core::db::{
    AbilityCost, AbilityDef, AbilityKind, AbilityTiming, CardColor, CardDb, CardStatic, CardType,
    EffectTemplate,
};
use weiss_core::effects::{
    EffectId, EffectSourceKind, ReplacementHook, ReplacementKind, ReplacementSpec,
};
use weiss_core::env::GameEnv;
use weiss_core::legal::{ActionDesc, Decision, DecisionKind};
use weiss_core::replay::ReplayConfig;
use weiss_core::state::{CardInstance, Phase, StageSlot, StageStatus, TargetSide, TerminalResult};

const CARD_DAMAGE_ACT: u32 = 90;
const CARD_BASIC: u32 = 91;

fn make_instance(card_id: u32, owner: u8, zone_tag: u32, index: usize) -> CardInstance {
    let instance_id = ((owner as u32) << 24) | (zone_tag << 16) | (index as u32);
    CardInstance::new(card_id, owner, instance_id)
}

fn enable_validate() {
    static VALIDATE_ONCE: OnceLock<()> = OnceLock::new();
    VALIDATE_ONCE.get_or_init(|| {
        std::env::set_var("WEISS_VALIDATE_STATE", "1");
    });
}

fn make_db() -> Arc<CardDb> {
    let mut cards = vec![
        CardStatic {
            id: CARD_DAMAGE_ACT,
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
            ability_defs: vec![AbilityDef {
                kind: AbilityKind::Auto,
                timing: Some(AbilityTiming::OnPlay),
                effects: vec![EffectTemplate::DealDamage {
                    amount: 1,
                    cancelable: true,
                }],
                targets: vec![],
                cost: AbilityCost::default(),
                target_card_type: None,
                target_trait: None,
                target_level_max: None,
                target_cost_max: None,
                target_limit: None,
            }],
            counter_timing: false,
            raw_text: None,
        },
        CardStatic {
            id: CARD_BASIC,
            card_set: None,
            card_type: CardType::Character,
            color: CardColor::Blue,
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
    ];
    deck_support::add_clone_cards(&mut cards);
    Arc::new(CardDb::new(cards).expect("db build"))
}

fn make_config(deck_a: Vec<u32>, deck_b: Vec<u32>) -> EnvConfig {
    let pool = [CARD_BASIC, CARD_DAMAGE_ACT];
    EnvConfig {
        deck_lists: [
            deck_support::legalize_deck(deck_a, &pool),
            deck_support::legalize_deck(deck_b, &pool),
        ],
        deck_ids: [900, 901],
        max_decisions: 200,
        max_ticks: 10_000,
        reward: RewardConfig::default(),
        error_policy: ErrorPolicy::Strict,
        observation_visibility: ObservationVisibility::Public,
        end_condition_policy: Default::default(),
    }
}

fn setup_player_state(
    env: &mut GameEnv,
    player: usize,
    stage_card: Option<u32>,
    deck_fill: u32,
    deck_count: usize,
) {
    let owner = player as u8;
    let p = &mut env.state.players[player];
    let mut deck: Vec<CardInstance> = env.config.deck_lists[player]
        .iter()
        .copied()
        .enumerate()
        .map(|(idx, id)| make_instance(id, owner, 8, idx))
        .collect();
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
    if let Some(stage_card) = stage_card {
        let mut slot_state = StageSlot::empty();
        if let Some(pos) = deck.iter().position(|card| card.id == stage_card) {
            slot_state.card = Some(deck.remove(pos));
        } else {
            slot_state.card = Some(make_instance(stage_card, owner, 4, 0));
        }
        slot_state.status = StageStatus::Stand;
        p.stage[0] = slot_state;
    }
    while deck.len() > deck_count {
        if let Some(pos) = deck.iter().position(|card| card.id == deck_fill) {
            deck.remove(pos);
        } else {
            deck.pop();
        }
    }
    p.deck = deck;
}

#[test]
fn replacements_apply_in_priority_order() {
    enable_validate();
    let db = make_db();
    let deck_a = vec![CARD_DAMAGE_ACT; 50];
    let deck_b = vec![CARD_BASIC; 50];
    let curriculum = CurriculumConfig {
        allow_character: true,
        ..Default::default()
    };
    let config = make_config(deck_a, deck_b);
    let replay_config = ReplayConfig {
        enabled: true,
        sample_rate: 1.0,
        ..Default::default()
    };
    let mut env = GameEnv::new(db, config, curriculum, 77, replay_config, None, 0);

    setup_player_state(&mut env, 0, None, CARD_DAMAGE_ACT, 49);
    setup_player_state(&mut env, 1, None, CARD_BASIC, 50);
    env.state.players[0]
        .hand
        .push(make_instance(CARD_DAMAGE_ACT, 0, 3, 0));
    env.state.turn.phase = Phase::Main;
    env.state.turn.active_player = 0;
    env.state.turn.starting_player = 0;
    env.state.turn.mulligan_done = [true, true];
    env.decision = Some(Decision {
        player: 0,
        kind: DecisionKind::Main,
        focus_slot: None,
    });

    env.state.replacements = vec![
        ReplacementSpec {
            id: EffectId::new(EffectSourceKind::Replacement, CARD_DAMAGE_ACT, 0, 0),
            source: CARD_DAMAGE_ACT,
            hook: ReplacementHook::Damage,
            kind: ReplacementKind::RedirectDamage {
                new_target: TargetSide::Opponent,
            },
            priority: 0,
            insertion: 1,
        },
        ReplacementSpec {
            id: EffectId::new(EffectSourceKind::Replacement, CARD_DAMAGE_ACT, 0, 1),
            source: CARD_DAMAGE_ACT,
            hook: ReplacementHook::Damage,
            kind: ReplacementKind::RedirectDamage {
                new_target: TargetSide::SelfSide,
            },
            priority: 1,
            insertion: 2,
        },
    ];

    env.apply_action(ActionDesc::MainPlayCharacter {
        hand_index: 0,
        stage_slot: 0,
    })
    .unwrap();

    let damage_committed = env.replay_events.iter().any(|e| {
        matches!(e,
            weiss_core::replay::ReplayEvent::DamageCommitted { target, .. } if *target == 0
        )
    });
    assert!(damage_committed);
    assert!(!matches!(
        env.state.terminal,
        Some(TerminalResult::Win { .. })
    ));
}
