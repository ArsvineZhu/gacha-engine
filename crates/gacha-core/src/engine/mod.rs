use crate::model::*;
use crate::{EngineError, Rational, Result, SplitMix64};
use std::collections::{BTreeMap, BTreeSet};

mod distribution;
use distribution::*;
mod selectors;
use selectors::*;
mod transitions;
use transitions::*;
mod sampling;
use sampling::*;
mod state;
pub use state::{materialize_state, state_slot_key};
use state::{read_bool, read_counter, validate_state_value};

pub fn enumerate_transitions(
    game: &CompiledGame,
    banner_id: &str,
    action_id: &str,
    state: &StateStore,
    context: &ScopeContext,
) -> Result<Vec<Branch>> {
    let (_, banner) = game
        .banner(banner_id)
        .ok_or_else(|| EngineError::new(format!("unknown banner: {banner_id}")))?;
    let action = banner.action(action_id).ok_or_else(|| {
        EngineError::new(format!("unknown action {action_id} for banner {banner_id}"))
    })?;

    let mut base_state = state.clone();
    materialize_state(game, banner, &mut base_state, context)?;

    let plan = resolve_outcomes(game, banner, action, &base_state, context, None)?;
    plan.choices
        .iter()
        .map(|choice| finish_outcome(game, banner, action, &base_state, context, &plan, choice))
        .collect()
}

pub(crate) type SelectorTables = BTreeMap<SelectorId, Vec<(ItemId, Rational)>>;

pub(crate) fn resolve_outcomes(
    game: &CompiledGame,
    banner: &CompiledBanner,
    action: &CompiledAction,
    state: &StateStore,
    context: &ScopeContext,
    tables: Option<&SelectorTables>,
) -> Result<OutcomePlan> {
    let matching = matching_guarantees(game, banner, action, state, context)?;
    let guarantee_ids = matching.iter().map(|g| g.id.clone()).collect::<Vec<_>>();

    if let Some(item) = matching.first().and_then(|g| match &g.effect {
        GuaranteeEffect::ForceItem(item) => Some(*item),
        _ => None,
    }) {
        let compiled_item = game
            .items
            .get(item.0)
            .ok_or_else(|| EngineError::new("forced item index is invalid"))?;
        let outcome = Outcome {
            item,
            rarity: compiled_item.rarity,
        };
        return make_plan(
            vec![OutcomeChoice {
                probability: Rational::ONE,
                outcome,
            }],
            guarantee_ids,
        );
    }

    let mut rarity_distribution = evaluate_distribution(
        game,
        banner,
        &game.distributions[action.distribution.0],
        state,
        context,
    )?;

    if let Some(guarantee) = matching.first() {
        match &guarantee.effect {
            GuaranteeEffect::ForceItem(_) => unreachable!("force_item returned above"),
            GuaranteeEffect::ForceRarity(rarity) => {
                rarity_distribution.clear();
                rarity_distribution.push((*rarity, Rational::ONE));
            }
            GuaranteeEffect::MinRarity { rarity, strategy } => {
                rarity_distribution = apply_min_rarity(rarity_distribution, *rarity, *strategy)?;
            }
        }
    }

    let mut choices = Vec::new();
    for (rarity, rarity_probability) in rarity_distribution {
        if rarity_probability.is_zero() {
            continue;
        }
        let selector_id = action.selectors.get(&rarity).copied().ok_or_else(|| {
            EngineError::new(format!(
                "action {} has no selector for rarity {}",
                action.id, rarity
            ))
        })?;
        let item_distribution = if let Some(table) = tables.and_then(|t| t.get(&selector_id)) {
            table.clone()
        } else {
            enumerate_selector(game, banner, state, context, selector_id)?
        };
        for (item_id, item_probability) in item_distribution {
            if item_probability.is_zero() {
                continue;
            }
            let item = &game.items[item_id.0];
            if item.rarity != rarity {
                return Err(EngineError::new(format!(
                    "selector {} produced item {} with rarity {}, expected {}",
                    game.selectors[selector_id.0].id, item.id, item.rarity, rarity
                )));
            }
            let probability = rarity_probability.checked_mul(item_probability)?;
            let outcome = Outcome {
                item: item_id,
                rarity,
            };
            choices.push(OutcomeChoice {
                probability,
                outcome,
            });
        }
    }

    let total = sum_probabilities(choices.iter().map(|choice| choice.probability))?;
    if total != Rational::ONE {
        return Err(EngineError::new(format!(
            "transition probabilities sum to {total}, expected 1"
        )));
    }
    make_plan(choices, guarantee_ids)
}

pub fn sample_transition(
    game: &CompiledGame,
    banner_id: &str,
    action_id: &str,
    state: &mut StateStore,
    context: &ScopeContext,
    rng: &mut SplitMix64,
) -> Result<Branch> {
    let branches = enumerate_transitions(game, banner_id, action_id, state, context)?;
    if branches.is_empty() {
        return Err(EngineError::new(
            "transition enumeration produced no branches",
        ));
    }
    let index = sample_branch_index(&branches, rng)?;
    let selected = branches[index].clone();
    *state = selected.next_state.clone();
    Ok(selected)
}

pub(super) fn matching_guarantees<'a>(
    game: &CompiledGame,
    banner: &CompiledBanner,
    action: &'a CompiledAction,
    state: &StateStore,
    context: &ScopeContext,
) -> Result<Vec<&'a CompiledGuarantee>> {
    let mut result = Vec::new();
    for guarantee in &action.guarantees {
        if evaluate_condition(game, banner, state, context, None, &guarantee.when)? {
            result.push(guarantee);
        }
    }
    Ok(result)
}

pub(crate) fn finish_outcome(
    game: &CompiledGame,
    banner: &CompiledBanner,
    action: &CompiledAction,
    state: &StateStore,
    context: &ScopeContext,
    plan: &OutcomePlan,
    choice: &OutcomeChoice,
) -> Result<Branch> {
    let (next_state, events) =
        apply_transitions(game, banner, action, state, context, choice.outcome)?;
    Ok(Branch {
        probability: choice.probability,
        outcome: choice.outcome,
        guarantees: plan.guarantees.clone(),
        events,
        next_state,
    })
}

pub(crate) fn static_selector_tables(game: &CompiledGame) -> Result<SelectorTables> {
    fn is_static(
        game: &CompiledGame,
        id: SelectorId,
        memo: &mut BTreeMap<SelectorId, bool>,
    ) -> bool {
        if let Some(value) = memo.get(&id) {
            return *value;
        }
        let value = match &game.selectors[id.0].expr {
            SelectorExpr::Conditional { .. } => false,
            SelectorExpr::Pool(_) => true,
            SelectorExpr::Weighted(branches) => {
                branches.iter().all(|b| is_static(game, b.selector, memo))
            }
        };
        memo.insert(id, value);
        value
    }
    let mut tables = SelectorTables::new();
    let mut memo = BTreeMap::new();
    if let Some(banner) = game.banners.first() {
        for i in 0..game.selectors.len() {
            let id = SelectorId(i);
            if is_static(game, id, &mut memo) {
                tables.insert(
                    id,
                    enumerate_selector(
                        game,
                        banner,
                        &StateStore::default(),
                        &ScopeContext::default(),
                        id,
                    )?,
                );
            }
        }
    }
    Ok(tables)
}

pub(crate) use distribution::evaluate_probability_expr;
pub(crate) use transitions::evaluate_condition;

fn make_plan(choices: Vec<OutcomeChoice>, guarantees: Vec<String>) -> Result<OutcomePlan> {
    Ok(OutcomePlan {
        choices,
        guarantees,
        sampling: Vec::new(),
    })
}
