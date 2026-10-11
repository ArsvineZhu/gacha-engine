mod support;
use gacha_core::*;
use gacha_pack::{compile_pack, parse_pack};
use serde_json::json;
use support::*;

#[test]
fn conditional_graph_validation_covers_both_paths_and_pre_draw_types() {
    let original = serde_json::to_value(raw("star-rail/character-reference.json")).unwrap();
    for change in [
        json!({"op":"outcome_rarity_eq","rarity":5}),
        json!({"op":"state_bool_eq","state":"pity","value":true}),
    ] {
        let mut value = original.clone();
        value["selectors"][5]["when"] = change;
        assert!(compile_pack(&parse_pack(&value.to_string()).unwrap()).is_err());
    }
    for target in ["top", "missing", "four"] {
        let mut value = original.clone();
        value["selectors"][5]["then_selector"] = json!(target);
        assert!(
            compile_pack(&parse_pack(&value.to_string()).unwrap()).is_err(),
            "{target}"
        );
    }
}

#[test]
fn selector_depth_limit_cannot_be_bypassed_by_definition_order() {
    let mut pack = raw("star-rail/character-reference.json");
    let mut previous = "featured".to_string();
    for i in 0..129 {
        let id = format!("chain_{i}");
        pack.selectors.push(gacha_schema::SelectorDef {
            id: id.clone(),
            selector: gacha_schema::SelectorExpr::Weighted {
                branches: vec![gacha_schema::WeightedSelectorBranch {
                    weight: "1".into(),
                    selector: previous,
                }],
            },
        });
        previous = id;
    }
    assert!(
        compile_pack(&pack)
            .unwrap_err()
            .to_string()
            .contains("maximum depth")
    );
}

#[test]
fn shared_selector_dag_evaluates_without_exponential_expansion() {
    let mut pack = raw("star-rail/character-reference.json");
    let mut previous = "featured".to_string();
    for i in 0..40 {
        let id = format!("diamond_{i}");
        pack.selectors.push(gacha_schema::SelectorDef {
            id: id.clone(),
            selector: gacha_schema::SelectorExpr::Weighted {
                branches: vec![
                    gacha_schema::WeightedSelectorBranch {
                        weight: "1".into(),
                        selector: previous.clone()
                    };
                    2
                ],
            },
        });
        previous = id;
    }
    pack.banners[0].actions[0]
        .selectors
        .insert("5".into(), previous);
    let game = compile_pack(&pack).unwrap();
    assert_eq!(
        mass(
            &game,
            &branches(&game, "star-rail.character", &StateStore::default()),
            "star-rail.featured"
        ),
        "0.006".parse().unwrap()
    );
}
