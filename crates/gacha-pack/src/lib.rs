use gacha_core::*;
use gacha_schema as schema;
use gacha_schema::{
    ConditionDef, GamePack, GuaranteeEffectDef, ProbabilityExpr as RawProbabilityExpr,
};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;
use std::str::FromStr;

const MAX_PACK_BYTES: u64 = 16 * 1024 * 1024;
const MAX_STATES: usize = 1024;
const MAX_ITEMS: usize = 100_000;
const MAX_POOLS: usize = 10_000;
const MAX_DISTRIBUTIONS: usize = 4096;
const MAX_SELECTORS: usize = 4096;
const MAX_BANNERS: usize = 10_000;
const MAX_SELECTOR_DEPTH: usize = 128;

pub fn load_pack(path: impl AsRef<Path>) -> Result<GamePack> {
    let path = path.as_ref();
    let metadata = fs::metadata(path).map_err(|error| {
        EngineError::new(format!("failed to stat pack {}: {error}", path.display()))
    })?;
    if metadata.len() > MAX_PACK_BYTES {
        return Err(EngineError::new(format!(
            "pack {} is {} bytes, exceeding the {} byte limit",
            path.display(),
            metadata.len(),
            MAX_PACK_BYTES
        )));
    }
    let text = fs::read_to_string(path).map_err(|error| {
        EngineError::new(format!("failed to read pack {}: {error}", path.display()))
    })?;
    parse_pack(&text)
}

pub fn parse_pack(text: &str) -> Result<GamePack> {
    if text.len() as u64 > MAX_PACK_BYTES {
        return Err(EngineError::new(format!(
            "pack text is {} bytes, exceeding the {} byte limit",
            text.len(),
            MAX_PACK_BYTES
        )));
    }
    serde_json::from_str(text)
        .map_err(|error| EngineError::new(format!("invalid pack JSON: {error}")))
}

pub fn validate_pack(pack: &GamePack) -> Result<()> {
    compile_pack(pack).map(|_| ())
}

pub fn compile_pack(pack: &GamePack) -> Result<CompiledGame> {
    if pack.schema_version != schema::SCHEMA_VERSION {
        return Err(EngineError::new(format!(
            "unsupported schema_version {}, expected {}",
            pack.schema_version,
            schema::SCHEMA_VERSION
        )));
    }
    require_nonempty("pack id", &pack.id)?;
    require_nonempty("pack version", &pack.version)?;
    enforce_limits(pack)?;

    let source_ids = unique_string_set("source", pack.sources.iter().map(|x| x.id.as_str()))?;
    let assumption_ids =
        unique_string_set("assumption", pack.assumptions.iter().map(|x| x.id.as_str()))?;

    let state_lookup = index_ids("state", pack.state.iter().map(|x| x.id.as_str()), StateId)?;
    let item_lookup = index_ids("item", pack.items.iter().map(|x| x.id.as_str()), ItemId)?;
    let pool_lookup = index_ids("pool", pack.pools.iter().map(|x| x.id.as_str()), PoolId)?;
    let distribution_lookup = index_ids(
        "distribution",
        pack.distributions.iter().map(|x| x.id.as_str()),
        DistributionId,
    )?;
    let selector_lookup = index_ids(
        "selector",
        pack.selectors.iter().map(|x| x.id.as_str()),
        SelectorId,
    )?;
    let banner_lookup = index_ids(
        "banner",
        pack.banners.iter().map(|x| x.id.as_str()),
        BannerId,
    )?;

    let states = compile_states(pack)?;
    let items = compile_items(pack)?;
    let pools = compile_pools(pack, &item_lookup)?;
    let distributions = compile_distributions(pack, &state_lookup, &source_ids, &assumption_ids)?;
    let selectors = compile_selectors(pack, &pool_lookup, &selector_lookup)?;
    validate_selector_cycles(&selectors)?;

    let mut game = CompiledGame {
        id: pack.id.clone(),
        version: pack.version.clone(),
        states,
        items,
        pools,
        distributions,
        selectors,
        banners: Vec::new(),
        state_lookup,
        item_lookup,
        pool_lookup,
        distribution_lookup,
        selector_lookup,
        banner_lookup,
    };

    game.banners = compile_banners(pack, &game, &source_ids, &assumption_ids)?;
    validate_action_selector_rarities(&game)?;
    validate_initial_distributions(&game)?;
    Ok(game)
}

fn compile_states(pack: &GamePack) -> Result<Vec<CompiledStateDef>> {
    let mut result = Vec::with_capacity(pack.state.len());
    for raw in &pack.state {
        require_nonempty("state id", &raw.id)?;
        let kind = match raw.kind {
            schema::StateKind::Counter => StateKind::Counter,
            schema::StateKind::Boolean => StateKind::Boolean,
        };
        let scope = match raw.scope {
            schema::StateScope::Account => StateScope::Account,
            schema::StateScope::Game => StateScope::Game,
            schema::StateScope::PityGroup => StateScope::PityGroup,
            schema::StateScope::ProgressGroup => StateScope::ProgressGroup,
            schema::StateScope::Banner => StateScope::Banner,
            schema::StateScope::Session => StateScope::Session,
            schema::StateScope::Batch => StateScope::Batch,
            schema::StateScope::Pull => StateScope::Pull,
        };
        let initial = match (&raw.kind, &raw.initial) {
            (schema::StateKind::Counter, schema::StateInitial::Counter(value)) => {
                StateValue::Counter(*value)
            }
            (schema::StateKind::Boolean, schema::StateInitial::Boolean(value)) => {
                StateValue::Boolean(*value)
            }
            _ => {
                return Err(EngineError::new(format!(
                    "state {} initial value does not match its kind",
                    raw.id
                )));
            }
        };
        if raw.kind == schema::StateKind::Boolean && raw.max.is_some() {
            return Err(EngineError::new(format!(
                "boolean state {} cannot declare max",
                raw.id
            )));
        }
        if let (Some(max), schema::StateInitial::Counter(value)) = (raw.max, &raw.initial) {
            if *value > max {
                return Err(EngineError::new(format!(
                    "state {} initial value {} exceeds max {}",
                    raw.id, value, max
                )));
            }
        }
        result.push(CompiledStateDef {
            id: raw.id.clone(),
            kind,
            scope,
            initial,
            max: raw.max,
        });
    }
    Ok(result)
}

fn compile_items(pack: &GamePack) -> Result<Vec<CompiledItem>> {
    let mut result = Vec::with_capacity(pack.items.len());
    for raw in &pack.items {
        require_nonempty("item id", &raw.id)?;
        if raw.rarity == 0 {
            return Err(EngineError::new(format!(
                "item {} has invalid rarity 0",
                raw.id
            )));
        }
        result.push(CompiledItem {
            id: raw.id.clone(),
            rarity: raw.rarity,
            tags: raw.tags.clone(),
            display_name: raw.display_name.clone(),
        });
    }
    Ok(result)
}

fn compile_pools(
    pack: &GamePack,
    item_lookup: &BTreeMap<String, ItemId>,
) -> Result<Vec<CompiledPool>> {
    let mut result = Vec::with_capacity(pack.pools.len());
    for raw in &pack.pools {
        require_nonempty("pool id", &raw.id)?;
        if raw.items.is_empty() {
            return Err(EngineError::new(format!("pool {} is empty", raw.id)));
        }
        let mut seen = BTreeSet::new();
        let mut items = Vec::with_capacity(raw.items.len());
        for item in &raw.items {
            let id = *item_lookup.get(item).ok_or_else(|| {
                EngineError::new(format!("pool {} references unknown item {item}", raw.id))
            })?;
            if !seen.insert(id) {
                return Err(EngineError::new(format!(
                    "pool {} contains duplicate item {item}",
                    raw.id
                )));
            }
            items.push(id);
        }
        result.push(CompiledPool {
            id: raw.id.clone(),
            items,
        });
    }
    Ok(result)
}

fn compile_distributions(
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

fn compile_selectors(
    pack: &GamePack,
    pool_lookup: &BTreeMap<String, PoolId>,
    selector_lookup: &BTreeMap<String, SelectorId>,
) -> Result<Vec<CompiledSelector>> {
    let mut result = Vec::with_capacity(pack.selectors.len());
    for raw in &pack.selectors {
        let expr = match &raw.selector {
            schema::SelectorExpr::Pool { pool } => {
                let pool = *pool_lookup.get(pool).ok_or_else(|| {
                    EngineError::new(format!(
                        "selector {} references unknown pool {pool}",
                        raw.id
                    ))
                })?;
                SelectorExpr::Pool(pool)
            }
            schema::SelectorExpr::Weighted { branches } => {
                if branches.is_empty() {
                    return Err(EngineError::new(format!(
                        "selector {} has no weighted branches",
                        raw.id
                    )));
                }
                let mut compiled = Vec::with_capacity(branches.len());
                for branch in branches {
                    let weight = Rational::from_str(&branch.weight).map_err(|error| {
                        EngineError::new(format!(
                            "selector {} has invalid weight {}: {error}",
                            raw.id, branch.weight
                        ))
                    })?;
                    if weight.is_zero() {
                        return Err(EngineError::new(format!(
                            "selector {} has zero branch weight",
                            raw.id
                        )));
                    }
                    let selector = *selector_lookup.get(&branch.selector).ok_or_else(|| {
                        EngineError::new(format!(
                            "selector {} references unknown selector {}",
                            raw.id, branch.selector
                        ))
                    })?;
                    compiled.push(WeightedSelectorBranch { weight, selector });
                }
                SelectorExpr::Weighted(compiled)
            }
        };
        result.push(CompiledSelector {
            id: raw.id.clone(),
            expr,
        });
    }
    Ok(result)
}

fn compile_banners(
    pack: &GamePack,
    game: &CompiledGame,
    source_ids: &BTreeSet<String>,
    assumption_ids: &BTreeSet<String>,
) -> Result<Vec<CompiledBanner>> {
    let mut banners = Vec::with_capacity(pack.banners.len());
    for raw in &pack.banners {
        require_nonempty("banner id", &raw.id)?;
        require_nonempty("pity_group", &raw.pity_group)?;
        require_nonempty("progress_group", &raw.progress_group)?;
        let mut action_lookup = BTreeMap::new();
        let mut actions = Vec::with_capacity(raw.actions.len());
        for (action_index, action) in raw.actions.iter().enumerate() {
            require_nonempty("action id", &action.id)?;
            if action_lookup
                .insert(action.id.clone(), action_index)
                .is_some()
            {
                return Err(EngineError::new(format!(
                    "banner {} contains duplicate action {}",
                    raw.id, action.id
                )));
            }
            let distribution = *game
                .distribution_lookup
                .get(&action.distribution)
                .ok_or_else(|| {
                    EngineError::new(format!(
                        "banner {} action {} references unknown distribution {}",
                        raw.id, action.id, action.distribution
                    ))
                })?;
            let mut selectors = BTreeMap::new();
            for (rarity, selector) in &action.selectors {
                let rarity = rarity.parse::<u8>().map_err(|_| {
                    EngineError::new(format!(
                        "banner {} action {} selector key is not u8: {rarity}",
                        raw.id, action.id
                    ))
                })?;
                let selector = *game.selector_lookup.get(selector).ok_or_else(|| {
                    EngineError::new(format!(
                        "banner {} action {} references unknown selector {selector}",
                        raw.id, action.id
                    ))
                })?;
                selectors.insert(rarity, selector);
            }

            let mut guarantee_ids = BTreeSet::new();
            let mut guarantees = Vec::with_capacity(action.guarantees.len());
            for guarantee in &action.guarantees {
                if !guarantee_ids.insert(guarantee.id.clone()) {
                    return Err(EngineError::new(format!(
                        "banner {} action {} contains duplicate guarantee {}",
                        raw.id, action.id, guarantee.id
                    )));
                }
                validate_evidence_refs(
                    &format!("guarantee {}", guarantee.id),
                    &guarantee.evidence,
                    &guarantee.assumptions,
                    source_ids,
                    assumption_ids,
                )?;
                validate_pre_draw_condition(&guarantee.when)?;
                let when = compile_condition(pack, game, &guarantee.when)?;
                let effect = match &guarantee.effect {
                    GuaranteeEffectDef::ForceItem { item } => GuaranteeEffect::ForceItem(
                        *game.item_lookup.get(item).ok_or_else(|| {
                            EngineError::new(format!(
                                "guarantee {} references unknown item {item}",
                                guarantee.id
                            ))
                        })?,
                    ),
                    GuaranteeEffectDef::ForceRarity { rarity } => {
                        GuaranteeEffect::ForceRarity(*rarity)
                    }
                    GuaranteeEffectDef::MinRarity { rarity, strategy } => {
                        GuaranteeEffect::MinRarity {
                            rarity: *rarity,
                            strategy: match strategy {
                                schema::MinRarityStrategy::ConditionalCurrent => {
                                    MinRarityStrategy::ConditionalCurrent
                                }
                                schema::MinRarityStrategy::PreserveHigherFillFloor => {
                                    MinRarityStrategy::PreserveHigherFillFloor
                                }
                            },
                        }
                    }
                };
                guarantees.push(CompiledGuarantee {
                    id: guarantee.id.clone(),
                    priority: guarantee.priority,
                    when,
                    effect,
                });
            }
            guarantees.sort_by_key(|b| std::cmp::Reverse(b.priority));
            for guarantee in &guarantees {
                let required_rarity = match &guarantee.effect {
                    GuaranteeEffect::ForceRarity(rarity) => Some(*rarity),
                    GuaranteeEffect::MinRarity { rarity, .. } => Some(*rarity),
                    GuaranteeEffect::ForceItem(_) => None,
                };
                if let Some(rarity) = required_rarity {
                    if !selectors.contains_key(&rarity) {
                        return Err(EngineError::new(format!(
                            "guarantee {} requires rarity {} but action {} has no selector for it",
                            guarantee.id, rarity, action.id
                        )));
                    }
                }
            }

            let mut transition_ids = BTreeSet::new();
            let mut transitions = Vec::with_capacity(action.transitions.len());
            for transition in &action.transitions {
                if !transition_ids.insert(transition.id.clone()) {
                    return Err(EngineError::new(format!(
                        "banner {} action {} contains duplicate transition {}",
                        raw.id, action.id, transition.id
                    )));
                }
                let when = compile_condition(pack, game, &transition.when)?;
                let mut updates = Vec::with_capacity(transition.updates.len());
                for update in &transition.updates {
                    updates.push(compile_update(pack, game, update)?);
                }
                let events = transition
                    .events
                    .iter()
                    .map(|event| EventTemplate {
                        kind: event.kind.clone(),
                        key: event.key.clone(),
                        amount: event.amount,
                    })
                    .collect();
                transitions.push(CompiledTransitionRule {
                    id: transition.id.clone(),
                    when,
                    updates,
                    events,
                });
            }

            actions.push(CompiledAction {
                id: action.id.clone(),
                distribution,
                selectors,
                guarantees,
                transitions,
            });
        }
        if actions.is_empty() {
            return Err(EngineError::new(format!(
                "banner {} has no actions",
                raw.id
            )));
        }
        banners.push(CompiledBanner {
            id: raw.id.clone(),
            pity_group: raw.pity_group.clone(),
            progress_group: raw.progress_group.clone(),
            actions,
            action_lookup,
        });
    }
    Ok(banners)
}

fn validate_pre_draw_condition(condition: &ConditionDef) -> Result<()> {
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

fn compile_condition(
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

fn compile_update(
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
            if let Some(max) = pack.state[id.0].max {
                if *value > max {
                    return Err(EngineError::new(format!(
                        "set_counter for state {} sets {} above max {}",
                        state, value, max
                    )));
                }
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

fn validate_selector_cycles(selectors: &[CompiledSelector]) -> Result<()> {
    fn visit(
        id: SelectorId,
        selectors: &[CompiledSelector],
        temporary: &mut BTreeSet<SelectorId>,
        permanent: &mut BTreeSet<SelectorId>,
        depth: usize,
    ) -> Result<()> {
        if depth > MAX_SELECTOR_DEPTH {
            return Err(EngineError::new(format!(
                "selector graph exceeds maximum depth {} at {}",
                MAX_SELECTOR_DEPTH, selectors[id.0].id
            )));
        }
        if permanent.contains(&id) {
            return Ok(());
        }
        if !temporary.insert(id) {
            return Err(EngineError::new(format!(
                "selector cycle detected at {}",
                selectors[id.0].id
            )));
        }
        if let SelectorExpr::Weighted(branches) = &selectors[id.0].expr {
            for branch in branches {
                visit(branch.selector, selectors, temporary, permanent, depth + 1)?;
            }
        }
        temporary.remove(&id);
        permanent.insert(id);
        Ok(())
    }

    let mut temporary = BTreeSet::new();
    let mut permanent = BTreeSet::new();
    for index in 0..selectors.len() {
        visit(
            SelectorId(index),
            selectors,
            &mut temporary,
            &mut permanent,
            0,
        )?;
    }
    Ok(())
}

fn validate_action_selector_rarities(game: &CompiledGame) -> Result<()> {
    fn collect_items(game: &CompiledGame, selector: SelectorId, output: &mut BTreeSet<ItemId>) {
        match &game.selectors[selector.0].expr {
            SelectorExpr::Pool(pool) => {
                output.extend(game.pools[pool.0].items.iter().copied());
            }
            SelectorExpr::Weighted(branches) => {
                for branch in branches {
                    collect_items(game, branch.selector, output);
                }
            }
        }
    }

    for banner in &game.banners {
        for action in &banner.actions {
            let distribution = &game.distributions[action.distribution.0];
            for entry in &distribution.entries {
                let selector = action.selectors.get(&entry.rarity).ok_or_else(|| {
                    EngineError::new(format!(
                        "banner {} action {} has distribution rarity {} but no selector",
                        banner.id, action.id, entry.rarity
                    ))
                })?;
                let mut items = BTreeSet::new();
                collect_items(game, *selector, &mut items);
                if items.is_empty() {
                    return Err(EngineError::new(format!(
                        "selector {} resolves to no items",
                        game.selectors[selector.0].id
                    )));
                }
                for item in items {
                    if game.items[item.0].rarity != entry.rarity {
                        return Err(EngineError::new(format!(
                            "selector {} used for rarity {} can produce item {} of rarity {}",
                            game.selectors[selector.0].id,
                            entry.rarity,
                            game.items[item.0].id,
                            game.items[item.0].rarity
                        )));
                    }
                }
            }
        }
    }
    Ok(())
}

fn validate_initial_distributions(game: &CompiledGame) -> Result<()> {
    for banner in &game.banners {
        let mut state = StateStore::default();
        materialize_state(game, banner, &mut state, &ScopeContext::default())?;
        for action in &banner.actions {
            let branches = enumerate_transitions(
                game,
                &banner.id,
                &action.id,
                &state,
                &ScopeContext::default(),
            )?;
            if branches.is_empty() {
                return Err(EngineError::new(format!(
                    "banner {} action {} produces no initial branches",
                    banner.id, action.id
                )));
            }
        }
    }
    Ok(())
}

fn require_counter_state(
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

fn require_boolean_state(
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

fn parse_probability(value: &str, context: &str) -> Result<Rational> {
    let probability = Rational::from_str(value)
        .map_err(|error| EngineError::new(format!("invalid {context} {value}: {error}")))?;
    ensure_unit_interval(probability, context)?;
    Ok(probability)
}

fn ensure_unit_interval(value: Rational, context: &str) -> Result<()> {
    if value > Rational::ONE {
        return Err(EngineError::new(format!("{context} must be in [0, 1]")));
    }
    Ok(())
}

fn validate_evidence_refs(
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

fn enforce_limits(pack: &GamePack) -> Result<()> {
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

fn require_nonempty(name: &str, value: &str) -> Result<()> {
    if value.trim().is_empty() {
        return Err(EngineError::new(format!("{name} cannot be empty")));
    }
    Ok(())
}

fn unique_string_set<'a>(
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

fn index_ids<'a, T: Copy>(
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
