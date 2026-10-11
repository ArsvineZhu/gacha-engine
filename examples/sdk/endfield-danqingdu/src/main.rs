use gacha_sdk::{AnalysisRequest, TargetSpec};
use serde_json::{Value, json};
use std::{error::Error, fs, path::Path, time::Instant};

fn main() -> Result<(), Box<dyn Error>> {
    let trials = std::env::args()
        .nth(1)
        .map(|s| s.parse::<u32>())
        .transpose()?
        .unwrap_or(10_000);
    fs::create_dir_all("output")?;
    let cases: Vec<Value> = serde_json::from_str(&fs::read_to_string("cases.json")?)?;
    let mut summary = Vec::new();
    for case in cases {
        let name = case["case"].as_str().ok_or("缺少场景名")?;
        let engine = gacha_games::load_with_profile(
            case["banner"].as_str().ok_or("缺少卡池")?,
            case["profile"].as_str(),
        )?;
        engine.save_bundle(Path::new("output").join(format!("{name}.bundle.json")))?;
        let action = engine
            .actions()
            .into_iter()
            .find(|a| a.action == case["action"])
            .ok_or("未找到动作")?;
        let request = AnalysisRequest::first_hit(
            &action.banner,
            &action.action,
            TargetSpec::Item {
                id: case["target"].as_str().ok_or("缺少目标")?.into(),
            },
            u32::try_from(case["actions"].as_u64().ok_or("缺少次数")?)?,
        );
        let start = Instant::now();
        let report = engine.study(&request, trials, 20261011)?;
        let elapsed_ms = start.elapsed().as_secs_f64() * 1000.0;
        if !report.comparison.six_sigma_passed || report.simulation.not_hit_trials != 0 {
            return Err(format!("{name} 分析与模拟未通过核对").into());
        }
        report.write_json(Path::new("output").join(format!("{name}.json")))?;
        report.write_csv(Path::new("output").join(format!("{name}.csv")))?;
        let mut session = engine.session(42);
        let trace = session.pull_trace(&action.banner, &action.action)?;
        fs::write(
            Path::new("output").join(format!("{name}.trace.json")),
            serde_json::to_vec(&trace)?,
        )?;
        let checkpoint = session.snapshot();
        let mut restored = engine.restore_session(checkpoint)?;
        let next = session.pull(&action.banner, &action.action)?;
        let replay = restored.pull(&action.banner, &action.action)?;
        if serde_json::to_value(next)? != serde_json::to_value(replay)? {
            return Err("会话续跑不一致".into());
        }
        fs::write(
            Path::new("output").join(format!("{name}.request.json")),
            serde_json::to_vec(&request)?,
        )?;
        summary.push(json!({"case":name,"label":case["label"],"model":case["model"],
            "fingerprint":engine.fingerprint(),"trials":trials,"elapsed_ms":elapsed_ms,
            "expected_actions":report.analysis.expected_actions_if_hit,
            "quantiles":report.analysis.action_quantiles,
            "comparison":report.comparison,"preparation":engine.preparation(),"session_replay":true}));
    }
    fs::write("output/summary.json", serde_json::to_vec_pretty(&summary)?)?;
    println!("{}", serde_json::to_string(&summary)?);
    Ok(())
}
