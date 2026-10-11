use super::*;

pub(super) fn evaluate_distribution(
    game: &CompiledGame,
    banner: &CompiledBanner,
    distribution: &CompiledDistribution,
    state: &StateStore,
    context: &ScopeContext,
) -> Result<Vec<(u8, Rational)>> {
    let mut values = vec![None; distribution.entries.len()];
    let mut fixed_sum = Rational::ZERO;
    let mut remainder_index = None;
    let mut shared = Vec::new();

    for (index, entry) in distribution.entries.iter().enumerate() {
        match &entry.probability {
            ProbabilityExpr::Remainder => {
                if remainder_index.replace(index).is_some() {
                    return Err(EngineError::new(format!(
                        "distribution {} contains multiple remainder entries",
                        distribution.id
                    )));
                }
            }
            ProbabilityExpr::ShareOfRemainder { weight } => {
                if *weight == 0 {
                    return Err(EngineError::new(format!(
                        "distribution {} contains zero remainder weight",
                        distribution.id
                    )));
                }
                shared.push((index, *weight));
            }
            expr => {
                let value = evaluate_probability_expr(game, banner, state, context, expr)?;
                if value > Rational::ONE {
                    return Err(EngineError::new(format!(
                        "distribution {} produced probability {} > 1",
                        distribution.id, value
                    )));
                }
                fixed_sum = fixed_sum.checked_add(value)?;
                if fixed_sum > Rational::ONE {
                    return Err(EngineError::new(format!(
                        "distribution {} fixed probabilities exceed 1",
                        distribution.id
                    )));
                }
                values[index] = Some(value);
            }
        }
    }

    let remainder = Rational::ONE.checked_sub(fixed_sum)?;
    if let Some(index) = remainder_index {
        if !shared.is_empty() {
            return Err(EngineError::new(format!(
                "distribution {} mixes remainder and share_of_remainder",
                distribution.id
            )));
        }
        values[index] = Some(remainder);
    } else if !shared.is_empty() {
        let total_weight = shared.iter().try_fold(0_u64, |acc, (_, weight)| {
            acc.checked_add(*weight)
                .ok_or_else(|| EngineError::new("remainder weights overflow"))
        })?;
        for (index, weight) in shared {
            let share = Rational::new(weight as u128, total_weight as u128)?;
            values[index] = Some(remainder.checked_mul(share)?);
        }
    } else if remainder != Rational::ZERO {
        return Err(EngineError::new(format!(
            "distribution {} sums to {}, but has no remainder rule",
            distribution.id, fixed_sum
        )));
    }

    let mut result = Vec::with_capacity(distribution.entries.len());
    for (entry, value) in distribution.entries.iter().zip(values) {
        result.push((
            entry.rarity,
            value.ok_or_else(|| EngineError::new("distribution entry was not resolved"))?,
        ));
    }
    Ok(result)
}

pub(crate) fn evaluate_probability_expr(
    game: &CompiledGame,
    banner: &CompiledBanner,
    state: &StateStore,
    context: &ScopeContext,
    expr: &ProbabilityExpr,
) -> Result<Rational> {
    match expr {
        ProbabilityExpr::Constant(value) => Ok(*value),
        ProbabilityExpr::LinearAfter {
            state: state_id,
            base,
            after,
            increment,
            cap,
        } => {
            let current = read_counter(game, banner, state, context, *state_id)?;
            let steps = if current >= *after {
                u128::from(current) - u128::from(*after) + 1
            } else {
                0
            };
            let increase = increment.checked_mul(Rational::new(steps, 1)?)?;
            Ok(base.checked_add(increase)?.min(*cap))
        }
        ProbabilityExpr::Table {
            state: state_id,
            values,
            default,
        } => {
            let current = read_counter(game, banner, state, context, *state_id)?;
            values
                .get(&current)
                .copied()
                .or(*default)
                .ok_or_else(|| EngineError::new(format!("table has no value for state {current}")))
        }
        ProbabilityExpr::Remainder | ProbabilityExpr::ShareOfRemainder { .. } => Err(
            EngineError::new("remainder expressions must be resolved by the distribution"),
        ),
    }
}

pub(super) fn apply_min_rarity(
    distribution: Vec<(u8, Rational)>,
    minimum: u8,
    strategy: MinRarityStrategy,
) -> Result<Vec<(u8, Rational)>> {
    match strategy {
        MinRarityStrategy::ConditionalCurrent => {
            let allowed_sum = sum_probabilities(
                distribution
                    .iter()
                    .filter(|(rarity, _)| *rarity >= minimum)
                    .map(|(_, p)| *p),
            )?;
            if allowed_sum.is_zero() {
                return Err(EngineError::new(format!(
                    "cannot condition distribution on rarity >= {minimum}: zero mass"
                )));
            }
            let mut result = Vec::new();
            for (rarity, probability) in distribution {
                if rarity >= minimum {
                    result.push((rarity, probability.checked_div(allowed_sum)?));
                }
            }
            Ok(result)
        }
        MinRarityStrategy::PreserveHigherFillFloor => {
            let mut higher_sum = Rational::ZERO;
            let mut floor_present = false;
            let mut result = Vec::new();
            for (rarity, probability) in distribution {
                if rarity > minimum {
                    higher_sum = higher_sum.checked_add(probability)?;
                    result.push((rarity, probability));
                } else if rarity == minimum {
                    floor_present = true;
                }
            }
            let floor_probability = Rational::ONE.checked_sub(higher_sum)?;
            if floor_present || !floor_probability.is_zero() {
                result.push((minimum, floor_probability));
            }
            result.sort_by_key(|(rarity, _)| *rarity);
            Ok(result)
        }
    }
}

pub(super) fn sum_probabilities<I>(values: I) -> Result<Rational>
where
    I: IntoIterator<Item = Rational>,
{
    let mut total = Rational::ZERO;
    for value in values {
        total = total.checked_add(value)?;
    }
    Ok(total)
}
