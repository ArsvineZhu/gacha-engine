use gacha_core::{Rational, ScopeContext, SplitMix64, StateStore, enumerate_step, sample_action};
use gacha_pack::{compile_pack, compose_pack, load_catalog, load_pack};
use gacha_schema::{ContentCatalog, PackManifest, RulePack};
use std::{fs, path::PathBuf};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}
fn json<T: serde::de::DeserializeOwned>(path: &str) -> T {
    serde_json::from_str(&fs::read_to_string(root().join(path)).unwrap()).unwrap()
}

#[test]
fn all_real_manifests_load_compile_and_use_real_items() {
    let mut count = 0;
    for game in fs::read_dir(root().join("official/gacha-games/pools")).unwrap() {
        for file in fs::read_dir(game.unwrap().path()).unwrap() {
            let file = file.unwrap();
            if !file.file_name().to_string_lossy().starts_with("banner.") {
                continue;
            }
            let pack = load_pack(file.path()).unwrap();
            assert!(
                pack.items
                    .iter()
                    .all(|i| !i.display_name.is_empty() && !i.source_ids.is_empty())
            );
            assert!(
                pack.items.iter().all(
                    |i| !i.id.contains(".current_featured") && !i.display_name.contains("示例")
                )
            );
            let game = compile_pack(&pack).unwrap();
            for banner in &game.banners {
                for action in &banner.actions {
                    let branches = enumerate_step(
                        &game,
                        &banner.id,
                        &action.id,
                        &StateStore::default(),
                        &ScopeContext::default(),
                    )
                    .unwrap();
                    assert_eq!(
                        branches
                            .iter()
                            .fold(Rational::ZERO, |p, b| p.checked_add(b.probability).unwrap()),
                        Rational::ONE
                    );
                }
            }
            count += 1;
        }
    }
    assert_eq!(count, 19);
}

#[test]
fn real_catalogs_preserve_equipment_concepts_and_native_rank() {
    for (file, name, kind, rarity, native) in [
        (
            "official/gacha-games/pools/genshin/catalog.character-01.json",
            "雷电将军",
            "character",
            5,
            "5",
        ),
        (
            "official/gacha-games/pools/star-rail/catalog.light_cone-01.json",
            "锋镝",
            "light_cone",
            3,
            "3",
        ),
        (
            "official/gacha-games/pools/zzz/catalog.character-01.json",
            "安比",
            "character",
            4,
            "A",
        ),
        (
            "official/gacha-games/pools/zzz/catalog.character-01.json",
            "艾莲",
            "character",
            5,
            "S",
        ),
        (
            "official/gacha-games/pools/nte/catalog.announced-1.4.json",
            "罪与罚",
            "arc",
            5,
            "S",
        ),
        (
            "official/gacha-games/pools/endfield/catalog.weapon-01.json",
            "熔铸火焰",
            "weapon",
            6,
            "6",
        ),
    ] {
        let catalog = load_catalog(root().join(file)).unwrap();
        let item = catalog
            .items
            .iter()
            .find(|i| i.display_name == name)
            .unwrap();
        assert_eq!(
            serde_json::to_value(item.kind).unwrap().as_str(),
            Some(kind)
        );
        assert_eq!(item.rarity, rarity);
        assert_eq!(item.native_rarity, native);
    }
}

#[test]
fn mechanisms_reject_embedded_rosters_and_catalogs_reject_mechanisms() {
    let mut rules: serde_json::Value =
        json("official/gacha-games/pools/star-rail/rule.character-reference.json");
    rules["items"] = serde_json::json!([]);
    assert!(serde_json::from_value::<RulePack>(rules).is_err());
    let mut catalog: serde_json::Value =
        json("official/gacha-games/pools/star-rail/catalog.character-01.json");
    catalog["guarantees"] = serde_json::json!([]);
    assert!(serde_json::from_value::<ContentCatalog>(catalog).is_err());
    let mut catalog: serde_json::Value =
        json("official/gacha-games/pools/star-rail/catalog.character-01.json");
    catalog["items"][0]["probability"] = serde_json::json!("0.5");
    assert!(serde_json::from_value::<ContentCatalog>(catalog).is_err());
}

#[test]
fn composition_rejects_cross_game_data_missing_bindings_and_duplicate_items() {
    let mut manifest: PackManifest =
        json("official/gacha-games/pools/endfield/banner.weapon-reference.json");
    let rules: RulePack = json("official/gacha-games/pools/endfield/rule.weapon-reference.json");
    let catalog =
        load_catalog(root().join("official/gacha-games/pools/endfield/catalog.weapon-01.json"))
            .unwrap();
    let wrong =
        load_catalog(root().join("official/gacha-games/pools/nte/catalog.arc-01.json")).unwrap();
    assert!(compose_pack(&manifest, &rules, &[wrong]).is_err());
    assert!(compose_pack(&manifest, &rules, &[catalog.clone(), catalog.clone()]).is_err());
    manifest.item_bindings.clear();
    assert!(compose_pack(&manifest, &rules, &[catalog]).is_err());
}

#[test]
fn swapping_featured_content_reuses_mechanism_and_rarity_validation() {
    let mut manifest: PackManifest =
        json("official/gacha-games/pools/endfield/banner.weapon-reference.json");
    let rules: RulePack = json("official/gacha-games/pools/endfield/rule.weapon-reference.json");
    let catalog =
        load_catalog(root().join("official/gacha-games/pools/endfield/catalog.weapon-01.json"))
            .unwrap();
    let featured = "endfield.weapon.热熔切割器";
    manifest
        .item_bindings
        .insert("featured".into(), featured.into());
    manifest
        .pools
        .iter_mut()
        .find(|p| p.id == "featured")
        .unwrap()
        .items = vec![featured.into()];
    let pack = compose_pack(&manifest, &rules, std::slice::from_ref(&catalog)).unwrap();
    let game = compile_pack(&pack).unwrap();
    assert!(
        game.item(game.item_id(featured).unwrap())
            .tags
            .contains(&"featured".into())
    );
    manifest
        .pools
        .iter_mut()
        .find(|p| p.id == "featured")
        .unwrap()
        .items = vec!["endfield.weapon.长路".into()];
    assert!(compile_pack(&compose_pack(&manifest, &rules, &[catalog]).unwrap()).is_err());
}

#[test]
fn nte_featured_character_is_guaranteed_at_90_and_arc_actions_return_ten() {
    let game = compile_pack(
        &load_pack(root().join("official/gacha-games/pools/nte/banner.character-reference.json"))
            .unwrap(),
    )
    .unwrap();
    let (_, banner) = game.banner("nte.character").unwrap();
    let def = &game.states[game.state_lookup["pity"].0];
    let mut state = StateStore::default();
    state.slots.insert(
        gacha_core::state_slot_key(&game, banner, def, &ScopeContext::default()),
        gacha_core::StateValue::Counter(89),
    );
    let branches = enumerate_step(
        &game,
        "nte.character",
        "single_pull",
        &state,
        &ScopeContext::default(),
    )
    .unwrap();
    assert_eq!(branches.len(), 1);
    assert_eq!(game.item(branches[0].outcome.item).display_name, "黑羽");
    let game = compile_pack(
        &load_pack(root().join("official/gacha-games/pools/nte/banner.arc-reference.json"))
            .unwrap(),
    )
    .unwrap();
    let result = sample_action(
        &game,
        "nte.arc",
        "research",
        &mut StateStore::default(),
        &ScopeContext::default(),
        &mut SplitMix64::new(42),
    )
    .unwrap();
    assert_eq!(result.len(), 10);
    assert!(
        result
            .iter()
            .all(|b| game.item(b.outcome.item).kind == "arc")
    );
}
