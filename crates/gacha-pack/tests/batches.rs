mod support;
use gacha_core::*;
use gacha_pack::compile_pack;
use support::*;

#[test]
fn endfield_free_ten_is_isolated_and_each_invocation_resets_batch() {
    let game = game("endfield/chartered-proportional.json");
    let banner = "endfield.banner.reference";
    let mut state = StateStore::default();
    let (_, b) = game.banner(banner).unwrap();
    materialize_state(&game, b, &mut state, &ScopeContext::default()).unwrap();
    set(
        &game,
        banner,
        &mut state,
        "six_star_pity",
        StateValue::Counter(79),
    );
    set(
        &game,
        banner,
        &mut state,
        "banner_pulls",
        StateValue::Counter(119),
    );
    let original = state.clone();
    for seed in 0..200 {
        let result = sample_action(
            &game,
            banner,
            "free_ten_pull",
            &mut state,
            &ScopeContext::default(),
            &mut SplitMix64::new(seed),
        )
        .unwrap();
        assert_eq!(result.len(), 10);
        assert!(result.iter().any(|b| b.outcome.rarity >= 5));
        assert!(result.iter().all(|b| b.events.is_empty()));
        for name in [
            "six_star_pity",
            "five_plus_pity",
            "banner_pulls",
            "featured_guarantee_available",
        ] {
            let def = &game.states[game.state_lookup[name].0];
            let key = state_slot_key(&game, b, def, &ScopeContext::default());
            assert_eq!(state.slots[&key], original.slots[&key]);
        }
    }
}

#[test]
fn weapon_requisitions_guarantee_per_batch_and_emit_alternating_rewards() {
    let game = game("endfield/weapon-reference.json");
    for seed in 0..100 {
        let mut state = StateStore::default();
        set(
            &game,
            "endfield.weapon",
            &mut state,
            "six_miss_batches",
            StateValue::Counter(3),
        );
        set(
            &game,
            "endfield.weapon",
            &mut state,
            "featured_miss_batches",
            StateValue::Counter(7),
        );
        let result = sample_action(
            &game,
            "endfield.weapon",
            "requisition",
            &mut state,
            &ScopeContext::default(),
            &mut SplitMix64::new(seed),
        )
        .unwrap();
        assert!(
            result
                .iter()
                .any(|b| game.item(b.outcome.item).id == "endfield.weapon.featured")
        );
    }
    let mut state = StateStore::default();
    let mut rng = SplitMix64::new(42);
    let mut rewards = Vec::new();
    for index in 1..=34 {
        let result = sample_action(
            &game,
            "endfield.weapon",
            "requisition",
            &mut state,
            &ScopeContext::default(),
            &mut rng,
        )
        .unwrap();
        assert_eq!(result.len(), 10);
        assert!(result.iter().any(|b| b.outcome.rarity >= 5));
        for b in result {
            for event in b.events {
                rewards.push((index, event.key));
            }
        }
    }
    assert_eq!(
        rewards,
        vec![
            (10, "arsenal_box".into()),
            (18, "endfield.weapon.featured".into()),
            (26, "arsenal_box".into()),
            (34, "endfield.weapon.featured".into())
        ]
    );
}

#[test]
fn failed_batch_commits_neither_state_nor_rng() {
    let mut pack = raw("endfield/weapon-reference.json");
    pack.banners[0].actions[0]
        .transitions
        .push(gacha_schema::TransitionRuleDef {
            id: "fail_later".into(),
            when: gacha_schema::ConditionDef::StateCounterEq {
                state: "batch_draws".into(),
                value: 5,
            },
            updates: vec![gacha_schema::StateUpdateDef::Increment {
                state: "six_miss_batches".into(),
                by: u64::MAX,
            }],
            events: vec![],
        });
    let game = compile_pack(&pack).unwrap();
    let mut state = StateStore::default();
    let before = state.clone();
    let mut rng = SplitMix64::new(42);
    let mut expected = rng.clone();
    assert!(
        sample_action(
            &game,
            "endfield.weapon",
            "requisition",
            &mut state,
            &ScopeContext::default(),
            &mut rng
        )
        .is_err()
    );
    assert_eq!(state, before);
    assert_eq!(rng.next_u64(), expected.next_u64());
}
