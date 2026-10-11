use super::*;

pub(super) fn compile_condition(
    pack: &GamePack,
    game: &CompiledGame,
    raw: &ConditionDef,
) -> Result<Condition> {
    Ok(match raw {
        ConditionDef::Always => Condition::Always,
        ConditionDef::All { conditions } => Condition::All(
            conditions
                .iter()
                .map(|condition| compile_condition(pack, game, condition))
                .collect::<Result<Vec<_>>>()?,
        ),
        ConditionDef::Any { conditions } => Condition::Any(
            conditions
                .iter()
                .map(|condition| compile_condition(pack, game, condition))
                .collect::<Result<Vec<_>>>()?,
        ),
        ConditionDef::Not { condition } => {
            Condition::Not(Box::new(compile_condition(pack, game, condition)?))
        }
        ConditionDef::StateCounterGte { state, value } => Condition::StateCounterGte {
            state: require_counter_state(pack, &game.state_lookup, state, "condition")?,
            value: *value,
        },
        ConditionDef::StateCounterEq { state, value } => Condition::StateCounterEq {
            state: require_counter_state(pack, &game.state_lookup, state, "condition")?,
            value: *value,
        },
        ConditionDef::StateBoolEq { state, value } => Condition::StateBoolEq {
            state: require_boolean_state(pack, &game.state_lookup, state, "condition")?,
            value: *value,
        },
        ConditionDef::StateModuloEq {
            state,
            modulus,
            value,
        } => {
            if *modulus == 0 || *value >= *modulus {
                return Err(EngineError::new(
                    "state_modulo_eq requires modulus > 0 and value < modulus",
                ));
            }
            Condition::StateModuloEq {
                state: require_counter_state(pack, &game.state_lookup, state, "condition")?,
                modulus: *modulus,
                value: *value,
            }
        }
        ConditionDef::OutcomeRarityEq { rarity } => Condition::OutcomeRarityEq(*rarity),
        ConditionDef::OutcomeRarityGte { rarity } => Condition::OutcomeRarityGte(*rarity),
        ConditionDef::OutcomeItemInPool { pool } => {
            Condition::OutcomeItemInPool(*game.pool_lookup.get(pool).ok_or_else(|| {
                EngineError::new(format!("condition references unknown pool {pool}"))
            })?)
        }
    })
}

pub(super) fn compile_update(
    pack: &GamePack,
    game: &CompiledGame,
    raw: &schema::StateUpdateDef,
) -> Result<StateUpdate> {
    Ok(match raw {
        schema::StateUpdateDef::Increment { state, by } => StateUpdate::Increment {
            state: require_counter_state(pack, &game.state_lookup, state, "increment")?,
            by: *by,
        },
        schema::StateUpdateDef::Reset { state } => {
            let id = *game.state_lookup.get(state).ok_or_else(|| {
                EngineError::new(format!("reset references unknown state {state}"))
            })?;
            StateUpdate::Reset { state: id }
        }
        schema::StateUpdateDef::SetCounter { state, value } => {
            let id = require_counter_state(pack, &game.state_lookup, state, "set_counter")?;
            if let Some(max) = pack.state[id.0].max
                && *value > max
            {
                return Err(EngineError::new(format!(
                    "set_counter for state {} sets {} above max {}",
                    state, value, max
                )));
            }
            StateUpdate::SetCounter {
                state: id,
                value: *value,
            }
        }
        schema::StateUpdateDef::SetBool { state, value } => StateUpdate::SetBool {
            state: require_boolean_state(pack, &game.state_lookup, state, "set_bool")?,
            value: *value,
        },
    })
}
