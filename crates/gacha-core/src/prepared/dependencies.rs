use crate::*;

/// 记录所有结果条件的真值；同类条目触发完全相同的状态更新与事件。
pub(super) fn outcome_classes(game: &CompiledGame, action: &CompiledAction) -> Vec<Vec<bool>> {
    fn collect<'a>(condition: &'a Condition, leaves: &mut Vec<&'a Condition>) {
        match condition {
            Condition::All(values) | Condition::Any(values) => {
                for value in values {
                    collect(value, leaves);
                }
            }
            Condition::Not(value) => collect(value, leaves),
            Condition::OutcomeRarityEq(_)
            | Condition::OutcomeRarityGte(_)
            | Condition::OutcomeItemInPool(_) => leaves.push(condition),
            _ => {}
        }
    }
    let mut leaves = Vec::new();
    for rule in &action.transitions {
        collect(&rule.when, &mut leaves);
    }
    game.items
        .iter()
        .enumerate()
        .map(|(index, item)| {
            leaves
                .iter()
                .map(|c| match c {
                    Condition::OutcomeRarityEq(r) => item.rarity == *r,
                    Condition::OutcomeRarityGte(r) => item.rarity >= *r,
                    Condition::OutcomeItemInPool(p) => {
                        game.pools[p.0].items.contains(&ItemId(index))
                    }
                    _ => unreachable!(),
                })
                .collect()
        })
        .collect()
}
