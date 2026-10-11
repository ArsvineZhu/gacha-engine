use gacha_analysis::{FirstHitQuery, TargetSpec, analyze_first_hit, analyze_nth_hit};
use gacha_core::{ScopeContext, StateStore};
use gacha_pack::{compile_pack, parse_pack};

fn coin() -> gacha_core::CompiledGame {
    compile_pack(&parse_pack(r#"{
      "schema_version":1,"id":"coin","version":"0.1.0",
      "items":[{"id":"hit","rarity":5},{"id":"miss","rarity":3}],
      "pools":[{"id":"hit","items":["hit"]},{"id":"miss","items":["miss"]}],
      "selectors":[{"id":"hit","type":"pool","pool":"hit"},{"id":"miss","type":"pool","pool":"miss"}],
      "distributions":[{"id":"coin","entries":[{"rarity":5,"type":"constant","value":"1/2"},{"rarity":3,"type":"remainder"}]}],
      "banners":[{"id":"coin","pity_group":"coin","progress_group":"coin","actions":[{"id":"flip","distribution":"coin","selectors":{"5":"hit","3":"miss"}}]}]
    }"#).unwrap()).unwrap()
}
fn query(draws: u32) -> FirstHitQuery {
    FirstHitQuery {
        banner: "coin".into(),
        action: "flip".into(),
        target: TargetSpec::Item { id: "hit".into() },
        draws,
        quantiles: vec![0.5, 0.9],
    }
}

#[test]
fn second_hit_matches_negative_binomial_oracle() {
    let result = analyze_nth_hit(
        &coin(),
        &StateStore::default(),
        &ScopeContext::default(),
        &query(5),
        2,
    )
    .unwrap();
    // 独立公式：P(T2=n)=(n-1)/2^n，五次至少两次正面为 26/32。
    for (n, point) in result.distribution.iter().enumerate() {
        let draw = (n + 1) as i32;
        let expected = if draw >= 2 {
            f64::from(draw - 1) / 2_f64.powi(draw)
        } else {
            0.0
        };
        assert!((point.first_hit_probability - expected).abs() < 1e-14);
        assert!((point.cumulative_probability + point.survival_probability - 1.0).abs() < 1e-14);
    }
    assert!((result.probability_within - 26.0 / 32.0).abs() < 1e-14);
    assert_eq!(result.quantiles[0].draw, Some(3));
    assert_eq!(result.quantiles[1].draw, None);
    let original = analyze_first_hit(
        &coin(),
        &StateStore::default(),
        &ScopeContext::default(),
        &query(5),
    )
    .unwrap();
    assert!((original.probability_within - 31.0 / 32.0).abs() < 1e-14);
}

#[test]
fn zero_horizon_and_impossible_copy_counts_preserve_survival() {
    for (draws, copies) in [(0, 1), (3, 4)] {
        let result = analyze_nth_hit(
            &coin(),
            &StateStore::default(),
            &ScopeContext::default(),
            &query(draws),
            copies,
        )
        .unwrap();
        assert_eq!(result.probability_within, 0.0);
        assert_eq!(result.probability_not_hit, 1.0);
        assert_eq!(result.expected_draw_if_hit, None);
        assert_eq!(result.expected_draws_capped_at_horizon, f64::from(draws));
    }
    assert!(
        analyze_nth_hit(
            &coin(),
            &StateStore::default(),
            &ScopeContext::default(),
            &query(3),
            0
        )
        .is_err()
    );
    let mut invalid = query(0);
    invalid.banner = "missing".into();
    assert!(
        analyze_first_hit(
            &coin(),
            &StateStore::default(),
            &ScopeContext::default(),
            &invalid
        )
        .is_err()
    );
}
