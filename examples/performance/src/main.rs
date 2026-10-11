use gacha_analysis::{FirstHitQuery, analyze_first_hit};
use gacha_core::{CompiledGame, ScopeContext, SplitMix64, StateStore, sample_action};
use gacha_pack::compile_pack;
use gacha_sdk::{AnalysisRequest, TargetSpec};
use serde_json::json;
use std::{error::Error, fs, time::Instant};

fn median(values: &mut [f64]) -> f64 {
    values.sort_by(f64::total_cmp);
    values[values.len() / 2]
}

fn main() -> Result<(), Box<dyn Error>> {
    let mut rows = Vec::new();
    for (slug, target, actions, action_id) in [
        ("si", "endfield.character.祀", 120, "single_pull"),
        ("yuzhongshen", "endfield.weapon.玉中身", 8, "requisition"),
    ] {
        let path = format!("endfield/banner.preview-{slug}.json");
        let game = compile_pack(&gacha_games::pack(&path, None)?)?;
        let engine = gacha_games::load(&path)?;
        let banner = &game.banners[0].id;
        let size = engine.action_size(banner, action_id)?;
        let request = AnalysisRequest::first_hit(
            banner,
            action_id,
            TargetSpec::Item { id: target.into() },
            actions,
        );
        let query = FirstHitQuery {
            banner: banner.clone(),
            action: action_id.into(),
            target: request.target.clone(),
            draws: actions * size,
            quantiles: request.quantiles.clone(),
        };
        let mut ref_analysis = Vec::new();
        let mut fast_analysis = Vec::new();
        let mut ref_samples = Vec::new();
        let mut fast_samples = Vec::new();
        for _ in 0..3 {
            let start = Instant::now();
            let baseline = analyze_first_hit(
                &game,
                &StateStore::default(),
                &ScopeContext::default(),
                &query,
            )?;
            ref_analysis.push(start.elapsed().as_secs_f64() * 1000.0);
            let start = Instant::now();
            let actual = engine.analyze(&request)?;
            fast_analysis.push(start.elapsed().as_secs_f64() * 1000.0);
            for (a, b) in baseline
                .distribution
                .iter()
                .zip(&actual.draw_distribution.distribution)
            {
                assert!((a.cumulative_probability - b.cumulative_probability).abs() < 1e-12);
            }
            let start = Instant::now();
            let counts = reference_simulate(&game, banner, action_id, target, actions, size, 1000)?;
            ref_samples.push(start.elapsed().as_secs_f64() * 1000.0);
            let start = Instant::now();
            let sample = engine.simulate(&request, 1000, 20261011)?;
            fast_samples.push(start.elapsed().as_secs_f64() * 1000.0);
            assert_eq!(
                counts,
                sample
                    .distribution
                    .iter()
                    .map(|p| p.count)
                    .collect::<Vec<_>>()
            );
        }
        let r = median(&mut ref_analysis);
        let f = median(&mut fast_analysis);
        let rs = median(&mut ref_samples);
        let fs = median(&mut fast_samples);
        rows.push(json!({"case":slug,"trials":1000,"repeats":3,"analysis_reference_ms":r,
            "analysis_prepared_ms":f,"analysis_speedup":r/f,
            "sampling_reference_ms":rs,"sampling_prepared_ms":fs,"sampling_speedup":rs/fs,
            "cdf_max_tolerance":1e-12,"seeded_histograms_identical":true,"preparation":engine.preparation()}));
        println!("{slug} 分析 {r:.1}→{f:.1}ms，抽样 {rs:.1}→{fs:.1}ms");
    }
    fs::create_dir_all("examples/performance/reports")?;
    fs::write(
        "examples/performance/reports/performance.json",
        serde_json::to_vec_pretty(&rows)?,
    )?;
    Ok(())
}

fn reference_simulate(
    game: &CompiledGame,
    banner: &str,
    action: &str,
    target: &str,
    actions: u32,
    size: u32,
    trials: u32,
) -> gacha_core::Result<Vec<u32>> {
    let target = game.item_id(target).unwrap();
    let mut counts = vec![0; (actions * size) as usize];
    let mut rng = SplitMix64::new(20261011);
    for _ in 0..trials {
        let mut state = StateStore::default();
        for batch in 0..actions {
            let branches = sample_action(
                game,
                banner,
                action,
                &mut state,
                &ScopeContext::default(),
                &mut rng,
            )?;
            if let Some(position) = branches.iter().position(|b| b.outcome.item == target) {
                counts[(batch * size) as usize + position] += 1;
                break;
            }
        }
    }
    Ok(counts)
}
