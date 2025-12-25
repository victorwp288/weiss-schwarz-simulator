    use super::*;
    use crate::config::{
        CurriculumConfig, EnvConfig, ErrorPolicy, ObservationVisibility, RewardConfig,
        SimultaneousLossPolicy,
    };
    use crate::db::{CardColor, CardDb, CardId, CardStatic, CardType};
    use crate::encode::{encode_observation, OBS_LEN};
    use crate::effects::{EffectId, EffectKind, EffectPayload, EffectSourceKind, EffectSpec};
    use crate::events::{RevealAudience, RevealReason};
    use crate::replay::ReplayConfig;
    use crate::replay::ReplayEvent;
    use crate::state::{
        CardInstance, ChoiceReason, ChoiceZone, PendingTargetEffect, StackItem, StageSlot,
        TargetSelectionState, TargetSide, TargetSlotFilter, TargetSpec, TargetZone, TerminalResult,
    };
    use std::collections::HashMap;
    use std::sync::Arc;

    const CLONE_OFFSET: u32 = 1000;
    const CLONE_GROUPS: usize = 12;
    const MAX_COPIES: usize = 4;

    fn make_instance(id: CardId, owner: u8, next_id: &mut u32) -> CardInstance {
        let instance = CardInstance::new(id, owner, *next_id);
        *next_id = next_id.wrapping_add(1);
        instance
    }

    fn add_clone_cards(cards: &mut Vec<CardStatic>) {
        let base_cards = cards.clone();
        for base in base_cards {
            for idx in 1..=CLONE_GROUPS {
                let mut clone = base.clone();
                clone.id = base.id + CLONE_OFFSET * idx as u32;
                cards.push(clone);
            }
        }
    }

    fn legalize_deck(mut deck: Vec<u32>, filler_pool: &[u32]) -> Vec<u32> {
        let max_deck = crate::encode::MAX_DECK;
        if deck.len() > max_deck {
            panic!("deck length {} exceeds MAX_DECK {}", deck.len(), max_deck);
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
        while deck.len() < max_deck {
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

    fn make_env() -> GameEnv {
        let mut cards = vec![
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
                ability_defs: vec![],
                counter_timing: false,
                raw_text: None,
            },
            CardStatic {
                id: 2,
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
        add_clone_cards(&mut cards);
        let db = Arc::new(CardDb::new(cards).expect("db"));
        let config = EnvConfig {
            deck_lists: [
                legalize_deck(vec![1; 50], &[1]),
                legalize_deck(vec![2; 50], &[2]),
            ],
            deck_ids: [1, 2],
            max_decisions: 100,
            max_ticks: 1000,
            reward: RewardConfig::default(),
            error_policy: ErrorPolicy::Strict,
            observation_visibility: ObservationVisibility::Public,
            end_condition_policy: Default::default(),
        };
        GameEnv::new(
            db,
            config,
            CurriculumConfig::default(),
            1,
            Default::default(),
            None,
            0,
        )
    }

    fn make_env_with_replay(replay_config: ReplayConfig) -> GameEnv {
        let mut cards = vec![
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
                ability_defs: vec![],
                counter_timing: false,
                raw_text: None,
            },
            CardStatic {
                id: 2,
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
        add_clone_cards(&mut cards);
        let db = Arc::new(CardDb::new(cards).expect("db"));
        let config = EnvConfig {
            deck_lists: [
                legalize_deck(vec![1; 50], &[1]),
                legalize_deck(vec![2; 50], &[2]),
            ],
            deck_ids: [1, 2],
            max_decisions: 100,
            max_ticks: 1000,
            reward: RewardConfig::default(),
            error_policy: ErrorPolicy::Strict,
            observation_visibility: ObservationVisibility::Public,
            end_condition_policy: Default::default(),
        };
        GameEnv::new(
            db,
            config,
            CurriculumConfig::default(),
            2,
            replay_config,
            None,
            0,
        )
    }

    fn enumerate_targets_for_test(
        env: &GameEnv,
        controller: u8,
        spec: &TargetSpec,
        selected: &[TargetRef],
    ) -> Vec<TargetRef> {
        let mut out = Vec::new();
        GameEnv::enumerate_target_candidates_into(
            &env.state,
            &env.db,
            &env.curriculum,
            controller,
            spec,
            selected,
            &mut out,
        );
        out
    }

    #[test]
    fn stack_group_ordering_stable() {
        let mut env = make_env();
        let spec_a = EffectSpec {
            id: EffectId::new(EffectSourceKind::System, 2, 0, 0),
            kind: EffectKind::Draw { count: 1 },
            target: None,
        };
        let spec_b = EffectSpec {
            id: EffectId::new(EffectSourceKind::System, 1, 0, 0),
            kind: EffectKind::Draw { count: 1 },
            target: None,
        };
        let item_a = StackItem {
            id: 2,
            controller: 0,
            source_id: 2,
            effect_id: spec_a.id,
            payload: EffectPayload {
                spec: spec_a,
                targets: Vec::new(),
            },
        };
        let item_b = StackItem {
            id: 1,
            controller: 0,
            source_id: 1,
            effect_id: spec_b.id,
            payload: EffectPayload {
                spec: spec_b,
                targets: Vec::new(),
            },
        };
        env.enqueue_stack_items(vec![item_a, item_b]);
        let order = env.state.turn.stack_order.as_ref().expect("stack order");
        assert_eq!(order.items[0].source_id, 1);
        assert_eq!(order.items[1].source_id, 2);
    }

    #[test]
    fn target_candidate_ordering_by_zone() {
        let mut env = make_env();
        let p = 0usize;
        let owner = p as u8;
        let mut next_id = 1u32;
        env.state.players[p].hand = vec![
            make_instance(1, owner, &mut next_id),
            make_instance(2, owner, &mut next_id),
            make_instance(1, owner, &mut next_id),
        ];
        env.state.players[p].waiting_room = vec![
            make_instance(1, owner, &mut next_id),
            make_instance(2, owner, &mut next_id),
            make_instance(1, owner, &mut next_id),
        ];
        env.state.players[p].clock = vec![
            make_instance(1, owner, &mut next_id),
            make_instance(2, owner, &mut next_id),
        ];
        env.state.players[p].level = vec![
            make_instance(2, owner, &mut next_id),
            make_instance(1, owner, &mut next_id),
        ];
        env.state.players[p].stock = vec![
            make_instance(1, owner, &mut next_id),
            make_instance(2, owner, &mut next_id),
            make_instance(1, owner, &mut next_id),
        ];
        env.state.players[p].memory = vec![make_instance(1, owner, &mut next_id)];
        env.state.players[p].climax = vec![make_instance(2, owner, &mut next_id)];
        env.state.players[p].resolution = vec![
            make_instance(1, owner, &mut next_id),
            make_instance(2, owner, &mut next_id),
        ];
        env.state.players[p].deck = vec![
            make_instance(1, owner, &mut next_id),
            make_instance(2, owner, &mut next_id),
            make_instance(1, owner, &mut next_id),
            make_instance(2, owner, &mut next_id),
        ];
        env.state.players[p].stage = [
            {
                let mut s = StageSlot::empty();
                s.card = Some(make_instance(1, owner, &mut next_id));
                s
            },
            {
                let mut s = StageSlot::empty();
                s.card = Some(make_instance(2, owner, &mut next_id));
                s
            },
            StageSlot::empty(),
            StageSlot::empty(),
            StageSlot::empty(),
        ];

        let spec = |zone| TargetSpec {
            zone,
            side: TargetSide::SelfSide,
            slot_filter: TargetSlotFilter::Any,
            card_type: None,
            count: 3,
        };

        let stage = enumerate_targets_for_test(&env, owner, &spec(TargetZone::Stage), &[]);
        assert_eq!(
            stage.iter().map(|t| t.index).collect::<Vec<_>>(),
            vec![0, 1]
        );

        let waiting = enumerate_targets_for_test(&env, owner, &spec(TargetZone::WaitingRoom), &[]);
        assert_eq!(
            waiting.iter().map(|t| t.index).collect::<Vec<_>>(),
            vec![0, 1, 2]
        );

        let hand = enumerate_targets_for_test(&env, owner, &spec(TargetZone::Hand), &[]);
        assert_eq!(
            hand.iter().map(|t| t.index).collect::<Vec<_>>(),
            vec![0, 1, 2]
        );

        let deck = enumerate_targets_for_test(&env, owner, &spec(TargetZone::DeckTop), &[]);
        assert_eq!(
            deck.iter().map(|t| t.index).collect::<Vec<_>>(),
            vec![0, 1, 2, 3]
        );

        let clock = enumerate_targets_for_test(&env, owner, &spec(TargetZone::Clock), &[]);
        assert_eq!(
            clock.iter().map(|t| t.index).collect::<Vec<_>>(),
            vec![0, 1]
        );

        let level = enumerate_targets_for_test(&env, owner, &spec(TargetZone::Level), &[]);
        assert_eq!(
            level.iter().map(|t| t.index).collect::<Vec<_>>(),
            vec![0, 1]
        );

        let stock = enumerate_targets_for_test(&env, owner, &spec(TargetZone::Stock), &[]);
        assert_eq!(
            stock.iter().map(|t| t.index).collect::<Vec<_>>(),
            vec![0, 1, 2]
        );

        let memory = enumerate_targets_for_test(&env, owner, &spec(TargetZone::Memory), &[]);
        assert_eq!(memory.iter().map(|t| t.index).collect::<Vec<_>>(), vec![0]);

        let climax = enumerate_targets_for_test(&env, owner, &spec(TargetZone::Climax), &[]);
        assert_eq!(climax.iter().map(|t| t.index).collect::<Vec<_>>(), vec![0]);

        let resolution = enumerate_targets_for_test(&env, owner, &spec(TargetZone::Resolution), &[]);
        assert_eq!(
            resolution.iter().map(|t| t.index).collect::<Vec<_>>(),
            vec![0, 1]
        );
    }

    #[test]
    fn visibility_policy_masks_opponent_hidden_choices() {
        let replay_config = ReplayConfig {
            enabled: true,
            sample_rate: 1.0,
            ..Default::default()
        };
        let mut env = make_env_with_replay(replay_config);
        env.curriculum.enable_visibility_policies = true;
        let mut next_id = 1u32;
        env.state.players[1].hand = vec![
            make_instance(1, 1, &mut next_id),
            make_instance(2, 1, &mut next_id),
        ];

        let spec = TargetSpec {
            zone: TargetZone::Hand,
            side: TargetSide::Opponent,
            slot_filter: TargetSlotFilter::Any,
            card_type: None,
            count: 1,
        };
        let effect_spec = EffectSpec {
            id: EffectId::new(EffectSourceKind::Activated, 1, 0, 0),
            kind: EffectKind::MoveToHand,
            target: Some(spec.clone()),
        };
        env.state.turn.target_selection = Some(TargetSelectionState {
            controller: 0,
            source_id: 1,
            remaining: 1,
            spec,
            selected: Vec::new(),
            effect: PendingTargetEffect::EffectPending {
                instance_id: 1,
                payload: EffectPayload {
                    spec: effect_spec,
                    targets: Vec::new(),
                },
            },
        });
        env.present_target_choice();

        let (choice_id, options) = env
            .replay_events
            .iter()
            .find_map(|e| {
                if let ReplayEvent::ChoicePresented {
                    reason: ChoiceReason::TargetSelect,
                    choice_id,
                    options,
                    ..
                } = e
                {
                    Some((*choice_id, options))
                } else {
                    None
                }
            })
            .expect("choice presented");
        assert!(options.iter().all(|opt| opt.reference.card_id == 0));
        assert!(options.iter().all(|opt| opt.reference.index.is_none()));
        assert!(options
            .iter()
            .all(|opt| opt.option_id >> 32 == choice_id as u64));
        let mut unique = std::collections::BTreeSet::new();
        for opt in options {
            assert!(unique.insert(opt.option_id));
        }

        env.replay_events.clear();
        env.state.turn.choice = None;
        let revealed = env.state.players[1].hand[1];
        env.reveal_card(1, &revealed, RevealReason::TriggerCheck, RevealAudience::Public);
        env.present_target_choice();

        let options = env
            .replay_events
            .iter()
            .find_map(|e| {
                if let ReplayEvent::ChoicePresented {
                    reason: ChoiceReason::TargetSelect,
                    options,
                    ..
                } = e
                {
                    Some(options)
                } else {
                    None
                }
            })
            .expect("choice presented");
        assert!(options.iter().any(|opt| opt.reference.card_id == 2));
        assert!(options.iter().any(|opt| opt.reference.card_id == 0));
    }

    #[test]
    fn public_replay_masks_hidden_action_params() {
        let replay_config = ReplayConfig {
            enabled: true,
            sample_rate: 1.0,
            ..Default::default()
        };
        let mut env = make_env_with_replay(replay_config);
        env.curriculum.enable_visibility_policies = true;
        env.replay_actions.clear();

        env.log_action(
            1,
            ActionDesc::MainPlayCharacter {
                hand_index: 3,
                stage_slot: 2,
            },
        );

        let last = env.replay_actions.last().expect("action logged");
        match last {
            ActionDesc::MainPlayCharacter {
                hand_index,
                stage_slot,
            } => {
                assert_eq!(*hand_index, u8::MAX);
                assert_eq!(*stage_slot, 2);
            }
            _ => panic!("unexpected action: {last:?}"),
        }
    }

    #[test]
    fn public_observation_masks_opponent_last_action_params() {
        let mut env = make_env();
        env.curriculum.enable_visibility_policies = true;
        env.last_action_desc = Some(ActionDesc::MainPlayCharacter {
            hand_index: 4,
            stage_slot: 1,
        });
        env.last_action_player = Some(1);
        let mut obs = vec![0; OBS_LEN];
        encode_observation(
            &env.state,
            &env.db,
            &env.curriculum,
            0,
            env.decision.as_ref(),
            env.last_action_desc.as_ref(),
            env.last_action_player,
            env.config.observation_visibility,
            env.curriculum.enable_visibility_policies,
            &mut obs,
        );
        assert_eq!(obs[5], 6);
        assert_eq!(obs[6], -1);
        assert_eq!(obs[7], 1);
    }

    #[test]
    fn public_replay_masks_hidden_draws() {
        let replay_config = ReplayConfig {
            enabled: true,
            sample_rate: 1.0,
            ..Default::default()
        };
        let mut env = make_env_with_replay(replay_config);
        env.curriculum.enable_visibility_policies = true;
        env.recording = true;
        env.replay_events.clear();

        env.log_event(Event::Draw { player: 1, card: 99 });

        let last = env.replay_events.last().expect("draw event");
        match last {
            ReplayEvent::Draw { card, .. } => assert_eq!(*card, 0),
            _ => panic!("unexpected event: {last:?}"),
        }
    }

    #[test]
    fn public_replay_no_hidden_zone_leaks() {
        let replay_config = ReplayConfig {
            enabled: true,
            sample_rate: 1.0,
            ..Default::default()
        };
        let mut env = make_env_with_replay(replay_config);
        env.curriculum.enable_visibility_policies = true;
        env.recording = true;
        env.replay_events.clear();

        env.draw_to_hand(1, 1);

        let mut next_id = 1u32;
        env.state.players[1].hand.clear();
        env.state.players[1]
            .hand
            .push(make_instance(2, 1, &mut next_id));

        let spec = TargetSpec {
            zone: TargetZone::Hand,
            side: TargetSide::Opponent,
            slot_filter: TargetSlotFilter::Any,
            card_type: None,
            count: 1,
        };
        let effect_spec = EffectSpec {
            id: EffectId::new(EffectSourceKind::Activated, 1, 0, 0),
            kind: EffectKind::MoveToHand,
            target: Some(spec.clone()),
        };
        env.state.turn.target_selection = Some(TargetSelectionState {
            controller: 0,
            source_id: 1,
            remaining: 1,
            spec,
            selected: Vec::new(),
            effect: PendingTargetEffect::EffectPending {
                instance_id: 1,
                payload: EffectPayload {
                    spec: effect_spec,
                    targets: Vec::new(),
                },
            },
        });
        env.present_target_choice();

        for event in &env.replay_events {
            match event {
                ReplayEvent::Draw { card, .. } => assert_eq!(*card, 0),
                ReplayEvent::ZoneMove {
                    card,
                    from,
                    to,
                    from_slot,
                    to_slot,
                    ..
                } => {
                    let hidden_from = matches!(
                        from,
                        Zone::Deck | Zone::Hand | Zone::Stock | Zone::Memory
                    );
                    let hidden_to =
                        matches!(to, Zone::Deck | Zone::Hand | Zone::Stock | Zone::Memory);
                    if hidden_from && hidden_to {
                        assert_eq!(*card, 0);
                        assert_eq!(*from_slot, None);
                        assert_eq!(*to_slot, None);
                    }
                }
                ReplayEvent::ChoicePresented { options, .. } => {
                    for opt in options {
                        if matches!(
                            opt.reference.zone,
                            ChoiceZone::Hand
                                | ChoiceZone::DeckTop
                                | ChoiceZone::Stock
                                | ChoiceZone::Memory
                        ) {
                            assert_eq!(opt.reference.card_id, 0);
                            assert_eq!(opt.reference.instance_id, 0);
                            assert!(opt.reference.index.is_none());
                        }
                    }
                }
                _ => {}
            }
        }
    }

    #[test]
    fn reveal_one_copy_does_not_unmask_duplicates() {
        let replay_config = ReplayConfig {
            enabled: true,
            sample_rate: 1.0,
            ..Default::default()
        };
        let mut env = make_env_with_replay(replay_config);
        env.curriculum.enable_visibility_policies = true;
        env.replay_events.clear();

        let mut next_id = 1u32;
        let first = make_instance(1, 1, &mut next_id);
        let second = make_instance(1, 1, &mut next_id);
        env.state.players[1].hand = vec![first, second];

        env.reveal_card(1, &first, RevealReason::TriggerCheck, RevealAudience::Public);

        let spec = TargetSpec {
            zone: TargetZone::Hand,
            side: TargetSide::Opponent,
            slot_filter: TargetSlotFilter::Any,
            card_type: None,
            count: 1,
        };
        let effect_spec = EffectSpec {
            id: EffectId::new(EffectSourceKind::Activated, 1, 0, 0),
            kind: EffectKind::MoveToHand,
            target: Some(spec.clone()),
        };
        env.state.turn.target_selection = Some(TargetSelectionState {
            controller: 0,
            source_id: 1,
            remaining: 1,
            spec,
            selected: Vec::new(),
            effect: PendingTargetEffect::EffectPending {
                instance_id: 1,
                payload: EffectPayload {
                    spec: effect_spec,
                    targets: Vec::new(),
                },
            },
        });
        env.present_target_choice();

        let options = env
            .replay_events
            .iter()
            .find_map(|e| {
                if let ReplayEvent::ChoicePresented {
                    reason: ChoiceReason::TargetSelect,
                    options,
                    ..
                } = e
                {
                    Some(options)
                } else {
                    None
                }
            })
            .expect("choice presented");
        let revealed = options.iter().filter(|opt| opt.reference.card_id == 1).count();
        let hidden = options.iter().filter(|opt| opt.reference.card_id == 0).count();
        assert_eq!(revealed, 1);
        assert_eq!(hidden, 1);
        assert!(options.iter().all(|opt| opt.reference.instance_id == 0));
    }

    #[test]
    fn alternate_end_conditions_simultaneous_loss_policies() {
        let mut env = make_env();
        env.curriculum.use_alternate_end_conditions = true;

        env.state.turn.active_player = 0;
        env.config.end_condition_policy.simultaneous_loss = SimultaneousLossPolicy::Draw;
        env.config
            .end_condition_policy
            .allow_draw_on_simultaneous_loss = true;
        env.state.turn.pending_losses = [true, true];
        env.resolve_pending_losses();
        assert!(matches!(env.state.terminal, Some(TerminalResult::Draw)));

        env.state.terminal = None;
        env.state.turn.pending_losses = [true, true];
        env.config.end_condition_policy.simultaneous_loss =
            SimultaneousLossPolicy::ActivePlayerWins;
        env.resolve_pending_losses();
        assert!(matches!(
            env.state.terminal,
            Some(TerminalResult::Win { winner: 0 })
        ));

        env.state.terminal = None;
        env.state.turn.pending_losses = [true, true];
        env.config.end_condition_policy.simultaneous_loss =
            SimultaneousLossPolicy::NonActivePlayerWins;
        env.resolve_pending_losses();
        assert!(matches!(
            env.state.terminal,
            Some(TerminalResult::Win { winner: 1 })
        ));

        env.state.terminal = None;
        env.state.turn.pending_losses = [true, true];
        env.config.end_condition_policy.simultaneous_loss = SimultaneousLossPolicy::Draw;
        env.config
            .end_condition_policy
            .allow_draw_on_simultaneous_loss = false;
        env.resolve_pending_losses();
        assert!(matches!(
            env.state.terminal,
            Some(TerminalResult::Win { winner: 0 })
        ));
    }
