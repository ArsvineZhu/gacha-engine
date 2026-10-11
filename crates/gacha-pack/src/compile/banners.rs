use super::*;

pub(super) fn compile_banners(
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
            if !(1..=1000).contains(&action.draws) {
                return Err(EngineError::new("action draws must be in 1..=1000"));
            }
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
                    GuaranteeEffectDef::ForceItem { item } => {
                        let id = *game.item_lookup.get(item).ok_or_else(|| {
                            EngineError::new(format!(
                                "guarantee {} references unknown item {item}",
                                guarantee.id
                            ))
                        })?;
                        if pack.items[id.0].gacha_eligible == Some(false) {
                            return Err(EngineError::new(format!(
                                "guarantee {} forces non-gacha item {item}",
                                guarantee.id
                            )));
                        }
                        GuaranteeEffect::ForceItem(id)
                    }
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
                if let Some(rarity) = required_rarity
                    && !selectors.contains_key(&rarity)
                {
                    return Err(EngineError::new(format!(
                        "guarantee {} requires rarity {} but action {} has no selector for it",
                        guarantee.id, rarity, action.id
                    )));
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
                draws: action.draws,
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
