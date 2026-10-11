use gacha_analysis::{FirstHitQuery, TargetSpec, analyze_first_hit};
use gacha_core::{ScopeContext, StateStore, StateValue, enumerate_step, state_slot_key};
use gacha_pack::{compile_pack, load_pack};
use std::path::PathBuf;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

#[test]
fn announced_targets_bind_to_first_copy_guarantees_without_administrators() {
    for (slug, name, kind) in [
        ("si", "祀", "character"),
        ("minghe", "明河", "character"),
        ("yuzhongshen", "玉中身", "weapon"),
        ("huanyue", "唤月", "weapon"),
    ] {
        let pack = load_pack(root().join(format!(
            "official/gacha-games/pools/endfield/banner.preview-{slug}.json"
        )))
        .unwrap();
        assert!(
            pack.pools
                .iter()
                .flat_map(|p| &p.items)
                .all(|id| !id.contains("管理员"))
        );
        let game = compile_pack(&pack).unwrap();
        let banner = &game.banners[0];
        let mut state = StateStore::default();
        let context = ScopeContext::default();
        let states = if kind == "character" {
            vec![("banner_pulls", 119)]
        } else {
            vec![("batch_draws", 9), ("featured_miss_batches", 7)]
        };
        for (id, value) in states {
            let def = &game.states[game.state_lookup[id].0];
            state.slots.insert(
                state_slot_key(&game, banner, def, &context),
                StateValue::Counter(value),
            );
        }
        let action = if kind == "character" {
            "single_pull"
        } else {
            "requisition"
        };
        let branches = enumerate_step(&game, &banner.id, action, &state, &context).unwrap();
        assert!(
            branches
                .iter()
                .all(|b| game.item(b.outcome.item).display_name == name)
        );
    }
}

#[test]
fn target_cdf_does_not_depend_on_non_target_roster_size() {
    let mut pack =
        load_pack(root().join("official/gacha-games/pools/endfield/banner.preview-si.json"))
            .unwrap();
    let full = compile_pack(&pack).unwrap();
    pack.pools
        .iter_mut()
        .find(|p| p.id == "pool.six_off_banner")
        .unwrap()
        .items = vec!["endfield.character.余烬".into()];
    let reduced = compile_pack(&pack).unwrap();
    let query = FirstHitQuery {
        banner: full.banners[0].id.clone(),
        action: "single_pull".into(),
        target: TargetSpec::Item {
            id: "endfield.character.祀".into(),
        },
        draws: 80,
        quantiles: vec![],
    };
    let a = analyze_first_hit(
        &full,
        &StateStore::default(),
        &ScopeContext::default(),
        &query,
    )
    .unwrap();
    let b = analyze_first_hit(
        &reduced,
        &StateStore::default(),
        &ScopeContext::default(),
        &query,
    )
    .unwrap();
    for (left, right) in a.distribution.iter().zip(&b.distribution) {
        assert!((left.cumulative_probability - right.cumulative_probability).abs() < 1e-12);
    }
}
