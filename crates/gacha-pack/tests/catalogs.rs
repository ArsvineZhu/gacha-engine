mod support;
use gacha_core::*;
use support::*;

#[test]
fn every_reference_pack_compiles_and_all_initial_actions_normalize() {
    for path in [
        "endfield/chartered-proportional.json",
        "endfield/chartered-fixed-five.json",
        "endfield/weapon-reference.json",
        "endfield/reconstruction-reference.json",
        "arknights/kernel-reference.json",
        "genshin/character-reference.json",
        "star-rail/character-reference.json",
        "zzz/character-reference.json",
        "wuthering-waves/character-reference.json",
        "wuthering-waves/weapon-reference.json",
    ] {
        let game = game(path);
        for banner in &game.banners {
            for action in &banner.actions {
                let result = enumerate_step(
                    &game,
                    &banner.id,
                    &action.id,
                    &StateStore::default(),
                    &ScopeContext::default(),
                )
                .unwrap();
                assert_eq!(
                    result.iter().fold(Rational::ZERO, |sum, b| sum
                        .checked_add(b.probability)
                        .unwrap()),
                    Rational::ONE,
                    "{path}"
                );
            }
        }
    }
}
