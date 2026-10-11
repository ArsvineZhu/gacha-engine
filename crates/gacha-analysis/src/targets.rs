use super::*;
pub fn target_mask(game: &CompiledGame, target: &TargetSpec) -> Result<Vec<bool>> {
    let target = compile_target(game, target)?;
    Ok(game
        .items
        .iter()
        .enumerate()
        .map(|(i, item)| {
            target.matches(
                game,
                Outcome {
                    item: ItemId(i),
                    rarity: item.rarity,
                },
            )
        })
        .collect())
}
pub(super) enum CompiledTarget {
    Items(BTreeSet<ItemId>),
    RarityAtLeast(u8),
    Tag(String),
}

impl CompiledTarget {
    pub(super) fn matches(&self, game: &CompiledGame, outcome: Outcome) -> bool {
        match self {
            Self::Items(items) => items.contains(&outcome.item),
            Self::RarityAtLeast(rarity) => outcome.rarity >= *rarity,
            Self::Tag(tag) => game
                .item(outcome.item)
                .tags
                .iter()
                .any(|item_tag| item_tag == tag),
        }
    }
}

pub(super) fn compile_target(game: &CompiledGame, target: &TargetSpec) -> Result<CompiledTarget> {
    match target {
        TargetSpec::Item { id } => {
            let item = game
                .item_id(id)
                .ok_or_else(|| EngineError::new(format!("unknown target item: {id}")))?;
            Ok(CompiledTarget::Items(BTreeSet::from([item])))
        }
        TargetSpec::AnyItem { ids } => {
            if ids.is_empty() {
                return Err(EngineError::new(
                    "any_item target must contain at least one item",
                ));
            }
            let mut items = BTreeSet::new();
            for id in ids {
                let item = game
                    .item_id(id)
                    .ok_or_else(|| EngineError::new(format!("unknown target item: {id}")))?;
                items.insert(item);
            }
            Ok(CompiledTarget::Items(items))
        }
        TargetSpec::RarityAtLeast { rarity } => {
            if *rarity == 0 {
                return Err(EngineError::new(
                    "rarity_at_least target must be greater than zero",
                ));
            }
            Ok(CompiledTarget::RarityAtLeast(*rarity))
        }
        TargetSpec::Tag { tag } => {
            if tag.trim().is_empty() {
                return Err(EngineError::new("tag target must not be empty"));
            }
            if !game
                .items
                .iter()
                .any(|item| item.tags.iter().any(|item_tag| item_tag == tag))
            {
                return Err(EngineError::new(format!("unknown target tag: {tag}")));
            }
            Ok(CompiledTarget::Tag(tag.clone()))
        }
    }
}

pub(super) fn validate_quantiles(quantiles: &[f64]) -> Result<()> {
    for quantile in quantiles {
        if !quantile.is_finite() || *quantile <= 0.0 || *quantile > 1.0 {
            return Err(EngineError::new(format!(
                "quantile must be finite and in (0, 1], got {quantile}"
            )));
        }
    }
    Ok(())
}
