#![allow(dead_code)]
use gacha_core::*;
use gacha_pack::{compile_pack, parse_pack};
use std::path::PathBuf;

pub fn raw(path: &str) -> gacha_schema::GamePack {
    parse_pack(
        &std::fs::read_to_string(
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../tests/fixtures/packs")
                .join(path),
        )
        .unwrap(),
    )
    .unwrap()
}
pub fn game(path: &str) -> CompiledGame {
    compile_pack(&raw(path)).unwrap()
}
pub fn set(
    game: &CompiledGame,
    banner: &str,
    state: &mut StateStore,
    name: &str,
    value: StateValue,
) {
    let (_, banner) = game.banner(banner).unwrap();
    let def = &game.states[game.state_lookup[name].0];
    state.slots.insert(
        state_slot_key(game, banner, def, &ScopeContext::default()),
        value,
    );
}
pub fn branches(game: &CompiledGame, banner: &str, state: &StateStore) -> Vec<Branch> {
    enumerate_step(game, banner, "single_pull", state, &ScopeContext::default()).unwrap()
}
pub fn mass(game: &CompiledGame, branches: &[Branch], item: &str) -> Rational {
    branches
        .iter()
        .filter(|b| game.item(b.outcome.item).id == item)
        .fold(Rational::ZERO, |sum, b| {
            sum.checked_add(b.probability).unwrap()
        })
}
