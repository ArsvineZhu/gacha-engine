use crate::model::*;
use crate::{EngineError, Rational, Result, SplitMix64};
use std::collections::{BTreeMap, BTreeSet};

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

    let matching = matching_guarantees(game, banner, action, &base_state, context)?;
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
        let (next_state, events) =
            apply_transitions(game, banner, action, &base_state, context, outcome)?;
        return Ok(vec![Branch {
            probability: Rational::ONE,
            outcome,
            guarantees: guarantee_ids,
            events,
            next_state,
        }]);
    }

    let mut rarity_distribution = evaluate_distribution(
        game,
        banner,
        &game.distributions[action.distribution.0],
        &base_state,
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

    let mut branches = Vec::new();
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
        let item_distribution = enumerate_selector(game, selector_id)?;
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
            let (next_state, events) =
                apply_transitions(game, banner, action, &base_state, context, outcome)?;
            branches.push(Branch {
                probability,
                outcome,
                guarantees: guarantee_ids.clone(),
                events,
                next_state,
            });
        }
    }

    let total = sum_probabilities(branches.iter().map(|branch| branch.probability))?;
    if total != Rational::ONE {
        return Err(EngineError::new(format!(
            "transition probabilities sum to {total}, expected 1"
        )));
    }
    Ok(branches)
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

pub fn materialize_state(
    game: &CompiledGame,
    banner: &CompiledBanner,
    state: &mut StateStore,
    context: &ScopeContext,
) -> Result<()> {
    for def in &game.states {
        let key = state_slot_key(game, banner, def, context);
        match state.slots.get(&key) {
            Some(value) => validate_state_value(def, value)?,
            None => {
                state.slots.insert(key, def.initial.clone());
            }
        }
    }
    Ok(())
}

pub fn state_slot_key(
    game: &CompiledGame,
    banner: &CompiledBanner,
    def: &CompiledStateDef,
    context: &ScopeContext,
) -> String {
    let scope = match def.scope {
        StateScope::Account => "account".to_string(),
        StateScope::Game => format!("game:{}", game.id),
        StateScope::PityGroup => format!("pity:{}", banner.pity_group),
        StateScope::ProgressGroup => format!("progress:{}", banner.progress_group),
        StateScope::Banner => format!("banner:{}", banner.id),
        StateScope::Session => format!("session:{}", context.session),
        StateScope::Batch => format!("batch:{}", context.batch),
        StateScope::Pull => format!("pull:{}", context.pull),
    };
    format!("{}::{scope}", def.id)
}

fn matching_guarantees<'a>(
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

fn evaluate_distribution(
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

fn evaluate_probability_expr(
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
                current - *after + 1
            } else {
                0
            };
            let increase = increment.checked_mul(Rational::from_u64(steps))?;
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

fn apply_min_rarity(
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

fn enumerate_selector(game: &CompiledGame, root: SelectorId) -> Result<Vec<(ItemId, Rational)>> {
    let mut visiting = BTreeSet::new();
    enumerate_selector_inner(game, root, &mut visiting)
}

fn enumerate_selector_inner(
    game: &CompiledGame,
    selector_id: SelectorId,
    visiting: &mut BTreeSet<SelectorId>,
) -> Result<Vec<(ItemId, Rational)>> {
    if !visiting.insert(selector_id) {
        return Err(EngineError::new(format!(
            "selector cycle detected at {}",
            game.selectors[selector_id.0].id
        )));
    }

    let result = match &game.selectors[selector_id.0].expr {
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
                let nested = enumerate_selector_inner(game, branch.selector, visiting)?;
                for (item, probability) in nested {
                    let weighted = probability.checked_mul(branch_probability)?;
                    let current = merged.get(&item).copied().unwrap_or(Rational::ZERO);
                    merged.insert(item, current.checked_add(weighted)?);
                }
            }
            merged.into_iter().collect()
        }
    };

    visiting.remove(&selector_id);
    Ok(result)
}

fn apply_transitions(
    game: &CompiledGame,
    banner: &CompiledBanner,
    action: &CompiledAction,
    pre_state: &StateStore,
    context: &ScopeContext,
    outcome: Outcome,
) -> Result<(StateStore, Vec<Event>)> {
    let mut next_state = pre_state.clone();
    let mut events = Vec::new();
    for rule in &action.transitions {
        if evaluate_condition(game, banner, pre_state, context, Some(outcome), &rule.when)? {
            for update in &rule.updates {
                apply_update(game, banner, &mut next_state, context, update)?;
            }
            for template in &rule.events {
                events.push(Event {
                    kind: template.kind.clone(),
                    key: template.key.clone(),
                    amount: template.amount,
                });
            }
        }
    }
    Ok((next_state, events))
}

fn evaluate_condition(
    game: &CompiledGame,
    banner: &CompiledBanner,
    state: &StateStore,
    context: &ScopeContext,
    outcome: Option<Outcome>,
    condition: &Condition,
) -> Result<bool> {
    match condition {
        Condition::Always => Ok(true),
        Condition::All(conditions) => {
            for condition in conditions {
                if !evaluate_condition(game, banner, state, context, outcome, condition)? {
                    return Ok(false);
                }
            }
            Ok(true)
        }
        Condition::Any(conditions) => {
            for condition in conditions {
                if evaluate_condition(game, banner, state, context, outcome, condition)? {
                    return Ok(true);
                }
            }
            Ok(false)
        }
        Condition::Not(condition) => Ok(!evaluate_condition(
            game, banner, state, context, outcome, condition,
        )?),
        Condition::StateCounterGte { state: id, value } => {
            Ok(read_counter(game, banner, state, context, *id)? >= *value)
        }
        Condition::StateCounterEq { state: id, value } => {
            Ok(read_counter(game, banner, state, context, *id)? == *value)
        }
        Condition::StateBoolEq { state: id, value } => {
            Ok(read_bool(game, banner, state, context, *id)? == *value)
        }
        Condition::StateModuloEq {
            state: id,
            modulus,
            value,
        } => {
            if *modulus == 0 {
                return Err(EngineError::new("state_modulo_eq modulus cannot be zero"));
            }
            Ok(read_counter(game, banner, state, context, *id)? % *modulus == *value)
        }
        Condition::OutcomeRarityEq(rarity) => Ok(outcome.is_some_and(|o| o.rarity == *rarity)),
        Condition::OutcomeRarityGte(rarity) => Ok(outcome.is_some_and(|o| o.rarity >= *rarity)),
        Condition::OutcomeItemInPool(pool) => Ok(outcome.is_some_and(|o| {
            game.pools[pool.0].items.binary_search(&o.item).is_ok()
                || game.pools[pool.0].items.contains(&o.item)
        })),
    }
}

fn apply_update(
    game: &CompiledGame,
    banner: &CompiledBanner,
    state: &mut StateStore,
    context: &ScopeContext,
    update: &StateUpdate,
) -> Result<()> {
    match update {
        StateUpdate::Increment { state: id, by } => {
            let def = &game.states[id.0];
            let key = state_slot_key(game, banner, def, context);
            let value = state
                .slots
                .get_mut(&key)
                .ok_or_else(|| EngineError::new(format!("state slot not materialized: {key}")))?;
            match value {
                StateValue::Counter(current) => {
                    *current = current
                        .checked_add(*by)
                        .ok_or_else(|| EngineError::new(format!("state {} overflow", def.id)))?;
                    validate_state_value(def, value)
                }
                StateValue::Boolean(_) => Err(EngineError::new(format!(
                    "cannot increment boolean state {}",
                    def.id
                ))),
            }
        }
        StateUpdate::Reset { state: id } => {
            let def = &game.states[id.0];
            let key = state_slot_key(game, banner, def, context);
            state.slots.insert(key, def.initial.clone());
            Ok(())
        }
        StateUpdate::SetCounter { state: id, value } => {
            let def = &game.states[id.0];
            if def.kind != StateKind::Counter {
                return Err(EngineError::new(format!(
                    "cannot set counter value on boolean state {}",
                    def.id
                )));
            }
            let new_value = StateValue::Counter(*value);
            validate_state_value(def, &new_value)?;
            let key = state_slot_key(game, banner, def, context);
            state.slots.insert(key, new_value);
            Ok(())
        }
        StateUpdate::SetBool { state: id, value } => {
            let def = &game.states[id.0];
            if def.kind != StateKind::Boolean {
                return Err(EngineError::new(format!(
                    "cannot set boolean value on counter state {}",
                    def.id
                )));
            }
            let key = state_slot_key(game, banner, def, context);
            state.slots.insert(key, StateValue::Boolean(*value));
            Ok(())
        }
    }
}

fn read_counter(
    game: &CompiledGame,
    banner: &CompiledBanner,
    state: &StateStore,
    context: &ScopeContext,
    id: StateId,
) -> Result<u64> {
    let def = &game.states[id.0];
    let key = state_slot_key(game, banner, def, context);
    match state.slots.get(&key) {
        Some(StateValue::Counter(value)) => Ok(*value),
        Some(StateValue::Boolean(_)) => Err(EngineError::new(format!(
            "state {} is boolean, expected counter",
            def.id
        ))),
        None => Err(EngineError::new(format!(
            "state slot not materialized: {key}"
        ))),
    }
}

fn read_bool(
    game: &CompiledGame,
    banner: &CompiledBanner,
    state: &StateStore,
    context: &ScopeContext,
    id: StateId,
) -> Result<bool> {
    let def = &game.states[id.0];
    let key = state_slot_key(game, banner, def, context);
    match state.slots.get(&key) {
        Some(StateValue::Boolean(value)) => Ok(*value),
        Some(StateValue::Counter(_)) => Err(EngineError::new(format!(
            "state {} is counter, expected boolean",
            def.id
        ))),
        None => Err(EngineError::new(format!(
            "state slot not materialized: {key}"
        ))),
    }
}

fn validate_state_value(def: &CompiledStateDef, value: &StateValue) -> Result<()> {
    match (def.kind, value) {
        (StateKind::Counter, StateValue::Counter(value)) => {
            if let Some(max) = def.max {
                if *value > max {
                    return Err(EngineError::new(format!(
                        "state {} value {} exceeds max {}",
                        def.id, value, max
                    )));
                }
            }
            Ok(())
        }
        (StateKind::Boolean, StateValue::Boolean(_)) => Ok(()),
        (StateKind::Counter, StateValue::Boolean(_)) => Err(EngineError::new(format!(
            "state {} expects a counter",
            def.id
        ))),
        (StateKind::Boolean, StateValue::Counter(_)) => Err(EngineError::new(format!(
            "state {} expects a boolean",
            def.id
        ))),
    }
}

fn sum_probabilities<I>(values: I) -> Result<Rational>
where
    I: IntoIterator<Item = Rational>,
{
    let mut total = Rational::ZERO;
    for value in values {
        total = total.checked_add(value)?;
    }
    Ok(total)
}

fn sample_branch_index(branches: &[Branch], rng: &mut SplitMix64) -> Result<usize> {
    const SCALE: u128 = 1_u128 << 53;
    let target = rng.next_53_bits() as u128;
    let mut cumulative = Rational::ZERO;
    for (index, branch) in branches.iter().enumerate() {
        cumulative = cumulative.checked_add(branch.probability)?;
        let left = target.checked_mul(cumulative.denominator());
        let right = cumulative.numerator().checked_mul(SCALE);
        let selected = match (left, right) {
            (Some(left), Some(right)) => left < right,
            _ => (target as f64 / SCALE as f64) < cumulative.to_f64(),
        };
        if selected {
            return Ok(index);
        }
    }
    Ok(branches.len() - 1)
}
