use super::*;

/// 计算有限抽数范围内目标首次命中的分布。
///
/// 单步转移来自模拟共用的精确有理数规则图，
/// 用 f64 汇总多步状态质量，控制长区间计算成本。
pub fn analyze_first_hit(
    game: &CompiledGame,
    initial_state: &StateStore,
    context: &ScopeContext,
    query: &FirstHitQuery,
) -> Result<FirstHitAnalysis> {
    analyze_nth_hit(game, initial_state, context, query, 1)
}

/// 计算第 `copies` 次目标命中所在抽数的分布。
/// 仅计抽取结果；库存层处理的奖励事件不计入命中。
pub fn analyze_nth_hit(
    game: &CompiledGame,
    initial_state: &StateStore,
    context: &ScopeContext,
    query: &FirstHitQuery,
    copies: u32,
) -> Result<FirstHitAnalysis> {
    analyze_impl(game, initial_state, context, query, copies, None)
}

pub fn analyze_nth_hit_prepared(
    runtime: &gacha_core::PreparedRuntime,
    initial_state: &StateStore,
    context: &ScopeContext,
    query: &FirstHitQuery,
    copies: u32,
) -> Result<FirstHitAnalysis> {
    analyze_impl(
        runtime.game(),
        initial_state,
        context,
        query,
        copies,
        Some(runtime),
    )
}

fn analyze_impl(
    game: &CompiledGame,
    initial_state: &StateStore,
    context: &ScopeContext,
    query: &FirstHitQuery,
    copies: u32,
    prepared: Option<&gacha_core::PreparedRuntime>,
) -> Result<FirstHitAnalysis> {
    if copies == 0 {
        return Err(EngineError::new("copies must be positive"));
    }
    let (_, banner) = game
        .banner(&query.banner)
        .ok_or_else(|| EngineError::new(format!("unknown banner: {}", query.banner)))?;
    let action = banner
        .action(&query.action)
        .ok_or_else(|| EngineError::new(format!("unknown action: {}", query.action)))?;
    if !query.draws.is_multiple_of(action.draws) {
        return Err(EngineError::new(
            "analysis horizon must contain complete actions",
        ));
    }
    let target = compile_target(game, &query.target)?;
    validate_quantiles(&query.quantiles)?;

    let mut active = BTreeMap::<(StateStore, u32), f64>::new();
    active.insert((initial_state.clone(), 0), 1.0);

    let mut distribution = Vec::with_capacity(query.draws as usize);
    let mut cumulative_hit = 0.0_f64;
    let mut weighted_hit_draws = 0.0_f64;
    let mut peak_surviving_states = active.len();

    for draw in 1..=query.draws {
        let mut draw_context = context.clone();
        draw_context.pull = format!("{}:{draw}", context.pull);

        let mut next = BTreeMap::<(StateStore, u32), f64>::new();
        let mut first_hit = 0.0_f64;

        for ((mut state, hits), mass) in active {
            if mass == 0.0 {
                continue;
            }
            if (draw - 1).is_multiple_of(action.draws) {
                gacha_core::reset_action_state(game, &mut state);
            }
            let branches = if let Some(runtime) = prepared {
                runtime.enumerate_grouped_step(
                    &query.banner,
                    &query.action,
                    &state,
                    &draw_context,
                    |outcome| target.matches(game, outcome),
                )?
            } else {
                enumerate_step(game, &query.banner, &query.action, &state, &draw_context)?
            };
            for branch in branches {
                let branch_mass = mass * branch.probability.to_f64();
                let next_hits = hits + u32::from(target.matches(game, branch.outcome));
                if next_hits >= copies {
                    first_hit += branch_mass;
                } else {
                    *next.entry((branch.next_state, next_hits)).or_insert(0.0) += branch_mass;
                }
            }
        }

        if !first_hit.is_finite() || first_hit < 0.0 {
            return Err(EngineError::new(
                "analysis produced an invalid first-hit probability",
            ));
        }

        cumulative_hit += first_hit;
        weighted_hit_draws += first_hit * f64::from(draw);
        let survival = next.values().copied().sum::<f64>();
        if !survival.is_finite() || survival < 0.0 {
            return Err(EngineError::new(
                "analysis produced an invalid survival probability",
            ));
        }

        peak_surviving_states = peak_surviving_states.max(next.len());
        distribution.push(FirstHitPoint {
            draw,
            first_hit_probability: first_hit,
            cumulative_probability: cumulative_hit.clamp(0.0, 1.0),
            survival_probability: survival.clamp(0.0, 1.0),
            surviving_states: next.len(),
        });
        active = next;
    }

    let probability_not_hit = active.values().copied().sum::<f64>().clamp(0.0, 1.0);
    let probability_within = (1.0 - probability_not_hit).clamp(0.0, 1.0);
    if !probability_within.is_finite() {
        return Err(EngineError::new(
            "analysis produced a non-finite probability",
        ));
    }

    let expected_draw_if_hit = if probability_within > 0.0 {
        Some(weighted_hit_draws / probability_within)
    } else {
        None
    };
    let expected_draws_capped_at_horizon =
        weighted_hit_draws + f64::from(query.draws) * probability_not_hit;

    const QUANTILE_EPSILON: f64 = 1e-12;
    let quantiles = query
        .quantiles
        .iter()
        .copied()
        .map(|quantile| {
            let draw = distribution
                .iter()
                .find(|point| point.cumulative_probability + QUANTILE_EPSILON >= quantile)
                .map(|point| point.draw);
            QuantileResult { quantile, draw }
        })
        .collect();

    Ok(FirstHitAnalysis {
        horizon: query.draws,
        probability_within,
        probability_not_hit,
        expected_draw_if_hit,
        expected_draws_capped_at_horizon,
        peak_surviving_states,
        final_surviving_states: active.len(),
        distribution,
        quantiles,
    })
}
