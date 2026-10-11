use crate::io::read_document;
use crate::validation::{index_ids, require_nonempty, unique_string_set};
use gacha_core::{EngineError, Result};
use gacha_schema::{
    ContentCatalog, GamePack, GuaranteeEffectDef, PackManifest, RulePack, SCHEMA_VERSION,
};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

fn version(version: u32, kind: &str) -> Result<()> {
    if version != SCHEMA_VERSION {
        return Err(EngineError::new(format!(
            "unsupported {kind} schema_version: {version}"
        )));
    }
    Ok(())
}

pub fn load_catalog(path: impl AsRef<Path>) -> Result<ContentCatalog> {
    let path = path.as_ref();
    let mut catalog: ContentCatalog = serde_json::from_str(&read_document(path)?)
        .map_err(|error| EngineError::new(format!("invalid content catalog: {error}")))?;
    crate::resolver::resolve_sources(
        &mut catalog.sources,
        &catalog.source_documents,
        path.parent().unwrap_or_else(|| Path::new(".")),
        &mut |p| read_document(p),
    )?;
    validate_catalog(&catalog)?;
    Ok(catalog)
}

fn validate_catalog(catalog: &ContentCatalog) -> Result<()> {
    version(catalog.schema_version, "catalog")?;
    require_nonempty("catalog id", &catalog.id)?;
    require_nonempty("catalog game", &catalog.game)?;
    require_nonempty("catalog version", &catalog.version)?;
    if catalog.items.len() > 100_000 {
        return Err(EngineError::new("catalog item count exceeds limit"));
    }
    index_ids(
        "catalog item",
        catalog.items.iter().map(|i| i.id.as_str()),
        |i| i,
    )?;
    let sources = unique_string_set(
        "catalog source",
        catalog.sources.iter().map(|s| s.id.as_str()),
    )?;
    for item in &catalog.items {
        require_nonempty("item display_name", &item.display_name)?;
        if item.rarity == 0 {
            return Err(EngineError::new("catalog item rarity must be positive"));
        }
        if item.source_ids.is_empty() {
            return Err(EngineError::new(format!(
                "item {} lacks provenance",
                item.id
            )));
        }
        for source in &item.source_ids {
            if !sources.contains(source) {
                return Err(EngineError::new(format!(
                    "item {} references unknown content source {source}",
                    item.id
                )));
            }
        }
    }
    Ok(())
}

/// 从清单所在目录解析相对文件引用。
pub fn load_pack(path: impl AsRef<Path>) -> Result<GamePack> {
    crate::resolve_pack(path.as_ref(), None, |p| read_document(p))
}

/// 纯函数文档组装；编译阶段验证最终规则图。
pub fn compose_pack(
    manifest: &PackManifest,
    rules: &RulePack,
    catalogs: &[ContentCatalog],
) -> Result<GamePack> {
    version(manifest.schema_version, "manifest")?;
    version(rules.schema_version, "rules")?;
    require_nonempty("manifest id", &manifest.id)?;
    require_nonempty("manifest game", &manifest.game)?;
    require_nonempty("manifest version", &manifest.version)?;
    require_nonempty("rules path", &manifest.rules)?;
    if catalogs.is_empty() {
        return Err(EngineError::new("manifest must include content catalogs"));
    }
    let mut items = BTreeMap::new();
    let mut sources = BTreeMap::new();
    let mut catalog_ids = BTreeSet::new();
    for source in &rules.sources {
        if sources.insert(source.id.clone(), source.clone()).is_some() {
            return Err(EngineError::new(format!(
                "duplicate rule source {}",
                source.id
            )));
        }
    }
    for catalog in catalogs {
        validate_catalog(catalog)?;
        if catalog.game != manifest.game {
            return Err(EngineError::new(format!(
                "catalog {} belongs to {}, expected {}",
                catalog.id, catalog.game, manifest.game
            )));
        }
        if !catalog_ids.insert(&catalog.id) {
            return Err(EngineError::new(format!(
                "duplicate catalog {}",
                catalog.id
            )));
        }
        for source in &catalog.sources {
            if let Some(existing) = sources.get(&source.id) {
                if existing != source {
                    return Err(EngineError::new(format!(
                        "conflicting source {}",
                        source.id
                    )));
                }
            } else {
                sources.insert(source.id.clone(), source.clone());
            }
        }
        for item in &catalog.items {
            if items.insert(item.id.clone(), item.clone()).is_some() {
                return Err(EngineError::new(format!(
                    "duplicate content item {}",
                    item.id
                )));
            }
        }
        if items.len() > 100_000 {
            return Err(EngineError::new("composed item count exceeds limit"));
        }
    }
    for (role, id) in &manifest.item_bindings {
        require_nonempty("item binding role", role)?;
        if !items.contains_key(id) {
            return Err(EngineError::new(format!(
                "binding {role} references unknown item {id}"
            )));
        }
        let item = items.get_mut(id).expect("binding resolved above");
        if !item.tags.contains(role) {
            item.tags.push(role.clone());
        }
    }
    let mut banners = rules.banners.clone();
    for (template, binding) in &manifest.banner_bindings {
        let banner = banners
            .iter_mut()
            .find(|banner| &banner.id == template)
            .ok_or_else(|| EngineError::new(format!("unknown banner template {template}")))?;
        require_nonempty("banner binding id", &binding.id)?;
        require_nonempty("banner binding pity_group", &binding.pity_group)?;
        require_nonempty("banner binding progress_group", &binding.progress_group)?;
        banner.id = binding.id.clone();
        banner.pity_group = binding.pity_group.clone();
        banner.progress_group = binding.progress_group.clone();
    }
    for banner in &mut banners {
        for action in &mut banner.actions {
            for guarantee in &mut action.guarantees {
                if let GuaranteeEffectDef::ForceItem { item } = &mut guarantee.effect {
                    *item =
                        manifest.item_bindings.get(item).cloned().ok_or_else(|| {
                            EngineError::new(format!("missing item binding {item}"))
                        })?;
                }
            }
            for transition in &mut action.transitions {
                for event in &mut transition.events {
                    if let Some(id) = manifest.item_bindings.get(&event.key) {
                        event.key = id.clone();
                    }
                }
            }
        }
    }
    Ok(GamePack {
        schema_version: SCHEMA_VERSION,
        id: manifest.id.clone(),
        version: manifest.version.clone(),
        description: manifest.description.clone(),
        sources: sources.into_values().collect(),
        assumptions: rules.assumptions.clone(),
        state: rules.state.clone(),
        items: items.into_values().collect(),
        pools: manifest.pools.clone(),
        distributions: rules.distributions.clone(),
        selectors: rules.selectors.clone(),
        banners,
    })
}
