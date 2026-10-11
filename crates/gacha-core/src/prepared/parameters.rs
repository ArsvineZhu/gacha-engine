use super::*;
use crate::engine::{evaluate_condition, evaluate_probability_expr};

pub(super) struct Parameter {
    expr: ProbabilityExpr,
    dense: Option<(StateId, Vec<Option<Rational>>)>,
}

pub(super) struct DecisionInputs {
    parameters: Vec<Parameter>,
    guards: Vec<Condition>,
    force_items: Vec<bool>,
}

impl DecisionInputs {
    pub fn compile(game: &CompiledGame, banner: &CompiledBanner, action: &CompiledAction) -> Self {
        let context = ScopeContext::default();
        let parameters = game.distributions[action.distribution.0]
            .entries
            .iter()
            .filter_map(|entry| {
                let id = match &entry.probability {
                    ProbabilityExpr::LinearAfter { state, .. }
                    | ProbabilityExpr::Table { state, .. } => *state,
                    _ => return None,
                };
                let dense = game.states[id.0].max.filter(|max| *max <= 4096).map(|max| {
                    let key = state_slot_key(game, banner, &game.states[id.0], &context);
                    let values = (0..=max)
                        .map(|current| {
                            let state = StateStore {
                                slots: BTreeMap::from([(
                                    key.clone(),
                                    StateValue::Counter(current),
                                )]),
                            };
                            evaluate_probability_expr(
                                game,
                                banner,
                                &state,
                                &context,
                                &entry.probability,
                            )
                            .ok()
                        })
                        .collect();
                    (id, values)
                });
                Some(Parameter {
                    expr: entry.probability.clone(),
                    dense,
                })
            })
            .collect();
        let mut guards: Vec<_> = action.guarantees.iter().map(|g| g.when.clone()).collect();
        fn selectors(
            game: &CompiledGame,
            id: SelectorId,
            seen: &mut std::collections::BTreeSet<SelectorId>,
            guards: &mut Vec<Condition>,
        ) {
            if !seen.insert(id) {
                return;
            }
            match &game.selectors[id.0].expr {
                SelectorExpr::Conditional {
                    when,
                    then_selector,
                    else_selector,
                } => {
                    guards.push(when.clone());
                    selectors(game, *then_selector, seen, guards);
                    selectors(game, *else_selector, seen, guards);
                }
                SelectorExpr::Weighted(branches) => {
                    for branch in branches {
                        selectors(game, branch.selector, seen, guards);
                    }
                }
                SelectorExpr::Pool(_) => {}
            }
        }
        let mut seen = std::collections::BTreeSet::new();
        for id in action.selectors.values() {
            selectors(game, *id, &mut seen, &mut guards);
        }
        let force_items = action
            .guarantees
            .iter()
            .map(|g| matches!(g.effect, GuaranteeEffect::ForceItem(_)))
            .collect();
        Self {
            parameters,
            guards,
            force_items,
        }
    }

    pub fn values(
        &self,
        game: &CompiledGame,
        banner: &CompiledBanner,
        state: &StateStore,
        context: &ScopeContext,
    ) -> Result<(Vec<Rational>, Vec<bool>)> {
        let guarantee_values = self.guards[..self.force_items.len()]
            .iter()
            .map(|c| evaluate_condition(game, banner, state, context, None, c))
            .collect::<Result<Vec<_>>>()?;
        if guarantee_values
            .iter()
            .position(|matched| *matched)
            .is_some_and(|i| self.force_items[i])
        {
            return Ok((Vec::new(), guarantee_values));
        }
        let parameters = self
            .parameters
            .iter()
            .map(|parameter| {
                if let Some((id, table)) = &parameter.dense {
                    let key = state_slot_key(game, banner, &game.states[id.0], context);
                    if let StateValue::Counter(current) = state.slots[&key]
                        && let Some(Some(value)) =
                            usize::try_from(current).ok().and_then(|i| table.get(i))
                    {
                        return Ok(*value);
                    }
                }
                evaluate_probability_expr(game, banner, state, context, &parameter.expr)
            })
            .collect::<Result<_>>()?;
        let mut guards = guarantee_values;
        guards.extend(
            self.guards[self.force_items.len()..]
                .iter()
                .map(|c| evaluate_condition(game, banner, state, context, None, c))
                .collect::<Result<Vec<_>>>()?,
        );
        Ok((parameters, guards))
    }

    pub fn dense_tables(&self) -> usize {
        self.parameters.iter().filter(|p| p.dense.is_some()).count()
    }
}
