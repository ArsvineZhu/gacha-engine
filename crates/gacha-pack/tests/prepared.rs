use gacha_core::*;
use gacha_pack::{compile_pack, load_pack};
use std::path::PathBuf;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

#[test]
fn cached_sampling_replays_interpreter_for_every_maintained_action() {
    for folder in std::fs::read_dir(root().join("official/gacha-games/pools")).unwrap() {
        for file in std::fs::read_dir(folder.unwrap().path()).unwrap() {
            let file = file.unwrap();
            if !file.file_name().to_string_lossy().starts_with("banner.") {
                continue;
            }
            let game = compile_pack(&load_pack(file.path()).unwrap()).unwrap();
            let prepared = PreparedRuntime::with_capacity(game.clone(), 8).unwrap();
            for banner in &game.banners {
                for action in &banner.actions {
                    let mut left = StateStore::default();
                    let mut right = StateStore::default();
                    let mut rng = SplitMix64::new(42);
                    let mut cached_rng = rng.clone();
                    for _ in 0..20 {
                        let a = sample_action(
                            &game,
                            &banner.id,
                            &action.id,
                            &mut left,
                            &ScopeContext::default(),
                            &mut rng,
                        )
                        .unwrap();
                        let b = prepared
                            .sample_action(
                                &banner.id,
                                &action.id,
                                &mut right,
                                &ScopeContext::default(),
                                &mut cached_rng,
                            )
                            .unwrap();
                        for (a, b) in a.iter().zip(&b) {
                            assert_eq!(a.probability, b.probability);
                            assert_eq!(a.outcome, b.outcome);
                            assert_eq!(a.events, b.events);
                            assert_eq!(a.guarantees, b.guarantees);
                            assert_eq!(a.next_state, b.next_state);
                        }
                        assert_eq!(left, right);
                        assert_eq!(rng, cached_rng);
                    }
                }
            }
        }
    }
}

#[test]
fn cache_ignores_reward_progress_but_applies_live_events() {
    let game = compile_pack(
        &load_pack(
            root().join("official/gacha-games/pools/endfield/banner.cold-shadow-2026-09.json"),
        )
        .unwrap(),
    )
    .unwrap();
    let prepared = PreparedRuntime::new(game.clone()).unwrap();
    let banner = &game.banners[0];
    let context = ScopeContext::default();
    let mut rewards = Vec::new();
    for progress in [9, 17] {
        let mut state = StateStore::default();
        for (id, value) in [("batch_draws", 9), ("requisitions", progress)] {
            let def = &game.states[game.state_lookup[id].0];
            state.slots.insert(
                state_slot_key(&game, banner, def, &context),
                StateValue::Counter(value),
            );
        }
        let branches = prepared
            .enumerate_step(&banner.id, "requisition", &state, &context)
            .unwrap();
        rewards.push(branches[0].events.clone());
    }
    assert!(prepared.stats().cache_hits > 0);
    assert_ne!(rewards[0], rewards[1]);
    assert_eq!(rewards[0][0].key, "arsenal_box");
    assert_eq!(rewards[1][0].key, "endfield.weapon.寒夜幽影");
}

#[test]
fn force_item_bypasses_undefined_probability_table_entries() {
    let raw = load_pack(root().join("official/gacha-games/pools/endfield/banner.preview-si.json"))
        .unwrap();
    let mut json = serde_json::to_value(raw).unwrap();
    json["distributions"][0]["entries"][0] = serde_json::json!({
        "rarity":6,"type":"table","state":"six_star_pity","values":{"0":"0.008"}
    });
    let game = compile_pack(&gacha_pack::parse_pack(&json.to_string()).unwrap()).unwrap();
    let runtime = PreparedRuntime::new(game.clone()).unwrap();
    let banner = &game.banners[0];
    let context = ScopeContext::default();
    let mut state = StateStore::default();
    for (id, value) in [("banner_pulls", 119), ("six_star_pity", 1)] {
        let def = &game.states[game.state_lookup[id].0];
        state.slots.insert(
            state_slot_key(&game, banner, def, &context),
            StateValue::Counter(value),
        );
    }
    let old = enumerate_step(&game, &banner.id, "single_pull", &state, &context).unwrap();
    let new = runtime
        .enumerate_step(&banner.id, "single_pull", &state, &context)
        .unwrap();
    assert_eq!(old[0].probability, new[0].probability);
    assert_eq!(old[0].next_state, new[0].next_state);
    assert_eq!(game.item(new[0].outcome.item).display_name, "祀");
}
