use std::fs;
use std::path::PathBuf;

use gacha_engine::{HitQuery, Rational, Source, Target, compile};

fn read(path: PathBuf) -> (String, String) {
    let name = path.file_name().unwrap().to_string_lossy().into_owned();
    let text = fs::read_to_string(&path).unwrap();
    (name, text)
}

fn positive() -> gacha_engine::Compiled {
    let base = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("spec/gacharule/positive");
    let order = [
        "endfield.gmr",
        "genshin.gmr",
        "weapon_ten.gmr",
        "sample.gml",
        "banners.gmb",
    ];
    let owned: Vec<(String, String)> = order.iter().map(|name| read(base.join(name))).collect();
    let sources: Vec<Source<'_>> = owned
        .iter()
        .map(|(name, text)| Source { name, text })
        .collect();
    compile(&sources).unwrap()
}

fn near(actual: f64, expected: f64) {
    assert!((actual - expected).abs() <= 1e-9, "{actual} != {expected}");
}

#[test]
fn positive_hits_match_exact_step_probabilities() {
    let compiled = positive();
    let mut session = compiled.session();
    session.set_count("endfield_event_a", "pity6", 65).unwrap();
    let report = session
        .analyze(&HitQuery {
            banner: "endfield_event_a".to_string(),
            action: "draw".to_string(),
            times: 1,
            actions: 2,
            copies: 1,
            target: Target::RarityAtLeast(6),
            quantiles: vec![29.0 / 500.0, 0.5],
        })
        .unwrap();
    near(report.draws_distribution[0].hit_probability, 29.0 / 500.0);
    near(
        report.draws_distribution[1].hit_probability,
        (471.0 / 500.0) * (54.0 / 500.0),
    );
    near(
        report.probability_within,
        report.draws_distribution[1].cumulative_probability,
    );
    assert_eq!(report.draw_quantiles[0].index, Some(1));
    assert_eq!(report.draw_quantiles[1].index, None);
    assert_eq!(report.actions_distribution.len(), 2);
    near(
        report.actions_distribution[0].hit_probability,
        report.draws_distribution[0].hit_probability,
    );

    let fresh = compiled.session();
    let base = fresh
        .analyze(&HitQuery {
            banner: "endfield_event_a".to_string(),
            action: "draw".to_string(),
            times: 1,
            actions: 2,
            copies: 2,
            target: Target::RarityAtLeast(6),
            quantiles: vec![],
        })
        .unwrap();
    near(base.draws_distribution[0].hit_probability, 0.0);
    near(
        base.draws_distribution[1].hit_probability,
        (1.0 / 125.0) * (1.0 / 125.0),
    );

    let tagged = fresh
        .analyze(&HitQuery {
            banner: "endfield_event_a".to_string(),
            action: "draw".to_string(),
            times: 1,
            actions: 1,
            copies: 1,
            target: Target::Tag("limited".to_string()),
            quantiles: vec![],
        })
        .unwrap();
    near(tagged.probability_within, 1.0 / 250.0);

    let featured = fresh
        .analyze(&HitQuery {
            banner: "endfield_event_a".to_string(),
            action: "draw".to_string(),
            times: 1,
            actions: 1,
            copies: 1,
            target: Target::Item("ef_featured".to_string()),
            quantiles: vec![],
        })
        .unwrap();
    near(featured.probability_within, 1.0 / 250.0);

    let either = fresh
        .analyze(&HitQuery {
            banner: "endfield_event_a".to_string(),
            action: "draw".to_string(),
            times: 1,
            actions: 1,
            copies: 1,
            target: Target::Any(vec!["ef_featured".to_string(), "ef_standard".to_string()]),
            quantiles: vec![1.0 / 125.0],
        })
        .unwrap();
    near(either.probability_within, 1.0 / 125.0);
    near(either.expected_draw_if_hit.unwrap(), 1.0);
    near(either.expected_draws_capped, 1.0);
    assert_eq!(either.draw_quantiles[0].index, Some(1));

    let genshin = fresh
        .analyze(&HitQuery {
            banner: "genshin_event_a".to_string(),
            action: "draw".to_string(),
            times: 1,
            actions: 1,
            copies: 1,
            target: Target::RarityAtLeast(5),
            quantiles: vec![],
        })
        .unwrap();
    near(genshin.probability_within, 3.0 / 500.0);
    let four = fresh.draw("genshin_event_a", "draw").unwrap();
    let four_star = four
        .branches
        .iter()
        .filter(|branch| branch.rarity == 4)
        .try_fold(Rational::zero(), |acc, branch| {
            acc.checked_add(branch.probability)
        })
        .unwrap();
    assert_eq!(four_star, Rational::new(51, 1000));
}

#[test]
fn action_analysis_resets_action_state_and_leaves_the_session() {
    let compiled = positive();
    let mut session = compiled.session();
    for _ in 0..9 {
        session.apply("weapon_sample_a", "ten", "wp_four").unwrap();
    }
    let before = session.snapshot();
    let report = session
        .analyze(&HitQuery {
            banner: "weapon_sample_a".to_string(),
            action: "ten".to_string(),
            times: 10,
            actions: 1,
            copies: 1,
            target: Target::RarityAtLeast(5),
            quantiles: vec![0.1],
        })
        .unwrap();
    near(report.draws_distribution[0].hit_probability, 0.1);
    assert_eq!(report.draws, 10);
    assert_eq!(report.actions_distribution.len(), 1);
    let mass: f64 = report
        .draws_distribution
        .iter()
        .map(|point| point.hit_probability)
        .sum();
    near(mass, 1.0);
    near(report.probability_within, 1.0);
    near(report.probability_not_hit, 0.0);
    assert_eq!(session.snapshot(), before);
    let draw = session.draw("weapon_sample_a", "ten").unwrap();
    let six = draw
        .branches
        .iter()
        .filter(|branch| branch.rarity == 6)
        .try_fold(Rational::zero(), |acc, branch| {
            acc.checked_add(branch.probability)
        })
        .unwrap();
    assert_eq!(six, Rational::new(1, 10));
}

#[test]
fn snapshot_restores_counters_and_seeded_sample_is_stable() {
    let compiled = positive();
    let mut session = compiled.session();
    session.set_count("genshin_event_a", "pity5", 73).unwrap();
    session
        .set_bool("genshin_event_a", "missed_featured5", true)
        .unwrap();
    let saved = session.snapshot();
    session.set_count("genshin_event_a", "pity5", 0).unwrap();
    session.restore(&saved).unwrap();
    assert_eq!(session.get_count("genshin_event_a", "pity5").unwrap(), 73);
    assert!(
        session
            .get_bool("genshin_event_a", "missed_featured5")
            .unwrap()
    );
    let first = compiled
        .session()
        .sample_with_seed("genshin_event_a", "draw", 7)
        .unwrap();
    let second = compiled
        .session()
        .sample_with_seed("genshin_event_a", "draw", 7)
        .unwrap();
    assert_eq!(first.entry, second.entry);
    assert_eq!(first.probability, second.probability);
}
