use crate::*;
use gacha_core::PreparedRuntime;
use gacha_pack::{compile_pack, load_pack, load_pack_with_rules};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{fs, path::Path, sync::Arc};

#[derive(Clone)]
pub struct Engine {
    pub(crate) runtime: Arc<PreparedRuntime>,
    pub(crate) pack: Arc<GamePack>,
    pub(crate) fingerprint: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ItemInfo {
    pub id: String,
    pub name: String,
    pub rarity: u8,
    pub kind: String,
    pub native_rarity: String,
    pub upstream_id: String,
    pub names: std::collections::BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ActionInfo {
    pub banner: String,
    pub action: String,
    pub draws_per_action: u32,
    pub pity_group: String,
    pub progress_group: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct EngineInfo {
    pub id: String,
    pub version: String,
    pub fingerprint: String,
    pub states: usize,
    pub items: usize,
    pub pools: usize,
    pub distributions: usize,
    pub selectors: usize,
    pub banners: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct TransitionRecord {
    pub item: ItemInfo,
    pub probability: Rational,
    pub guarantees: Vec<String>,
    pub events: Vec<Event>,
    pub next_state: StateStore,
}

impl Engine {
    pub fn load(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let document: serde_json::Value = read_json(path)?;
        if document.get("catalogs").is_some() {
            Self::from_pack(load_pack(path)?)
        } else {
            Self::from_pack(serde_json::from_value(document).map_err(json_error)?)
        }
    }

    pub fn load_with_rules(path: impl AsRef<Path>, rules: impl AsRef<Path>) -> Result<Self> {
        Self::from_pack(load_pack_with_rules(path, rules)?)
    }

    pub fn from_pack(pack: GamePack) -> Result<Self> {
        let fingerprint = Sha256::digest(serde_json::to_vec(&pack).map_err(json_error)?)
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect();
        let runtime = Arc::new(PreparedRuntime::new(compile_pack(&pack)?)?);
        Ok(Self {
            runtime,
            pack: Arc::new(pack),
            fingerprint,
        })
    }

    pub fn fingerprint(&self) -> &str {
        &self.fingerprint
    }
    pub fn id(&self) -> &str {
        &self.pack.id
    }
    pub fn version(&self) -> &str {
        &self.pack.version
    }
    pub fn info(&self) -> EngineInfo {
        EngineInfo {
            id: self.id().into(),
            version: self.version().into(),
            fingerprint: self.fingerprint.clone(),
            states: self.pack.state.len(),
            items: self.pack.items.len(),
            pools: self.pack.pools.len(),
            distributions: self.pack.distributions.len(),
            selectors: self.pack.selectors.len(),
            banners: self.pack.banners.len(),
        }
    }
    pub fn transitions(
        &self,
        banner: &str,
        action: &str,
        state: &StateStore,
        context: &ScopeContext,
    ) -> Result<Vec<TransitionRecord>> {
        self.runtime
            .enumerate_step(banner, action, state, context)?
            .into_iter()
            .map(|branch| {
                let id = &self.runtime.game().item(branch.outcome.item).id;
                Ok(TransitionRecord {
                    item: self.item(id)?,
                    probability: branch.probability,
                    guarantees: branch.guarantees,
                    events: branch.events,
                    next_state: branch.next_state,
                })
            })
            .collect()
    }
    pub fn preparation(&self) -> PreparationStats {
        self.runtime.stats()
    }
    pub fn sources(&self) -> &[SourceDef] {
        &self.pack.sources
    }
    pub fn assumptions(&self) -> &[AssumptionDef] {
        &self.pack.assumptions
    }
    pub fn actions(&self) -> Vec<ActionInfo> {
        self.runtime
            .game()
            .banners
            .iter()
            .flat_map(|b| {
                b.actions.iter().map(move |a| ActionInfo {
                    banner: b.id.clone(),
                    action: a.id.clone(),
                    draws_per_action: a.draws,
                    pity_group: b.pity_group.clone(),
                    progress_group: b.progress_group.clone(),
                })
            })
            .collect()
    }

    pub fn item(&self, id: &str) -> Result<ItemInfo> {
        let game = self.runtime.game();
        let id = game
            .item_id(id)
            .ok_or_else(|| EngineError::new(format!("unknown item: {id}")))?;
        let item = game.item(id);
        Ok(ItemInfo {
            id: item.id.clone(),
            name: item.display_name.clone(),
            rarity: item.rarity,
            kind: item.kind.clone(),
            native_rarity: item.native_rarity.clone(),
            upstream_id: item.upstream_id.clone(),
            names: item.names.clone(),
        })
    }

    pub fn action_size(&self, banner: &str, action: &str) -> Result<u32> {
        let (_, b) = self
            .runtime
            .game()
            .banner(banner)
            .ok_or_else(|| EngineError::new(format!("unknown banner: {banner}")))?;
        Ok(b.action(action)
            .ok_or_else(|| EngineError::new(format!("unknown action: {action}")))?
            .draws)
    }

    /// 保存已组装规则，供独立分发；加载时仍进行语义编译与校验。
    pub fn save_bundle(&self, path: impl AsRef<Path>) -> Result<()> {
        fs::write(path, serde_json::to_vec(&*self.pack).map_err(json_error)?).map_err(io_error)
    }
    pub fn load_bundle(path: impl AsRef<Path>) -> Result<Self> {
        Self::from_pack(read_json(path.as_ref())?)
    }
}

pub(crate) fn json_error(error: serde_json::Error) -> EngineError {
    EngineError::new(error.to_string())
}
pub(crate) fn io_error(error: std::io::Error) -> EngineError {
    EngineError::new(error.to_string())
}
fn read_json<T: serde::de::DeserializeOwned>(path: &Path) -> Result<T> {
    let metadata = fs::metadata(path).map_err(io_error)?;
    if metadata.len() > 16 * 1024 * 1024 {
        return Err(EngineError::new("document exceeds 16 MiB"));
    }
    serde_json::from_slice(&fs::read(path).map_err(io_error)?).map_err(json_error)
}
