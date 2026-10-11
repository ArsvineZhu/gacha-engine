use gacha_sdk::*;
use std::path::PathBuf;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}
fn weapon() -> Engine {
    Engine::load(root().join("official/gacha-games/pools/endfield/banner.preview-yuzhongshen.json"))
        .unwrap()
}

#[test]
fn complete_action_report_export_and_replay_are_public_contracts() {
    let engine = weapon();
    let action = &engine.actions()[0];
    let request = AnalysisRequest::first_hit(
        &action.banner,
        &action.action,
        TargetSpec::Item {
            id: "endfield.weapon.玉中身".into(),
        },
        8,
    );
    let study = engine.study(&request, 500, 20261011).unwrap();
    assert!(study.comparison.six_sigma_passed);
    assert_eq!(study.analysis.draws_per_action, 10);
    assert_eq!(study.analysis.draw_distribution.distribution.len(), 80);
    assert_eq!(study.analysis.action_distribution.len(), 8);
    assert_eq!(study.analysis.action_quantiles[4].action, Some(8));
    assert!((study.analysis.expected_actions_if_hit.unwrap() - 5.547302748562227).abs() < 1e-12);
    let mut session = engine.session(42);
    session.pull(&action.banner, &action.action).unwrap();
    let snapshot: SessionSnapshot =
        serde_json::from_str(&serde_json::to_string(&session.snapshot()).unwrap()).unwrap();
    let mut restored = engine.restore_session(snapshot).unwrap();
    let left = session.pull_trace(&action.banner, &action.action).unwrap();
    let right = restored.pull_trace(&action.banner, &action.action).unwrap();
    assert_eq!(
        serde_json::to_value(&left).unwrap(),
        serde_json::to_value(&right).unwrap()
    );
    assert!(
        left.iter()
            .all(|r| r.trace.as_ref().unwrap().transition_checks.len() > 5)
    );
    assert!(engine.preparation().cache_hits > 0);
}

#[test]
fn rule_override_rebinds_target_and_snapshot_rejects_changed_rules() {
    let path = root().join("official/gacha-games/pools/endfield/banner.preview-si.json");
    let fixed = Engine::load(&path).unwrap();
    let alternative = Engine::load_with_rules(
        &path,
        root().join("official/gacha-games/pools/endfield/rule.chartered-proportional.json"),
    )
    .unwrap();
    assert_ne!(fixed.fingerprint(), alternative.fingerprint());
    assert!(
        alternative
            .restore_session(fixed.session(1).snapshot())
            .is_err()
    );
    let a = &alternative.actions()[0];
    let mut session = alternative.session(42);
    session
        .set_state(&a.banner, "banner_pulls", StateValue::Counter(119))
        .unwrap();
    let pull = session.pull_trace(&a.banner, &a.action).unwrap();
    assert_eq!(pull[0].item.name, "祀");
    assert_eq!(pull[0].probability, Rational::ONE);
    assert_eq!(
        pull[0].trace.as_ref().unwrap().applied_guarantee.as_deref(),
        Some("featured_120")
    );
}

#[test]
fn invalid_state_and_failed_action_do_not_change_session() {
    let engine = weapon();
    let mut session = engine.session(42);
    let snapshot = serde_json::to_value(session.snapshot()).unwrap();
    assert!(
        session
            .set_state(
                &engine.actions()[0].banner,
                "batch_draws",
                StateValue::Counter(11)
            )
            .is_err()
    );
    assert!(session.pull("missing", "requisition").is_err());
    assert_eq!(snapshot, serde_json::to_value(session.snapshot()).unwrap());
}
