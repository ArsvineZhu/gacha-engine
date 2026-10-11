use crate::validation::*;
use gacha_core::*;
use gacha_schema as schema;
use gacha_schema::{
    ConditionDef, GamePack, GuaranteeEffectDef, ProbabilityExpr as RawProbabilityExpr,
};
use std::collections::{BTreeMap, BTreeSet};
use std::str::FromStr;
mod entities;
use entities::*;
mod distributions;
use distributions::*;
mod selectors;
use selectors::*;
mod banners;
use banners::*;
mod conditions;
use conditions::*;

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
    let mut game = CompiledGame {
        id: pack.id.clone(),
        version: pack.version.clone(),
        states,
        items,
        pools,
        distributions,
        selectors: Vec::new(),
        banners: Vec::new(),
        state_lookup,
        item_lookup,
        pool_lookup,
        distribution_lookup,
        selector_lookup,
        banner_lookup,
    };

    game.selectors = compile_selectors(pack, &game)?;
    validate_selector_cycles(&game.selectors)?;
    game.banners = compile_banners(pack, &game, &source_ids, &assumption_ids)?;
    validate_action_selector_rarities(&game)?;
    validate_initial_distributions(&game)?;
    Ok(game)
}
