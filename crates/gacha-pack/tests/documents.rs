use gacha_pack::resolve_pack;
use serde_json::json;
use std::{collections::BTreeMap, path::Path};

fn documents() -> BTreeMap<String, serde_json::Value> {
    BTreeMap::from([
        (
            "pool/banner.json".into(),
            json!({"schema_version":1,"id":"test.banner","game":"test",
            "version":"0.1.0","rules":"rules/profile.json","catalogs":["data/items.json"],"pools":[]}),
        ),
        (
            "pool/rules/profile.json".into(),
            json!({"schema_version":1,"id":"test.rules","version":"0.1.0",
            "source_documents":["sources.json"],"distributions":[],"selectors":[],"banners":[]}),
        ),
        (
            "pool/data/items.json".into(),
            json!({"schema_version":1,"id":"test.catalog","game":"test",
            "version":"0.1.0","source_documents":["sources.json"],"items":[]}),
        ),
        (
            "pool/rules/sources.json".into(),
            json!([{"id":"test.source","kind":"inferred","title":"规则与内容依据"}]),
        ),
        (
            "pool/data/sources.json".into(),
            json!([{"id":"test.source","kind":"inferred","title":"规则与内容依据"}]),
        ),
    ])
}

fn resolve(
    docs: &BTreeMap<String, serde_json::Value>,
) -> gacha_core::Result<gacha_schema::GamePack> {
    resolve_pack(Path::new("pool/banner.json"), None, |path| {
        let key = path.to_string_lossy().replace('\\', "/");
        docs.get(&key)
            .map(ToString::to_string)
            .ok_or_else(|| gacha_core::EngineError::new(format!("missing {key}")))
    })
}

#[test]
fn source_paths_are_relative_to_their_own_document_and_sources_are_deduplicated() {
    let pack = resolve(&documents()).unwrap();
    assert_eq!(pack.sources.len(), 1);
    assert_eq!(pack.sources[0].id, "test.source");
}

#[test]
fn missing_and_conflicting_source_definitions_fail_resolution() {
    let mut docs = documents();
    docs.get_mut("pool/data/sources.json").unwrap()[0]["title"] = json!("另一份定义");
    assert!(
        resolve(&docs)
            .unwrap_err()
            .message()
            .contains("conflicting source")
    );
    docs.remove("pool/data/sources.json");
    assert!(resolve(&docs).unwrap_err().message().contains("missing"));
}
