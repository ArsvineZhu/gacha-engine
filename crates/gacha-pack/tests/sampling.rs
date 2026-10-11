mod support;
use gacha_core::*;
use support::*;

#[test]
fn sampling_matches_independent_base_rate_and_is_reproducible() {
    let game = game("star-rail/character-reference.json");
    let mut rng = SplitMix64::new(20261011);
    let mut top = 0;
    for _ in 0..50000 {
        let result = sample_step(
            &game,
            "star-rail.character",
            "single_pull",
            &mut StateStore::default(),
            &ScopeContext::default(),
            &mut rng,
        )
        .unwrap();
        top += usize::from(result.outcome.rarity == 5);
    }
    // 二项分布校验：n=50,000，p=.006，期望 300，标准差 17.27，采用六倍标准差边界。
    assert!((196..=404).contains(&top), "top-rarity count {top}");
    let mut a = SplitMix64::new(7);
    let mut b = a.clone();
    for _ in 0..100 {
        assert_eq!(a.below_u128(125).unwrap(), b.below_u128(125).unwrap());
    }
}
