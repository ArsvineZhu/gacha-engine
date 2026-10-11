use super::*;

pub(super) fn enumerate_selector(
    game: &CompiledGame,
    banner: &CompiledBanner,
    state: &StateStore,
    context: &ScopeContext,
    root: SelectorId,
) -> Result<Vec<(ItemId, Rational)>> {
    let mut visiting = BTreeSet::new();
    enumerate_selector_inner(
        game,
        banner,
        state,
        context,
        root,
        &mut visiting,
        &mut BTreeMap::new(),
    )
}

pub(super) fn enumerate_selector_inner(
    game: &CompiledGame,
    banner: &CompiledBanner,
    state: &StateStore,
    context: &ScopeContext,
    selector_id: SelectorId,
    visiting: &mut BTreeSet<SelectorId>,
    cache: &mut BTreeMap<SelectorId, Vec<(ItemId, Rational)>>,
) -> Result<Vec<(ItemId, Rational)>> {
    if let Some(result) = cache.get(&selector_id) {
        return Ok(result.clone());
    }
    if !visiting.insert(selector_id) {
        return Err(EngineError::new(format!(
            "selector cycle detected at {}",
            game.selectors[selector_id.0].id
        )));
    }

    let result = match &game.selectors[selector_id.0].expr {
        SelectorExpr::Conditional {
            when,
            then_selector,
            else_selector,
        } => {
            let selected = if evaluate_condition(game, banner, state, context, None, when)? {
                *then_selector
            } else {
                *else_selector
            };
            enumerate_selector_inner(game, banner, state, context, selected, visiting, cache)?
        }
        SelectorExpr::Pool(pool_id) => {
            let pool = &game.pools[pool_id.0];
            if pool.items.is_empty() {
                return Err(EngineError::new(format!("pool {} is empty", pool.id)));
            }
            let probability = Rational::new(1, pool.items.len() as u128)?;
            pool.items
                .iter()
                .copied()
                .map(|item| (item, probability))
                .collect::<Vec<_>>()
        }
        SelectorExpr::Weighted(branches) => {
            if branches.is_empty() {
                return Err(EngineError::new(format!(
                    "selector {} has no branches",
                    game.selectors[selector_id.0].id
                )));
            }
            let total_weight = sum_probabilities(branches.iter().map(|branch| branch.weight))?;
            if total_weight.is_zero() {
                return Err(EngineError::new("weighted selector has zero total weight"));
            }
            let mut merged = BTreeMap::<ItemId, Rational>::new();
            for branch in branches {
                let branch_probability = branch.weight.checked_div(total_weight)?;
                let nested = enumerate_selector_inner(
                    game,
                    banner,
                    state,
                    context,
                    branch.selector,
                    visiting,
                    cache,
                )?;
                for (item, probability) in nested {
                    let weighted = probability.checked_mul(branch_probability)?;
                    let current = merged.get(&item).copied().unwrap_or(Rational::ZERO);
                    merged.insert(item, current.checked_add(weighted)?);
                }
            }
            merged.into_iter().collect::<Vec<_>>()
        }
    };

    visiting.remove(&selector_id);
    cache.insert(selector_id, result.clone());
    Ok(result)
}
