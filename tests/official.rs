use std::fs;
use std::path::PathBuf;

use gacha_engine::{Rational, Source, compile};

fn read(path: PathBuf) -> (String, String) {
    let name = path.file_name().unwrap().to_string_lossy().into_owned();
    (name, fs::read_to_string(path).unwrap())
}

fn compile_game(game: &str) -> gacha_engine::Compiled {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("official/games")
        .join(game);
    let mut paths = fs::read_dir(&dir)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.is_file())
        .collect::<Vec<_>>();
    paths.sort_by_key(|path| {
        let rank = match path.extension().and_then(|ext| ext.to_str()) {
            Some("gmr") => 0,
            Some("gml") => 1,
            Some("gmb") => 2,
            _ => 3,
        };
        (rank, path.file_name().unwrap().to_owned())
    });
    let owned = paths.into_iter().map(read).collect::<Vec<_>>();
    let sources = owned
        .iter()
        .map(|(name, text)| Source { name, text })
        .collect::<Vec<_>>();
    compile(&sources).unwrap_or_else(|error| {
        panic!(
            "{game} {}:{}:{} {} 期望 {} 实际 {}",
            error.file, error.line, error.column, error.code, error.expected, error.actual
        )
    })
}

fn rarity_of(draw: &gacha_engine::Draw, rarity: u128) -> Rational {
    draw.branches
        .iter()
        .filter(|branch| branch.rarity == rarity)
        .try_fold(Rational::zero(), |acc, branch| {
            acc.checked_add(branch.probability)
        })
        .unwrap()
}

fn find_banner<'a>(compiled: &'a gacha_engine::Compiled, needle: &str) -> &'a str {
    compiled
        .banners()
        .find(|name| name.contains(needle))
        .unwrap_or_else(|| panic!("没有卡池 {needle}"))
}

#[test]
fn official_games_compile() {
    for game in [
        "arknights",
        "endfield",
        "genshin",
        "nte",
        "star-rail",
        "wuthering-waves",
        "zzz",
    ] {
        let _ = compile_game(game);
    }
}

#[test]
fn official_base_rates_match_the_old_rules() {
    let genshin = compile_game("genshin");
    let banner = find_banner(&genshin, "genshin_reference_character");
    let draw = genshin.session().draw(banner, "single_pull").unwrap();
    assert_eq!(rarity_of(&draw, 5), Rational::new(3, 500));
    assert_eq!(rarity_of(&draw, 4), Rational::new(51, 1000));

    let star = compile_game("star-rail");
    let banner = find_banner(&star, "star_rail_reference_character");
    let draw = star.session().draw(banner, "single_pull").unwrap();
    assert_eq!(rarity_of(&draw, 5), Rational::new(3, 500));
    assert_eq!(rarity_of(&draw, 4), Rational::new(51, 1000));

    let zzz = compile_game("zzz");
    let banner = find_banner(&zzz, "zzz_reference_character");
    let draw = zzz.session().draw(banner, "single_pull").unwrap();
    assert_eq!(rarity_of(&draw, 5), Rational::new(3, 500));
    assert_eq!(rarity_of(&draw, 4), Rational::new(47, 500));

    let waves = compile_game("wuthering-waves");
    let banner = find_banner(&waves, "wuthering_waves_reference_character");
    let draw = waves.session().draw(banner, "single_pull").unwrap();
    assert_eq!(rarity_of(&draw, 5), Rational::new(1, 125));
    assert_eq!(rarity_of(&draw, 4), Rational::new(3, 50));

    let arknights = compile_game("arknights");
    let banner = find_banner(&arknights, "arknights_reference_kernel");
    let draw = arknights.session().draw(banner, "single_pull").unwrap();
    assert_eq!(rarity_of(&draw, 6), Rational::new(1, 50));
    assert_eq!(rarity_of(&draw, 5), Rational::new(2, 25));
    assert_eq!(rarity_of(&draw, 4), Rational::new(1, 2));
    assert_eq!(rarity_of(&draw, 3), Rational::new(2, 5));

    let endfield = compile_game("endfield");
    let banner = find_banner(&endfield, "chartered_fixed_five__single_pull");
    let mut session = endfield.session();
    session.set_count(banner, "six_star_pity", 65).unwrap();
    let draw = session.draw(banner, "single_pull").unwrap();
    assert_eq!(rarity_of(&draw, 6), Rational::new(29, 500));
    assert_eq!(rarity_of(&draw, 5), Rational::new(2, 25));

    let nte = compile_game("nte");
    let banner = find_banner(&nte, "nte_reference_character");
    let draw = nte.session().draw(banner, "single_pull").unwrap();
    assert_eq!(rarity_of(&draw, 5), Rational::new(99, 10000));
    assert_eq!(rarity_of(&draw, 4), Rational::new(1, 10));
}
