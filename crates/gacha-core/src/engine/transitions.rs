use super::*;

pub(super) fn apply_transitions(
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

pub(crate) fn evaluate_condition(
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

pub(super) fn apply_update(
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
