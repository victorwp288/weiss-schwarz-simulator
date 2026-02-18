use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
#[serde(untagged)]
enum StringOrU64 {
    String(String),
    U64(u64),
}

fn de_u64_from_string_or_number<'de, D>(deserializer: D) -> Result<u64, D::Error>
where
    D: serde::Deserializer<'de>,
{
    match StringOrU64::deserialize(deserializer)? {
        StringOrU64::String(value) => value.parse::<u64>().map_err(serde::de::Error::custom),
        StringOrU64::U64(value) => Ok(value),
    }
}

fn de_opt_u64_from_string_or_number<'de, D>(deserializer: D) -> Result<Option<u64>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = Option::<StringOrU64>::deserialize(deserializer)?;
    match value {
        Some(StringOrU64::String(value)) => value
            .parse::<u64>()
            .map(Some)
            .map_err(serde::de::Error::custom),
        Some(StringOrU64::U64(value)) => Ok(Some(value)),
        None => Ok(None),
    }
}

use weiss_core::config::{CurriculumConfig, EnvConfig};
use weiss_core::encode::{
    ACTION_ENCODING_VERSION, ACTION_SPACE_SIZE, CHOICE_COUNT, CHOICE_NEXT_ID, OBS_ENCODING_VERSION,
    POLICY_VERSION, SPEC_HASH,
};
use weiss_core::env::{legal_action_ids_cached_into, GameEnv};
use weiss_core::events::Event;
use weiss_core::fingerprint::{
    config_fingerprint, events_fingerprint, hash_bytes, hash_postcard, state_fingerprint,
    FINGERPRINT_ALGO,
};
use weiss_core::legal::{DecisionKind, LegalActionIds};
use weiss_core::replay::ReplayConfig;

#[path = "engine_support.rs"]
mod engine_support;

const FIXTURE_PATH: &str = "tests/fixtures/golden_transcripts.json";
const TRANSCRIPT_VERSION: u32 = 2;
const MAX_STEPS: usize = 128;
const STRESS_STEPS: usize = 256;
const STRESS_SEARCH_MAX_STEPS_DEFAULT: usize = 512;
const STRESS_SEARCH_MAX_SEED_DEFAULT: u64 = 5000;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct TranscriptFile {
    version: u32,
    cases: Vec<TranscriptCase>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct TranscriptHeader {
    #[serde(deserialize_with = "de_u64_from_string_or_number")]
    spec_hash: u64,
    obs_encoding_version: u32,
    action_encoding_version: u32,
    policy_version: u32,
    #[serde(deserialize_with = "de_u64_from_string_or_number")]
    config_hash: u64,
    #[serde(deserialize_with = "de_u64_from_string_or_number")]
    reward_config_hash: u64,
    #[serde(deserialize_with = "de_u64_from_string_or_number")]
    deck_hash: u64,
    fingerprint_algo: String,
    harness: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct TranscriptCase {
    name: String,
    seed: u64,
    deck_a: Vec<u32>,
    deck_b: Vec<u32>,
    header: TranscriptHeader,
    steps: Vec<TranscriptStep>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct TranscriptStep {
    step_index: u32,
    decision_id: u32,
    decision_kind: i8,
    actor: i8,
    action_id: u16,
    #[serde(deserialize_with = "de_u64_from_string_or_number")]
    legal_ids_fingerprint: u64,
    #[serde(deserialize_with = "de_u64_from_string_or_number")]
    canonical_mask_fingerprint: u64,
    runtime_mask_present: bool,
    #[serde(default, deserialize_with = "de_opt_u64_from_string_or_number")]
    runtime_mask_fingerprint: Option<u64>,
    reward: f32,
    terminated: bool,
    truncated: bool,
    illegal_action: bool,
    engine_error: bool,
    #[serde(deserialize_with = "de_u64_from_string_or_number")]
    state_fingerprint: u64,
    #[serde(deserialize_with = "de_u64_from_string_or_number")]
    events_fingerprint: u64,
}

#[derive(Debug, Clone)]
struct StressCaseSpec {
    seed: u64,
    deck_a: Vec<u32>,
    deck_b: Vec<u32>,
    harness: bool,
}

#[derive(Default, Debug, Clone)]
struct StressFlags {
    saw_refresh: bool,
    saw_choice_paging: bool,
    saw_sanitized_event: bool,
}

#[cfg(feature = "test-harness")]
fn install_choice_paging_harness(env: &mut GameEnv, total: usize) {
    weiss_core::env::harness::install_choice_paging(env, total);
}

#[cfg(not(feature = "test-harness"))]
fn install_choice_paging_harness(_: &mut GameEnv, _: usize) {
    panic!("stress harness requested but weiss_core built without feature \"test-harness\"");
}

#[cfg(feature = "test-harness")]
fn force_deck_refresh_harness(env: &mut GameEnv, player: usize) {
    weiss_core::env::harness::force_deck_refresh(env, player);
}

#[cfg(not(feature = "test-harness"))]
fn force_deck_refresh_harness(_: &mut GameEnv, _: usize) {
    panic!("stress harness requested but weiss_core built without feature \"test-harness\"");
}

fn fixture_path() -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push(FIXTURE_PATH);
    path
}

fn stress_search_max_steps() -> usize {
    std::env::var("STRESS_SEARCH_MAX_STEPS")
        .ok()
        .and_then(|v| v.parse().ok())
        .filter(|&v| v > 0)
        .unwrap_or(STRESS_SEARCH_MAX_STEPS_DEFAULT)
}

fn stress_search_max_seed() -> u64 {
    std::env::var("STRESS_SEARCH_MAX_SEED")
        .ok()
        .and_then(|v| v.parse().ok())
        .filter(|&v| v > 0)
        .unwrap_or(STRESS_SEARCH_MAX_SEED_DEFAULT)
}

fn decision_kind_code(kind: DecisionKind) -> i8 {
    match kind {
        DecisionKind::Mulligan => 0,
        DecisionKind::Clock => 1,
        DecisionKind::Main => 2,
        DecisionKind::Climax => 3,
        DecisionKind::AttackDeclaration => 4,
        DecisionKind::LevelUp => 5,
        DecisionKind::Encore => 6,
        DecisionKind::TriggerOrder => 7,
        DecisionKind::Choice => 8,
    }
}

fn canonical_mask_fingerprint(ids: &[u16]) -> u64 {
    let mut mask = vec![0u8; ACTION_SPACE_SIZE];
    for &id in ids {
        let index = id as usize;
        if index < mask.len() {
            mask[index] = 1;
        }
    }
    hash_bytes(&mask)
}

fn runtime_mask_fingerprint(mask: &[u8]) -> Option<u64> {
    if mask.iter().any(|&v| v != 0) {
        Some(hash_bytes(mask))
    } else {
        None
    }
}

fn legal_ids_fingerprint(ids: &[u16]) -> u64 {
    let mut sorted = ids.to_vec();
    sorted.sort_unstable();
    let mut bytes = Vec::with_capacity(sorted.len() * 2);
    for id in sorted {
        bytes.extend_from_slice(&id.to_le_bytes());
    }
    hash_bytes(&bytes)
}

fn choose_action_id(env: &GameEnv, legal_ids: &[u16]) -> u16 {
    let Some(decision) = env.decision.as_ref() else {
        panic!("no decision");
    };
    if decision.kind == DecisionKind::Choice {
        if let Some(choice) = env.state.turn.choice.as_ref() {
            if choice.total_candidates as usize > CHOICE_COUNT && choice.page_start == 0 {
                let next_id = CHOICE_NEXT_ID as u16;
                if legal_ids.contains(&next_id) {
                    return next_id;
                }
            }
        }
    }
    *legal_ids.iter().min().expect("no legal actions available")
}

fn build_header(
    config: &EnvConfig,
    curriculum: &CurriculumConfig,
    harness: bool,
) -> TranscriptHeader {
    TranscriptHeader {
        spec_hash: SPEC_HASH,
        obs_encoding_version: OBS_ENCODING_VERSION,
        action_encoding_version: ACTION_ENCODING_VERSION,
        policy_version: POLICY_VERSION,
        config_hash: config_fingerprint(config, curriculum),
        reward_config_hash: hash_postcard(&config.reward),
        deck_hash: hash_postcard(&config.deck_lists),
        fingerprint_algo: FINGERPRINT_ALGO.to_string(),
        harness,
    }
}

fn compute_legal_ids(env: &GameEnv) -> Vec<u16> {
    let decision = env.decision.as_ref().expect("decision");
    let mut ids = LegalActionIds::new();
    legal_action_ids_cached_into(
        &env.state,
        decision,
        &env.db,
        &env.curriculum,
        env.curriculum.allowed_card_sets_cache.as_ref(),
        &mut ids,
    );
    ids.into_iter().collect()
}

fn is_choice_paging_event(event: &Event) -> bool {
    matches!(event, Event::ChoicePageChanged { .. })
        || matches!(
            event,
            Event::ChoicePresented {
                total_candidates, ..
            } if *total_candidates as usize > CHOICE_COUNT
        )
}

fn choice_option_hidden(
    canon: &weiss_core::state::ChoiceOptionRef,
    replay: &weiss_core::state::ChoiceOptionRef,
) -> bool {
    canon.card_id != 0 && replay.card_id == 0
}

fn event_has_hidden_card(canon: &Event, replay: &Event) -> bool {
    match (canon, replay) {
        (Event::Draw { card: c, .. }, Event::Draw { card: r, .. }) => *c != 0 && *r == 0,
        (Event::Reveal { card: c, .. }, Event::Reveal { card: r, .. }) => *c != 0 && *r == 0,
        (Event::ZoneMove { card: c, .. }, Event::ZoneMove { card: r, .. }) => *c != 0 && *r == 0,
        (Event::Play { card: c, .. }, Event::Play { card: r, .. }) => *c != 0 && *r == 0,
        (Event::PlayEvent { card: c, .. }, Event::PlayEvent { card: r, .. }) => *c != 0 && *r == 0,
        (Event::PlayClimax { card: c, .. }, Event::PlayClimax { card: r, .. }) => {
            *c != 0 && *r == 0
        }
        (Event::Counter { card: c, .. }, Event::Counter { card: r, .. }) => *c != 0 && *r == 0,
        (Event::Clock { card: Some(c), .. }, Event::Clock { card: Some(r), .. }) => {
            *c != 0 && *r == 0
        }
        (Event::Clock { card: Some(_), .. }, Event::Clock { card: None, .. }) => true,
        (Event::Trigger { card: Some(c), .. }, Event::Trigger { card: None, .. }) => *c != 0,
        (Event::RefreshPenalty { card: c, .. }, Event::RefreshPenalty { card: r, .. }) => {
            *c != 0 && *r == 0
        }
        (Event::LevelUpChoice { card: c, .. }, Event::LevelUpChoice { card: r, .. }) => {
            *c != 0 && *r == 0
        }
        (
            Event::ChoicePresented { options: canon, .. },
            Event::ChoicePresented {
                options: replay, ..
            },
        ) => canon
            .iter()
            .zip(replay.iter())
            .any(|(c, r)| choice_option_hidden(&c.reference, &r.reference)),
        (Event::ChoiceMade { option: c, .. }, Event::ChoiceMade { option: r, .. }) => {
            choice_option_hidden(c, r)
        }
        (Event::ChoiceAutopicked { option: c, .. }, Event::ChoiceAutopicked { option: r, .. }) => {
            choice_option_hidden(c, r)
        }
        _ => false,
    }
}

fn update_stress_flags(env: &GameEnv, flags: &mut StressFlags, last_index: &mut usize) {
    let canon = env.canonical_events();
    let replay = &env.replay_events;
    let end = canon.len().min(replay.len());
    for idx in *last_index..end {
        let c = &canon[idx];
        let r = &replay[idx];
        if matches!(c, Event::Refresh { .. }) || matches!(r, Event::Refresh { .. }) {
            flags.saw_refresh = true;
        }
        if is_choice_paging_event(r) {
            flags.saw_choice_paging = true;
        }
        if event_has_hidden_card(c, r) {
            flags.saw_sanitized_event = true;
        }
    }
    *last_index = end;
}

fn run_case(
    name: &str,
    seed: u64,
    deck_a: Vec<u32>,
    deck_b: Vec<u32>,
    harness: bool,
) -> TranscriptCase {
    let db = engine_support::make_db();
    let config = engine_support::make_config(deck_a.clone(), deck_b.clone());
    let curriculum = engine_support::default_curriculum();
    let header = build_header(&config, &curriculum, harness);
    let mut replay_config = ReplayConfig::default();
    if name == "stress_path" {
        replay_config.enabled = true;
        replay_config.sample_rate = 1.0;
        replay_config.rebuild_cache();
    }
    let mut env = GameEnv::new_or_panic(db, config, curriculum, seed, replay_config, None, 0);
    env.set_output_mask_enabled(true);

    let mut steps = Vec::new();
    let max_steps = if name == "stress_path" {
        STRESS_STEPS
    } else {
        MAX_STEPS
    };

    if harness && env.decision.is_some() {
        install_choice_paging_harness(&mut env, CHOICE_COUNT + 4);
    }

    for step_index in 0..max_steps {
        if harness && name == "stress_path" && step_index == 4 {
            force_deck_refresh_harness(&mut env, 0);
        }

        let Some(decision) = env.decision.as_ref() else {
            break;
        };
        let legal_ids = compute_legal_ids(&env);
        let action_id = choose_action_id(&env, &legal_ids);

        let decision_kind = decision_kind_code(decision.kind);
        let actor = decision.player as i8;
        let decision_id = env.decision_id();

        let legal_fp = legal_ids_fingerprint(&legal_ids);
        let canonical_mask_fp = canonical_mask_fingerprint(&legal_ids);
        let runtime_mask = env.action_mask();
        let runtime_mask_fp = runtime_mask_fingerprint(runtime_mask);
        let runtime_mask_present = runtime_mask_fp.is_some();

        let outcome = env
            .apply_action_id(action_id as usize)
            .expect("apply action id");

        let state_fp = state_fingerprint(&env.state);
        let events_fp = events_fingerprint(env.canonical_events());

        steps.push(TranscriptStep {
            step_index: step_index as u32,
            decision_id,
            decision_kind,
            actor,
            action_id,
            legal_ids_fingerprint: legal_fp,
            canonical_mask_fingerprint: canonical_mask_fp,
            runtime_mask_present,
            runtime_mask_fingerprint: runtime_mask_fp,
            reward: outcome.reward,
            terminated: outcome.terminated,
            truncated: outcome.truncated,
            illegal_action: outcome.info.illegal_action,
            engine_error: outcome.info.engine_error,
            state_fingerprint: state_fp,
            events_fingerprint: events_fp,
        });

        if outcome.terminated || outcome.truncated {
            break;
        }
    }

    if name == "stress_path" {
        env.finish_episode_replay();
    }

    TranscriptCase {
        name: name.to_string(),
        seed,
        deck_a,
        deck_b,
        header,
        steps,
    }
}

fn stress_case_satisfies(env: &mut GameEnv) -> bool {
    let mut flags = StressFlags::default();
    let mut last_index = 0usize;

    for _ in 0..stress_search_max_steps() {
        if env.decision.is_none() {
            break;
        }
        let legal_ids = compute_legal_ids(env);
        let action_id = choose_action_id(env, &legal_ids);
        let outcome = env
            .apply_action_id(action_id as usize)
            .expect("apply action id");

        update_stress_flags(env, &mut flags, &mut last_index);

        if outcome.terminated || outcome.truncated {
            break;
        }
    }

    env.finish_episode_replay();
    update_stress_flags(env, &mut flags, &mut last_index);

    flags.saw_refresh && flags.saw_choice_paging && flags.saw_sanitized_event
}

fn find_stress_case() -> StressCaseSpec {
    let deck_a = vec![1, 2, 3, 4, 5, 6, 7, 8, 9];
    let deck_b = vec![1, 2, 3, 4, 5, 6, 7, 8, 9];
    let db = engine_support::make_db();
    let config = engine_support::make_config(deck_a.clone(), deck_b.clone());
    let curriculum = engine_support::default_curriculum();
    let mut replay_config = ReplayConfig {
        enabled: true,
        sample_rate: 1.0,
        ..Default::default()
    };
    replay_config.rebuild_cache();

    for seed in 1..=stress_search_max_seed() {
        let mut env = GameEnv::new_or_panic(
            db.clone(),
            config.clone(),
            curriculum.clone(),
            seed,
            replay_config.clone(),
            None,
            0,
        );
        env.set_output_mask_enabled(true);
        if stress_case_satisfies(&mut env) {
            return StressCaseSpec {
                seed,
                deck_a,
                deck_b,
                harness: false,
            };
        }
    }

    if !cfg!(feature = "test-harness") {
        panic!(
            "stress case search failed; re-run tests with --features test-harness to enable harness fallback"
        );
    }

    StressCaseSpec {
        seed: 9001,
        deck_a,
        deck_b,
        harness: true,
    }
}

fn build_cases_for_update() -> Vec<TranscriptCase> {
    let stress = find_stress_case();
    vec![
        run_case("case_a", 1, vec![1, 1, 1, 3, 4], vec![2, 2, 2, 3, 4], false),
        run_case(
            "case_b",
            42,
            vec![5, 5, 4, 1, 9],
            vec![6, 6, 7, 8, 2],
            false,
        ),
        run_case("case_c", 1337, vec![1, 5, 9, 4], vec![2, 6, 7, 8], false),
        run_case(
            "stress_path",
            stress.seed,
            stress.deck_a,
            stress.deck_b,
            stress.harness,
        ),
    ]
}

fn read_expected_transcripts() -> TranscriptFile {
    let path = fixture_path();
    let bytes = fs::read(&path).expect("read golden transcript");
    serde_json::from_slice(&bytes).expect("parse golden transcript")
}

#[test]
#[cfg_attr(not(feature = "test-harness"), ignore)]
fn golden_transcripts() {
    let path = fixture_path();
    let update = std::env::var("UPDATE_GOLDEN").ok().as_deref() == Some("1");
    if update {
        let file = TranscriptFile {
            version: TRANSCRIPT_VERSION,
            cases: build_cases_for_update(),
        };
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("create fixture directory");
        }
        let json = serde_json::to_string_pretty(&file).expect("serialize golden transcript");
        fs::write(&path, json).expect("write golden transcript");
        return;
    }

    let expected = read_expected_transcripts();

    let cases = expected
        .cases
        .iter()
        .map(|case| {
            run_case(
                &case.name,
                case.seed,
                case.deck_a.clone(),
                case.deck_b.clone(),
                case.header.harness,
            )
        })
        .collect::<Vec<_>>();
    let file = TranscriptFile {
        version: TRANSCRIPT_VERSION,
        cases,
    };

    assert_eq!(expected, file);
}

#[test]
fn golden_transcripts_non_harness_cases() {
    let expected = read_expected_transcripts();
    let expected_cases: Vec<TranscriptCase> = expected
        .cases
        .iter()
        .filter(|case| !case.header.harness)
        .cloned()
        .collect();
    assert!(
        !expected_cases.is_empty(),
        "fixture must include at least one non-harness case"
    );
    let cases = expected_cases
        .iter()
        .map(|case| {
            run_case(
                &case.name,
                case.seed,
                case.deck_a.clone(),
                case.deck_b.clone(),
                false,
            )
        })
        .collect::<Vec<_>>();
    assert_eq!(expected_cases, cases);
}
