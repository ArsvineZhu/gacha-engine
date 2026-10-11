use super::*;

pub(crate) fn validate_selector_cycles(selectors: &[CompiledSelector]) -> Result<()> {
    fn visit(
        id: SelectorId,
        selectors: &[CompiledSelector],
        temporary: &mut BTreeSet<SelectorId>,
        permanent: &mut BTreeMap<SelectorId, usize>,
        depth: usize,
    ) -> Result<usize> {
        if depth > MAX_SELECTOR_DEPTH {
            return Err(EngineError::new(format!(
                "selector graph exceeds maximum depth {} at {}",
                MAX_SELECTOR_DEPTH, selectors[id.0].id
            )));
        }
        if let Some(height) = permanent.get(&id) {
            return Ok(*height);
        }
        if !temporary.insert(id) {
            return Err(EngineError::new(format!(
                "selector cycle detected at {}",
                selectors[id.0].id
            )));
        }
        let mut height = 0;
        match &selectors[id.0].expr {
            SelectorExpr::Weighted(branches) => {
                for branch in branches {
                    height = height.max(
                        1 + visit(branch.selector, selectors, temporary, permanent, depth + 1)?,
                    );
                }
            }
            SelectorExpr::Conditional {
                then_selector,
                else_selector,
                ..
            } => {
                height =
                    1 + visit(*then_selector, selectors, temporary, permanent, depth + 1)?.max(
                        visit(*else_selector, selectors, temporary, permanent, depth + 1)?,
                    );
            }
            SelectorExpr::Pool(_) => {}
        }
        if height > MAX_SELECTOR_DEPTH {
            return Err(EngineError::new(format!(
                "selector graph exceeds maximum depth {MAX_SELECTOR_DEPTH} at {}",
                selectors[id.0].id
            )));
        }
        temporary.remove(&id);
        permanent.insert(id, height);
        Ok(height)
    }

    let mut temporary = BTreeSet::new();
    let mut permanent = BTreeMap::new();
    for index in 0..selectors.len() {
        visit(
            SelectorId(index),
            selectors,
            &mut temporary,
            &mut permanent,
            0,
        )?;
    }
    Ok(())
}

pub(crate) fn validate_action_selector_rarities(game: &CompiledGame) -> Result<()> {
    fn collect_items(
        game: &CompiledGame,
        selector: SelectorId,
        output: &mut BTreeSet<ItemId>,
        visited: &mut BTreeSet<SelectorId>,
    ) {
        if !visited.insert(selector) {
            return;
        }
        match &game.selectors[selector.0].expr {
            SelectorExpr::Conditional {
                then_selector,
                else_selector,
                ..
            } => {
                collect_items(game, *then_selector, output, visited);
                collect_items(game, *else_selector, output, visited);
            }
            SelectorExpr::Pool(pool) => {
                output.extend(game.pools[pool.0].items.iter().copied());
            }
            SelectorExpr::Weighted(branches) => {
                for branch in branches {
                    collect_items(game, branch.selector, output, visited);
                }
            }
        }
    }

    for banner in &game.banners {
        for action in &banner.actions {
            let distribution = &game.distributions[action.distribution.0];
            for rarity in distribution
                .entries
                .iter()
                .map(|entry| entry.rarity)
                .chain(action.selectors.keys().copied())
                .collect::<BTreeSet<_>>()
            {
                let selector = action.selectors.get(&rarity).ok_or_else(|| {
                    EngineError::new(format!(
                        "banner {} action {} has distribution rarity {} but no selector",
                        banner.id, action.id, rarity
                    ))
                })?;
                let mut items = BTreeSet::new();
                collect_items(game, *selector, &mut items, &mut BTreeSet::new());
                if items.is_empty() {
                    return Err(EngineError::new(format!(
                        "selector {} resolves to no items",
                        game.selectors[selector.0].id
                    )));
                }
                for item in items {
                    if game.items[item.0].rarity != rarity {
                        return Err(EngineError::new(format!(
                            "selector {} used for rarity {} can produce item {} of rarity {}",
                            game.selectors[selector.0].id,
                            rarity,
                            game.items[item.0].id,
                            game.items[item.0].rarity
                        )));
                    }
                }
            }
        }
    }
    Ok(())
}
