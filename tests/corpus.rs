use std::fs;
use std::path::PathBuf;

use gacha_engine::{FiredArg, Rational, Source, compile};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("spec/gacharule")
}

fn read(path: PathBuf) -> (String, String) {
    let name = path.file_name().unwrap().to_string_lossy().into_owned();
    let text = fs::read_to_string(&path).unwrap();
    (name, text)
}

fn files_in(dir: PathBuf) -> Vec<(String, String)> {
    let mut names = fs::read_dir(&dir)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.is_file())
        .collect::<Vec<_>>();
    names.sort_by_key(|path| {
        let name = path.file_name().unwrap().to_string_lossy().into_owned();
        let rank = match path.extension().and_then(|ext| ext.to_str()) {
            Some("gmr") => 0,
            Some("gml") => 1,
            Some("gmb") => 2,
            _ => 3,
        };
        (rank, name)
    });
    names.into_iter().map(read).collect()
}

#[test]
fn positive_examples_match_exact_probabilities() {
    let base = root().join("positive");
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
    let compiled = compile(&sources).unwrap_or_else(|error| {
        panic!(
            "{}:{}:{} {}",
            error.file, error.line, error.column, error.code
        )
    });
    let mut session = compiled.session();

    session.set_count("endfield_event_a", "pity6", 64);
    let draw = session.draw("endfield_event_a", "draw").unwrap();
    assert_eq!(rarity(&draw, 6), Rational::new(1, 125));

    session.set_count("endfield_event_a", "pity6", 65);
    let draw = session.draw("endfield_event_a", "draw").unwrap();
    assert_eq!(rarity(&draw, 6), Rational::new(29, 500));

    session.set_count("endfield_event_a", "pity6", 79);
    session.set_count("endfield_event_a", "pulls_this_banner", 0);
    let draw = session.draw("endfield_event_a", "draw").unwrap();
    assert_eq!(draw.guarantee.as_deref(), Some("six_80"));
    assert_eq!(rarity(&draw, 6), Rational::one());

    session.set_count("endfield_event_a", "pulls_this_banner", 119);
    session.set_bool("endfield_event_a", "obtained_featured", false);
    let draw = session.draw("endfield_event_a", "draw").unwrap();
    assert_eq!(draw.guarantee.as_deref(), Some("limited_120"));
    assert_eq!(entry(&draw, "ef_featured"), Rational::one());

    session.set_count("endfield_event_a", "progress_pulls", 239);
    let draw = session.draw("endfield_event_a", "draw").unwrap();
    assert!(draw.branches.iter().all(|branch| {
        branch
            .events
            .iter()
            .any(|event| event.name == "reward_token" && event.args == vec![FiredArg::Count(1)])
    }));

    let mut genshin = compiled.session();
    let draw = genshin.draw("genshin_event_a", "draw").unwrap();
    assert_eq!(rarity(&draw, 5), Rational::new(3, 500));
    assert_eq!(rarity(&draw, 4), Rational::new(51, 1000));
    assert_eq!(rarity(&draw, 3), Rational::new(943, 1000));

    genshin.set_count("genshin_event_a", "pity5", 73);
    let draw = genshin.draw("genshin_event_a", "draw").unwrap();
    assert_eq!(rarity(&draw, 5), Rational::new(33, 500));

    genshin.set_count("genshin_event_a", "pity5", 0);
    genshin.set_count("genshin_event_a", "pity4", 9);
    let draw = genshin.draw("genshin_event_a", "draw").unwrap();
    assert_eq!(draw.guarantee.as_deref(), Some("four_hard"));
    assert_eq!(rarity(&draw, 5), Rational::new(3, 500));
    assert_eq!(rarity(&draw, 4), Rational::new(497, 500));
    assert_eq!(rarity(&draw, 3), Rational::zero());

    genshin.set_count("genshin_event_a", "pity5", 89);
    genshin.set_count("genshin_event_a", "pity4", 9);
    let draw = genshin.draw("genshin_event_a", "draw").unwrap();
    assert_eq!(draw.guarantee.as_deref(), Some("five_hard"));
    assert_eq!(rarity(&draw, 5), Rational::one());

    let mut weapon = compiled.session();
    for _ in 0..9 {
        weapon.apply("weapon_sample_a", "ten", "wp_four").unwrap();
    }
    let draw = weapon.draw("weapon_sample_a", "ten").unwrap();
    assert_eq!(rarity(&draw, 6), Rational::new(1, 10));
    assert_eq!(rarity(&draw, 5), Rational::new(9, 10));

    let mut weapon = compiled.session();
    weapon.apply("weapon_sample_a", "ten", "wp_four").unwrap();
    weapon.apply("weapon_sample_a", "ten", "wp_four").unwrap();
    weapon.apply("weapon_sample_a", "ten", "wp_five").unwrap();
    for _ in 0..6 {
        weapon.apply("weapon_sample_a", "ten", "wp_four").unwrap();
    }
    let draw = weapon.draw("weapon_sample_a", "ten").unwrap();
    assert_eq!(draw.guarantee, None);
    assert_eq!(rarity(&draw, 6), Rational::new(1, 100));

    let mut rng = gacha_engine::Rng::new(7);
    let sampled = compiled
        .session()
        .sample("genshin_event_a", "draw", &mut rng)
        .unwrap();
    assert!(!sampled.probability.is_zero());
}

#[test]
fn rejection_corpus_matches_codes_and_positions() {
    let readme = fs::read_to_string(root().join("rejection-tests/README.md")).unwrap();
    let mut cases = Vec::new();
    for line in readme.lines() {
        let parts: Vec<&str> = line.split('`').collect();
        if parts.len() < 6 || !parts[3].starts_with('E') {
            continue;
        }
        let dir = parts[1].trim_end_matches('/');
        let code = parts[3];
        let mut loc = parts[5].split(':');
        let file = loc.next().unwrap();
        let line_no: u32 = loc.next().unwrap().parse().unwrap();
        let column: u32 = loc.next().unwrap().parse().unwrap();
        cases.push((
            dir.to_string(),
            code.to_string(),
            file.to_string(),
            line_no,
            column,
        ));
    }
    assert_eq!(cases.len(), 58, "拒绝测试数量");
    for (dir, code, file, line, column) in cases {
        let owned = files_in(root().join("rejection-tests").join(&dir));
        let sources: Vec<Source<'_>> = owned
            .iter()
            .map(|(name, text)| Source { name, text })
            .collect();
        let error = match compile(&sources) {
            Err(error) => error,
            Ok(_) => panic!("{dir} 应当被拒绝"),
        };
        assert_eq!(error.code, code, "{dir}");
        assert_eq!(error.file, file, "{dir}");
        assert_eq!(
            (error.line, error.column),
            (line, column),
            "{dir} got {}:{}:{}",
            error.file,
            error.line,
            error.column
        );
    }
}

fn rarity(draw: &gacha_engine::Draw, rarity: u128) -> Rational {
    draw.branches
        .iter()
        .filter(|branch| branch.rarity == rarity)
        .try_fold(Rational::zero(), |acc, branch| {
            acc.checked_add(branch.probability)
        })
        .unwrap()
}

fn entry(draw: &gacha_engine::Draw, name: &str) -> Rational {
    draw.branches
        .iter()
        .filter(|branch| branch.entry == name)
        .try_fold(Rational::zero(), |acc, branch| {
            acc.checked_add(branch.probability)
        })
        .unwrap()
}
