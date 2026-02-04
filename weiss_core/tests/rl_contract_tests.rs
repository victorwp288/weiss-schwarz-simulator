use weiss_core::config::{ErrorPolicy, ObservationVisibility};
use weiss_core::encode::{ACTION_SPACE_SIZE, ACTOR_NONE, DECISION_KIND_NONE};
use weiss_core::pool::{BatchOutMinimalBuffers, EnvPool};
use weiss_core::DebugConfig;

#[path = "engine_support.rs"]
mod engine_support;

fn make_pool(visibility: ObservationVisibility, seed: u64) -> EnvPool {
    let db = engine_support::make_db();
    let mut config = engine_support::make_config(vec![1; 50], vec![1; 50]);
    config.observation_visibility = visibility;
    config.error_policy = ErrorPolicy::Strict;
    let mut curriculum = engine_support::default_curriculum();
    curriculum.enable_visibility_policies = true;
    EnvPool::new_debug(
        4,
        db,
        config,
        curriculum,
        seed,
        None,
        DebugConfig::default(),
    )
    .expect("pool init")
}

fn assert_live_invariants(out: &BatchOutMinimalBuffers, env_index: usize) {
    let actor = out.actor[env_index];
    assert!(actor == 0 || actor == 1);
    assert_ne!(out.decision_kind[env_index], DECISION_KIND_NONE);
    let offset = env_index * ACTION_SPACE_SIZE;
    let mask = &out.masks[offset..offset + ACTION_SPACE_SIZE];
    assert!(mask.contains(&1));
}

#[test]
fn decision_step_invariants_public_and_full() {
    for visibility in [ObservationVisibility::Public, ObservationVisibility::Full] {
        for seed in 0u64..5 {
            let mut pool = make_pool(visibility, seed);
            let num_envs = pool.envs.len();
            let mut buffers = BatchOutMinimalBuffers::new(num_envs);
            pool.reset_into(&mut buffers.view_mut()).expect("reset");
            for i in 0..num_envs {
                assert_eq!(buffers.decision_id[i], 0);
                if buffers.terminated[i] || buffers.truncated[i] {
                    assert_eq!(buffers.actor[i], ACTOR_NONE);
                } else {
                    assert_live_invariants(&buffers, i);
                }
            }

            let mut actions = vec![0u32; num_envs];
            for _ in 0..20 {
                for (env_index, action) in actions.iter_mut().enumerate().take(num_envs) {
                    let done = buffers.terminated[env_index] || buffers.truncated[env_index];
                    if done {
                        *action = 0;
                        continue;
                    }
                    let offset = env_index * ACTION_SPACE_SIZE;
                    let mask = &buffers.masks[offset..offset + ACTION_SPACE_SIZE];
                    let action_id = mask.iter().position(|&v| v == 1).expect("legal action");
                    *action = action_id as u32;
                }
                let prev_ids = buffers.decision_id.clone();
                let prev_done = buffers.terminated.clone();
                let prev_truncated = buffers.truncated.clone();
                pool.step_into(&actions, &mut buffers.view_mut())
                    .expect("step");
                for env_index in 0..num_envs {
                    if buffers.terminated[env_index] || buffers.truncated[env_index] {
                        assert_eq!(buffers.actor[env_index], ACTOR_NONE);
                    } else {
                        assert_live_invariants(&buffers, env_index);
                    }
                    if !prev_done[env_index]
                        && !prev_truncated[env_index]
                        && !buffers.terminated[env_index]
                        && !buffers.truncated[env_index]
                    {
                        assert_eq!(
                            buffers.decision_id[env_index],
                            prev_ids[env_index].wrapping_add(1)
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn legal_action_ids_match_dense_mask() {
    for seed in 10u64..15 {
        let mut pool = make_pool(ObservationVisibility::Public, seed);
        let num_envs = pool.envs.len();
        let mut buffers = BatchOutMinimalBuffers::new(num_envs);
        pool.reset_into(&mut buffers.view_mut()).expect("reset");

        let mut ids = vec![0u16; num_envs * ACTION_SPACE_SIZE];
        let mut offsets = vec![0u32; num_envs + 1];
        let total = pool
            .legal_action_ids_batch_into(&mut ids, &mut offsets)
            .expect("legal ids");
        assert!(total <= ids.len());

        for env_index in 0..num_envs {
            if buffers.terminated[env_index] || buffers.truncated[env_index] {
                continue;
            }
            let offset = env_index * ACTION_SPACE_SIZE;
            let mask = &buffers.masks[offset..offset + ACTION_SPACE_SIZE];
            let expected = mask.iter().filter(|&&v| v == 1).count() as u32;
            let start = offsets[env_index] as usize;
            let end = offsets[env_index + 1] as usize;
            assert_eq!(expected, (end - start) as u32);
            for &action_id in &ids[start..end] {
                assert_eq!(mask[action_id as usize], 1);
            }
        }
    }
}
