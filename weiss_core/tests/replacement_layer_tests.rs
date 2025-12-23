use std::sync::{Arc, OnceLock};

use weiss_core::config::{
    CurriculumConfig, EnvConfig, ErrorPolicy, ObservationVisibility, RewardConfig,
};
use weiss_core::db::{
    AbilityDef, AbilityKind, AbilityTiming, CardColor, CardDb, CardStatic, CardType, EffectTemplate,
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
    let cards = vec![
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
    Arc::new(CardDb::new(cards).expect("db build"))
}

fn make_config(deck_a: Vec<u32>, deck_b: Vec<u32>) -> EnvConfig {
    EnvConfig {
        deck_lists: [deck_a, deck_b],
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
    p.deck = (0..deck_count)
        .map(|idx| make_instance(deck_fill, owner, 8, idx))
        .collect();
    p.hand.clear();
    p.waiting_room.clear();
    p.clock.clear();
    p.level.clear();
    p.stock.clear();
    p.memory.clear();
    p.climax.clear();
    p.stage = [
        StageSlot::empty(),
        StageSlot::empty(),
        StageSlot::empty(),
        StageSlot::empty(),
        StageSlot::empty(),
    ];
    if let Some(stage_card) = stage_card {
        let mut slot_state = StageSlot::empty();
        slot_state.card = Some(make_instance(stage_card, owner, 4, 0));
        slot_state.status = StageStatus::Stand;
        p.stage[0] = slot_state;
    }
}

#[test]
fn replacements_apply_in_priority_order() {
    enable_validate();
    let db = make_db();
    let deck_a = vec![CARD_DAMAGE_ACT; 20];
    let deck_b = vec![CARD_BASIC; 20];
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
    let mut env = GameEnv::new(db, config, curriculum, 77, replay_config, None);

    setup_player_state(&mut env, 0, None, CARD_DAMAGE_ACT, 19);
    setup_player_state(&mut env, 1, None, CARD_BASIC, 20);
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
