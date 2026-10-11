use super::*;
use gacha_sdk::{AnalysisRequest, Engine, SessionSnapshot};

pub(super) fn sdk_engine(args: &[String]) -> Result<Engine, Box<dyn std::error::Error>> {
    let path = required(args, 0, "需要卡池清单或独立规则包文件")?;
    Ok(if let Some(rules) = option_value(args, "--rules") {
        Engine::load_with_rules(path, rules)?
    } else {
        Engine::load(path)?
    })
}

pub(crate) fn command_study(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    let engine = sdk_engine(args)?;
    let request: AnalysisRequest = serde_json::from_str(&fs::read_to_string(required(
        args,
        1,
        "study 需要 REQUEST.json",
    )?)?)?;
    let trials = option_value(args, "--trials")
        .map(str::parse)
        .transpose()?
        .unwrap_or(10_000);
    let seed = option_value(args, "--seed")
        .map(str::parse)
        .transpose()?
        .unwrap_or(20261011);
    let report = engine.study(&request, trials, seed)?;
    if let Some(path) = option_value(args, "--csv") {
        report.write_csv(path)?;
    }
    if let Some(path) = option_value(args, "--output") {
        report.write_json(path)?;
    }
    println!("{}", serde_json::to_string(&report)?);
    Ok(())
}

pub(crate) fn command_trace(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    let engine = sdk_engine(args)?;
    let banner = required(args, 1, "trace 需要 BANNER ACTION")?;
    let action = required(args, 2, "trace 需要 BANNER ACTION")?;
    let seed = option_value(args, "--seed")
        .map(str::parse)
        .transpose()?
        .unwrap_or(42);
    let count = option_value(args, "--count")
        .map(str::parse::<u32>)
        .transpose()?
        .unwrap_or(1);
    let mut session = if let Some(path) = option_value(args, "--session") {
        let snapshot: SessionSnapshot = serde_json::from_str(&fs::read_to_string(path)?)?;
        engine.restore_session(snapshot)?
    } else {
        let mut snapshot = engine.session(seed).snapshot();
        snapshot.state = load_optional_state(option_value(args, "--state"))?;
        engine.restore_session(snapshot)?
    };
    let mut records = Vec::new();
    for _ in 0..count {
        records.extend(session.pull_trace(banner, action)?);
    }
    if let Some(path) = option_value(args, "--save-session") {
        fs::write(path, serde_json::to_vec(&session.snapshot())?)?;
    }
    println!(
        "{}",
        serde_json::to_string(&json!({"results":records,"session":session.snapshot()}))?
    );
    Ok(())
}

pub(crate) fn command_bundle(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    let engine = sdk_engine(args)?;
    engine.save_bundle(required(args, 1, "bundle 需要 OUTPUT.json")?)?;
    println!(
        "{}",
        serde_json::to_string(
            &json!({"fingerprint":engine.fingerprint(),"preparation":engine.preparation()})
        )?
    );
    Ok(())
}
