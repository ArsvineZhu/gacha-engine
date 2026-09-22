use gacha_analysis::probability_of_item_within;
use gacha_core::{ScopeContext, StateStore, enumerate_transitions};
use gacha_pack::{compile_pack, load_pack};
use std::path::PathBuf;

#[test]
fn one_draw_analysis_matches_enumeration() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../packs/endfield/chartered-proportional.json");
    let pack = load_pack(path).expect("load");
    let game = compile_pack(&pack).expect("compile");
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
