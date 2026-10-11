use super::*;
use gacha_sdk::{AnalysisRequest, Engine, TargetSpec};

fn analyze_query(
    engine: &Engine,
    query: &FirstHitQuery,
    copies: u32,
    state: StateStore,
) -> Result<gacha_sdk::AnalysisReport, Box<dyn std::error::Error>> {
    let size = engine.action_size(&query.banner, &query.action)?;
    if !query.draws.is_multiple_of(size) {
        return Err("分析区间必须包含完整动作".into());
    }
    let mut request = AnalysisRequest::first_hit(
        &query.banner,
        &query.action,
        query.target.clone(),
        query.draws / size,
    );
    request.copies = copies;
    request.quantiles = query.quantiles.clone();
    request.state = state;
    Ok(engine.analyze(&request)?)
}

pub(crate) fn command_analyze(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    let engine = sdk_engine(args)?;
    let query: FirstHitQuery = serde_json::from_str(&fs::read_to_string(required(
        args,
        1,
        "analyze 需要 QUERY.json",
    )?)?)?;
    let state = load_optional_state(option_value(args, "--state"))?;
    let copies = option_value(args, "--copies")
        .map(str::parse)
        .transpose()?
        .unwrap_or(1);
    let report = analyze_query(&engine, &query, copies, state)?;
    println!(
        "{}",
        serde_json::to_string_pretty(
            &json!({"query":query,"copies":copies,"result":report.draw_distribution,
        "action_distribution":report.action_distribution,"action_quantiles":report.action_quantiles,
        "expected_actions_if_hit":report.expected_actions_if_hit,"preparation":report.preparation,
        "note":"单步概率为精确有理数，多步质量用 f64 汇总；draws 为底层抽数"})
        )?
    );
    Ok(())
}

pub(crate) fn command_probability(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    let engine = sdk_engine(args)?;
    let target = required(args, 3, "probability 需要 TARGET DRAWS")?;
    let draws = required(args, 4, "probability 需要 DRAWS")?.parse()?;
    let query = FirstHitQuery {
        banner: required(args, 1, "需要 BANNER")?.into(),
        action: required(args, 2, "需要 ACTION")?.into(),
        target: TargetSpec::Item { id: target.into() },
        draws,
        quantiles: vec![],
    };
    let report = analyze_query(
        &engine,
        &query,
        1,
        load_optional_state(option_value(args, "--state"))?,
    )?;
    println!(
        "{}",
        serde_json::to_string_pretty(&json!({"target":target,"within_draws":draws,
        "probability":report.draw_distribution.probability_within,"surviving_states":report.draw_distribution.final_surviving_states}))?
    );
    Ok(())
}
