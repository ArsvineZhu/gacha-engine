use gacha_analysis::{
    FirstHitQuery, TargetSpec, analyze_first_hit, probability_of_item_within,
};
use gacha_core::{
    CompiledStateDef, ScopeContext, StateKind, StateScope, StateStore, StateValue,
    enumerate_step, enumerate_transitions, prune_ephemeral_state,
};
use gacha_pack::{compile_pack, load_pack};
use std::path::PathBuf;

fn reference_game() -> gacha_core::CompiledGame {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../packs/endfield/chartered-proportional.json");
    let pack = load_pack(path).expect("load");
    compile_pack(&pack).expect("compile")
}

#[test]
fn one_draw_analysis_matches_enumeration() {
    let game = reference_game();
    let state = StateStore::default();
    let context = ScopeContext::default();
    let target = "endfield.operator.current_featured";

    let direct: f64 = enumerate_transitions(
        &game,
        "endfield.banner.reference",
        "single_pull",
        &state,
        &context,
    )
    .expect("enumerate")
    .into_iter()
    .filter(|branch| game.item(branch.outcome.item).id == target)
    .map(|branch| branch.probability.to_f64())
    .sum();

    let analyzed = probability_of_item_within(
        &game,
        "endfield.banner.reference",
        "single_pull",
        &state,
        &context,
        target,
        1,
    )
    .expect("analyze");

    assert!((direct - analyzed.probability).abs() < 1e-15);
}

#[test]
fn featured_first_hit_distribution_is_normalized_and_guaranteed_by_120() {
    let game = reference_game();
    let query = FirstHitQuery {
        banner: "endfield.banner.reference".to_string(),
        action: "single_pull".to_string(),
        target: TargetSpec::Item {
            id: "endfield.operator.current_featured".to_string(),
        },
        draws: 120,
        quantiles: vec![0.5, 0.9, 0.99, 1.0],
    };

    let result = analyze_first_hit(
        &game,
        &StateStore::default(),
        &ScopeContext::default(),
        &query,
    )
    .expect("analyze");

    assert_eq!(result.distribution.len(), 120);
    assert!((result.probability_within - 1.0).abs() < 1e-10);
    assert!(result.probability_not_hit < 1e-10);
    assert!((result.probability_within + result.probability_not_hit - 1.0).abs() < 1e-10);

    let mut previous = 0.0;
    for point in &result.distribution {
        assert!(point.first_hit_probability >= 0.0);
        assert!(point.cumulative_probability + 1e-12 >= previous);
        assert!((point.cumulative_probability + point.survival_probability - 1.0).abs() < 1e-10);
        previous = point.cumulative_probability;
    }

    assert_eq!(result.quantiles.last().and_then(|q| q.draw), Some(120));
}

#[test]
fn target_variants_are_supported() {
    let game = reference_game();
    let state = StateStore::default();
    let context = ScopeContext::default();

    let six_star = analyze_first_hit(
        &game,
        &state,
        &context,
        &FirstHitQuery {
            banner: "endfield.banner.reference".to_string(),
            action: "single_pull".to_string(),
            target: TargetSpec::RarityAtLeast { rarity: 6 },
            draws: 1,
            quantiles: vec![],
        },
    )
    .expect("six-star analysis");
    assert!((six_star.probability_within - 0.008).abs() < 1e-12);

    let any_operator = analyze_first_hit(
        &game,
        &state,
        &context,
        &FirstHitQuery {
            banner: "endfield.banner.reference".to_string(),
            action: "single_pull".to_string(),
            target: TargetSpec::Tag {
                tag: "operator".to_string(),
            },
            draws: 1,
            quantiles: vec![],
        },
    )
    .expect("tag analysis");
    assert!((any_operator.probability_within - 1.0).abs() < 1e-12);
}

#[test]
fn pull_scoped_slots_do_not_accumulate_across_steps() {
    let mut game = reference_game();
    game.states.push(CompiledStateDef {
        id: "scratch_pull_state".to_string(),
        kind: StateKind::Counter,
        scope: StateScope::Pull,
        initial: StateValue::Counter(0),
        max: None,
    });

    let mut state = StateStore::default();
    state.slots.insert(
        "scratch_pull_state::pull:old".to_string(),
        StateValue::Counter(7),
    );
    let context = ScopeContext {
        pull: "new".to_string(),
        ..ScopeContext::default()
    };

    prune_ephemeral_state(
        &game,
        "endfield.banner.reference",
        &mut state,
        &context,
    )
    .expect("prune");
    assert!(!state.slots.contains_key("scratch_pull_state::pull:old"));

    let branches = enumerate_step(
        &game,
        "endfield.banner.reference",
        "single_pull",
        &state,
        &context,
    )
    .expect("enumerate scoped step");
    assert!(!branches.is_empty());
    for branch in branches {
        assert!(!branch
            .next_state
            .slots
            .contains_key("scratch_pull_state::pull:old"));
        assert_eq!(
            branch
                .next_state
                .slots
                .get("scratch_pull_state::pull:new"),
            Some(&StateValue::Counter(0))
        );
    }
}
