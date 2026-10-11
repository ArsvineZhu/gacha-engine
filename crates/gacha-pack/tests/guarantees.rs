mod support;
use gacha_core::*;
use gacha_pack::compile_pack;
use support::*;

#[test]
fn off_banner_hit_guarantees_next_top_rarity_and_survives_banner_rotation() {
    let mut pack = raw("star-rail/character-reference.json");
    let mut second = pack.banners[0].clone();
    second.id = "next".into();
    second.progress_group = "next".into();
    pack.banners.push(second);
    let game = compile_pack(&pack).unwrap();
    let lost = branches(&game, "star-rail.character", &StateStore::default())
        .into_iter()
        .find(|b| game.item(b.outcome.item).id == "star-rail.standard")
        .unwrap();
    let mut state = lost.next_state;
    set(&game, "next", &mut state, "pity", StateValue::Counter(89));
    let next = branches(&game, "next", &state);
    assert_eq!(mass(&game, &next, "star-rail.featured"), Rational::ONE);
    set(&game, "next", &mut state, "pity", StateValue::Counter(0));
    let lower = branches(&game, "next", &state);
    assert_eq!(
        mass(&game, &lower, "star-rail.featured"),
        "0.006".parse().unwrap()
    );
    assert_eq!(mass(&game, &lower, "star-rail.standard"), Rational::ZERO);
}

#[test]
fn genshin_radiance_model_preserves_losses_on_guaranteed_hits() {
    let game = game("genshin/character-reference.json");
    let mut state = StateStore::default();
    set(
        &game,
        "genshin.character",
        &mut state,
        "pity",
        StateValue::Counter(89),
    );
    set(
        &game,
        "genshin.character",
        &mut state,
        "radiance_losses",
        StateValue::Counter(2),
    );
    let result = branches(&game, "genshin.character", &state);
    assert_eq!(
        mass(&game, &result, "genshin.featured"),
        "3/4".parse().unwrap()
    );
    let lost = result
        .into_iter()
        .find(|b| game.item(b.outcome.item).id == "genshin.standard")
        .unwrap();
    let mut state = lost.next_state;
    set(
        &game,
        "genshin.character",
        &mut state,
        "pity",
        StateValue::Counter(89),
    );
    let guaranteed = branches(&game, "genshin.character", &state);
    assert_eq!(mass(&game, &guaranteed, "genshin.featured"), Rational::ONE);
    let state = &guaranteed[0].next_state;
    let (_, banner) = game.banner("genshin.character").unwrap();
    let def = &game.states[game.state_lookup["radiance_losses"].0];
    assert_eq!(
        state.slots[&state_slot_key(&game, banner, def, &ScopeContext::default())],
        StateValue::Counter(3)
    );
}

#[test]
fn arknights_soft_pity_boundary_and_first_ten_only() {
    let game = game("arknights/kernel-reference.json");
    for (pity, rate) in [(49, "0.02"), (50, "0.04"), (98, "1")] {
        let mut state = StateStore::default();
        set(
            &game,
            "arknights.kernel",
            &mut state,
            "pity",
            StateValue::Counter(pity),
        );
        assert_eq!(
            mass(
                &game,
                &branches(&game, "arknights.kernel", &state),
                "arknights.6"
            ),
            rate.parse().unwrap()
        );
    }
    let mut state = StateStore::default();
    set(
        &game,
        "arknights.kernel",
        &mut state,
        "banner_pulls",
        StateValue::Counter(9),
    );
    assert!(
        branches(&game, "arknights.kernel", &state)
            .iter()
            .all(|b| b.outcome.rarity >= 5)
    );
    set(
        &game,
        "arknights.kernel",
        &mut state,
        "initial_five_seen",
        StateValue::Boolean(true),
    );
    assert!(
        branches(&game, "arknights.kernel", &state)
            .iter()
            .any(|b| b.outcome.rarity == 3)
    );
}

#[test]
fn reconstruction_rewards_and_progress_survive_same_name_rerun() {
    let mut pack = raw("endfield/reconstruction-reference.json");
    let mut rerun = pack.banners[0].clone();
    rerun.id = "rerun".into();
    pack.banners.push(rerun);
    let game = compile_pack(&pack).unwrap();
    for progress in [29, 59, 89] {
        let mut state = StateStore::default();
        set(
            &game,
            "endfield.reconstruction",
            &mut state,
            "banner_pulls",
            StateValue::Counter(progress),
        );
        assert!(
            branches(&game, "rerun", &state)
                .iter()
                .all(|b| b.events.iter().any(|e| e.key == "urgent_recruitment_10"))
        );
    }
    let mut state = StateStore::default();
    set(
        &game,
        "endfield.reconstruction",
        &mut state,
        "banner_pulls",
        StateValue::Counter(119),
    );
    assert_eq!(
        mass(
            &game,
            &branches(&game, "rerun", &state),
            "endfield.operator.current_featured"
        ),
        Rational::ONE
    );
}
