use super::*;
/// 单条目概率查询的便捷接口。
pub fn probability_of_item_within(
    game: &CompiledGame,
    banner: &str,
    action: &str,
    initial_state: &StateStore,
    context: &ScopeContext,
    target_item: &str,
    draws: u32,
) -> Result<ProbabilityResult> {
    let query = FirstHitQuery {
        banner: banner.to_string(),
        action: action.to_string(),
        target: TargetSpec::Item {
            id: target_item.to_string(),
        },
        draws,
        quantiles: Vec::new(),
    };
    let result = analyze_first_hit(game, initial_state, context, &query)?;
    Ok(ProbabilityResult {
        probability: result.probability_within,
        surviving_states: result.final_surviving_states,
    })
}
