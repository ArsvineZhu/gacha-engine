use super::*;

pub(super) fn compile_distributions(
    pack: &GamePack,
    state_lookup: &BTreeMap<String, StateId>,
    source_ids: &BTreeSet<String>,
    assumption_ids: &BTreeSet<String>,
) -> Result<Vec<CompiledDistribution>> {
    let mut result = Vec::with_capacity(pack.distributions.len());
    for raw in &pack.distributions {
        validate_evidence_refs(
            &format!("distribution {}", raw.id),
            &raw.evidence,
            &raw.assumptions,
            source_ids,
            assumption_ids,
        )?;
        if raw.entries.is_empty() {
            return Err(EngineError::new(format!(
                "distribution {} has no entries",
                raw.id
            )));
        }
        let mut seen_rarities = BTreeSet::new();
        let mut entries = Vec::with_capacity(raw.entries.len());
        let mut direct_remainders = 0_u32;
        for entry in &raw.entries {
            if !seen_rarities.insert(entry.rarity) {
                return Err(EngineError::new(format!(
                    "distribution {} contains duplicate rarity {}",
                    raw.id, entry.rarity
                )));
            }
            let probability = match &entry.probability {
                RawProbabilityExpr::Constant { value } => {
                    ProbabilityExpr::Constant(parse_probability(value, "constant probability")?)
                }
                RawProbabilityExpr::LinearAfter {
                    state,
                    base,
                    after,
                    increment,
                    cap,
                } => {
                    let state = require_counter_state(pack, state_lookup, state, "linear_after")?;
                    let base = parse_probability(base, "linear_after base")?;
                    let increment = parse_probability(increment, "linear_after increment")?;
                    let cap = match cap {
                        Some(value) => parse_probability(value, "linear_after cap")?,
                        None => Rational::ONE,
                    };
                    ensure_unit_interval(base, "linear_after base")?;
                    ensure_unit_interval(cap, "linear_after cap")?;
                    ProbabilityExpr::LinearAfter {
                        state,
                        base,
                        after: *after,
                        increment,
                        cap,
                    }
                }
                RawProbabilityExpr::Table {
                    state,
                    values,
                    default,
                } => {
                    let state = require_counter_state(pack, state_lookup, state, "table")?;
                    let mut compiled_values = BTreeMap::new();
                    for (key, value) in values {
                        let key = key.parse::<u64>().map_err(|_| {
                            EngineError::new(format!("table state key must be u64: {key}"))
                        })?;
                        let value = parse_probability(value, "table probability")?;
                        ensure_unit_interval(value, "table probability")?;
                        compiled_values.insert(key, value);
                    }
                    let default = match default {
                        Some(value) => {
                            let value = parse_probability(value, "table default")?;
                            ensure_unit_interval(value, "table default")?;
                            Some(value)
                        }
                        None => None,
                    };
                    ProbabilityExpr::Table {
                        state,
                        values: compiled_values,
                        default,
                    }
                }
                RawProbabilityExpr::Remainder => {
                    direct_remainders += 1;
                    ProbabilityExpr::Remainder
                }
                RawProbabilityExpr::ShareOfRemainder { weight } => {
                    if *weight == 0 {
                        return Err(EngineError::new(format!(
                            "distribution {} has zero share_of_remainder weight",
                            raw.id
                        )));
                    }
                    ProbabilityExpr::ShareOfRemainder { weight: *weight }
                }
            };
            entries.push(CompiledDistributionEntry {
                rarity: entry.rarity,
                probability,
            });
        }
        if direct_remainders > 1 {
            return Err(EngineError::new(format!(
                "distribution {} has multiple remainder entries",
                raw.id
            )));
        }
        result.push(CompiledDistribution {
            id: raw.id.clone(),
            entries,
        });
    }
    Ok(result)
}
