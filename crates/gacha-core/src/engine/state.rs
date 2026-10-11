use super::*;

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

pub(super) fn read_counter(
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

pub(super) fn read_bool(
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

pub(super) fn validate_state_value(def: &CompiledStateDef, value: &StateValue) -> Result<()> {
    match (def.kind, value) {
        (StateKind::Counter, StateValue::Counter(value)) => {
            if let Some(max) = def.max
                && *value > max
            {
                return Err(EngineError::new(format!(
                    "state {} value {} exceeds max {}",
                    def.id, value, max
                )));
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
