use gacha_sdk::Engine;
use std::path::PathBuf;

#[test]
fn embedded_banners_match_canonical_files_and_profiles() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("pools");
    assert_eq!(gacha_games::banners().count(), 19);
    for id in gacha_games::banners() {
        let embedded = gacha_games::load(id).unwrap();
        let file = Engine::load(root.join(id)).unwrap();
        assert_eq!(embedded.fingerprint(), file.fingerprint(), "{id}");
    }
    let id = "endfield/banner.preview-si.json";
    let profile = "endfield/rule.chartered-proportional.json";
    let embedded = gacha_games::load_with_profile(id, Some(profile)).unwrap();
    let file = Engine::load_with_rules(root.join(id), root.join(profile)).unwrap();
    assert_eq!(embedded.fingerprint(), file.fingerprint());
    assert!(gacha_games::load("endfield/../sources.json").is_err());
}
