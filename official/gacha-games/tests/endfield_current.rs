use gacha_core::{ScopeContext, StateStore, StateValue, enumerate_step, state_slot_key};
use gacha_pack::{compile_pack, load_catalog, load_pack};
use std::path::PathBuf;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

#[test]
fn refreshed_endfield_catalog_contains_new_content_and_real_client_ids() {
    let characters =
        load_catalog(root().join("official/gacha-games/pools/endfield/catalog.character-01.json"))
            .unwrap();
    let weapons =
        load_catalog(root().join("official/gacha-games/pools/endfield/catalog.weapon-01.json"))
            .unwrap();
    assert_eq!(characters.items.len(), 33);
    assert_eq!(weapons.items.len(), 80);
    let administrators: Vec<_> = characters
        .items
        .iter()
        .filter(|i| i.display_name.starts_with("管理员"))
        .collect();
    assert_eq!(administrators.len(), 2);
    assert!(
        administrators
            .iter()
            .all(|i| i.gacha_eligible == Some(false))
    );
    for (catalog, name, rarity) in [
        (&characters, "提弗洛斯", 6),
        (&characters, "梨诺", 6),
        (&characters, "噗切娜", 5),
        (&weapons, "寒夜幽影", 6),
        (&weapons, "苦难的尽头", 6),
        (&weapons, "点心时刻", 5),
    ] {
        let item = catalog
            .items
            .iter()
            .find(|i| i.display_name == name)
            .unwrap();
        assert_eq!(item.rarity, rarity);
        assert!(!item.upstream_id.is_empty());
    }
    assert!(
        characters
            .items
            .iter()
            .all(|i| i.upstream_id != "chr_9000_endmin")
    );
}

#[test]
fn administrators_cannot_enter_draw_pools_or_forced_draw_outcomes() {
    let path = root().join("official/gacha-games/pools/endfield/banner.winter-hunt-2026-09.json");
    let admin = "endfield.character.管理员（男）";
    let mut pack = load_pack(&path).unwrap();
    assert!(
        pack.pools
            .iter()
            .all(|p| !p.items.iter().any(|i| i.contains("管理员")))
    );
    pack.pools[0].items.push(admin.into());
    assert!(
        compile_pack(&pack)
            .unwrap_err()
            .to_string()
            .contains("non-gacha")
    );
    let mut pack = load_pack(&path).unwrap();
    pack.banners[0].actions[0].guarantees[0].effect =
        gacha_schema::GuaranteeEffectDef::ForceItem { item: admin.into() };
    assert!(
        compile_pack(&pack)
            .unwrap_err()
            .to_string()
            .contains("non-gacha")
    );
}

#[test]
fn dated_rosters_match_announcements_and_exclude_activity_only_items() {
    let winter = load_pack(
        root().join("official/gacha-games/pools/endfield/banner.winter-hunt-2026-09.json"),
    )
    .unwrap();
    let cold = load_pack(
        root().join("official/gacha-games/pools/endfield/banner.cold-shadow-2026-09.json"),
    )
    .unwrap();
    let six = &winter
        .pools
        .iter()
        .find(|p| p.id == "pool.six_off_banner")
        .unwrap()
        .items;
    assert!(six.contains(&"endfield.character.梨诺".into()));
    assert!(six.contains(&"endfield.character.诀".into()));
    assert!(
        !winter
            .pools
            .iter()
            .any(|p| p.items.contains(&"endfield.character.噗切娜".into()))
    );
    assert!(
        !cold
            .pools
            .iter()
            .any(|p| p.items.contains(&"endfield.weapon.点心时刻".into()))
    );
    assert_eq!(
        cold.pools
            .iter()
            .find(|p| p.id == "featured")
            .unwrap()
            .items,
        vec!["endfield.weapon.寒夜幽影"]
    );
}

#[test]
fn reconstruction_weapon_preserves_series_progress_and_resets_local_six_pity() {
    let mut pack = load_pack(
        root()
            .join("official/gacha-games/pools/endfield/banner.reconstruction-weapon-2026-09.json"),
    )
    .unwrap();
    let first = pack.banners[0].id.clone();
    let mut rerun = pack.banners[0].clone();
    rerun.id = "rerun.2".into();
    pack.banners.push(rerun);
    let game = compile_pack(&pack).unwrap();
    let context = ScopeContext::default();
    let (_, banner) = game.banner(&first).unwrap();
    let mut state = StateStore::default();
    for (id, value) in [
        ("six_miss_batches", StateValue::Counter(3)),
        ("featured_miss_batches", StateValue::Counter(7)),
        ("requisitions", StateValue::Counter(9)),
        ("featured_available", StateValue::Boolean(true)),
        ("batch_draws", StateValue::Counter(9)),
    ] {
        let def = &game.states[game.state_lookup[id].0];
        state
            .slots
            .insert(state_slot_key(&game, banner, def, &context), value);
    }
    let branches = enumerate_step(&game, "rerun.2", "requisition", &state, &context).unwrap();
    assert!(
        branches
            .iter()
            .all(|b| game.item(b.outcome.item).display_name == "艺术暴君")
    );
    assert!(
        branches
            .iter()
            .all(|b| b.events.iter().any(|e| e.key == "arsenal_box"))
    );
    let six = &game.states[game.state_lookup["six_miss_batches"].0];
    let (_, rerun) = game.banner("rerun.2").unwrap();
    assert_ne!(
        state_slot_key(&game, banner, six, &context),
        state_slot_key(&game, rerun, six, &context)
    );
    let progress = &game.states[game.state_lookup["requisitions"].0];
    assert_eq!(
        state_slot_key(&game, banner, progress, &context),
        state_slot_key(&game, rerun, progress, &context)
    );
}
