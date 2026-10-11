use gacha_core::*;
use gacha_schema as schema;
use gacha_schema::{ConditionDef, GamePack};
use std::collections::{BTreeMap, BTreeSet};
use std::str::FromStr;
const MAX_STATES: usize = 1024;
const MAX_ITEMS: usize = 100_000;
const MAX_POOLS: usize = 10_000;
const MAX_DISTRIBUTIONS: usize = 4096;
const MAX_SELECTORS: usize = 4096;
const MAX_BANNERS: usize = 10_000;
const MAX_SELECTOR_DEPTH: usize = 128;
mod selectors;
pub(crate) use selectors::*;
mod distributions;
pub(crate) use distributions::*;

pub(crate) fn validate_pre_draw_condition(condition: &ConditionDef) -> Result<()> {
    match condition {
        ConditionDef::OutcomeRarityEq { .. }
        | ConditionDef::OutcomeRarityGte { .. }
        | ConditionDef::OutcomeItemInPool { .. } => Err(EngineError::new(
            "guarantee conditions cannot reference an outcome before the draw is resolved",
        )),
        ConditionDef::All { conditions } | ConditionDef::Any { conditions } => {
            for condition in conditions {
                validate_pre_draw_condition(condition)?;
            }
            Ok(())
        }
        ConditionDef::Not { condition } => validate_pre_draw_condition(condition),
        _ => Ok(()),
    }
}

pub(crate) fn require_counter_state(
    pack: &GamePack,
    state_lookup: &BTreeMap<String, StateId>,
    state: &str,
    context: &str,
) -> Result<StateId> {
    let id = *state_lookup
        .get(state)
        .ok_or_else(|| EngineError::new(format!("{context} references unknown state {state}")))?;
    if pack.state[id.0].kind != schema::StateKind::Counter {
        return Err(EngineError::new(format!(
            "{context} requires counter state {state}"
        )));
    }
    Ok(id)
}

pub(crate) fn require_boolean_state(
    pack: &GamePack,
    state_lookup: &BTreeMap<String, StateId>,
    state: &str,
    context: &str,
) -> Result<StateId> {
    let id = *state_lookup
        .get(state)
        .ok_or_else(|| EngineError::new(format!("{context} references unknown state {state}")))?;
    if pack.state[id.0].kind != schema::StateKind::Boolean {
        return Err(EngineError::new(format!(
            "{context} requires boolean state {state}"
        )));
    }
    Ok(id)
}

pub(crate) fn parse_probability(value: &str, context: &str) -> Result<Rational> {
    let probability = Rational::from_str(value)
        .map_err(|error| EngineError::new(format!("invalid {context} {value}: {error}")))?;
    ensure_unit_interval(probability, context)?;
    Ok(probability)
}

pub(crate) fn ensure_unit_interval(value: Rational, context: &str) -> Result<()> {
    if value > Rational::ONE {
        return Err(EngineError::new(format!("{context} must be in [0, 1]")));
    }
    Ok(())
}

pub(crate) fn validate_evidence_refs(
    owner: &str,
    evidence: &[String],
    assumptions: &[String],
    source_ids: &BTreeSet<String>,
    assumption_ids: &BTreeSet<String>,
) -> Result<()> {
    for id in evidence {
        if !source_ids.contains(id) {
            return Err(EngineError::new(format!(
                "{owner} references unknown evidence source {id}"
            )));
        }
    }
    for id in assumptions {
        if !assumption_ids.contains(id) {
            return Err(EngineError::new(format!(
                "{owner} references unknown assumption {id}"
            )));
        }
    }
    Ok(())
}

pub(crate) fn enforce_limits(pack: &GamePack) -> Result<()> {
    let checks = [
        ("states", pack.state.len(), MAX_STATES),
        ("items", pack.items.len(), MAX_ITEMS),
        ("pools", pack.pools.len(), MAX_POOLS),
        ("distributions", pack.distributions.len(), MAX_DISTRIBUTIONS),
        ("selectors", pack.selectors.len(), MAX_SELECTORS),
        ("banners", pack.banners.len(), MAX_BANNERS),
    ];
    for (kind, count, limit) in checks {
        if count > limit {
            return Err(EngineError::new(format!(
                "pack contains {count} {kind}, exceeding limit {limit}"
            )));
        }
    }
    for pool in &pack.pools {
        if pool.items.len() > 20_000 {
            return Err(EngineError::new(format!(
                "pool {} contains too many items",
                pool.id
            )));
        }
    }
    for banner in &pack.banners {
        if banner.actions.len() > 256 {
            return Err(EngineError::new(format!(
                "banner {} contains too many actions",
                banner.id
            )));
        }
        for action in &banner.actions {
            if action.guarantees.len() > 256 || action.transitions.len() > 2048 {
                return Err(EngineError::new(format!(
                    "banner {} action {} exceeds rule-count limits",
                    banner.id, action.id
                )));
            }
        }
    }
    Ok(())
}

pub(crate) fn require_nonempty(name: &str, value: &str) -> Result<()> {
    if value.trim().is_empty() {
        return Err(EngineError::new(format!("{name} cannot be empty")));
    }
    Ok(())
}

pub(crate) fn unique_string_set<'a>(
    kind: &str,
    values: impl Iterator<Item = &'a str>,
) -> Result<BTreeSet<String>> {
    let mut result = BTreeSet::new();
    for value in values {
        require_nonempty(&format!("{kind} id"), value)?;
        if !result.insert(value.to_string()) {
            return Err(EngineError::new(format!("duplicate {kind} id: {value}")));
        }
    }
    Ok(result)
}

pub(crate) fn index_ids<'a, T: Copy>(
    kind: &str,
    values: impl Iterator<Item = &'a str>,
    make: impl Fn(usize) -> T,
) -> Result<BTreeMap<String, T>> {
    let mut result = BTreeMap::new();
    for (index, value) in values.enumerate() {
        require_nonempty(&format!("{kind} id"), value)?;
        if result.insert(value.to_string(), make(index)).is_some() {
            return Err(EngineError::new(format!("duplicate {kind} id: {value}")));
        }
    }
    Ok(result)
}
