use gacha_core::{
    Rational, ScopeContext, SplitMix64, StateStore, StateValue, enumerate_transitions,
    materialize_state, sample_transition, state_slot_key,
};
use gacha_pack::{compile_pack, load_pack};
use std::path::PathBuf;

fn pack_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../packs/endfield")
        .join(name)
}

fn load() -> gacha_core::CompiledGame {
    let pack = load_pack(pack_path("chartered-proportional.json")).expect("load pack");
    compile_pack(&pack).expect("compile pack")
}

fn state_with_counter(game: &gacha_core::CompiledGame, state_id: &str, value: u64) -> StateStore {
    let (_, banner) = game.banner("endfield.banner.reference").expect("banner");
    let mut state = StateStore::default();
    let context = ScopeContext::default();
    materialize_state(game, banner, &mut state, &context).expect("materialize");
    let id = game.state_lookup[state_id];
    let key = state_slot_key(game, banner, &game.states[id.0], &context);
    state.slots.insert(key, StateValue::Counter(value));
    state
}

#[test]
fn initial_rarity_probabilities_match_base_rates() {
    let game = load();
    let branches = enumerate_transitions(
        &game,
        "endfield.banner.reference",
        "single_pull",
        &StateStore::default(),
        &ScopeContext::default(),
    )
    .expect("enumerate");

    let mut six = Rational::ZERO;
    let mut five = Rational::ZERO;
    let mut four = Rational::ZERO;
    for branch in branches {
        match branch.outcome.rarity {
            6 => six = six.checked_add(branch.probability).expect("sum"),
            5 => five = five.checked_add(branch.probability).expect("sum"),
            4 => four = four.checked_add(branch.probability).expect("sum"),
            rarity => panic!("unexpected rarity {rarity}"),
        }
    }
    assert_eq!(six, "0.008".parse().expect("probability"));
    assert_eq!(five, "0.08".parse().expect("probability"));
    assert_eq!(four, "0.912".parse().expect("probability"));
}

#[test]
fn soft_pity_starts_after_65_failures() {
    let game = load();
    let state = state_with_counter(&game, "six_star_pity", 65);
    let branches = enumerate_transitions(
        &game,
        "endfield.banner.reference",
        "single_pull",
        &state,
        &ScopeContext::default(),
    )
    .expect("enumerate");
    let mut six = Rational::ZERO;
    for branch in branches {
        if branch.outcome.rarity == 6 {
            six = six.checked_add(branch.probability).expect("sum");
        }
    }
    assert_eq!(six, "0.058".parse().expect("probability"));
}

#[test]
fn pull_80_is_forced_six_star() {
    let game = load();
    let state = state_with_counter(&game, "six_star_pity", 79);
    let branches = enumerate_transitions(
        &game,
        "endfield.banner.reference",
        "single_pull",
        &state,
        &ScopeContext::default(),
    )
    .expect("enumerate");
    assert!(branches.iter().all(|branch| branch.outcome.rarity == 6));
    assert!(
        branches
            .iter()
            .all(|branch| branch.guarantees.iter().any(|id| id == "six_star_80"))
    );
}

#[test]
fn tenth_pull_excludes_four_star() {
    let game = load();
    let state = state_with_counter(&game, "five_plus_pity", 9);
    let branches = enumerate_transitions(
        &game,
        "endfield.banner.reference",
        "single_pull",
        &state,
        &ScopeContext::default(),
    )
    .expect("enumerate");
    assert!(branches.iter().all(|branch| branch.outcome.rarity >= 5));
}

#[test]
fn pull_120_forces_featured_if_available() {
    let game = load();
    let (_, banner) = game.banner("endfield.banner.reference").expect("banner");
    let context = ScopeContext::default();
    let mut state = StateStore::default();
    materialize_state(&game, banner, &mut state, &context).expect("materialize");

    let pulls = game.state_lookup["banner_pulls"];
    let pulls_key = state_slot_key(&game, banner, &game.states[pulls.0], &context);
    state.slots.insert(pulls_key, StateValue::Counter(119));

    let branches = enumerate_transitions(
        &game,
        "endfield.banner.reference",
        "single_pull",
        &state,
        &context,
    )
    .expect("enumerate");
    assert_eq!(branches.len(), 1);
    assert_eq!(
        game.item(branches[0].outcome.item).id,
        "endfield.operator.current_featured"
    );
    assert!(branches[0].guarantees.iter().any(|id| id == "featured_120"));
}

#[test]
fn pull_30_emits_reward_event() {
    let game = load();
    let state = state_with_counter(&game, "banner_pulls", 29);
    let branches = enumerate_transitions(
        &game,
        "endfield.banner.reference",
        "single_pull",
        &state,
        &ScopeContext::default(),
    )
    .expect("enumerate");
    assert!(branches.iter().all(|branch| {
        branch
            .events
            .iter()
            .any(|event| event.key == "urgent_recruitment_10")
    }));
}

#[test]
fn seeded_sampling_is_reproducible() {
    let game = load();
    let context = ScopeContext::default();
    let mut a = StateStore::default();
    let mut b = StateStore::default();
    let mut rng_a = SplitMix64::new(42);
    let mut rng_b = SplitMix64::new(42);

    for _ in 0..100 {
        let x = sample_transition(
            &game,
            "endfield.banner.reference",
            "single_pull",
            &mut a,
            &context,
            &mut rng_a,
        )
        .expect("sample");
        let y = sample_transition(
            &game,
            "endfield.banner.reference",
            "single_pull",
            &mut b,
            &context,
            &mut rng_b,
        )
        .expect("sample");
        assert_eq!(x.outcome, y.outcome);
        assert_eq!(a, b);
    }
}

#[test]
fn pity_scope_is_shared_while_progress_scope_can_differ() {
    let mut pack = load_pack(pack_path("chartered-proportional.json")).expect("load pack");
    let mut second = pack.banners[0].clone();
    second.id = "endfield.banner.reference.second".to_string();
    second.progress_group = "endfield.banner.reference.second".to_string();
    pack.banners.push(second);
    let game = compile_pack(&pack).expect("compile pack");
    let context = ScopeContext::default();
    let (_, first) = game
        .banner("endfield.banner.reference")
        .expect("first banner");
    let (_, second) = game
        .banner("endfield.banner.reference.second")
        .expect("second banner");
    let mut state = StateStore::default();
    materialize_state(&game, first, &mut state, &context).expect("materialize first");

    let pity = game.state_lookup["six_star_pity"];
    let progress = game.state_lookup["banner_pulls"];
    let first_pity_key = state_slot_key(&game, first, &game.states[pity.0], &context);
    let first_progress_key = state_slot_key(&game, first, &game.states[progress.0], &context);
    state
        .slots
        .insert(first_pity_key.clone(), StateValue::Counter(23));
    state
        .slots
        .insert(first_progress_key, StateValue::Counter(77));

    materialize_state(&game, second, &mut state, &context).expect("materialize second");
    let second_pity_key = state_slot_key(&game, second, &game.states[pity.0], &context);
    let second_progress_key = state_slot_key(&game, second, &game.states[progress.0], &context);

    assert_eq!(first_pity_key, second_pity_key);
    assert_eq!(state.slots[&second_pity_key], StateValue::Counter(23));
    assert_eq!(state.slots[&second_progress_key], StateValue::Counter(0));
}

#[test]
fn obtaining_featured_disables_120_guarantee_state() {
    let game = load();
    let context = ScopeContext::default();
    let branches = enumerate_transitions(
        &game,
        "endfield.banner.reference",
        "single_pull",
        &StateStore::default(),
        &context,
    )
    .expect("enumerate");
    let branch = branches
        .iter()
        .find(|branch| game.item(branch.outcome.item).id == "endfield.operator.current_featured")
        .expect("featured branch");
    let (_, banner) = game.banner("endfield.banner.reference").expect("banner");
    let available = game.state_lookup["featured_guarantee_available"];
    let key = state_slot_key(&game, banner, &game.states[available.0], &context);
    assert_eq!(branch.next_state.slots[&key], StateValue::Boolean(false));
}

#[test]
fn fixed_five_model_keeps_five_star_at_eight_percent_during_soft_pity() {
    let pack = load_pack(pack_path("chartered-fixed-five.json")).expect("load pack");
    let game = compile_pack(&pack).expect("compile pack");
    let state = state_with_counter(&game, "six_star_pity", 65);
    let branches = enumerate_transitions(
        &game,
        "endfield.banner.reference",
        "single_pull",
        &state,
        &ScopeContext::default(),
    )
    .expect("enumerate");
    let mut five = Rational::ZERO;
    for branch in branches {
        if branch.outcome.rarity == 5 {
            five = five.checked_add(branch.probability).expect("sum");
        }
    }
    assert_eq!(five, "0.08".parse().expect("probability"));
}
