use super::*;

pub(super) fn compile_states(pack: &GamePack) -> Result<Vec<CompiledStateDef>> {
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
        if let (Some(max), schema::StateInitial::Counter(value)) = (raw.max, &raw.initial)
            && *value > max
        {
            return Err(EngineError::new(format!(
                "state {} initial value {} exceeds max {}",
                raw.id, value, max
            )));
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

pub(super) fn compile_items(pack: &GamePack) -> Result<Vec<CompiledItem>> {
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
            kind: raw.kind.as_str().to_string(),
            names: raw.names.clone(),
            native_rarity: raw.native_rarity.clone(),
            upstream_id: raw.upstream_id.clone(),
        });
    }
    Ok(result)
}

pub(super) fn compile_pools(
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
            if pack.items[id.0].gacha_eligible == Some(false) {
                return Err(EngineError::new(format!(
                    "pool {} includes non-gacha item {item}",
                    raw.id
                )));
            }
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
