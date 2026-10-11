use gacha_analysis::{FirstHitQuery, TargetSpec, analyze_nth_hit};
use gacha_core::{ScopeContext, StateStore, StateValue};
use gacha_pack::{compile_pack, load_pack, parse_pack};
use std::path::PathBuf;

#[test]
fn batch_analysis_resets_initial_and_between_action_state() {
    let pack = parse_pack(r#"{
      "schema_version":1,"id":"batch","version":"0.1.0",
      "state":[{"id":"count","kind":"counter","scope":"batch","initial":0,"max":2}],
      "items":[{"id":"hit","rarity":5},{"id":"miss","rarity":3}],
      "pools":[{"id":"hit","items":["hit"]},{"id":"miss","items":["miss"]}],
      "selectors":[{"id":"hit","type":"pool","pool":"hit"},{"id":"miss","type":"pool","pool":"miss"}],
      "distributions":[{"id":"d","entries":[{"rarity":5,"type":"constant","value":"0"},{"rarity":3,"type":"remainder"}]}],
      "banners":[{"id":"b","pity_group":"b","progress_group":"b","actions":[{
        "id":"pair","draws":2,"distribution":"d","selectors":{"5":"hit","3":"miss"},
        "guarantees":[{"id":"last","priority":1,"when":{"op":"state_counter_eq","state":"count","value":1},"effect":{"type":"force_item","item":"hit"}}],
        "transitions":[{"id":"inc","when":{"op":"always"},"updates":[{"type":"increment","state":"count"}]}]
      }]}]
    }"#).unwrap();
    let game = compile_pack(&pack).unwrap();
    let mut state = StateStore::default();
    state
        .slots
        .insert("count::batch:default".into(), StateValue::Counter(1));
    let mut query = FirstHitQuery {
        banner: "b".into(),
        action: "pair".into(),
        target: TargetSpec::Item { id: "hit".into() },
        draws: 4,
        quantiles: vec![1.0],
    };
    let result = analyze_nth_hit(&game, &state, &ScopeContext::default(), &query, 2).unwrap();
    assert_eq!(result.quantiles[0].draw, Some(4));
    assert_eq!(result.distribution[3].first_hit_probability, 1.0);
    assert!(
        result.distribution[..3]
            .iter()
            .all(|p| p.first_hit_probability == 0.0)
    );
    query.draws = 3;
    assert!(analyze_nth_hit(&game, &state, &ScopeContext::default(), &query, 1).is_err());
}

#[test]
fn weapon_batch_cdf_matches_independent_four_state_recurrence() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let game = compile_pack(
        &load_pack(
            root.join("official/gacha-games/pools/endfield/banner.cold-shadow-2026-09.json"),
        )
        .unwrap(),
    )
    .unwrap();
    let query = FirstHitQuery {
        banner: "endfield.cold-shadow.2026-09".into(),
        action: "requisition".into(),
        target: TargetSpec::Tag {
            tag: "featured".into(),
        },
        draws: 80,
        quantiles: vec![0.5, 1.0],
    };
    let result = analyze_nth_hit(
        &game,
        &StateStore::default(),
        &ScopeContext::default(),
        &query,
        1,
    )
    .unwrap();
    let mut remaining = [1.0, 0.0, 0.0, 0.0];
    for batch in 1..=8 {
        let mut next = [0.0; 4];
        if batch < 8 {
            for (misses, mass) in remaining.iter().enumerate() {
                if misses == 3 {
                    next[0] += mass * (0.99_f64.powi(10) - 0.24 * 0.96_f64.powi(9));
                } else {
                    next[0] += mass * (0.99_f64.powi(10) - 0.96_f64.powi(10));
                    next[misses + 1] += mass * 0.96_f64.powi(10);
                }
            }
        }
        let expected = 1.0 - next.iter().sum::<f64>();
        let point = &result.distribution[batch * 10 - 1];
        assert!((point.cumulative_probability - expected).abs() < 1e-12);
        remaining = next;
    }
    assert!(
        result
            .distribution
            .iter()
            .all(|p| (p.cumulative_probability + p.survival_probability - 1.0).abs() < 1e-12)
    );
    assert_eq!(result.quantiles[1].draw, Some(80));
}
