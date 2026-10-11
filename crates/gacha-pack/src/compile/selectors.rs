use super::*;

pub(super) fn compile_selectors(
    pack: &GamePack,
    game: &CompiledGame,
) -> Result<Vec<CompiledSelector>> {
    let pool_lookup = &game.pool_lookup;
    let selector_lookup = &game.selector_lookup;
    let mut result = Vec::with_capacity(pack.selectors.len());
    for raw in &pack.selectors {
        let expr = match &raw.selector {
            schema::SelectorExpr::Conditional {
                when,
                then_selector,
                else_selector,
            } => {
                validate_pre_draw_condition(when)?;
                let resolve = |name: &String| {
                    selector_lookup.get(name).copied().ok_or_else(|| {
                        EngineError::new(format!(
                            "selector {} references unknown selector {name}",
                            raw.id
                        ))
                    })
                };
                SelectorExpr::Conditional {
                    when: compile_condition(pack, game, when)?,
                    then_selector: resolve(then_selector)?,
                    else_selector: resolve(else_selector)?,
                }
            }
            schema::SelectorExpr::Pool { pool } => {
                let pool = *pool_lookup.get(pool).ok_or_else(|| {
                    EngineError::new(format!(
                        "selector {} references unknown pool {pool}",
                        raw.id
                    ))
                })?;
                SelectorExpr::Pool(pool)
            }
            schema::SelectorExpr::Weighted { branches } => {
                if branches.is_empty() {
                    return Err(EngineError::new(format!(
                        "selector {} has no weighted branches",
                        raw.id
                    )));
                }
                let mut compiled = Vec::with_capacity(branches.len());
                for branch in branches {
                    let weight = Rational::from_str(&branch.weight).map_err(|error| {
                        EngineError::new(format!(
                            "selector {} has invalid weight {}: {error}",
                            raw.id, branch.weight
                        ))
                    })?;
                    if weight.is_zero() {
                        return Err(EngineError::new(format!(
                            "selector {} has zero branch weight",
                            raw.id
                        )));
                    }
                    let selector = *selector_lookup.get(&branch.selector).ok_or_else(|| {
                        EngineError::new(format!(
                            "selector {} references unknown selector {}",
                            raw.id, branch.selector
                        ))
                    })?;
                    compiled.push(WeightedSelectorBranch { weight, selector });
                }
                SelectorExpr::Weighted(compiled)
            }
        };
        result.push(CompiledSelector {
            id: raw.id.clone(),
            expr,
        });
    }
    Ok(result)
}
